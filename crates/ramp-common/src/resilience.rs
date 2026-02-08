//! Resilience patterns: CircuitBreaker and RetryPolicy
//!
//! Provides production-grade circuit breaking and retry with exponential backoff
//! for external service calls.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Too many failures - requests are rejected immediately
    Open,
    /// Testing recovery - one request allowed through
    HalfOpen,
}

/// Configuration for a circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// How long to wait before transitioning from Open to HalfOpen
    pub reset_timeout: Duration,
    /// Number of successes in HalfOpen before closing the circuit
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 2,
        }
    }
}

/// A circuit breaker that tracks failures and prevents cascading failures
/// to external services.
///
/// Thread-safe: uses interior mutability with Mutex for state transitions.
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Mutex<CircuitBreakerInner>,
    /// Service name for logging
    service_name: String,
}

struct CircuitBreakerInner {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    last_state_change: Instant,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given config
    pub fn new(service_name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(CircuitBreakerInner {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                last_state_change: Instant::now(),
            }),
            service_name: service_name.into(),
        }
    }

    /// Create a circuit breaker with default settings
    pub fn with_defaults(service_name: impl Into<String>) -> Self {
        Self::new(service_name, CircuitBreakerConfig::default())
    }

    /// Check if a request is allowed through the circuit breaker.
    /// Returns the current state. Callers should proceed only if this returns Ok.
    pub fn allow_request(&self) -> Result<(), CircuitBreakerError> {
        let mut inner = self.state.lock().unwrap();

        match inner.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if reset timeout has elapsed
                if inner.last_state_change.elapsed() >= self.config.reset_timeout {
                    inner.state = CircuitState::HalfOpen;
                    inner.success_count = 0;
                    inner.last_state_change = Instant::now();
                    tracing::info!(
                        service = %self.service_name,
                        "Circuit breaker transitioning to HalfOpen"
                    );
                    Ok(())
                } else {
                    Err(CircuitBreakerError::Open {
                        service: self.service_name.clone(),
                        remaining: self.config.reset_timeout - inner.last_state_change.elapsed(),
                    })
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let mut inner = self.state.lock().unwrap();

        match inner.state {
            CircuitState::HalfOpen => {
                inner.success_count += 1;
                if inner.success_count >= self.config.success_threshold {
                    inner.state = CircuitState::Closed;
                    inner.failure_count = 0;
                    inner.success_count = 0;
                    inner.last_state_change = Instant::now();
                    tracing::info!(
                        service = %self.service_name,
                        "Circuit breaker closed (recovered)"
                    );
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                inner.failure_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let mut inner = self.state.lock().unwrap();
        inner.last_failure_time = Some(Instant::now());

        match inner.state {
            CircuitState::Closed => {
                inner.failure_count += 1;
                if inner.failure_count >= self.config.failure_threshold {
                    inner.state = CircuitState::Open;
                    inner.last_state_change = Instant::now();
                    tracing::warn!(
                        service = %self.service_name,
                        failure_count = inner.failure_count,
                        "Circuit breaker opened due to failures"
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in HalfOpen goes back to Open
                inner.state = CircuitState::Open;
                inner.last_state_change = Instant::now();
                inner.success_count = 0;
                tracing::warn!(
                    service = %self.service_name,
                    "Circuit breaker re-opened from HalfOpen"
                );
            }
            CircuitState::Open => {}
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitState {
        self.state.lock().unwrap().state
    }

    /// Get current failure count
    pub fn failure_count(&self) -> u32 {
        self.state.lock().unwrap().failure_count
    }
}

/// Errors from the circuit breaker
#[derive(Debug, Clone)]
pub enum CircuitBreakerError {
    /// Circuit is open, requests are being rejected
    Open {
        service: String,
        remaining: Duration,
    },
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::Open { service, remaining } => {
                write!(
                    f,
                    "Circuit breaker open for service '{}', retry after {:.1}s",
                    service,
                    remaining.as_secs_f64()
                )
            }
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

// ---------------------------------------------------------------------------
// RetryPolicy
// ---------------------------------------------------------------------------

/// Configuration for retry with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retries (0 = no retries, just the initial attempt)
    pub max_retries: u32,
    /// Base delay between retries (doubles each attempt)
    pub base_delay: Duration,
    /// Maximum delay cap
    pub max_delay: Duration,
    /// Whether to add jitter to prevent thundering herd
    pub use_jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            use_jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(max_retries: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            use_jitter: true,
        }
    }

    /// Disable jitter
    pub fn without_jitter(mut self) -> Self {
        self.use_jitter = false;
        self
    }

    /// Calculate the delay for a given attempt (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_ms = self.base_delay.as_millis() as u64;
        let exp_delay_ms = base_ms.saturating_mul(1u64 << attempt.min(20));
        let max_ms = self.max_delay.as_millis() as u64;
        let capped_ms = exp_delay_ms.min(max_ms);

        if self.use_jitter {
            // Full jitter: uniform random between 0 and capped delay
            let jitter_ms = fastrand_u64() % (capped_ms.max(1));
            Duration::from_millis(jitter_ms)
        } else {
            Duration::from_millis(capped_ms)
        }
    }

    /// Returns true if the given attempt number is within retry limits
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

/// Simple pseudo-random u64 using thread-local state (no external dependency).
/// Good enough for jitter; not cryptographic.
fn fastrand_u64() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let tick = COUNTER.fetch_add(1, Ordering::Relaxed);
    // xorshift-style mix of time + counter
    let seed = {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        t ^ tick.wrapping_mul(6364136223846793005)
    };
    let mut x = seed;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    x.wrapping_mul(0x2545F4914F6CDD1D)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // CircuitBreaker tests
    // -----------------------------------------------------------------------

    #[test]
    fn circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::with_defaults("test-service");
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request().is_ok());
    }

    #[test]
    fn circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(
            "test-service",
            CircuitBreakerConfig {
                failure_threshold: 3,
                reset_timeout: Duration::from_secs(30),
                success_threshold: 1,
            },
        );

        // First 2 failures: still closed
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request().is_ok());

        // Third failure: opens
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.allow_request().is_err());
    }

    #[test]
    fn circuit_breaker_success_resets_failure_count() {
        let cb = CircuitBreaker::new(
            "test-service",
            CircuitBreakerConfig {
                failure_threshold: 3,
                reset_timeout: Duration::from_secs(30),
                success_threshold: 1,
            },
        );

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);

        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn circuit_breaker_transitions_to_half_open() {
        let cb = CircuitBreaker::new(
            "test-service",
            CircuitBreakerConfig {
                failure_threshold: 2,
                reset_timeout: Duration::from_millis(50),
                success_threshold: 1,
            },
        );

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for reset timeout
        std::thread::sleep(Duration::from_millis(60));

        // Should transition to HalfOpen on next allow_request
        assert!(cb.allow_request().is_ok());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn circuit_breaker_closes_from_half_open_on_success() {
        let cb = CircuitBreaker::new(
            "test-service",
            CircuitBreakerConfig {
                failure_threshold: 2,
                reset_timeout: Duration::from_millis(50),
                success_threshold: 1,
            },
        );

        cb.record_failure();
        cb.record_failure();

        std::thread::sleep(Duration::from_millis(60));
        let _ = cb.allow_request(); // transitions to HalfOpen

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn circuit_breaker_reopens_from_half_open_on_failure() {
        let cb = CircuitBreaker::new(
            "test-service",
            CircuitBreakerConfig {
                failure_threshold: 2,
                reset_timeout: Duration::from_millis(50),
                success_threshold: 2,
            },
        );

        cb.record_failure();
        cb.record_failure();

        std::thread::sleep(Duration::from_millis(60));
        let _ = cb.allow_request(); // transitions to HalfOpen

        cb.record_failure(); // back to Open
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn circuit_breaker_open_error_message() {
        let cb = CircuitBreaker::new(
            "stripe",
            CircuitBreakerConfig {
                failure_threshold: 1,
                reset_timeout: Duration::from_secs(30),
                success_threshold: 1,
            },
        );

        cb.record_failure();
        let err = cb.allow_request().unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("stripe"));
        assert!(msg.contains("Circuit breaker open"));
    }

    // -----------------------------------------------------------------------
    // RetryPolicy tests
    // -----------------------------------------------------------------------

    #[test]
    fn retry_policy_should_retry_within_limits() {
        let policy = RetryPolicy {
            max_retries: 3,
            ..Default::default()
        };

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
        assert!(!policy.should_retry(4));
    }

    #[test]
    fn retry_policy_exponential_delay_without_jitter() {
        let policy = RetryPolicy {
            max_retries: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            use_jitter: false,
        };

        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(800));
    }

    #[test]
    fn retry_policy_caps_at_max_delay() {
        let policy = RetryPolicy {
            max_retries: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            use_jitter: false,
        };

        // Attempt 3: 1 * 2^3 = 8s, capped at 5s
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(5));
        assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(5));
    }

    #[test]
    fn retry_policy_jitter_is_bounded() {
        let policy = RetryPolicy {
            max_retries: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(10),
            use_jitter: true,
        };

        // With jitter, delay for attempt 0 should be in [0, 1000ms)
        for _ in 0..100 {
            let delay = policy.delay_for_attempt(0);
            assert!(delay < Duration::from_millis(1000));
        }
    }

    #[test]
    fn retry_policy_default_values() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.base_delay, Duration::from_millis(500));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert!(policy.use_jitter);
    }

    #[test]
    fn retry_policy_without_jitter_builder() {
        let policy = RetryPolicy::default().without_jitter();
        assert!(!policy.use_jitter);
    }
}
