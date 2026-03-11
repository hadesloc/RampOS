from argparse import Namespace
from pathlib import Path

from rampos.cli.config import build_cli_context, save_config
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
