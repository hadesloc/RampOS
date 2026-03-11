from rampos.cli.main import main


def test_cli_main_exists() -> None:
    assert callable(main)
