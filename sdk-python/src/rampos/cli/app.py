"""Parser construction and command handlers for the RampOS CLI."""

from __future__ import annotations

import argparse
import re
from typing import Any

from rampos.cli.config import DEFAULT_BASE_URL, DEFAULT_PROFILE, build_cli_context, load_config, save_config
from rampos.cli.manifest import ManifestOperation, load_manifest
from rampos.cli.output import print_output
from rampos.cli.request import append_query, load_body, request_json


def _add_common_runtime_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--profile", default=DEFAULT_PROFILE)
    parser.add_argument("--base-url")
    parser.add_argument("--auth-mode", choices=["api", "admin", "portal", "lp"])
    parser.add_argument("--api-key")
    parser.add_argument("--api-secret")
    parser.add_argument("--admin-key")
    parser.add_argument("--admin-role")
    parser.add_argument("--admin-user-id")
    parser.add_argument("--portal-token")
    parser.add_argument("--lp-key")
    parser.add_argument("--tenant-id")
    parser.add_argument("--output", choices=["json", "jsonl", "table"])
    parser.add_argument("--compact", action="store_true")
    parser.add_argument("--body")
    parser.add_argument("--body-file")
    parser.add_argument("--body-stdin", action="store_true")
    parser.add_argument("--timeout", type=float)
    parser.add_argument("--request-id")
    parser.add_argument("--idempotency-key")


def cmd_login(args: argparse.Namespace) -> int:
    config = load_config()
    profiles = config.setdefault("profiles", {})
    profiles[args.profile] = {
        "base_url": args.base_url.rstrip("/"),
        "auth_mode": args.auth_mode,
        "api_key": args.api_key,
        "api_secret": args.api_secret,
        "admin_key": args.admin_key,
        "admin_role": args.admin_role,
        "admin_user_id": args.admin_user_id,
        "portal_token": args.portal_token,
        "lp_key": args.lp_key,
        "tenant_id": args.tenant_id,
    }
    resolved = save_config(config)
    print_output(
        {
            "message": f"Saved profile '{args.profile}'",
            "configPath": str(resolved),
            "profile": profiles[args.profile],
        },
        output="json",
        compact=False,
    )
    return 0


def cmd_sandbox_seed(args: argparse.Namespace) -> int:
    ctx = build_cli_context(args)
    payload = {
        "tenantName": args.tenant_name,
        "presetCode": args.preset_code,
        "scenarioCode": args.scenario_code,
        "configOverrides": args.config_overrides,
    }
    result = request_json(ctx, "POST", "/v1/admin/sandbox/seed", payload=payload, require_operator=True)
    print_output(result, output=ctx.output, compact=ctx.compact)
    return 0


def cmd_sandbox_run(args: argparse.Namespace) -> int:
    ctx = build_cli_context(args)
    print_output(
        {
            "status": "placeholder",
            "message": "Scenario execution is not live yet in the backend. Use sandbox seed and sandbox replay today.",
            "request": {
                "tenant_id": args.tenant_id,
                "preset_code": args.preset_code,
                "scenario_code": args.scenario_code,
            },
        },
        output=ctx.output,
        compact=ctx.compact,
    )
    return 0


def cmd_sandbox_replay(args: argparse.Namespace) -> int:
    ctx = build_cli_context(args)
    suffix = "/export" if args.export else ""
    result = request_json(
        ctx,
        "GET",
        f"/v1/admin/sandbox/replay/{args.journey_id}{suffix}",
    )
    print_output(result, output=ctx.output, compact=ctx.compact)
    return 0


def cmd_reconciliation_workbench(args: argparse.Namespace) -> int:
    ctx = build_cli_context(args)
    path = append_query(
        "/v1/admin/reconciliation/export" if args.export else "/v1/admin/reconciliation/workbench",
        {"format": args.format if args.export else "", "scenario": args.scenario},
    )
    result = request_json(ctx, "GET", path)
    print_output(result, output=ctx.output, compact=ctx.compact)
    return 0


def cmd_reconciliation_evidence(args: argparse.Namespace) -> int:
    ctx = build_cli_context(args)
    suffix = "/export" if args.export else ""
    path = append_query(
        f"/v1/admin/reconciliation/evidence/{args.discrepancy_id}{suffix}",
        {"scenario": args.scenario},
    )
    result = request_json(ctx, "GET", path)
    print_output(result, output=ctx.output, compact=ctx.compact)
    return 0


def cmd_treasury_workbench(args: argparse.Namespace) -> int:
    ctx = build_cli_context(args)
    path = append_query(
        "/v1/admin/treasury/export" if args.export else "/v1/admin/treasury/workbench",
        {"scenario": args.scenario, "format": args.format if args.export else ""},
    )
    result = request_json(ctx, "GET", path)
    print_output(result, output=ctx.output, compact=ctx.compact)
    return 0


PATH_PARAM_RE = re.compile(r":([A-Za-z_][A-Za-z0-9_]*)|\{([A-Za-z_][A-Za-z0-9_]*)\}")


def _path_params(path: str) -> list[str]:
    params: list[str] = []
    for colon_param, brace_param in PATH_PARAM_RE.findall(path):
        params.append(colon_param or brace_param)
    return params


def cmd_manifest_operation(args: argparse.Namespace) -> int:
    ctx = build_cli_context(args)
    operation = ManifestOperation(
        operation_id=args.operation["operation_id"],
        command=tuple(args.operation["command"]),
        auth_mode=args.operation["auth_mode"],
        contract_source=args.operation["contract_source"],
        method=args.operation["method"],
        path=args.operation["path"],
    )
    path = operation.path
    for param in _path_params(path):
        value = getattr(args, param)
        path = path.replace(f":{param}", str(value))
        path = path.replace(f"{{{param}}}", str(value))

    payload = load_body(ctx)
    require_operator = operation.auth_mode == "admin" and operation.method != "GET"
    result = request_json(
        ctx,
        operation.method,
        path,
        payload=payload,
        require_operator=require_operator,
    )
    print_output(result, output=ctx.output, compact=ctx.compact)
    return 0


