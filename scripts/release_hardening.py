#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_ROOT = ROOT / "docs" / "operations" / "evidence"

PYTHON = [sys.executable]

GROUPS = {
    "contract-surface": {
        "purpose": "Cross-surface OpenAPI, CLI, and SDK drift checks.",
        "steps": [
            {
                "name": "validate-openapi-script",
                "command": ["bash", "scripts/validate-openapi.sh"],
                "fallback_command": [
                    "cargo",
                    "test",
                    "-p",
                    "ramp-api",
                    "test_openapi_spec_valid",
                    "--no-fail-fast",
                ],
                "requires": ["cargo"],
                "requires_bash": True,
                "evidence": "Contract drift validation tied to scripts/validate-openapi.sh.",
            },
            {
                "name": "rampos-cli-smoke-script",
                "command": ["bash", "scripts/test-rampos-cli.sh"],
                "fallback_command": [
                    *PYTHON,
                    "-m",
                    "pytest",
                    "sdk-python/tests/test_cli_entrypoint.py",
                    "sdk-python/tests/test_cli_output.py",
                    "-q",
                ],
                "requires": ["python"],
                "requires_bash": True,
                "evidence": "CLI smoke validation tied to scripts/test-rampos-cli.sh.",
            },
            {
                "name": "python-cli-drift",
                "command": [
                    *PYTHON,
                    "-m",
                    "pytest",
                    "sdk-python/tests/test_cli_openapi_drift.py",
                    "sdk-python/tests/test_cli_manifest.py",
                    "sdk-python/tests/test_cli_generated_commands.py",
                    "-q",
                ],
                "requires": ["python"],
                "evidence": "Python CLI drift checks when contract surfaces change.",
            },
        ],
    },
    "backend-admin": {
        "purpose": "Admin and operator control-plane regressions.",
        "steps": [
            {
                "name": "kyb-admin",
                "command": ["cargo", "test", "-p", "ramp-api", "--test", "kyb_admin_test", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "KYB evidence and admin review.",
            },
            {
                "name": "treasury-admin",
                "command": ["cargo", "test", "-p", "ramp-api", "--test", "treasury_admin_test", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Treasury evidence and workbench.",
            },
            {
                "name": "reconciliation-admin",
                "command": ["cargo", "test", "-p", "ramp-api", "--test", "reconciliation_admin_test", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Reconciliation lineage and gated actions.",
            },
            {
                "name": "liquidity-admin",
                "command": ["cargo", "test", "-p", "ramp-api", "--test", "liquidity_admin_test", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Liquidity explainability.",
            },
            {
                "name": "net-settlement-admin",
                "command": ["cargo", "test", "-p", "ramp-api", "--test", "net_settlement_admin_test", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Settlement governance and approvals.",
            },
            {
                "name": "travel-rule-admin",
                "command": ["cargo", "test", "-p", "ramp-api", "--test", "travel_rule_admin_test", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Travel Rule governed flows.",
            },
            {
                "name": "partner-registry-admin",
                "command": ["cargo", "test", "-p", "ramp-api", "--test", "partner_registry_test", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Partner registry governance.",
            },
        ],
    },
    "core-services": {
        "purpose": "Scoring, normalization, and idempotent evidence pipelines.",
        "steps": [
            {
                "name": "route-scoring",
                "command": ["cargo", "test", "-p", "ramp-core", "select_route_with_constraints", "--lib", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Constraint-aware route scoring.",
            },
            {
                "name": "quote-normalization",
                "command": [
                    "cargo",
                    "test",
                    "-p",
                    "ramp-core",
                    "test_normalize_quote_signal_captures_governance_and_amounts",
                    "--lib",
                    "--",
                    "--nocapture",
                ],
                "requires": ["cargo"],
                "evidence": "Liquidity quote normalization.",
            },
            {
                "name": "settlement-quality-normalization",
                "command": [
                    "cargo",
                    "test",
                    "-p",
                    "ramp-core",
                    "test_normalize_settlement_quality_signal_includes_status_latency_and_dispute_flags",
                    "--lib",
                    "--",
                    "--nocapture",
                ],
                "requires": ["cargo"],
                "evidence": "Settlement quality normalization.",
            },
            {
                "name": "rfq-finalization",
                "command": [
                    "cargo",
                    "test",
                    "-p",
                    "ramp-core",
                    "test_finalize_rfq_records_normalized_fill_and_cancel_metadata",
                    "--lib",
                    "--",
                    "--nocapture",
                ],
                "requires": ["cargo"],
                "evidence": "RFQ settlement/cancel normalization.",
            },
            {
                "name": "treasury-balance-normalization",
                "command": ["cargo", "test", "-p", "ramp-core", "normalize_treasury_balances_clamps_negative_values", "--lib", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Treasury balance normalization.",
            },
            {
                "name": "treasury-import-idempotency",
                "command": ["cargo", "test", "-p", "ramp-core", "db_gated_import_is_replay_safe_by_idempotency_key", "--lib", "--", "--nocapture"],
                "requires": ["cargo"],
                "evidence": "Replay-safe treasury import.",
            },
        ],
    },
    "cli-certification": {
        "purpose": "Certification artifact and fail-closed compatibility gate coverage.",
        "steps": [
            {
                "name": "cli-certification-suite",
                "command": [
                    *PYTHON,
                    "-m",
                    "pytest",
                    "sdk-python/tests/test_cli_certification.py",
                    "sdk-python/tests/test_cli_compatibility_gate.py",
                    "-q",
                ],
                "requires": ["python"],
                "evidence": "Certification and compatibility gate suite.",
            },
        ],
    },
    "audit-controls": {
        "purpose": "Break-glass and immutable audit export checks.",
        "steps": [
            {
                "name": "break-glass-response-shape",
                "command": [
                    "cargo",
                    "test",
                    "-p",
                    "ramp-api",
                    "build_break_glass_export_response_preserves_scope_and_immutability",
                    "--lib",
                    "--",
                    "--nocapture",
                ],
                "requires": ["cargo"],
                "evidence": "Break-glass export shape preservation.",
            },
            {
                "name": "break-glass-filtering-and-linkage",
                "command": [
                    "cargo",
                    "test",
                    "-p",
                    "ramp-api",
                    "break_glass_actions_are_filtered_and_export_linked",
                    "--lib",
                    "--",
                    "--nocapture",
                ],
                "requires": ["cargo"],
                "evidence": "Break-glass linkage and filtering.",
            },
        ],
    },
    "migration-rehearsal": {
        "purpose": "Manual-only migration and rollback rehearsal in an isolated database.",
        "manual_only": True,
        "steps": [
            {
                "name": "sqlx-migrate-run",
                "command": ["sqlx", "migrate", "run"],
                "requires": ["sqlx", "DATABASE_URL"],
                "evidence": "Forward migration rehearsal.",
            },
            {
                "name": "sqlx-migrate-revert",
                "command": ["sqlx", "migrate", "revert"],
                "requires": ["sqlx", "DATABASE_URL"],
                "evidence": "Rollback rehearsal.",
            },
            {
                "name": "post-migration-regression",
                "command": ["cargo", "test", "-p", "ramp-api", "--test", "partner_registry_test", "--", "--nocapture"],
                "requires": ["cargo", "DATABASE_URL"],
                "evidence": "Cheap DB-backed post-migration regression.",
            },
        ],
    },
}


def has_bash() -> bool:
    return shutil.which("bash") is not None or shutil.which("sh") is not None


def available(command: str) -> bool:
    return shutil.which(command) is not None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run the RampOS M6 release-hardening matrix.")
    parser.add_argument("--group", action="append", dest="groups", help="Group to execute. Defaults to all non-manual groups.")
    parser.add_argument("--list-groups", action="store_true", help="List available groups and exit.")
    parser.add_argument("--dry-run", action="store_true", help="Print and record the execution plan without running commands.")
    parser.add_argument("--include-manual", action="store_true", help="Allow manual-only groups like migration rehearsal.")
    parser.add_argument("--release-candidate", help="Candidate SHA or label for evidence output.")
    parser.add_argument("--evidence-dir", default=str(EVIDENCE_ROOT), help="Evidence root directory.")
    parser.add_argument("--stop-on-failure", action="store_true", help="Stop after the first failed command.")
    return parser.parse_args()


def selected_groups(args: argparse.Namespace) -> list[str]:
    groups = args.groups or [name for name, cfg in GROUPS.items() if not cfg.get("manual_only")]
    missing = [name for name in groups if name not in GROUPS]
    if missing:
        raise SystemExit(f"Unknown group(s): {', '.join(missing)}")
    if not args.include_manual:
        blocked = [name for name in groups if GROUPS[name].get("manual_only")]
        if blocked:
            raise SystemExit(f"Group(s) require --include-manual: {', '.join(blocked)}")
    return groups


def ensure_run_dir(base: Path, candidate: str | None) -> Path:
    label = candidate or dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    run_dir = base / label
    run_dir.mkdir(parents=True, exist_ok=True)
    return run_dir


def step_command(step: dict) -> list[str]:
    if step.get("requires_bash") and has_bash():
        return step["command"]
    return step.get("fallback_command", step["command"])


def missing_requires(step: dict) -> list[str]:
    missing: list[str] = []
    for requirement in step.get("requires", []):
        if requirement == "cargo" and not available("cargo"):
            missing.append(requirement)
        elif requirement == "python" and not Path(sys.executable).exists():
            missing.append(requirement)
        elif requirement == "sqlx" and not available("sqlx"):
            missing.append(requirement)
        elif requirement == "DATABASE_URL" and not os.environ.get("DATABASE_URL"):
            missing.append(requirement)
    return missing


def write_summary(run_dir: Path, candidate: str | None, mode: str, groups: list[str], results: list[dict]) -> None:
    lines = [
        "# Release Hardening Evidence",
        "",
        f"- Release candidate: `{candidate or 'unspecified'}`",
        f"- Generated at: `{dt.datetime.now(dt.timezone.utc).isoformat()}`",
        f"- Mode: `{mode}`",
        "",
        "## Groups",
    ]
    for group in groups:
        lines.append(f"- `{group}`: {GROUPS[group]['purpose']}")
    lines.extend(["", "## Results", "", "| Step | Status | Evidence | Log |", "|---|---|---|---|"])
    for result in results:
        lines.append(
            f"| `{result['step']}` | `{result['status']}` | {result['evidence']} | `{result['log']}` |"
        )
    (run_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (run_dir / "summary.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")


def dry_run(run_dir: Path, candidate: str | None, groups: list[str]) -> int:
    lines = ["# Planned Release Hardening Run", ""]
    results: list[dict] = []
    for group in groups:
        lines.append(f"## {group}")
        lines.append(f"- Purpose: {GROUPS[group]['purpose']}")
        for step in GROUPS[group]["steps"]:
            command = " ".join(step_command(step))
            lines.append(f"- `{step['name']}` -> `{command}`")
            results.append(
                {
                    "step": step["name"],
                    "status": "planned",
                    "evidence": step["evidence"],
                    "log": str((run_dir / f"{step['name']}.log").relative_to(ROOT)),
                }
            )
        lines.append("")
    (run_dir / "dry-run-plan.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    write_summary(run_dir, candidate, "dry-run", groups, results)
    print("\n".join(lines))
    print(f"Planned evidence written to {run_dir}")
    return 0


def execute(run_dir: Path, candidate: str | None, groups: list[str], stop_on_failure: bool) -> int:
    results: list[dict] = []
    failed = False
    for group in groups:
        for step in GROUPS[group]["steps"]:
            log_path = run_dir / f"{step['name']}.log"
            missing = missing_requires(step)
            if missing:
                log_path.write_text(f"Missing prerequisites: {', '.join(missing)}\n", encoding="utf-8")
                results.append(
                    {
                        "step": step["name"],
                        "status": "skipped",
                        "evidence": f"Missing prerequisites: {', '.join(missing)}",
                        "log": str(log_path.relative_to(ROOT)),
                    }
                )
                failed = True
                print(f"[skipped] {step['name']}")
                if stop_on_failure:
                    write_summary(run_dir, candidate, "execute", groups, results)
                    return 1
                continue

            command = step_command(step)
            completed = subprocess.run(
                command,
                cwd=ROOT,
                capture_output=True,
                text=True,
                errors="replace",
                shell=False,
            )
            log_path.write_text(
                "$ "
                + " ".join(command)
                + "\n\n[stdout]\n"
                + completed.stdout
                + "\n\n[stderr]\n"
                + completed.stderr
                + "\n",
                encoding="utf-8",
            )
            status = "passed" if completed.returncode == 0 else "failed"
            results.append(
                {
                    "step": step["name"],
                    "status": status,
                    "evidence": step["evidence"],
                    "log": str(log_path.relative_to(ROOT)),
                }
            )
            print(f"[{status}] {step['name']}")
            if status == "failed":
                failed = True
                if stop_on_failure:
                    write_summary(run_dir, candidate, "execute", groups, results)
                    return 1

    write_summary(run_dir, candidate, "execute", groups, results)
    return 1 if failed else 0


def main() -> int:
    args = parse_args()
    if args.list_groups:
        for name, cfg in GROUPS.items():
            suffix = " (manual)" if cfg.get("manual_only") else ""
            print(f"{name}{suffix}: {cfg['purpose']}")
        return 0

    groups = selected_groups(args)
    run_dir = ensure_run_dir(Path(args.evidence_dir), args.release_candidate)
    if args.dry_run:
        return dry_run(run_dir, args.release_candidate, groups)
    return execute(run_dir, args.release_candidate, groups, args.stop_on_failure)


if __name__ == "__main__":
    raise SystemExit(main())
