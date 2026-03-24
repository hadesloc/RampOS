from __future__ import annotations

from rampos.cli.manifest import load_manifest, load_mcp_v1_manifest


def test_mcp_catalog_includes_delegation_prerequisites_snapshot() -> None:
    manifest = load_mcp_v1_manifest()
    operations = {operation["operation_id"]: operation for operation in manifest["operations"]}

    operation = operations["admin.venue_trust.delegation_prerequisites_snapshot"]

    assert operation["command"] == ["venue-trust", "delegation-prerequisites"]
    assert (
        operation["path"]
        == "/v1/admin/venue-trust/delegation-prerequisites/{subject_type}/{subject_id}/delegates/{delegate}"
    )
    assert operation["auth_mode"] == "admin"
    assert operation["method"] == "GET"
    assert operation["safety_class"] == "read"
    assert operation["mcp_exposure"] == "v1_default"
    assert operation["mcp_default"] is True
    assert operation["cli_runtime_exposed"] is False


def test_runtime_manifest_keeps_delegation_prerequisites_snapshot_catalog_only() -> None:
    runtime_manifest = load_manifest(runtime_only=True)

    assert "admin.venue_trust.delegation_prerequisites_snapshot" not in runtime_manifest["operation_ids"]
