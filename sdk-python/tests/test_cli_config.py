from argparse import Namespace
from pathlib import Path

import pytest

from rampos.cli.app import cmd_manifest_mcp_v1, cmd_sandbox_run
from rampos.cli.config import build_cli_context, save_config
from rampos.cli.errors import CliAuthError, CliHttpError
from rampos.cli.request import build_auth_headers


def test_profile_precedence_is_flag_then_env_then_profile_then_default(tmp_path: Path) -> None:
    config_path = tmp_path / "rampos-cli.json"
    save_config(
        {
            "profiles": {
                "default": {
                    "base_url": "https://profile.example",
                    "auth_mode": "portal",
                    "portal_token": "profile-token",
                }
            }
        },
        config_path=config_path,
    )

    args = Namespace(
        profile="default",
        base_url="https://flag.example",
        auth_mode=None,
        api_key=None,
        api_secret=None,
        admin_jwt=None,
        admin_key=None,
        admin_role=None,
        admin_user_id=None,
        portal_token=None,
        lp_key=None,
        tenant_id=None,
        output=None,
        compact=False,
        body=None,
        body_file=None,
        body_stdin=False,
        timeout=None,
        request_id=None,
        idempotency_key=None,
    )

    ctx = build_cli_context(
        args,
        environ={
            "RAMPOS_BASE_URL": "https://env.example",
            "RAMPOS_AUTH_MODE": "lp",
            "RAMPOS_PORTAL_TOKEN": "env-token",
        },
        config_path=config_path,
    )

    assert ctx.base_url == "https://flag.example"
    assert ctx.auth_mode == "lp"
    assert ctx.portal_token == "env-token"


def test_profile_auth_mode_defaults_when_flag_and_env_missing(tmp_path: Path) -> None:
    config_path = tmp_path / "rampos-cli.json"
    save_config(
        {
            "profiles": {
                "default": {
                    "auth_mode": "admin",
                    "admin_key": "profile-admin-key",
                }
            }
        },
        config_path=config_path,
    )

    args = Namespace(
        profile="default",
        base_url=None,
        auth_mode=None,
        api_key=None,
        api_secret=None,
        admin_jwt=None,
        admin_key=None,
        admin_role=None,
        admin_user_id=None,
        portal_token=None,
        lp_key=None,
        tenant_id=None,
        output=None,
        compact=False,
        body=None,
        body_file=None,
        body_stdin=False,
        timeout=None,
        request_id=None,
        idempotency_key=None,
    )

    ctx = build_cli_context(args, environ={}, config_path=config_path)

    assert ctx.auth_mode == "admin"
    assert ctx.admin_key == "profile-admin-key"


def test_build_auth_headers_supports_portal_and_lp_modes() -> None:
    portal_ctx = build_cli_context(
        Namespace(
            profile="default",
            base_url=None,
            auth_mode="portal",
            api_key=None,
            api_secret=None,
            admin_jwt=None,
            admin_key=None,
            admin_role=None,
            admin_user_id=None,
            portal_token="portal-token",
            lp_key=None,
            tenant_id="tenant-1",
            output=None,
            compact=False,
            body=None,
            body_file=None,
            body_stdin=False,
            timeout=None,
            request_id="req-1",
            idempotency_key="idem-1",
        ),
        environ={},
    )
    lp_ctx = build_cli_context(
        Namespace(
            profile="default",
            base_url=None,
            auth_mode="lp",
            api_key=None,
            api_secret=None,
            admin_key=None,
            admin_role=None,
            admin_user_id=None,
            portal_token=None,
            lp_key="lp-key",
            tenant_id=None,
            output=None,
            compact=False,
            body=None,
            body_file=None,
            body_stdin=False,
            timeout=None,
            request_id=None,
            idempotency_key=None,
        ),
        environ={},
    )

    portal_headers = build_auth_headers(portal_ctx)
    lp_headers = build_auth_headers(lp_ctx)

    assert portal_headers["Authorization"] == "Bearer portal-token"
    assert portal_headers["X-Tenant-ID"] == "tenant-1"
    assert portal_headers["X-Request-Id"] == "req-1"
    assert portal_headers["Idempotency-Key"] == "idem-1"
    assert lp_headers["X-LP-Key"] == "lp-key"


