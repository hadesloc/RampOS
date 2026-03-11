from rampos.cli.context import CliContext
from rampos.cli.errors import CliAuthError, CliTransportError, CliUsageError, exit_code_for_error
from rampos.cli.request import load_body


def test_cli_errors_map_to_stable_exit_codes() -> None:
    assert exit_code_for_error(CliAuthError("missing auth")) == 3
    assert exit_code_for_error(CliTransportError("network")) == 4
    assert exit_code_for_error(CliUsageError("bad usage")) == 2


def test_load_body_supports_inline_and_stdin_json(tmp_path) -> None:
    inline_ctx = CliContext(
        profile="default",
        base_url="http://localhost:8080",
        auth_mode="admin",
        body='{"ok": true}',
    )
    stdin_ctx = CliContext(
        profile="default",
        base_url="http://localhost:8080",
        auth_mode="admin",
        body_stdin=True,
    )
    file_path = tmp_path / "payload.json"
    file_path.write_text('{"from_file": true}', encoding="utf-8")
    file_ctx = CliContext(
        profile="default",
        base_url="http://localhost:8080",
        auth_mode="admin",
        body_file=str(file_path),
    )

    assert load_body(inline_ctx) == {"ok": True}
    assert load_body(stdin_ctx, stdin_text='{"from_stdin": true}') == {"from_stdin": True}
    assert load_body(file_ctx) == {"from_file": True}
