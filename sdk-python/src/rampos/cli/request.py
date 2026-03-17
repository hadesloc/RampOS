"""Shared request helpers for the CLI."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any
from urllib import error, parse, request

from rampos.cli.context import CliContext
from rampos.cli.errors import CliAuthError, CliHttpError, CliTransportError, CliUsageError


def append_query(path: str, params: dict[str, str]) -> str:
    query = parse.urlencode({key: value for key, value in params.items() if value not in ("", None)})
    return f"{path}?{query}" if query else path


# --- Approval-Aware Boundaries (T-RR-016) ---
DANGEROUS_PATH_PATTERNS = (
    "/admin/sandbox/seed",
    "/admin/sandbox/run",
    "/admin/bridge/transfer",
    "/admin/rfq/",  # finalize
    "/admin/config-bundles/",
    "/admin/licensing/upload",
    "/admin/rescreening/",
    "/admin/travel-rule/",
    "/admin/passport/",
    "/admin/kyb/",
)


def is_dangerous_write(method: str, path: str) -> bool:
    """Classify whether a request is a dangerous write that should require confirmation."""
    if method in ("GET", "HEAD", "OPTIONS"):
        return False
    return any(pattern in path for pattern in DANGEROUS_PATH_PATTERNS)


def confirm_dangerous_operation(
    ctx: CliContext,
    method: str,
    path: str,
    payload: Any,
) -> bool:
    """Prompt for confirmation before executing a dangerous write. Returns True if approved."""
    if getattr(ctx, "yes", False):
        return True

    if getattr(ctx, "dry_run", False):
        import json as _json
        print(
            _json.dumps(
                {
                    "dryRun": True,
                    "method": method,
                    "path": path,
                    "payload": payload,
                    "message": "Dangerous operation. Use --yes to skip confirmation.",
                },
                sort_keys=True,
            )
        )
        return False

    # Non-interactive check (piped stdin)
    if not sys.stdin.isatty():
        raise CliUsageError(
            f"Dangerous write to {method} {path} requires --yes flag in non-interactive mode."
        )

    response = input(f"\n⚠️  DANGEROUS: {method} {path}\nProceed? [y/N] ").strip().lower()
    return response in ("y", "yes")


def load_body(ctx: CliContext, stdin_text: str | None = None) -> Any:
    sources = [ctx.body is not None, ctx.body_file is not None, ctx.body_stdin]
    if sum(bool(source) for source in sources) > 1:
        raise CliUsageError("Use only one of --body, --body-file, or --body-stdin.")

    raw: str | None = None
    if ctx.body is not None:
        raw = ctx.body
    elif ctx.body_file is not None:
        raw = Path(ctx.body_file).read_text(encoding="utf-8")
    elif ctx.body_stdin:
        raw = stdin_text if stdin_text is not None else sys.stdin.read()

    if raw is None or raw == "":
        return None

    try:
        return json.loads(raw)
    except json.JSONDecodeError as exc:
        raise CliUsageError(f"Request body must be valid JSON: {exc}") from exc


def build_auth_headers(ctx: CliContext, *, require_operator: bool = False) -> dict[str, str]:
    headers: dict[str, str] = {"Content-Type": "application/json"}

    if ctx.auth_mode == "admin":
        if not ctx.admin_key:
            raise CliAuthError("Missing admin key. Use a profile or set RAMPOS_ADMIN_KEY.")
        role = "operator" if require_operator else (ctx.admin_role or "viewer")
        headers["X-Admin-Key"] = f"{ctx.admin_key}:{role}"
        if ctx.admin_user_id:
            headers["X-Admin-User-Id"] = ctx.admin_user_id
    elif ctx.auth_mode == "api":
        if not ctx.api_key:
            raise CliAuthError("Missing API key. Use a profile or set RAMPOS_API_KEY.")
        headers["Authorization"] = f"Bearer {ctx.api_key}"
        if ctx.api_secret:
            headers["X-Api-Secret"] = ctx.api_secret
    elif ctx.auth_mode == "portal":
        if not ctx.portal_token:
            raise CliAuthError("Missing portal token. Use a profile or set RAMPOS_PORTAL_TOKEN.")
        headers["Authorization"] = f"Bearer {ctx.portal_token}"
    elif ctx.auth_mode == "lp":
        if not ctx.lp_key:
            raise CliAuthError("Missing LP key. Use a profile or set RAMPOS_LP_KEY.")
        headers["X-LP-Key"] = ctx.lp_key
    else:
        raise CliUsageError(f"Unsupported auth mode: {ctx.auth_mode}")

    if ctx.tenant_id:
        headers["X-Tenant-ID"] = ctx.tenant_id
    if ctx.request_id:
        headers["X-Request-Id"] = ctx.request_id
    if ctx.idempotency_key:
        headers["Idempotency-Key"] = ctx.idempotency_key
    return headers


def request_json(
    ctx: CliContext,
    method: str,
    path: str,
    *,
    payload: Any = None,
    require_operator: bool = False,
) -> Any:
    # Approval boundary for dangerous writes
    if is_dangerous_write(method, path):
        if not confirm_dangerous_operation(ctx, method, path, payload):
            return {"aborted": True, "message": "Operation cancelled by operator."}

    url = f"{ctx.base_url.rstrip('/')}{path}"
    body = None if payload is None else json.dumps(payload).encode("utf-8")
    req = request.Request(
        url,
        data=body,
        method=method,
        headers=build_auth_headers(ctx, require_operator=require_operator),
    )

    try:
        with request.urlopen(req, timeout=ctx.timeout) as response:
            raw = response.read()
            if not raw:
                return {}
            return json.loads(raw.decode("utf-8"))
    except error.HTTPError as exc:
        body_text = exc.read().decode("utf-8", errors="replace")
        raise CliHttpError(
            f"{exc.code} {exc.reason}",
            status_code=exc.code,
            body=body_text,
        ) from exc
    except error.URLError as exc:
        raise CliTransportError(f"Request failed: {exc}") from exc
