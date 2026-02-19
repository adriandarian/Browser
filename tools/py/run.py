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
    raise ValueError("Host OS is unsupported for this runner; pass --os explicitly for cross-build.")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build and run tessera")
    parser.add_argument("--release", action="store_true", help="Build in release mode")
    parser.add_argument("--os", choices=["macos", "windows"], help="Target OS triple family")
    parser.add_argument("--headless", action="store_true", help="No-op for future non-windowed mode")
    args = parser.parse_args()

    if args.headless:
        print("[run.py] --headless requested; currently a no-op.")

    try:
        target = host_target(args.os)
    except ValueError as exc:
        print(f"[run.py] {exc}")
        return 2

    cmd = ["cargo", "run", "-p", "tessera", "--target", target]
    if args.release:
        cmd.append("--release")

    print("[run.py] executing:", " ".join(cmd))
    result = subprocess.run(cmd, cwd=REPO_ROOT, env=os.environ.copy())
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