def _register_manifest_commands(
    subparsers: argparse._SubParsersAction[argparse.ArgumentParser],
) -> None:
    parser_cache: dict[tuple[str, ...], argparse.ArgumentParser] = {}
    subparser_cache: dict[tuple[str, ...], argparse._SubParsersAction[argparse.ArgumentParser]] = {(): subparsers}
    registered_leaves: set[tuple[str, ...]] = set()
    manifest = load_manifest()

    for operation_dict in manifest["operations"]:
        operation = ManifestOperation(
            operation_id=operation_dict["operation_id"],
            command=tuple(operation_dict["command"]),
            auth_mode=operation_dict["auth_mode"],
            contract_source=operation_dict["contract_source"],
            method=operation_dict["method"],
            path=operation_dict["path"],
        )
        prefix: tuple[str, ...] = ()
        for index, part in enumerate(operation.command):
            is_leaf = index == len(operation.command) - 1
            parent_subparsers = subparser_cache[prefix]
            next_prefix = prefix + (part,)

            if is_leaf:
                if next_prefix in registered_leaves:
                    prefix = next_prefix
                    continue
                leaf = parent_subparsers.add_parser(part, help=f"{operation.method} {operation.path}")
                _add_common_runtime_args(leaf)
                for param in _path_params(operation.path):
                    leaf.add_argument(f"--{param.replace('_', '-')}", dest=param, required=True)
                leaf.set_defaults(func=cmd_manifest_operation, operation=operation.to_dict())
                registered_leaves.add(next_prefix)
            else:
                if next_prefix not in parser_cache:
                    branch = parent_subparsers.add_parser(part, help=f"{part} commands")
                    branch.set_defaults(func=lambda parsed, branch=branch: branch.print_help() or 0)
                    branch_subparsers = branch.add_subparsers(dest="_".join(next_prefix) + "_command")
                    parser_cache[next_prefix] = branch
                    subparser_cache[next_prefix] = branch_subparsers
            prefix = next_prefix


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="RampOS CLI foundation for operator and agent workflows.")
    parser.set_defaults(func=lambda parsed: parser.print_help() or 0)
    subparsers = parser.add_subparsers(dest="command")

    login = subparsers.add_parser("login", help="Save a local CLI profile.")
    _add_common_runtime_args(login)
    login.set_defaults(func=cmd_login)
    login.set_defaults(base_url=DEFAULT_BASE_URL)
    login.set_defaults(auth_mode="admin")
    login.set_defaults(admin_role="operator")

    sandbox = subparsers.add_parser("sandbox", help="Sandbox admin commands.")
    _add_common_runtime_args(sandbox)
    sandbox.set_defaults(func=lambda parsed: sandbox.print_help() or 0)
    sandbox_subparsers = sandbox.add_subparsers(dest="sandbox_command")

    seed = sandbox_subparsers.add_parser("seed", help="Seed a sandbox tenant from a preset.")
    seed.add_argument("--tenant-name", required=True)
    seed.add_argument("--preset-code", required=True)
    seed.add_argument("--scenario-code", default="")
    seed.add_argument("--config-overrides", default={})
    seed.set_defaults(func=cmd_sandbox_seed)

    run = sandbox_subparsers.add_parser("run", help="Show the placeholder for scenario execution.")
    run.add_argument("--tenant-id", required=True)
    run.add_argument("--preset-code", required=True)
    run.add_argument("--scenario-code", required=True)
    run.set_defaults(func=cmd_sandbox_run)

    replay = sandbox_subparsers.add_parser("replay", help="Fetch or export a replay bundle.")
    replay.add_argument("--journey-id", required=True)
    replay.add_argument("--export", action="store_true")
    replay.set_defaults(func=cmd_sandbox_replay)

    reconciliation = subparsers.add_parser("reconciliation", help="Reconciliation commands.")
    _add_common_runtime_args(reconciliation)
    reconciliation.set_defaults(func=lambda parsed: reconciliation.print_help() or 0)
    reconciliation_subparsers = reconciliation.add_subparsers(dest="reconciliation_command")

    workbench = reconciliation_subparsers.add_parser("workbench", help="Fetch the reconciliation workbench.")
    workbench.add_argument("--scenario", default="")
    workbench.add_argument("--export", action="store_true")
    workbench.add_argument("--format", default="json", choices=["json", "csv"])
    workbench.set_defaults(func=cmd_reconciliation_workbench)

    evidence = reconciliation_subparsers.add_parser("evidence", help="Fetch reconciliation evidence.")
    evidence.add_argument("--discrepancy-id", required=True)
    evidence.add_argument("--scenario", default="")
    evidence.add_argument("--export", action="store_true")
    evidence.set_defaults(func=cmd_reconciliation_evidence)

    treasury = subparsers.add_parser("treasury", help="Treasury control-tower commands.")
    _add_common_runtime_args(treasury)
    treasury.set_defaults(func=lambda parsed: treasury.print_help() or 0)
    treasury_subparsers = treasury.add_subparsers(dest="treasury_command")

    treasury_workbench = treasury_subparsers.add_parser("workbench", help="Fetch the treasury workbench.")
    treasury_workbench.add_argument("--scenario", default="")
    treasury_workbench.add_argument("--export", action="store_true")
    treasury_workbench.add_argument("--format", default="json", choices=["json", "csv"])
    treasury_workbench.set_defaults(func=cmd_treasury_workbench)

    _register_manifest_commands(subparsers)

    return parser
