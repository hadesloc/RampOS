from rampos.cli.main import main


def test_cli_main_exists() -> None:
    assert callable(main)


def test_sandbox_run_returns_blocked_non_zero_exit(monkeypatch, capsys) -> None:
    monkeypatch.setenv("RAMPOS_ADMIN_KEY", "test-admin-key")

    exit_code = main(
        [
            "sandbox",
            "run",
            "--tenant-id",
            "tenant_cli_test",
            "--preset-code",
            "default",
            "--scenario-code",
            "smoke",
            "--output",
            "json",
        ]
    )

    captured = capsys.readouterr()

    assert exit_code == 1
    assert '"status": "blocked"' in captured.out
    assert "not live" in captured.out.lower()
