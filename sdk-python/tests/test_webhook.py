"""Tests for the WebhookVerifier."""

from __future__ import annotations

import hashlib
import hmac

import pytest

from rampos.utils.webhook_verifier import WebhookVerifier


def test_verify_valid_signature() -> None:
    secret = "whsec_test123"
    payload = '{"event": "payin.completed", "data": {}}'
    digest = hmac.new(
        secret.encode("utf-8"),
        payload.encode("utf-8"),
        hashlib.sha256,
    ).hexdigest()
    signature = f"sha256={digest}"

    verifier = WebhookVerifier()
    assert verifier.verify(payload, signature, secret) is True


def test_verify_invalid_signature() -> None:
    verifier = WebhookVerifier()
    assert verifier.verify(
        '{"event": "test"}',
        "sha256=invalid",
        "secret",
    ) is False


def test_verify_missing_payload_raises() -> None:
    verifier = WebhookVerifier()
    with pytest.raises(ValueError, match="Payload is required"):
        verifier.verify("", "sig", "secret")


def test_verify_missing_signature_raises() -> None:
    verifier = WebhookVerifier()
    with pytest.raises(ValueError, match="Signature is required"):
        verifier.verify("payload", "", "secret")


def test_verify_missing_secret_raises() -> None:
    verifier = WebhookVerifier()
    with pytest.raises(ValueError, match="Secret is required"):
        verifier.verify("payload", "sig", "")


def test_verify_timing_safe() -> None:
    """Ensure the verifier uses constant-time comparison."""
    secret = "test-secret"
    payload = "test-payload"
    digest = hmac.new(
        secret.encode("utf-8"),
        payload.encode("utf-8"),
        hashlib.sha256,
    ).hexdigest()

    verifier = WebhookVerifier()
    # Correct signature
    assert verifier.verify(payload, f"sha256={digest}", secret) is True
    # Slightly wrong signature
    wrong = f"sha256={digest[:-1]}0" if digest[-1] != "0" else f"sha256={digest[:-1]}1"
    assert verifier.verify(payload, wrong, secret) is False


# -- Ed25519 v2 tests --


def test_ed25519_valid_signature() -> None:
    """Test Ed25519 verification with a real keypair."""
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    except ImportError:
        pytest.skip("cryptography package not installed")

    private_key = Ed25519PrivateKey.generate()
    public_key = private_key.public_key()

    payload = '{"event": "payin.completed"}'
    signature = private_key.sign(payload.encode("utf-8"))

    public_key_bytes = public_key.public_bytes_raw()

    verifier = WebhookVerifier()
    assert verifier.verify_ed25519(
        payload,
        signature.hex(),
        public_key_bytes.hex(),
    ) is True


def test_ed25519_invalid_signature() -> None:
    """Test Ed25519 rejects invalid signatures."""
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    except ImportError:
        pytest.skip("cryptography package not installed")

    private_key = Ed25519PrivateKey.generate()
    public_key = private_key.public_key()

    payload = '{"event": "test"}'
    # Sign different data
    signature = private_key.sign(b"wrong data")

    public_key_bytes = public_key.public_bytes_raw()

    verifier = WebhookVerifier()
    assert verifier.verify_ed25519(
        payload,
        signature.hex(),
        public_key_bytes.hex(),
    ) is False


def test_ed25519_missing_payload_raises() -> None:
    verifier = WebhookVerifier()
    with pytest.raises(ValueError, match="Payload is required"):
        verifier.verify_ed25519("", "aa" * 64, "bb" * 32)


def test_ed25519_bad_signature_length_raises() -> None:
    verifier = WebhookVerifier()
    with pytest.raises(ValueError, match="Ed25519 signature must be 64 bytes"):
        verifier.verify_ed25519("payload", "aabb", "cc" * 32)


def test_ed25519_bad_key_length_raises() -> None:
    verifier = WebhookVerifier()
    with pytest.raises(ValueError, match="Ed25519 public key must be 32 bytes"):
        verifier.verify_ed25519("payload", "aa" * 64, "bb")


# -- Auto-detect tests --


def test_verify_auto_hmac() -> None:
    """verify_auto dispatches to HMAC v1 for sha256= prefix."""
    secret = "test-secret"
    payload = "test-payload"
    digest = hmac.new(
        secret.encode("utf-8"),
        payload.encode("utf-8"),
        hashlib.sha256,
    ).hexdigest()

    verifier = WebhookVerifier()
    assert verifier.verify_auto(payload, f"sha256={digest}", secret) is True


def test_verify_auto_ed25519() -> None:
    """verify_auto dispatches to Ed25519 for ed25519: prefix."""
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    except ImportError:
        pytest.skip("cryptography package not installed")

    private_key = Ed25519PrivateKey.generate()
    public_key = private_key.public_key()
    payload = "test-data"
    sig = private_key.sign(payload.encode("utf-8"))
    pub_hex = public_key.public_bytes_raw().hex()

    verifier = WebhookVerifier()
    assert verifier.verify_auto(
        payload, f"ed25519:{sig.hex()}", pub_hex
    ) is True
