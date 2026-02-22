#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import platform
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def host_target(os_name: str | None) -> str:
    if os_name:
        if os_name == "macos":
            machine = platform.machine().lower()
            return "aarch64-apple-darwin" if "arm" in machine else "x86_64-apple-darwin"
        if os_name == "windows":
            return "x86_64-pc-windows-msvc"
        raise ValueError(f"unsupported --os value: {os_name}")

    system = platform.system().lower()
    if system == "darwin":
        machine = platform.machine().lower()
        return "aarch64-apple-darwin" if "arm" in machine else "x86_64-apple-darwin"
    if system == "windows":
        return "x86_64-pc-windows-msvc"
    raise ValueError("Host OS is unsupported for run mode; pass --os explicitly for cross-build.")


def run_command(cmd: list[str]) -> int:
    print("[run.py] executing:", " ".join(cmd))
    result = subprocess.run(cmd, cwd=REPO_ROOT, env=os.environ.copy())
    return result.returncode


def cargo_run_browser(
    args: list[str],
    release: bool = False,
    target: str | None = None,
) -> int:
    cmd = ["cargo", "run", "-p", "browser"]
    if target:
        cmd.extend(["--target", target])
    if release:
        cmd.append("--release")
    cmd.extend(["--"])
    cmd.extend(args)
    return run_command(cmd)


def cmd_run(parsed: argparse.Namespace) -> int:
    try:
        target = host_target(parsed.os)
    except ValueError as exc:
        print(f"[run.py] {exc}")
        return 2

    app_args = ["run"]
    if parsed.pattern:
        app_args.extend(["--pattern", parsed.pattern])
    if parsed.pattern_only:
        app_args.append("--pattern-only")
    if parsed.input:
        app_args.extend(["--input", parsed.input])
    if parsed.width:
        app_args.extend(["--width", str(parsed.width)])
    if parsed.height:
        app_args.extend(["--height", str(parsed.height)])

    return cargo_run_browser(app_args, release=parsed.release, target=target)


def cmd_golden(parsed: argparse.Namespace) -> int:
    app_args = ["golden"]
    if parsed.update:
        app_args.append("--update")
    if parsed.fixture_dir:
        app_args.extend(["--fixture-dir", parsed.fixture_dir])
    if parsed.golden_dir:
        app_args.extend(["--golden-dir", parsed.golden_dir])
    if parsed.width:
        app_args.extend(["--width", str(parsed.width)])
    if parsed.height:
        app_args.extend(["--height", str(parsed.height)])
    if parsed.frame is not None:
        app_args.extend(["--frame", str(parsed.frame)])

    return cargo_run_browser(app_args, release=parsed.release)


def cmd_test(parsed: argparse.Namespace) -> int:
    status = run_command(["cargo", "test", "--workspace"])
    if status != 0:
        return status

    golden_args = argparse.Namespace(
        update=False,
        fixture_dir=parsed.fixture_dir,
        golden_dir=parsed.golden_dir,
        width=parsed.width,
        height=parsed.height,
        frame=parsed.frame,
        release=parsed.release,
    )
    return cmd_golden(golden_args)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Build, run, and test browser")
    subparsers = parser.add_subparsers(dest="command", required=False)

    run_parser = subparsers.add_parser("run", help="Run the windowed app")
    run_parser.add_argument("--release", action="store_true", help="Build in release mode")
    run_parser.add_argument("--os", choices=["macos", "windows"], help="Target OS triple family")
    run_parser.add_argument("--pattern", choices=["gradient", "solid", "rects"], help="Pattern mode")
    run_parser.add_argument("--pattern-only", action="store_true", help="Force pattern mode instead of loading default fixture")
    run_parser.add_argument("--input", help="Optional local HTML file for document rendering")
    run_parser.add_argument("--width", type=int, help="Window width")
    run_parser.add_argument("--height", type=int, help="Window height")
    run_parser.set_defaults(handler=cmd_run)

    golden_parser = subparsers.add_parser("golden", help="Run or update golden frame hashes")
    golden_parser.add_argument("--release", action="store_true", help="Build in release mode")
    golden_parser.add_argument("--update", action="store_true", help="Update expected hashes")
    golden_parser.add_argument("--fixture-dir", help="HTML fixture directory")
    golden_parser.add_argument("--golden-dir", help="Golden hash directory")
    golden_parser.add_argument("--width", type=int, help="Render width")
    golden_parser.add_argument("--height", type=int, help="Render height")
    golden_parser.add_argument("--frame", type=int, help="Frame index")
    golden_parser.set_defaults(handler=cmd_golden)

    test_parser = subparsers.add_parser("test", help="Run workspace tests + golden checks")
    test_parser.add_argument("--release", action="store_true", help="Build in release mode")
    test_parser.add_argument("--fixture-dir", help="HTML fixture directory")
    test_parser.add_argument("--golden-dir", help="Golden hash directory")
    test_parser.add_argument("--width", type=int, help="Render width")
    test_parser.add_argument("--height", type=int, help="Render height")
    test_parser.add_argument("--frame", type=int, help="Frame index")
    test_parser.set_defaults(handler=cmd_test)

    return parser


def main() -> int:
    parser = build_parser()

    if len(sys.argv) == 1:
        # Backward compatibility: `python tools/py/run.py` still runs the app.
        parsed = parser.parse_args(["run"])
    else:
        parsed = parser.parse_args()

    handler = getattr(parsed, "handler", None)
    if handler is None:
        parser.print_help()
        return 2

    return handler(parsed)


if __name__ == "__main__":
    sys.exit(main())
