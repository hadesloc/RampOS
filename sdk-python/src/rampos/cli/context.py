"""Shared CLI runtime context models."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

AuthMode = Literal["api", "admin", "portal", "lp"]
OutputMode = Literal["json", "jsonl", "table"]


@dataclass(slots=True)
class CliContext:
    profile: str
    base_url: str
    auth_mode: AuthMode
    output: OutputMode = "json"
    compact: bool = True
    timeout: float = 30.0
    api_key: str | None = None
    api_secret: str | None = None
    admin_key: str | None = None
    admin_role: str | None = None
    admin_user_id: str | None = None
    portal_token: str | None = None
    lp_key: str | None = None
    tenant_id: str | None = None
    request_id: str | None = None
    idempotency_key: str | None = None
    body: str | None = None
    body_file: str | None = None
    body_stdin: bool = False
