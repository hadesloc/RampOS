from argparse import Namespace
import json
from pathlib import Path

from rampos.cli.app import build_certification_artifact, build_parser, cmd_certification_artifact


def test_build_certification_artifact_is_repeatable() -> None:
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
    assert artifact["corridorCode"] == "US_VN_OFFRAMP"
    assert artifact["partnerId"] == "lp_alpha"
    assert artifact["distributionSurface"] == "rampos-cli"
    assert artifact["checks"] == ["price_parity", "compatibility_gate"]
    assert artifact["compatibilityEvidence"] == [
        "openapi@2026-03-12",
        "sdk-python@2026-03-12",
        "cli@2026-03-12",
    ]
    assert artifact["compatibilityGate"]["status"] == "passed"


def test_cmd_certification_artifact_writes_output_file(tmp_path: Path) -> None:
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
        compatibility_evidence=[
            "openapi@2026-03-12",
            "sdk-python@2026-03-12",
            "cli@2026-03-12",
        ],
        output_file=str(output_file),
    )

    exit_code = cmd_certification_artifact(args)

    assert exit_code == 0
    payload = json.loads(output_file.read_text(encoding="utf-8"))
    assert payload["certificationStatus"] == "certified"
    assert payload["governance"]["profile"] == "default"
    assert payload["compatibilityGate"]["status"] == "passed"


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
