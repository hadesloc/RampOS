from __future__ import annotations

from rampos.cli.manifest import ManifestOperation, load_manifest


def command_registry(manifest: dict[str, object] | None = None) -> dict[str, tuple[str, ...]]:
    active_manifest = manifest or load_manifest()
    operations = active_manifest["operations"]
    registry: dict[str, tuple[str, ...]] = {}
    for operation in operations:
        command = tuple(operation["command"])
        registry[operation["operation_id"]] = command
    return registry


def command_path(operation: ManifestOperation) -> str:
    return " ".join(operation.command)
