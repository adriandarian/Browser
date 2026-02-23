#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_DIR = REPO_ROOT / "tests" / "fixtures"
GOLDEN_ROOT = REPO_ROOT / "tests" / "golden"
EXPECTED_DIR = GOLDEN_ROOT / "expected"
ACTUAL_DIR = GOLDEN_ROOT / "actual"
DIFF_DIR = GOLDEN_ROOT / "diff"


def run_command(cmd: list[str]) -> int:
    print("[run.py]", " ".join(cmd))
    result = subprocess.run(cmd, cwd=REPO_ROOT, env=os.environ.copy())
    return result.returncode


def cargo_run_browser(args: list[str], release: bool = False) -> int:
    cmd = ["cargo", "run", "-p", "browser"]
    if release:
        cmd.append("--release")
    cmd.append("--")
    cmd.extend(args)
    return run_command(cmd)


def cmd_run(parsed: argparse.Namespace) -> int:
    app_args = ["run"]
    if parsed.pattern:
        app_args.extend(["--pattern", parsed.pattern])
    return cargo_run_browser(app_args, release=parsed.release)


def sha256_hex(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            block = handle.read(1024 * 1024)
            if not block:
                break
            digest.update(block)
    return digest.hexdigest()


def ensure_golden_dirs() -> None:
    EXPECTED_DIR.mkdir(parents=True, exist_ok=True)
    ACTUAL_DIR.mkdir(parents=True, exist_ok=True)
    DIFF_DIR.mkdir(parents=True, exist_ok=True)


def collect_fixtures() -> list[Path]:
    fixtures = sorted(FIXTURE_DIR.glob("*.html"))
    if not fixtures:
        raise RuntimeError(f"no fixtures found in {FIXTURE_DIR}")
    return fixtures


def render_fixture(
    fixture: Path,
    out_rgba: Path,
    out_meta: Path,
    *,
    release: bool,
    width: int,
    height: int,
    frame: int,
) -> int:
    return cargo_run_browser(
        [
            "headless",
            "--input",
            str(fixture),
            "--width",
            str(width),
            "--height",
            str(height),
            "--frame",
            str(frame),
            "--out-rgba",
            str(out_rgba),
            "--out-meta",
            str(out_meta),
        ],
        release=release,
    )


def first_diff(expected: bytes, actual: bytes) -> int:
    limit = min(len(expected), len(actual))
    for idx in range(limit):
        if expected[idx] != actual[idx]:
            return idx
    if len(expected) != len(actual):
        return limit
    return -1


def load_meta(path: Path) -> dict[str, int | str]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def write_diff_report(
    *,
    fixture_name: str,
    expected_rgba: Path,
    actual_rgba: Path,
    expected_meta: Path,
    actual_meta: Path,
    expected_hash: str,
    actual_hash: str,
    diff_path: Path,
) -> None:
    expected_bytes = expected_rgba.read_bytes()
    actual_bytes = actual_rgba.read_bytes()
    expected_meta_obj = load_meta(expected_meta)
    actual_meta_obj = load_meta(actual_meta)
    mismatch_index = first_diff(expected_bytes, actual_bytes)

    lines = [
        f"fixture: {fixture_name}",
        f"expected_hash: {expected_hash}",
        f"actual_hash:   {actual_hash}",
        f"expected_rgba: {expected_rgba}",
        f"actual_rgba:   {actual_rgba}",
        f"expected_meta: {expected_meta}",
        f"actual_meta:   {actual_meta}",
    ]

    if expected_meta_obj != actual_meta_obj:
        lines.append("meta_mismatch: true")

    if mismatch_index >= 0:
        width = int(actual_meta_obj.get("width", 0) or 0)
        pixel_index = mismatch_index // 4
        channel = mismatch_index % 4
        if width > 0:
            x = pixel_index % width
            y = pixel_index // width
            lines.append(
                f"first_pixel_diff: byte={mismatch_index} pixel={pixel_index} x={x} y={y} channel={channel}"
            )
        else:
            lines.append(f"first_pixel_diff: byte={mismatch_index} channel={channel}")
    else:
        lines.append("first_pixel_diff: none")

    diff_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def update_expected(actual_rgba: Path, actual_meta: Path, expected_rgba: Path, expected_meta: Path) -> str:
    shutil.copyfile(actual_rgba, expected_rgba)
    shutil.copyfile(actual_meta, expected_meta)
    digest = sha256_hex(expected_rgba)
    expected_rgba.with_suffix(".sha256").write_text(f"{digest}\n", encoding="utf-8")
    return digest


def run_golden_checks(parsed: argparse.Namespace) -> int:
    ensure_golden_dirs()
    fixtures = collect_fixtures()

    passed = 0
    failed = 0

    for fixture in fixtures:
        name = fixture.stem
        actual_rgba = ACTUAL_DIR / f"{name}.rgba"
        actual_meta = ACTUAL_DIR / f"{name}.json"
        expected_rgba = EXPECTED_DIR / f"{name}.rgba"
        expected_meta = EXPECTED_DIR / f"{name}.json"
        expected_hash_path = EXPECTED_DIR / f"{name}.sha256"
        diff_path = DIFF_DIR / f"{name}.diff.txt"

        status = render_fixture(
            fixture,
            actual_rgba,
            actual_meta,
            release=parsed.release,
            width=parsed.width,
            height=parsed.height,
            frame=parsed.frame,
        )
        if status != 0:
            print(f"FAIL {name} (renderer exited with status {status})")
            failed += 1
            continue

        if parsed.update or not expected_rgba.exists() or not expected_meta.exists():
            digest = update_expected(actual_rgba, actual_meta, expected_rgba, expected_meta)
            print(f"UPDATED {name} ({digest[:12]})")
            passed += 1
            continue

        actual_hash = sha256_hex(actual_rgba)
        if expected_hash_path.exists():
            expected_hash = expected_hash_path.read_text(encoding="utf-8").strip()
        else:
            expected_hash = sha256_hex(expected_rgba)
            expected_hash_path.write_text(f"{expected_hash}\n", encoding="utf-8")

        meta_matches = expected_meta.read_text(encoding="utf-8") == actual_meta.read_text(encoding="utf-8")
        if actual_hash == expected_hash and meta_matches:
            if diff_path.exists():
                diff_path.unlink()
            print(f"PASS {name}")
            passed += 1
            continue

        write_diff_report(
            fixture_name=name,
            expected_rgba=expected_rgba,
            actual_rgba=actual_rgba,
            expected_meta=expected_meta,
            actual_meta=actual_meta,
            expected_hash=expected_hash,
            actual_hash=actual_hash,
            diff_path=diff_path,
        )
        print(f"FAIL {name} diff={diff_path}")
        failed += 1

    print(f"golden summary: pass={passed} fail={failed}")
    return 0 if failed == 0 else 1


def cmd_test(parsed: argparse.Namespace) -> int:
    status = run_command(["cargo", "test", "--workspace"])
    if status != 0:
        return status
    try:
        return run_golden_checks(parsed)
    except RuntimeError as exc:
        print(f"FAIL {exc}")
        return 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Build/run and golden-test browser")
    subparsers = parser.add_subparsers(dest="command", required=False)

    run_parser = subparsers.add_parser("run", help="Build and run the app")
    run_parser.add_argument("--release", action="store_true", help="Build in release mode")
    run_parser.add_argument(
        "--pattern",
        choices=["gradient", "solid", "rects"],
        help="Renderer pattern in windowed mode",
    )
    run_parser.set_defaults(handler=cmd_run)

    test_parser = subparsers.add_parser("test", help="Run unit tests and golden tests")
    test_parser.add_argument("--release", action="store_true", help="Build in release mode")
    test_parser.add_argument("--update", action="store_true", help="Refresh expected goldens")
    test_parser.add_argument("--width", type=int, default=960, help="Frame width")
    test_parser.add_argument("--height", type=int, default=540, help="Frame height")
    test_parser.add_argument("--frame", type=int, default=0, help="Frame index")
    test_parser.set_defaults(handler=cmd_test)

    return parser


def main() -> int:
    parser = build_parser()
    parsed = parser.parse_args(["run"] if len(sys.argv) == 1 else None)
    handler = getattr(parsed, "handler", None)
    if handler is None:
        parser.print_help()
        return 2
    return handler(parsed)


if __name__ == "__main__":
    sys.exit(main())
