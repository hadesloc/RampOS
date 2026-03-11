from __future__ import annotations

from pathlib import Path


def test_cli_coverage_ledger_exists_and_tracks_required_surfaces() -> None:
    ledger = Path(__file__).resolve().parents[2] / "docs" / "cli" / "coverage-ledger.md"

    assert ledger.exists()

    text = ledger.read_text(encoding="utf-8")

    assert "UNKNOWN" not in text
    assert "| REST |" in text
    assert "| GRAPHQL |" in text
    assert "| WS |" in text

    required_features = [
        "RFQ auction marketplace",
        "LP RFQ bidding",
        "Swap console",
        "Admin bridge operations",
        "Licensing compatibility uploads",
        "Treasury workbench",
        "Settlement workbench",
        "Sandbox preset operations",
        "Travel Rule queue",
        "KYC passport queue",
        "KYB corporate graph",
        "Continuous rescreening",
    ]

    for feature in required_features:
        assert feature in text
