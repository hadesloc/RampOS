from __future__ import annotations

from pathlib import Path

from rampos.cli.manifest import CURATED_OPERATIONS, load_manifest, parse_openapi_operation_ids


def test_parse_openapi_operation_ids_extracts_known_core_operations() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    openapi_source = (repo_root / "crates" / "ramp-api" / "src" / "openapi.rs").read_text(
        encoding="utf-8"
    )

    operation_ids = parse_openapi_operation_ids(openapi_source)

    assert "create_payin" in operation_ids
    assert "get_dashboard" in operation_ids
    assert "get_bridge_quote" in operation_ids
    assert "Intents" not in operation_ids


def test_manifest_covers_openapi_and_curated_required_operations() -> None:
    manifest = load_manifest()
    repo_root = Path(__file__).resolve().parents[2]
    openapi_source = (repo_root / "crates" / "ramp-api" / "src" / "openapi.rs").read_text(
        encoding="utf-8"
    )
    openapi_operation_ids = set(parse_openapi_operation_ids(openapi_source))
    required_curated = {operation.operation_id for operation in CURATED_OPERATIONS}

    missing = sorted((openapi_operation_ids | required_curated) - set(manifest["operation_ids"]))
    assert missing == []


def test_manifest_includes_required_curated_surface_details() -> None:
    manifest = load_manifest()
    operations = {operation["operation_id"]: operation for operation in manifest["operations"]}

    assert operations["portal.rfq.create"]["auth_mode"] == "portal"
    assert operations["lp.rfq.bid"]["path"] == "/v1/lp/rfq/:rfq_id/bid"
    assert operations["admin.bridge.transfer"]["method"] == "POST"
    assert operations["admin.licensing.upload"]["contract_source"] == "CURATED"


def test_manifest_extracts_route_metadata_for_core_openapi_operations() -> None:
    manifest = load_manifest()
    operations = {operation["operation_id"]: operation for operation in manifest["operations"]}

    assert operations["create_payin"]["method"] == "POST"
    assert operations["create_payin"]["path"] == "/v1/intents/payin"
    assert operations["get_intent"]["method"] == "GET"
    assert operations["get_intent"]["path"] == "/v1/intents/{id}"
