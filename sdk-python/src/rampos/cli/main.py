"""CLI entrypoint for the RampOS package."""

from __future__ import annotations

import sys
from typing import Sequence

from rampos.cli.app import build_parser
from rampos.cli.errors import CliError, exit_code_for_error


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(list(argv) if argv is not None else None)

    try:
        return int(args.func(args))
    except CliError as exc:
        print(str(exc), file=sys.stderr)
        return exit_code_for_error(exc)


if __name__ == "__main__":
    raise SystemExit(main())
