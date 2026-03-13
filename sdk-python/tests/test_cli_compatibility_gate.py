from argparse import Namespace
from pathlib import Path

from rampos.cli.app import (
    build_certification_artifact,
    build_parser,
    cmd_certification_artifact,
    evaluate_compatibility_gate,
)


def test_evaluate_compatibility_gate_blocks_missing_or_stale_evidence() -> None:
    gate = evaluate_compatibility_gate(
        [
            "openapi@2026-03-12",
            "cli@2026-03-11",
        ],
        generated_at="2026-03-12T14:00:00+00:00",
    )

    assert gate["status"] == "blocked"
    assert gate["missingSurfaces"] == ["sdk-python"]
    assert gate["staleSurfaces"] == ["cli"]


def test_build_certification_artifact_embeds_passed_compatibility_gate() -> None:
    args = Namespace(
        profile="default",
        auth_mode="admin",
        corridor_code="US_VN_OFFRAMP",
        partner_id="lp_alpha",
        rollout_scope="pilot-us-vn",
        simulator_code="corridor-smoke-v1",
        status="simulated",
        checks=["price_parity", "compatibility_gate"],
        compatibility_evidence=[
            "openapi@2026-03-12",
            "sdk-python@2026-03-12",
            "cli@2026-03-12",
        ],
    )

    artifact = build_certification_artifact(
        args,
        generated_at="2026-03-12T14:00:00+00:00",
    )

    assert artifact["artifactType"] == "corridor_rollout_certification"
    assert artifact["compatibilityGate"]["status"] == "passed"
    assert artifact["compatibilityGate"]["missingSurfaces"] == []
    assert artifact["compatibilityGate"]["staleSurfaces"] == []


def test_cmd_certification_artifact_blocks_output_when_gate_fails(tmp_path: Path) -> None:
    output_file = tmp_path / "certification.json"
    args = Namespace(
        profile="default",
        auth_mode="admin",
        output="json",
        compact=False,
        corridor_code="US_VN_OFFRAMP",
        partner_id="lp_alpha",
        rollout_scope="pilot-us-vn",
        simulator_code="corridor-smoke-v1",
        status="certified",
        checks=["price_parity"],
        compatibility_evidence=["openapi@2026-03-12"],
        output_file=str(output_file),
    )

    exit_code = cmd_certification_artifact(args)

    assert exit_code == 1
    assert not output_file.exists()


def test_build_parser_registers_certification_artifact_command() -> None:
    parser = build_parser()

    args = parser.parse_args(
        [
            "certification",
            "artifact",
            "--corridor-code",
            "US_VN_OFFRAMP",
            "--partner-id",
            "lp_alpha",
            "--rollout-scope",
            "pilot-us-vn",
            "--simulator-code",
            "corridor-smoke-v1",
            "--check",
            "price_parity",
            "--compatibility-evidence",
            "openapi@2026-03-12",
            "--compatibility-evidence",
            "sdk-python@2026-03-12",
            "--compatibility-evidence",
            "cli@2026-03-12",
        ]
    )

    assert args.corridor_code == "US_VN_OFFRAMP"
    assert args.partner_id == "lp_alpha"
    assert args.checks == ["price_parity"]
    assert args.compatibility_evidence == [
        "openapi@2026-03-12",
        "sdk-python@2026-03-12",
        "cli@2026-03-12",
    ]
