"""HMAC request signing for RampOS API authentication."""

from __future__ import annotations

import hashlib
import hmac


def sign_request(
    api_secret: str,
    method: str,
    path: str,
    body: str,
    timestamp: int,
) -> str:
    """Sign a request using HMAC-SHA256.

    Matches the TypeScript/Go SDK format: method\\npath\\ntimestamp\\nbody
    """
    message = f"{method}\n{path}\n{timestamp}\n{body}"
    return hmac.new(
        api_secret.encode("utf-8"),
        message.encode("utf-8"),
        hashlib.sha256,
    ).hexdigest()
