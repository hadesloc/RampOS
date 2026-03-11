#!/usr/bin/env python3
"""Thin CLI wrapper for bounded RampOS admin flows."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any
from urllib import error, parse, request


DEFAULT_BASE_URL = "http://localhost:8080"
DEFAULT_PROFILE = "default"
CONFIG_PATH = Path.home() / ".rampos-cli.json"


def load_config() -> dict[str, Any]:
    if not CONFIG_PATH.exists():
        return {}
    return json.loads(CONFIG_PATH.read_text(encoding="utf-8"))


def save_config(config: dict[str, Any]) -> None:
    CONFIG_PATH.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")


def profile_config(name: str) -> dict[str, Any]:
    config = load_config()
    return config.get("profiles", {}).get(name, {})


def resolve_setting(args: argparse.Namespace, key: str, env_name: str, default: str = "") -> str:
    cli_value = getattr(args, key, None)
    if cli_value:
        return cli_value

    env_value = os.getenv(env_name)
    if env_value:
        return env_value

    profile_value = profile_config(getattr(args, "profile", DEFAULT_PROFILE)).get(key)
    if profile_value:
        return str(profile_value)

    return default


def build_headers(args: argparse.Namespace, require_operator: bool = False) -> dict[str, str]:
    admin_key = resolve_setting(args, "admin_key", "RAMPOS_ADMIN_KEY")
    if not admin_key:
        raise SystemExit("Missing admin key. Use `login` or set RAMPOS_ADMIN_KEY.")

    role_default = "operator" if require_operator else "viewer"
    role = resolve_setting(args, "role", "RAMPOS_ADMIN_ROLE", role_default)
    headers = {
        "Content-Type": "application/json",
        "X-Admin-Key": f"{admin_key}:{role}",
    }

    admin_user_id = resolve_setting(args, "admin_user_id", "RAMPOS_ADMIN_USER_ID")
    if admin_user_id:
        headers["X-Admin-User-Id"] = admin_user_id
    return headers


def request_json(
    args: argparse.Namespace,
    method: str,
    path: str,
    payload: dict[str, Any] | None = None,
    require_operator: bool = False,
) -> Any:
    base_url = resolve_setting(args, "base_url", "RAMPOS_BASE_URL", DEFAULT_BASE_URL).rstrip("/")
    url = f"{base_url}{path}"
    body = None if payload is None else json.dumps(payload).encode("utf-8")
    req = request.Request(url, data=body, method=method, headers=build_headers(args, require_operator))

    try:
        with request.urlopen(req) as response:
            return json.loads(response.read().decode("utf-8"))
    except error.HTTPError as exc:
        response_body = exc.read().decode("utf-8", errors="replace")
        raise SystemExit(f"{exc.code} {exc.reason}: {response_body}") from exc
    except error.URLError as exc:
        raise SystemExit(f"Request failed: {exc.reason}") from exc


def print_json(payload: Any) -> None:
    print(json.dumps(payload, indent=2))


def cmd_login(args: argparse.Namespace) -> int:
    config = load_config()
    profiles = config.setdefault("profiles", {})
    profiles[args.profile] = {
        "base_url": args.base_url.rstrip("/"),
        "admin_key": args.admin_key,
        "role": args.role,
        "admin_user_id": args.admin_user_id,
    }
    save_config(config)

    print(f"Saved profile '{args.profile}' to {CONFIG_PATH}")
    print_json(profiles[args.profile])
    return 0


def cmd_sandbox_seed(args: argparse.Namespace) -> int:
    payload = {
        "tenantName": args.tenant_name,
        "presetCode": args.preset_code,
        "scenarioCode": args.scenario_code,
        "configOverrides": json.loads(args.config_overrides),
    }
    result = request_json(
        args,
        "POST",
        "/v1/admin/sandbox/seed",
        payload=payload,
        require_operator=True,
    )
    print_json(result)
    return 0


def cmd_sandbox_run(args: argparse.Namespace) -> int:
    payload = {
        "tenant_id": args.tenant_id,
        "preset_code": args.preset_code,
        "scenario_code": args.scenario_code,
    }
    print_json(
        {
            "status": "placeholder",
            "message": "Scenario execution is not live yet in the backend. Use `sandbox seed` and `sandbox replay` today.",
            "request": payload,
        }
    )
    return 0


def cmd_sandbox_replay(args: argparse.Namespace) -> int:
    suffix = "/export" if args.export else ""
    result = request_json(
        args,
        "GET",
        f"/v1/admin/sandbox/replay/{parse.quote(args.journey_id, safe='')}{suffix}",
        require_operator=False,
    )
    print_json(result)
    return 0


def append_query(path: str, params: dict[str, str]) -> str:
    query = parse.urlencode({key: value for key, value in params.items() if value})
    if not query:
        return path
    return f"{path}?{query}"


def cmd_reconciliation_workbench(args: argparse.Namespace) -> int:
    if args.export:
        path = append_query(
            "/v1/admin/reconciliation/export",
            {"format": args.format, "scenario": args.scenario},
        )
    else:
        path = append_query(
            "/v1/admin/reconciliation/workbench",
            {"scenario": args.scenario},
        )
    print_json(request_json(args, "GET", path, require_operator=False))
    return 0


def cmd_reconciliation_evidence(args: argparse.Namespace) -> int:
    suffix = "/export" if args.export else ""
    path = append_query(
        f"/v1/admin/reconciliation/evidence/{parse.quote(args.discrepancy_id, safe='')}{suffix}",
        {"scenario": args.scenario},
    )
    print_json(request_json(args, "GET", path, require_operator=False))
    return 0


def cmd_treasury_workbench(args: argparse.Namespace) -> int:
    path = append_query(
        "/v1/admin/treasury/export" if args.export else "/v1/admin/treasury/workbench",
        {"scenario": args.scenario, "format": args.format if args.export else ""},
    )
    print_json(request_json(args, "GET", path, require_operator=False))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Thin CLI wrapper for bounded RampOS admin flows.")
    parser.set_defaults(func=lambda _args: parser.print_help() or 0)

    subparsers = parser.add_subparsers(dest="command")

    login = subparsers.add_parser("login", help="Save a local CLI profile for bounded admin flows.")
    login.add_argument("--profile", default=DEFAULT_PROFILE)
    login.add_argument("--base-url", default=DEFAULT_BASE_URL)
    login.add_argument("--admin-key", required=True)
    login.add_argument("--role", default="operator")
    login.add_argument("--admin-user-id", default="")
    login.set_defaults(func=cmd_login)

    sandbox = subparsers.add_parser("sandbox", help="Bounded sandbox admin commands.")
    sandbox.set_defaults(func=lambda _args: sandbox.print_help() or 0)
    sandbox.add_argument("--profile", default=DEFAULT_PROFILE)
    sandbox.add_argument("--base-url")
    sandbox.add_argument("--admin-key")
    sandbox.add_argument("--role")
    sandbox.add_argument("--admin-user-id")
    sandbox_subparsers = sandbox.add_subparsers(dest="sandbox_command")

    seed = sandbox_subparsers.add_parser("seed", help="Seed a sandbox tenant from a preset.")
    seed.add_argument("--tenant-name", required=True)
    seed.add_argument("--preset-code", required=True)
    seed.add_argument("--scenario-code", default="")
    seed.add_argument("--config-overrides", default="{}")
    seed.set_defaults(func=cmd_sandbox_seed)

    run = sandbox_subparsers.add_parser("run", help="Show the bounded placeholder for scenario execution.")
    run.add_argument("--tenant-id", required=True)
    run.add_argument("--preset-code", required=True)
    run.add_argument("--scenario-code", required=True)
    run.set_defaults(func=cmd_sandbox_run)

    replay = sandbox_subparsers.add_parser("replay", help="Fetch or export a redacted replay bundle.")
    replay.add_argument("--journey-id", required=True)
    replay.add_argument("--export", action="store_true", help="Use the export endpoint instead of inline replay.")
    replay.set_defaults(func=cmd_sandbox_replay)

    reconciliation = subparsers.add_parser(
        "reconciliation",
        help="Bounded reconciliation workbench commands.",
    )
    reconciliation.set_defaults(func=lambda _args: reconciliation.print_help() or 0)
    reconciliation.add_argument("--profile", default=DEFAULT_PROFILE)
    reconciliation.add_argument("--base-url")
    reconciliation.add_argument("--admin-key")
    reconciliation.add_argument("--role")
    reconciliation.add_argument("--admin-user-id")
    reconciliation_subparsers = reconciliation.add_subparsers(dest="reconciliation_command")

    workbench = reconciliation_subparsers.add_parser(
        "workbench",
        help="Fetch the reconciliation workbench or export its queue snapshot.",
    )
    workbench.add_argument(
        "--scenario",
        default="",
        help="Optional bounded fixture scenario (for example: clean).",
    )
    workbench.add_argument(
        "--export",
        action="store_true",
        help="Use the export endpoint instead of inline workbench JSON.",
    )
    workbench.add_argument(
        "--format",
        default="json",
        choices=["json", "csv"],
        help="Export format when --export is used.",
    )
    workbench.set_defaults(func=cmd_reconciliation_workbench)

    evidence = reconciliation_subparsers.add_parser(
        "evidence",
        help="Fetch or export a reconciliation evidence pack by discrepancy ID.",
    )
    evidence.add_argument("--discrepancy-id", required=True)
    evidence.add_argument(
        "--scenario",
        default="",
        help="Optional bounded fixture scenario (for example: clean).",
    )
    evidence.add_argument(
        "--export",
        action="store_true",
        help="Use the evidence export endpoint instead of inline JSON.",
    )
    evidence.set_defaults(func=cmd_reconciliation_evidence)

    treasury = subparsers.add_parser(
        "treasury",
        help="Bounded treasury control-tower commands.",
    )
    treasury.set_defaults(func=lambda _args: treasury.print_help() or 0)
    treasury.add_argument("--profile", default=DEFAULT_PROFILE)
    treasury.add_argument("--base-url")
    treasury.add_argument("--admin-key")
    treasury.add_argument("--role")
    treasury.add_argument("--admin-user-id")
    treasury_subparsers = treasury.add_subparsers(dest="treasury_command")

    treasury_workbench = treasury_subparsers.add_parser(
        "workbench",
        help="Fetch the treasury workbench or export its recommendation set.",
    )
    treasury_workbench.add_argument(
        "--scenario",
        default="",
        help="Optional bounded fixture scenario (for example: stable).",
    )
    treasury_workbench.add_argument(
        "--export",
        action="store_true",
        help="Use the export endpoint instead of inline workbench JSON.",
    )
    treasury_workbench.add_argument(
        "--format",
        default="json",
        choices=["json", "csv"],
        help="Export format when --export is used.",
    )
    treasury_workbench.set_defaults(func=cmd_treasury_workbench)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())
