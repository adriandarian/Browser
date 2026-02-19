#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
GOLDEN_CASES_DIR = REPO_ROOT / "tests" / "golden" / "cases"
GOLDEN_DIFF_DIR = REPO_ROOT / "tests" / "golden" / "diffs"


def host_target(os_name: str | None) -> str:
    if os_name:
        if os_name == "macos":
            machine = platform.machine().lower()
            return "aarch64-apple-darwin" if "arm" in machine else "x86_64-apple-darwin"
        if os_name == "windows":
            return "x86_64-pc-windows-msvc"
        if os_name == "linux":
            return "x86_64-unknown-linux-gnu"
        raise ValueError(f"unsupported --os value: {os_name}")

    system = platform.system().lower()
    if system == "darwin":
        machine = platform.machine().lower()
        return "aarch64-apple-darwin" if "arm" in machine else "x86_64-apple-darwin"
    if system == "windows":
        return "x86_64-pc-windows-msvc"
    if system == "linux":
        return "x86_64-unknown-linux-gnu"
    raise ValueError("Host OS is unsupported for this runner; pass --os explicitly for cross-build.")


def cargo_run_cmd(target: str, release: bool) -> list[str]:
    cmd = ["cargo", "run", "-p", "tessera", "--target", target]
    if release:
        cmd.append("--release")
    cmd.append("--")
    return cmd


def do_run(args: argparse.Namespace) -> int:
    try:
        target = host_target(args.os)
    except ValueError as exc:
        print(f"[run.py] {exc}")
        return 2

    cmd = cargo_run_cmd(target, args.release)
    cmd.extend(["--pattern", args.pattern])
    print("[run.py] executing:", " ".join(cmd))
    result = subprocess.run(cmd, cwd=REPO_ROOT, env=os.environ.copy())
    return result.returncode


def sha256_of_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def render_case_to_temp(case: dict[str, object], target: str, release: bool, temp_dir: Path) -> tuple[int, Path, str, int, int, int]:
    name = str(case["name"])
    pattern = str(case.get("pattern", "test-pattern"))
    width = int(case["width"])
    height = int(case["height"])
    frame_index = int(case["frame_index"])
    out_rgba = temp_dir / f"{name}.rgba"

    cmd = cargo_run_cmd(target, release)
    cmd.extend(
        [
            "--headless-output",
            str(out_rgba),
            "--pattern",
            pattern,
            "--width",
            str(width),
            "--height",
            str(height),
            "--frame-index",
            str(frame_index),
        ]
    )
    result = subprocess.run(cmd, cwd=REPO_ROOT, env=os.environ.copy(), capture_output=True, text=True)
    if result.returncode != 0:
        print(result.stdout)
        print(result.stderr)
    return result.returncode, out_rgba, pattern, width, height, frame_index


def run_headless_case(case: dict[str, object], target: str, release: bool) -> tuple[bool, str | None]:
    name = str(case["name"])
    expected_hash = str(case["sha256"])

    with tempfile.TemporaryDirectory(prefix=f"golden-{name}-") as td:
        code, out_rgba, pattern, width, height, frame_index = render_case_to_temp(case, target, release, Path(td))
        if code != 0:
            print(f"FAIL {name} (renderer exited {code})")
            return False, None

        actual_hash = sha256_of_file(out_rgba)

        if actual_hash == expected_hash:
            print(f"PASS {name}")
            return True, None

        GOLDEN_DIFF_DIR.mkdir(parents=True, exist_ok=True)
        diff_rgba = GOLDEN_DIFF_DIR / f"{name}.actual.rgba"
        diff_report = GOLDEN_DIFF_DIR / f"{name}.diff.json"
        shutil.copy2(out_rgba, diff_rgba)
        diff_report.write_text(
            json.dumps(
                {
                    "name": name,
                    "expected_sha256": expected_hash,
                    "actual_sha256": actual_hash,
                    "pattern": pattern,
                    "width": width,
                    "height": height,
                    "frame_index": frame_index,
                    "actual_rgba": str(diff_rgba.relative_to(REPO_ROOT)),
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        diff_rel = str(diff_report.relative_to(REPO_ROOT))
        print(f"FAIL {name} diff={diff_rel}")
        return False, diff_rel


def do_test(args: argparse.Namespace) -> int:
    try:
        target = host_target(args.os)
    except ValueError as exc:
        print(f"[run.py] {exc}")
        return 2

    case_paths = sorted(GOLDEN_CASES_DIR.glob("*.json"))
    if not case_paths:
        print(f"[run.py] no golden cases found in {GOLDEN_CASES_DIR}")
        return 2

    failures = 0
    updated = 0
    executed = 0
    diff_paths: list[str] = []

    for case_path in case_paths:
        case = json.loads(case_path.read_text(encoding="utf-8"))
        if args.update:
            with tempfile.TemporaryDirectory(prefix=f"golden-update-{case['name']}-") as td:
                code, out_rgba, *_ = render_case_to_temp(case, target, args.release, Path(td))
                if code != 0:
                    print(f"FAIL {case['name']} (renderer exited {code})")
                    failures += 1
                    executed += 1
                    continue
                case["sha256"] = sha256_of_file(out_rgba)
                case_path.write_text(json.dumps(case, indent=2) + "\n", encoding="utf-8")
                print(f"UPDATED {case['name']} => {case['sha256']}")
                updated += 1
                executed += 1
            continue

        expected_hash = str(case.get("sha256", "")).strip()
        if not expected_hash:
            print(f"FAIL {case.get('name', case_path.stem)} (missing sha256, run with --update)")
            failures += 1
            executed += 1
            continue

        ok, diff_path = run_headless_case(case, target, args.release)
        if not ok:
            failures += 1
            if diff_path:
                diff_paths.append(diff_path)
        executed += 1

    print("\nGolden report")
    print(f"- executed: {executed}")
    if args.update:
        print(f"- updated: {updated}")
    print(f"- failed: {failures}")
    for diff_path in diff_paths:
        print(f"- diff: {diff_path}")

    if failures:
        print("Golden tests: FAIL")
        return 1

    print("Golden tests: PASS")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Python tooling for tessera")
    subparsers = parser.add_subparsers(dest="command", required=True)

    run_parser = subparsers.add_parser("run", help="Build and run tessera")
    run_parser.add_argument("--release", action="store_true", help="Build in release mode")
    run_parser.add_argument("--os", choices=["macos", "windows", "linux"], help="Target OS triple family")
    run_parser.add_argument("--pattern", default="test-pattern", help="Renderer pattern name")
    run_parser.set_defaults(func=do_run)

    test_parser = subparsers.add_parser("test", help="Run golden renderer tests")
    test_parser.add_argument("--release", action="store_true", help="Build in release mode")
    test_parser.add_argument("--os", choices=["macos", "windows", "linux"], help="Target OS triple family")
    test_parser.add_argument("--update", action="store_true", help="Refresh stored golden hashes")
    test_parser.set_defaults(func=do_test)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
