from __future__ import annotations

import pytest

from rampos.cli.app import build_parser
from rampos.cli.main import main


def test_dynamic_command_help_is_registered_for_curated_surfaces(capsys: pytest.CaptureFixture[str]) -> None:
    parser = build_parser()

    for argv in (
        ["rfq", "list-open", "--help"],
        ["lp", "rfq", "bid", "--help"],
        ["swap", "quote", "--help"],
        ["bridge", "routes", "--help"],
        ["licensing", "upload", "--help"],
    ):
        with pytest.raises(SystemExit) as exc:
            parser.parse_args(argv)
        assert exc.value.code == 0

    assert "usage:" in capsys.readouterr().out


def test_dynamic_command_help_is_registered_for_core_openapi_surfaces(
    capsys: pytest.CaptureFixture[str],
) -> None:
    parser = build_parser()

    for argv in (
        ["intents", "create-payin", "--help"],
        ["intents", "list", "--help"],
        ["users", "balances", "--help"],
        ["chain", "quote", "--help"],
    ):
        with pytest.raises(SystemExit) as exc:
            parser.parse_args(argv)
        assert exc.value.code == 0

    assert "usage:" in capsys.readouterr().out


def test_dynamic_command_dispatches_expected_request(monkeypatch: pytest.MonkeyPatch) -> None:
    captured: dict[str, object] = {}

    def fake_request_json(ctx, method: str, path: str, *, payload=None, require_operator: bool = False):
        captured["auth_mode"] = ctx.auth_mode
        captured["method"] = method
        captured["path"] = path
        captured["payload"] = payload
        captured["require_operator"] = require_operator
        return {"ok": True}

    monkeypatch.setattr("rampos.cli.app.request_json", fake_request_json)

    exit_code = main(
        [
            "rfq",
            "list-open",
            "--base-url",
            "https://api.example",
            "--auth-mode",
            "admin",
            "--admin-key",
            "test-admin-key",
        ]
    )

    assert exit_code == 0
    assert captured == {
        "auth_mode": "admin",
        "method": "GET",
        "path": "/v1/admin/rfq/open",
        "payload": None,
        "require_operator": False,
    }
