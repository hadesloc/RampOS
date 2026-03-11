"""CLI config loading and precedence resolution."""

from __future__ import annotations

import json
import os
from argparse import Namespace
from pathlib import Path
from typing import Any, Mapping, cast

from rampos.cli.context import CliContext

DEFAULT_BASE_URL = "http://localhost:8080"
DEFAULT_PROFILE = "default"
DEFAULT_OUTPUT = "json"
DEFAULT_TIMEOUT = 30.0
CONFIG_ENV_VAR = "RAMPOS_CLI_CONFIG"
CONFIG_PATH = Path.home() / ".rampos-cli.json"


def get_config_path(config_path: str | Path | None = None, environ: Mapping[str, str] | None = None) -> Path:
    if config_path is not None:
        return Path(config_path)
    env = environ if environ is not None else os.environ
    if env_path := env.get(CONFIG_ENV_VAR):
        return Path(env_path)
    return CONFIG_PATH


def load_config(config_path: str | Path | None = None, environ: Mapping[str, str] | None = None) -> dict[str, Any]:
    resolved = get_config_path(config_path=config_path, environ=environ)
    if not resolved.exists():
        return {}
    return cast(dict[str, Any], json.loads(resolved.read_text(encoding="utf-8")))


def save_config(
    config: dict[str, Any],
    *,
    config_path: str | Path | None = None,
    environ: Mapping[str, str] | None = None,
) -> Path:
    resolved = get_config_path(config_path=config_path, environ=environ)
    resolved.parent.mkdir(parents=True, exist_ok=True)
    resolved.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
    return resolved


def profile_config(
    profile: str,
    *,
    config_path: str | Path | None = None,
    environ: Mapping[str, str] | None = None,
) -> dict[str, Any]:
    config = load_config(config_path=config_path, environ=environ)
    profiles = cast(dict[str, Any], config.get("profiles", {}))
    return cast(dict[str, Any], profiles.get(profile, {}))


def _resolve(
    args: Namespace,
    arg_key: str,
    env_name: str,
    profile: Mapping[str, Any],
    environ: Mapping[str, str],
    default: Any = None,
) -> Any:
    cli_value = getattr(args, arg_key, None)
    if cli_value not in (None, ""):
        return cli_value
    env_value = environ.get(env_name)
    if env_value not in (None, ""):
        return env_value
    profile_value = profile.get(arg_key)
    if profile_value not in (None, ""):
        return profile_value
    return default


def build_cli_context(
    args: Namespace,
    *,
    environ: Mapping[str, str] | None = None,
    config_path: str | Path | None = None,
) -> CliContext:
    env = environ if environ is not None else os.environ
    profile_name = getattr(args, "profile", DEFAULT_PROFILE) or DEFAULT_PROFILE
    profile = profile_config(profile_name, config_path=config_path, environ=env)

    return CliContext(
        profile=profile_name,
        base_url=str(_resolve(args, "base_url", "RAMPOS_BASE_URL", profile, env, DEFAULT_BASE_URL)).rstrip("/"),
        auth_mode=cast(str, _resolve(args, "auth_mode", "RAMPOS_AUTH_MODE", profile, env, "admin")),
        output=cast(str, _resolve(args, "output", "RAMPOS_OUTPUT", profile, env, DEFAULT_OUTPUT)),
        compact=bool(getattr(args, "compact", False)),
        timeout=float(_resolve(args, "timeout", "RAMPOS_TIMEOUT", profile, env, DEFAULT_TIMEOUT)),
        api_key=_resolve(args, "api_key", "RAMPOS_API_KEY", profile, env),
        api_secret=_resolve(args, "api_secret", "RAMPOS_API_SECRET", profile, env),
        admin_key=_resolve(args, "admin_key", "RAMPOS_ADMIN_KEY", profile, env),
        admin_role=_resolve(args, "admin_role", "RAMPOS_ADMIN_ROLE", profile, env, "operator"),
        admin_user_id=_resolve(args, "admin_user_id", "RAMPOS_ADMIN_USER_ID", profile, env),
        portal_token=_resolve(args, "portal_token", "RAMPOS_PORTAL_TOKEN", profile, env),
        lp_key=_resolve(args, "lp_key", "RAMPOS_LP_KEY", profile, env),
        tenant_id=_resolve(args, "tenant_id", "RAMPOS_TENANT_ID", profile, env),
        request_id=_resolve(args, "request_id", "RAMPOS_REQUEST_ID", profile, env),
        idempotency_key=_resolve(args, "idempotency_key", "RAMPOS_IDEMPOTENCY_KEY", profile, env),
        body=getattr(args, "body", None),
        body_file=getattr(args, "body_file", None),
        body_stdin=bool(getattr(args, "body_stdin", False)),
    )
