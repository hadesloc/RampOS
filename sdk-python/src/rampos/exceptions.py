"""Custom exceptions for the RampOS SDK."""

from __future__ import annotations

from typing import Any


class RampOSError(Exception):
    """Base exception for all RampOS SDK errors."""

    def __init__(
        self,
        message: str,
        status_code: int | None = None,
        code: str | None = None,
        details: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.status_code = status_code
        self.code = code
        self.details = details or {}


class RampOSAuthError(RampOSError):
    """Raised when authentication fails (401/403)."""


class RampOSValidationError(RampOSError):
    """Raised when request validation fails (400/422)."""


class RampOSRateLimitError(RampOSError):
    """Raised when the API rate limit is exceeded (429)."""

    def __init__(
        self,
        message: str = "Rate limit exceeded",
        retry_after: float | None = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(message, status_code=429, **kwargs)
        self.retry_after = retry_after


class RampOSNotFoundError(RampOSError):
    """Raised when a resource is not found (404)."""
