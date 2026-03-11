from __future__ import annotations

import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


OPENAPI_PATHS_BLOCK_RE = re.compile(r"paths\s*\((?P<body>.*?)\)\s*,\s*components\(", re.DOTALL)
IDENTIFIER_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\b")
UTOIPA_ROUTE_RE = re.compile(
    r"#\[utoipa::path\((?P<body>.*?)\)\]\s*(?:#\[[^\]]+\]\s*)*pub\s+async\s+fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    re.DOTALL,
)
METHOD_RE = re.compile(r"\b(get|post|put|delete|patch)\b", re.IGNORECASE)
PATH_RE = re.compile(r'path\s*=\s*"(?P<path>[^"]+)"')


@dataclass(frozen=True)
class ManifestOperation:
    operation_id: str
    command: tuple[str, ...]
    auth_mode: str
    contract_source: str
    method: str
    path: str

    def to_dict(self) -> dict[str, object]:
        payload = asdict(self)
        payload["command"] = list(self.command)
        return payload


CURATED_OPERATIONS: tuple[ManifestOperation, ...] = (
    ManifestOperation("portal.rfq.create", ("rfq", "create"), "portal", "CURATED", "POST", "/v1/portal/rfq"),
    ManifestOperation("portal.rfq.get", ("rfq", "get"), "portal", "CURATED", "GET", "/v1/portal/rfq/:id"),
    ManifestOperation("portal.rfq.accept", ("rfq", "accept"), "portal", "CURATED", "POST", "/v1/portal/rfq/:id/accept"),
    ManifestOperation("portal.rfq.cancel", ("rfq", "cancel"), "portal", "CURATED", "POST", "/v1/portal/rfq/:id/cancel"),
    ManifestOperation("admin.rfq.list_open", ("rfq", "list-open"), "admin", "CURATED", "GET", "/v1/admin/rfq/open"),
    ManifestOperation("admin.rfq.finalize", ("rfq", "finalize"), "admin", "CURATED", "POST", "/v1/admin/rfq/:id/finalize"),
    ManifestOperation("lp.rfq.bid", ("lp", "rfq", "bid"), "lp", "CURATED", "POST", "/v1/lp/rfq/:rfq_id/bid"),
    ManifestOperation("swap.quote", ("swap", "quote"), "api", "CURATED", "GET", "/v1/swap/quote"),
    ManifestOperation("swap.execute", ("swap", "execute"), "api", "CURATED", "POST", "/v1/swap/execute"),
    ManifestOperation("swap.history", ("swap", "history"), "api", "CURATED", "GET", "/v1/swap/history"),
    ManifestOperation("admin.bridge.routes", ("bridge", "routes"), "admin", "CURATED", "GET", "/v1/admin/bridge/routes"),
    ManifestOperation("admin.bridge.quote", ("bridge", "quote"), "admin", "CURATED", "GET", "/v1/admin/bridge/quote"),
    ManifestOperation("admin.bridge.transfer", ("bridge", "transfer"), "admin", "CURATED", "POST", "/v1/admin/bridge/transfer"),
    ManifestOperation("admin.licensing.upload", ("licensing", "upload"), "admin", "CURATED", "POST", "/v1/admin/licensing/upload"),
    ManifestOperation("admin.licensing.submissions", ("licensing", "submissions"), "admin", "CURATED", "GET", "/v1/admin/licensing/submissions"),
)


def default_repo_root(start: Path | None = None) -> Path:
    current = (start or Path(__file__)).resolve()
    for candidate in (current,) + tuple(current.parents):
        if (candidate / "crates" / "ramp-api" / "src" / "openapi.rs").exists():
            return candidate
    raise FileNotFoundError("Could not locate repo root containing crates/ramp-api/src/openapi.rs")


def parse_openapi_operation_ids(openapi_source: str) -> list[str]:
    match = OPENAPI_PATHS_BLOCK_RE.search(openapi_source)
    if not match:
        raise ValueError("Could not find paths(...) block in OpenAPI source")

    body = match.group("body")
    seen: list[str] = []
    for identifier in IDENTIFIER_RE.findall(body):
        if not identifier[0].islower():
            continue
        if identifier not in seen:
            seen.append(identifier)
    return seen


