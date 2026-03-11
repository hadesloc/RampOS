"""Output rendering helpers for the CLI."""

from __future__ import annotations

import json
from typing import Any


def render_output(payload: Any, output: str = "json", compact: bool = True) -> str:
    if output == "json":
        if compact:
            return json.dumps(payload, ensure_ascii=False)
        return json.dumps(payload, indent=2, ensure_ascii=False)
    if output == "jsonl":
        if isinstance(payload, list):
            return "\n".join(
                json.dumps(item, separators=(",", ":"), ensure_ascii=False) for item in payload
            )
        return json.dumps(payload, separators=(",", ":"), ensure_ascii=False)
    if output == "table":
        if isinstance(payload, dict):
            return "\n".join(f"{key}\t{value}" for key, value in payload.items())
        if isinstance(payload, list):
            return "\n".join(str(item) for item in payload)
        return str(payload)
    raise ValueError(f"Unsupported output mode: {output}")


def print_output(payload: Any, output: str = "json", compact: bool = True) -> None:
    print(render_output(payload, output=output, compact=compact))
