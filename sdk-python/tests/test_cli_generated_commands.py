from __future__ import annotations

from rampos.cli.generated_commands import command_path, command_registry
from rampos.cli.manifest import CURATED_OPERATIONS, load_manifest


def test_command_registry_contains_curated_aliases() -> None:
    registry = command_registry(load_manifest())

    assert registry["portal.rfq.create"] == ("rfq", "create")
    assert registry["admin.rfq.list_open"] == ("rfq", "list-open")
    assert registry["lp.rfq.bid"] == ("lp", "rfq", "bid")
    assert registry["admin.licensing.upload"] == ("licensing", "upload")


def test_command_path_renders_human_readable_command() -> None:
    operation = next(item for item in CURATED_OPERATIONS if item.operation_id == "admin.bridge.routes")
    assert command_path(operation) == "bridge routes"
