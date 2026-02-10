package rampos

import (
	"context"
	"math"
	"math/rand"
	"net/http"
	"time"
)

// RetryConfig configures the retry behavior for API requests.
type RetryConfig struct {
	// MaxRetries is the maximum number of retry attempts (default: 3).
	MaxRetries int

	// BaseDelay is the initial delay between retries (default: 1s).
	BaseDelay time.Duration

	// MaxDelay caps the delay between retries (default: 30s).
	MaxDelay time.Duration

	// RetryableStatusCodes defines which HTTP status codes should trigger a retry.
	// Defaults to [429, 500, 502, 503, 504].
	RetryableStatusCodes []int
}

// DefaultRetryConfig returns a sensible default retry configuration.
func DefaultRetryConfig() RetryConfig {
	return RetryConfig{
		MaxRetries:           3,
		BaseDelay:            1 * time.Second,
		MaxDelay:             30 * time.Second,
		RetryableStatusCodes: []int{429, 500, 502, 503, 504},
	}
}

// isRetryable checks if a status code should be retried.
func (rc RetryConfig) isRetryable(statusCode int) bool {
	for _, code := range rc.RetryableStatusCodes {
		if statusCode == code {
			return true
		}
	}
	return false
}

// backoffDuration calculates the delay for a given attempt with exponential
// backoff and jitter.
func (rc RetryConfig) backoffDuration(attempt int) time.Duration {
	backoff := float64(rc.BaseDelay) * math.Pow(2, float64(attempt))
	if backoff > float64(rc.MaxDelay) {
		backoff = float64(rc.MaxDelay)
	}
	// Add jitter: random value between 0 and backoff
	jitter := rand.Float64() * backoff
	return time.Duration(jitter)
}

// retryableTransport wraps http.RoundTripper with retry logic.
type retryableTransport struct {
	base   http.RoundTripper
	config RetryConfig
}

// RoundTrip implements http.RoundTripper with retry logic.
func (t *retryableTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	var lastResp *http.Response
	var lastErr error

	for attempt := 0; attempt <= t.config.MaxRetries; attempt++ {
		if attempt > 0 {
			delay := t.config.backoffDuration(attempt - 1)
			select {
			case <-req.Context().Done():
				return nil, req.Context().Err()
			case <-time.After(delay):
			}

			// Clone the request for retry (body may have been consumed)
			if req.GetBody != nil {
				body, err := req.GetBody()
				if err != nil {
					return lastResp, err
				}
				req.Body = body
			}
		}

		resp, err := t.base.RoundTrip(req)
		if err != nil {
			// Network errors are retryable
			lastErr = err
			lastResp = resp
			continue
		}

		if !t.config.isRetryable(resp.StatusCode) {
			return resp, nil
		}

		// Close the response body before retrying
		resp.Body.Close()
		lastResp = resp
		lastErr = nil
	}

	if lastErr != nil {
		return lastResp, lastErr
	}
	return lastResp, nil
}

// WithRetry configures retry behavior for the client.
func WithRetry(config RetryConfig) ClientOption {
	return func(c *Client) {
		c.retryConfig = &config
	}
}

// wrapWithRetry wraps the http client transport with retry logic.
func wrapWithRetry(client *http.Client, config RetryConfig) {
	base := client.Transport
	if base == nil {
		base = http.DefaultTransport
	}
	client.Transport = &retryableTransport{
		base:   base,
		config: config,
	}
}

// doWithRetry executes fn with retry logic, respecting context cancellation.
func doWithRetry[T any](ctx context.Context, config RetryConfig, fn func() (T, int, error)) (T, error) {
	var zero T
	var lastErr error

	for attempt := 0; attempt <= config.MaxRetries; attempt++ {
		if attempt > 0 {
			delay := config.backoffDuration(attempt - 1)
			select {
			case <-ctx.Done():
				return zero, ctx.Err()
			case <-time.After(delay):
			}
		}

		result, statusCode, err := fn()
		if err == nil {
			return result, nil
		}

		if !config.isRetryable(statusCode) {
			return zero, err
		}

		lastErr = err
	}

	return zero, lastErr
}
