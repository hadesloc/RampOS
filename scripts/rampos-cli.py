#!/usr/bin/env python3
"""Compatibility shim for the packaged RampOS CLI."""

from __future__ import annotations

import sys
from pathlib import Path


def _ensure_sdk_src_on_path() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    sdk_src = repo_root / "sdk-python" / "src"
    if str(sdk_src) not in sys.path:
        sys.path.insert(0, str(sdk_src))


def main() -> int:
    _ensure_sdk_src_on_path()
    from rampos.cli.main import main as packaged_main

    return packaged_main()


if __name__ == "__main__":
    raise SystemExit(main())
