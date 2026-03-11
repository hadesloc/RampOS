"""Webhook signature verification for RampOS webhooks.

Supports two verification modes:
- HMAC v1: HMAC-SHA256 with shared secret (default)
- Ed25519 v2: Ed25519 public key signature verification

Current webhook envelope contract:
- top-level ``id``
- top-level ``type``
- top-level ``created_at``
- event-specific fields nested under ``data``
"""

from __future__ import annotations

import hashlib
import hmac
import time
from binascii import unhexlify


def verify_webhook_signature(payload: str, signature: str, secret: str) -> bool:
    """Verify a webhook payload signature (convenience function).

    This is a standalone helper that wraps WebhookVerifier.verify()
    for simple usage without instantiating the class.

    Args:
        payload: The raw request body as a string.
        signature: The signature header sent by RampOS.
        secret: The webhook signing secret.

    Returns:
        True if the signature is valid, False otherwise.

    Raises:
        ValueError: If any parameter is missing.
    """
    return WebhookVerifier.verify(payload, signature, secret)


class WebhookVerifier:
    """Verifies the signature of webhook payloads from RampOS."""

    @staticmethod
    def verify(payload: str, signature: str, secret: str) -> bool:
        """Verify a webhook payload using HMAC-SHA256 (v1).

        Args:
            payload: The raw request body as a string.
            signature: The signature header sent by RampOS (X-RampOS-Signature).
            secret: The webhook signing secret provided by RampOS.

        Returns:
            True if the signature is valid, False otherwise.

        Raises:
            ValueError: If any parameter is missing.
        """
        if not payload:
            raise ValueError("Payload is required")
        if not signature:
            raise ValueError("Signature is required")
        if not secret:
            raise ValueError("Secret is required")

        digest = hmac.new(
            secret.encode("utf-8"),
            payload.encode("utf-8"),
            hashlib.sha256,
        ).hexdigest()
        expected_signature = f"sha256={digest}"

        return hmac.compare_digest(signature, expected_signature)

    @staticmethod
    def verify_timestamped_v1(
        payload: str,
        signature_header: str,
        secret: str,
        tolerance_seconds: int = 300,
    ) -> bool:
        """Verify a timestamped HMAC v1 webhook header.

        Expected header format: ``t=<unix timestamp>,v1=<hex digest>``.
        The signature is computed over ``{timestamp}.{raw_body}``.
        """
        if not payload:
            raise ValueError("Payload is required")
        if not signature_header:
            raise ValueError("Signature is required")
        if not secret:
            raise ValueError("Secret is required")

        timestamp, signature = WebhookVerifier._parse_timestamped_v1_header(
            signature_header
        )
        now = int(time.time())
        if abs(now - int(timestamp)) > tolerance_seconds:
            return False

        digest = hmac.new(
            secret.encode("utf-8"),
            f"{timestamp}.{payload}".encode("utf-8"),
            hashlib.sha256,
        ).hexdigest()
        return hmac.compare_digest(signature, digest)

    @staticmethod
    def verify_timestamped_v1(payload: str, signature: str, secret: str) -> bool:
        """Verify the current RampOS timestamped HMAC v1 header.

        Expected format:
        - `t=<unix timestamp>,v1=<hex digest>`

        The digest is computed over `{timestamp}.{payload}` using HMAC-SHA256.
        """
        if not payload:
            raise ValueError("Payload is required")
        if not signature:
            raise ValueError("Signature is required")
        if not secret:
            raise ValueError("Secret is required")

        timestamp = ""
        digest = ""

        for part in signature.split(","):
            key, _, value = part.partition("=")
            key = key.strip()
            value = value.strip()
            if key == "t":
                timestamp = value
            elif key == "v1":
                digest = value

        if not timestamp:
            raise ValueError("Timestamp is required")
        if not digest:
            raise ValueError("v1 signature is required")

        signed_payload = f"{timestamp}.{payload}"
        expected_digest = hmac.new(
            secret.encode("utf-8"),
            signed_payload.encode("utf-8"),
            hashlib.sha256,
        ).hexdigest()

        return hmac.compare_digest(digest, expected_digest)

    @staticmethod
    def verify_ed25519(
        payload: str,
        signature_hex: str,
        public_key_hex: str,
    ) -> bool:
        """Verify a webhook payload using Ed25519 (v2).

        Uses the standard library hashlib/hmac for HMAC and attempts
        to use the `cryptography` library for Ed25519 if available,
        falling back to a pure Python implementation otherwise.

        Args:
            payload: The raw request body as a string.
            signature_hex: The Ed25519 signature as a hex string (128 chars = 64 bytes).
            public_key_hex: The Ed25519 public key as a hex string (64 chars = 32 bytes).

        Returns:
            True if the signature is valid, False otherwise.

        Raises:
            ValueError: If any parameter is missing or invalid.
            ImportError: If the `cryptography` package is not installed.
        """
        if not payload:
            raise ValueError("Payload is required")
        if not signature_hex:
            raise ValueError("Signature is required")
        if not public_key_hex:
            raise ValueError("Public key is required")

        try:
            signature_bytes = unhexlify(signature_hex)
            public_key_bytes = unhexlify(public_key_hex)
        except Exception as exc:
            raise ValueError(f"Invalid hex encoding: {exc}") from exc

        if len(signature_bytes) != 64:
            raise ValueError(
                f"Ed25519 signature must be 64 bytes, got {len(signature_bytes)}"
            )
        if len(public_key_bytes) != 32:
            raise ValueError(
                f"Ed25519 public key must be 32 bytes, got {len(public_key_bytes)}"
            )

        try:
            from cryptography.hazmat.primitives.asymmetric.ed25519 import (
                Ed25519PublicKey,
            )

            public_key = Ed25519PublicKey.from_public_bytes(public_key_bytes)
            try:
                public_key.verify(signature_bytes, payload.encode("utf-8"))
                return True
            except Exception:
                return False
        except ImportError:
            raise ImportError(
                "The 'cryptography' package is required for Ed25519 verification. "
                "Install it with: pip install cryptography"
            )

    @staticmethod
    def verify_auto(
        payload: str,
        signature: str,
        secret_or_public_key: str,
    ) -> bool:
        """Auto-detect signature version and verify.

        - If signature starts with 't=' and contains 'v1=', uses timestamped HMAC v1
        - If signature starts with 'sha256=', uses bare HMAC v1
        - If signature starts with 'ed25519:', uses Ed25519 v2
        - Otherwise falls back to bare HMAC v1

        Args:
            payload: The raw request body.
            signature: The signature header value.
            secret_or_public_key: HMAC secret (v1) or Ed25519 public key hex (v2).

        Returns:
            True if the signature is valid.
        """
        if not payload:
            raise ValueError("Payload is required")
        if not signature:
            raise ValueError("Signature is required")
        if not secret_or_public_key:
            raise ValueError("Secret or public key is required")

        if signature.startswith("t="):
            return WebhookVerifier.verify_timestamped_v1(
                payload, signature, secret_or_public_key
            )

        if signature.startswith("t=") and "v1=" in signature:
            return WebhookVerifier.verify_timestamped_v1(
                payload, signature, secret_or_public_key
            )

        if signature.startswith("ed25519:"):
            sig_hex = signature[len("ed25519:"):]
            return WebhookVerifier.verify_ed25519(
                payload, sig_hex, secret_or_public_key
            )

        return WebhookVerifier.verify(payload, signature, secret_or_public_key)

    @staticmethod
    def _parse_timestamped_v1_header(signature_header: str) -> tuple[str, str]:
        timestamp: str | None = None
        signature: str | None = None

        for part in signature_header.split(","):
            key, _, value = part.strip().partition("=")
            if key == "t":
                timestamp = value
            elif key == "v1":
                signature = value

        if not timestamp:
            raise ValueError("Timestamp is required in signature header")
        if not signature:
            raise ValueError("v1 signature is required in signature header")

        return timestamp, signature