def map_openapi_operation_to_command(operation_id: str) -> tuple[str, ...]:
    aliases: dict[str, tuple[str, ...]] = {
        "create_payin": ("intents", "create-payin"),
        "confirm_payin": ("intents", "confirm-payin"),
        "create_payout": ("intents", "create-payout"),
        "list_intents": ("intents", "list"),
        "get_intent": ("intents", "get"),
        "list_entries": ("ledger", "entries"),
        "list_balances": ("ledger", "balances"),
        "get_dashboard": ("admin", "dashboard"),
        "health_check": ("health", "check"),
        "readiness_check": ("health", "ready"),
        "record_trade": ("events", "trade-executed"),
        "get_user_balances": ("users", "balances"),
        "get_user_balances_for_tenant": ("users", "balances"),
        "get_bridge_quote": ("chain", "quote"),
        "initiate_bridge": ("chain", "bridge"),
        "list_chains": ("chain", "list"),
        "get_chain_detail": ("chain", "get"),
    }
    if operation_id in aliases:
        return aliases[operation_id]

    verb_prefixes = (
        ("create_", "create"),
        ("confirm_", "confirm"),
        ("list_", "list"),
        ("get_", "get"),
        ("update_", "update"),
        ("delete_", "delete"),
        ("verify_", "verify"),
        ("provision_", "provision"),
        ("estimate_", "estimate"),
        ("handle_", "handle"),
        ("mint_", "mint"),
        ("burn_", "burn"),
    )
    for prefix, verb in verb_prefixes:
        if operation_id.startswith(prefix):
            noun = operation_id[len(prefix) :]
            return (noun.replace("_", "-"), verb)

    return tuple(part for part in operation_id.split("_") if part)


def infer_auth_mode(operation_id: str) -> str:
    if operation_id.startswith("admin.") or operation_id in {
        "list_cases",
        "get_case",
        "update_case",
        "get_case_stats",
        "list_users",
        "get_user",
        "get_dashboard",
    }:
        return "admin"
    if operation_id.startswith("portal."):
        return "portal"
    if operation_id.startswith("lp."):
        return "lp"
    return "api"


def extract_utoipa_route_metadata(repo_root: Path | None = None) -> dict[str, tuple[str, str]]:
    root = default_repo_root(repo_root)
    handlers_root = root / "crates" / "ramp-api" / "src" / "handlers"
    metadata: dict[str, tuple[str, str]] = {}

    for handler_path in handlers_root.rglob("*.rs"):
        source = handler_path.read_text(encoding="utf-8")
        for match in UTOIPA_ROUTE_RE.finditer(source):
            body = match.group("body")
            name = match.group("name")
            method_match = METHOD_RE.search(body)
            path_match = PATH_RE.search(body)
            if not method_match or not path_match:
                continue
            metadata[name] = (method_match.group(1).upper(), path_match.group("path"))

    return metadata


def build_manifest_operations(
    openapi_operation_ids: Iterable[str],
    *,
    route_metadata: dict[str, tuple[str, str]] | None = None,
    curated_operations: Iterable[ManifestOperation] = CURATED_OPERATIONS,
) -> list[ManifestOperation]:
    manifest: list[ManifestOperation] = []
    routes = route_metadata or {}

    for operation_id in openapi_operation_ids:
        method, path = routes.get(operation_id, ("UNKNOWN", "UNKNOWN"))
        manifest.append(
            ManifestOperation(
                operation_id=operation_id,
                command=map_openapi_operation_to_command(operation_id),
                auth_mode=infer_auth_mode(operation_id),
                contract_source="OPENAPI",
                method=method,
                path=path,
            )
        )

    existing = {operation.operation_id for operation in manifest}
    for operation in curated_operations:
        if operation.operation_id not in existing:
            manifest.append(operation)

    return manifest


def load_manifest(repo_root: Path | None = None) -> dict[str, object]:
    root = default_repo_root(repo_root)
    openapi_source = (root / "crates" / "ramp-api" / "src" / "openapi.rs").read_text(encoding="utf-8")
    openapi_operation_ids = parse_openapi_operation_ids(openapi_source)
    operations = build_manifest_operations(
        openapi_operation_ids,
        route_metadata=extract_utoipa_route_metadata(root),
    )
    return {
        "operation_ids": [operation.operation_id for operation in operations],
        "operations": [operation.to_dict() for operation in operations],
    }
