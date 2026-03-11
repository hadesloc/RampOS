from __future__ import annotations

from pathlib import Path


def test_cli_docs_and_manifest_scripts_exist() -> None:
    repo_root = Path(__file__).resolve().parents[2]

    assert (repo_root / "docs" / "cli" / "README.md").exists()
    assert (repo_root / "docs" / "cli" / "agent-usage.md").exists()
    assert (repo_root / "scripts" / "build-cli-manifest.py").exists()


def test_every_ready_feature_in_coverage_ledger_has_cli_command() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    ledger_path = repo_root / "docs" / "cli" / "coverage-ledger.md"
    lines = ledger_path.read_text(encoding="utf-8").splitlines()

    ready_rows = [line for line in lines if line.startswith("|") and "| READY |" in line]
    assert ready_rows

    for row in ready_rows:
        columns = [column.strip() for column in row.strip("|").split("|")]
        cli_command = columns[5]
        assert cli_command
