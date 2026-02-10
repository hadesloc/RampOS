"""RampOS API client with HMAC signing, retry, and service namespaces."""

from __future__ import annotations

import time
import asyncio
from dataclasses import dataclass, field
from typing import Any
from urllib.parse import urlparse

import httpx

from rampos.exceptions import (
    RampOSAuthError,
    RampOSError,
    RampOSNotFoundError,
    RampOSRateLimitError,
    RampOSValidationError,
)
from rampos.services.aa import AAService
from rampos.services.compliance import ComplianceService
from rampos.services.domain import DomainService
from rampos.services.health import HealthService
from rampos.services.intent import IntentService
from rampos.services.ledger import LedgerService
from rampos.services.multichain import MultichainService
from rampos.services.payin import PayinService
from rampos.services.passkey import PasskeyService
from rampos.services.payout import PayoutService
from rampos.services.stablecoin import StablecoinService
from rampos.services.trade import TradeService
from rampos.services.user import UserService
from rampos.services.webhook import WebhookService
from rampos.utils.hmac_signer import sign_request
from rampos.utils.webhook_verifier import WebhookVerifier


@dataclass
class RetryConfig:
    max_retries: int = 3
    base_delay: float = 1.0


@dataclass
class RampOSConfig:
    api_key: str
    api_secret: str
    base_url: str = "https://api.rampos.io/v1"
    tenant_id: str | None = None
    timeout: float = 10.0
    retry: RetryConfig = field(default_factory=RetryConfig)


class RampOSClient:
    """Async Python client for the RampOS API.

    Usage::

        config = RampOSConfig(api_key="...", api_secret="...")
        client = RampOSClient(config)

        # Use service namespaces
        intent = await client.intents.create_payin(data)
        balances = await client.users.get_balances("user-123")

        # Close when done
        await client.close()

    Or as an async context manager::

        async with RampOSClient(config) as client:
            intent = await client.intents.create_payin(data)
    """

    def __init__(self, config: RampOSConfig) -> None:
        self._config = config
        self._http = httpx.AsyncClient(
            base_url=config.base_url,
            timeout=config.timeout,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {config.api_key}",
            },
            event_hooks={
                "request": [self._sign_request],
                "response": [self._handle_error_response],
            },
        )

        # Service namespaces (13 total - mirrors TypeScript SDK + backend handlers)
        self.intents = IntentService(self._http)
        self.payin = PayinService(self._http)
        self.payout = PayoutService(self._http)
        self.users = UserService(self._http)
        self.ledger = LedgerService(self._http)
        self.aa = AAService(self._http)
        self.passkey = PasskeyService(self._http)
        self.compliance = ComplianceService(self._http)
        self.trade = TradeService(self._http)
        self.stablecoin = StablecoinService(self._http)
        self.domains = DomainService(self._http)
        self.multichain = MultichainService(self._http)
        self.health = HealthService(self._http)
        self.webhook_service = WebhookService(self._http)
        self.webhooks = WebhookVerifier()

    async def __aenter__(self) -> RampOSClient:
        return self

    async def __aexit__(self, *args: Any) -> None:
        await self.close()

    async def close(self) -> None:
        """Close the underlying HTTP client."""
        await self._http.aclose()

    # -- Request signing -------------------------------------------------------

    async def _sign_request(self, request: httpx.Request) -> None:
        """Add HMAC signature headers to every outgoing request."""
        timestamp = int(time.time())
        method = request.method.upper()
        path = urlparse(str(request.url)).path

        body = ""
        if request.content:
            body = request.content.decode("utf-8")

        signature = sign_request(
            api_secret=self._config.api_secret,
            method=method,
            path=path,
            body=body,
            timestamp=timestamp,
        )

        request.headers["X-Timestamp"] = str(timestamp)
        request.headers["X-Signature"] = signature
        if self._config.tenant_id:
            request.headers["X-Tenant-ID"] = self._config.tenant_id

    # -- Error handling --------------------------------------------------------

    async def _handle_error_response(self, response: httpx.Response) -> None:
        """Map HTTP error status codes to typed SDK exceptions."""
        if response.is_success:
            return

        status = response.status_code
        try:
            body = response.json()
        except Exception:
            body = {}

        message = body.get("message", response.reason_phrase or "Unknown error")
        code = body.get("code")
        details = body.get("details")

        if status == 401 or status == 403:
            raise RampOSAuthError(message, status_code=status, code=code, details=details)
        if status == 400 or status == 422:
            raise RampOSValidationError(message, status_code=status, code=code, details=details)
        if status == 404:
            raise RampOSNotFoundError(message, status_code=status, code=code, details=details)
        if status == 429:
            retry_after = response.headers.get("Retry-After")
            raise RampOSRateLimitError(
                message,
                retry_after=float(retry_after) if retry_after else None,
                code=code,
                details=details,
            )
        if status >= 500:
            raise RampOSError(message, status_code=status, code=code, details=details)

    # -- Retry wrapper ---------------------------------------------------------

    async def request_with_retry(
        self,
        method: str,
        url: str,
        **kwargs: Any,
    ) -> httpx.Response:
        """Make an HTTP request with exponential backoff retry."""
        cfg = self._config.retry
        last_error: Exception | None = None

        for attempt in range(cfg.max_retries):
            try:
                response = await self._http.request(method, url, **kwargs)
                return response
            except (httpx.TransportError, RampOSError) as exc:
                last_error = exc
                if attempt < cfg.max_retries - 1:
                    delay = cfg.base_delay * (2 ** attempt)
                    await asyncio.sleep(delay)

        raise last_error  # type: ignore[misc]