def test_build_auth_headers_prefers_admin_jwt_over_legacy_key() -> None:
    ctx = build_cli_context(
        Namespace(
            profile="default",
            base_url=None,
            auth_mode="admin",
            api_key=None,
            api_secret=None,
            admin_key="legacy-admin-key",
            admin_jwt="jwt-token-123",
            admin_role="operator",
            admin_user_id="admin-user-1",
            portal_token=None,
            lp_key=None,
            tenant_id="tenant-1",
            output=None,
            compact=False,
            body=None,
            body_file=None,
            body_stdin=False,
            timeout=None,
            request_id=None,
            idempotency_key=None,
        ),
        environ={},
    )

    headers = build_auth_headers(ctx, require_operator=True)

    assert headers["X-Admin-Authorization"] == "Bearer jwt-token-123"
    assert "X-Admin-Key" not in headers
    assert "X-Admin-User-Id" not in headers
    assert headers["X-Tenant-ID"] == "tenant-1"


def test_sandbox_run_does_not_mask_auth_errors(monkeypatch: pytest.MonkeyPatch) -> None:
    def fake_request_json(*args, **kwargs):
        raise CliAuthError("missing admin jwt")

    monkeypatch.setattr("rampos.cli.app.request_json", fake_request_json)

    args = Namespace(
        profile="default",
        base_url="https://api.example",
        auth_mode="admin",
        api_key=None,
        api_secret=None,
        admin_key=None,
        admin_jwt=None,
        admin_role="operator",
        admin_user_id=None,
        portal_token=None,
        lp_key=None,
        tenant_id="tenant-1",
        output="json",
        compact=False,
        body=None,
        body_file=None,
        body_stdin=False,
        timeout=None,
        request_id=None,
        idempotency_key=None,
        yes=False,
        dry_run=False,
        preset_code="BASELINE",
        scenario_code="PAYIN_BASELINE",
    )

    with pytest.raises(CliAuthError, match="missing admin jwt"):
        cmd_sandbox_run(args)


def test_sandbox_run_returns_truthful_placeholder_for_missing_backend(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}

    def fake_request_json(*args, **kwargs):
        raise CliHttpError("404 Not Found", status_code=404, body='{"error":"not found"}')

    def fake_print_output(payload, *, output, compact):
        captured["payload"] = payload
        captured["output"] = output
        captured["compact"] = compact

    monkeypatch.setattr("rampos.cli.app.request_json", fake_request_json)
    monkeypatch.setattr("rampos.cli.app.print_output", fake_print_output)

    args = Namespace(
        profile="default",
        base_url="https://api.example",
        auth_mode="admin",
        api_key=None,
        api_secret=None,
        admin_key="legacy-admin-key",
        admin_jwt=None,
        admin_role="operator",
        admin_user_id=None,
        portal_token=None,
        lp_key=None,
        tenant_id="tenant-1",
        output="json",
        compact=False,
        body=None,
        body_file=None,
        body_stdin=False,
        timeout=None,
        request_id=None,
        idempotency_key=None,
        yes=False,
        dry_run=False,
        preset_code="BASELINE",
        scenario_code="PAYIN_BASELINE",
    )

    exit_code = cmd_sandbox_run(args)

    assert exit_code == 1
    assert captured["output"] == "json"
    assert captured["compact"] is False
    payload = captured["payload"]
    assert isinstance(payload, dict)
    assert payload["status"] == "backend_unavailable"
    assert payload["contractStatus"] == "placeholder"
    assert payload["supportedAlternatives"] == ["sandbox seed", "sandbox replay"]


def test_manifest_mcp_v1_outputs_read_heavy_catalog(monkeypatch: pytest.MonkeyPatch) -> None:
    captured: dict[str, object] = {}

    def fake_print_output(payload, *, output, compact):
        captured["payload"] = payload
        captured["output"] = output
        captured["compact"] = compact

    monkeypatch.setattr("rampos.cli.app.print_output", fake_print_output)

    args = Namespace(output="json", compact=False)
    exit_code = cmd_manifest_mcp_v1(args)

    assert exit_code == 0
    assert captured["output"] == "json"
    payload = captured["payload"]
    assert isinstance(payload, dict)
    assert payload["catalog"] == "thin_mcp_v1"
    assert payload["contractStatus"] == "read_heavy_default"
    operation_ids = set(payload["operationIds"])
    assert "portal.watch.stream" in operation_ids
    assert "admin.reconciliation.workbench" in operation_ids
    assert "admin.venue_trust.lighter_readiness" in operation_ids
    assert "admin.venue_trust.cex_readiness" in operation_ids
    assert "admin.venue_trust.report_snapshot" in operation_ids
    assert "admin.venue_trust.report_export" in operation_ids
    assert "admin.bridge.transfer" not in operation_ids
