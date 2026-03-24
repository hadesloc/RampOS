from __future__ import annotations

from pathlib import Path

from rampos.cli.manifest import (
    CURATED_OPERATIONS,
    load_manifest,
    load_mcp_v1_manifest,
    parse_openapi_operation_ids,
)


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
    assert operations["admin.bridge.transfer"]["safety_class"] == "approval_bounded_write"
    assert operations["admin.bridge.transfer"]["mcp_default"] is False


def test_manifest_extracts_route_metadata_for_core_openapi_operations() -> None:
    manifest = load_manifest()
    operations = {operation["operation_id"]: operation for operation in manifest["operations"]}

    assert operations["create_payin"]["method"] == "POST"
    assert operations["create_payin"]["path"] == "/v1/intents/payin"
    assert operations["get_intent"]["method"] == "GET"
    assert operations["get_intent"]["path"] == "/v1/intents/{id}"


def test_mcp_v1_manifest_is_explicit_read_heavy_catalog() -> None:
    mcp_manifest = load_mcp_v1_manifest()
    operation_ids = set(mcp_manifest["operation_ids"])
    operations = {operation["operation_id"]: operation for operation in mcp_manifest["operations"]}

    required_ids = {
        "portal.watch.stream",
        "admin.reconciliation.workbench",
        "admin.reconciliation.evidence",
        "admin.treasury.workbench",
        "admin.webhooks.catalog",
        "admin.webhooks.history",
        "admin.certification.artifact",
        "admin.venue_trust.lighter_readiness",
        "admin.venue_trust.cex_readiness",
        "admin.venue_trust.report_snapshot",
        "admin.venue_trust.report_export",
    }
    assert required_ids.issubset(operation_ids)
    assert "admin.bridge.transfer" not in operation_ids
    assert "admin.rfq.finalize" not in operation_ids
    assert operations["portal.watch.stream"]["mcp_exposure"] == "v1_default"
    assert operations["admin.certification.artifact"]["safety_class"] == "read"
    assert operations["admin.venue_trust.report_export"]["path"] == "/v1/admin/venue-trust/reports/{subject_type}/{subject_id}/export"
