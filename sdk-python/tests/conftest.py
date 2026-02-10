"""Shared fixtures for RampOS SDK tests."""

from __future__ import annotations

import pytest
import httpx

from rampos.client import RampOSClient, RampOSConfig, RetryConfig


@pytest.fixture
def config() -> RampOSConfig:
    return RampOSConfig(
        api_key="test-api-key",
        api_secret="test-api-secret",
        base_url="https://api.test.rampos.io/v1",
        tenant_id="test-tenant-id",
        timeout=5.0,
        retry=RetryConfig(max_retries=1, base_delay=0.01),
    )


@pytest.fixture
def client(config: RampOSConfig) -> RampOSClient:
    return RampOSClient(config)
