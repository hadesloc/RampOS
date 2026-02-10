"""Tests for HMAC signing utility."""

from __future__ import annotations

import hashlib
import hmac

from rampos.utils.hmac_signer import sign_request


def test_sign_request_format() -> None:
    result = sign_request(
        api_secret="secret123",
        method="POST",
        path="/intents/payin",
        body='{"amount": 1000}',
        timestamp=1700000000,
    )

    expected_message = 'POST\n/intents/payin\n1700000000\n{"amount": 1000}'
    expected = hmac.new(
        "secret123".encode("utf-8"),
        expected_message.encode("utf-8"),
        hashlib.sha256,
    ).hexdigest()

    assert result == expected


def test_sign_request_empty_body() -> None:
    result = sign_request(
        api_secret="secret",
        method="GET",
        path="/test",
        body="",
        timestamp=1700000000,
    )
    assert len(result) == 64
    assert all(c in "0123456789abcdef" for c in result)


def test_sign_request_deterministic() -> None:
    args = {
        "api_secret": "key",
        "method": "POST",
        "path": "/test",
        "body": "body",
        "timestamp": 123,
    }
    assert sign_request(**args) == sign_request(**args)


def test_sign_request_different_secrets_differ() -> None:
    common = {"method": "GET", "path": "/", "body": "", "timestamp": 0}
    sig1 = sign_request(api_secret="secret1", **common)
    sig2 = sign_request(api_secret="secret2", **common)
    assert sig1 != sig2
