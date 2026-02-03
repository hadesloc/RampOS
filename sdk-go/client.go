// Package rampos provides a Go SDK for the RampOS API.
//
// RampOS is a BYOR (Bring Your Own Rails) crypto/VND exchange infrastructure.
// This SDK allows you to:
// - Create and manage pay-in/pay-out intents
// - Check user balances
// - Verify webhook signatures
//
// Example usage:
//
//	client := rampos.NewClient("your-api-key", "your-api-secret")
//	intent, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
//	    UserID:        "user_123",
//	    AmountVND:     1000000,
//	    RailsProvider: "mock",
//	})
package rampos

import (
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

const (
	// DefaultBaseURL is the default RampOS API base URL.
	DefaultBaseURL = "https://api.rampos.io"

	// DefaultTimeout is the default HTTP client timeout.
	DefaultTimeout = 30 * time.Second
)

// Client is the RampOS API client.
type Client struct {
	baseURL    string
	apiKey     string
	apiSecret  string
	httpClient *http.Client
	tenantID   string

	Payins     *PayinService
	Payouts    *PayoutService
	Users      *UsersService
	Ledger     *LedgerService
	Compliance *ComplianceService
	AA         *AAService
}

type PayinService struct {
	client *Client
}

func (s *PayinService) Create(ctx context.Context, req *CreatePayinRequest) (*CreatePayinResponse, error) {
	return s.client.CreatePayin(ctx, *req)
}

type PayoutService struct {
	client *Client
}

func (s *PayoutService) Create(ctx context.Context, req *CreatePayoutRequest) (*CreatePayoutResponse, error) {
	return s.client.CreatePayout(ctx, *req)
}

// ClientOption configures the Client.
type ClientOption func(*Client)

// WithBaseURL sets a custom base URL for the API.
func WithBaseURL(url string) ClientOption {
	return func(c *Client) {
		c.baseURL = url
	}
}

// WithHTTPClient sets a custom HTTP client.
func WithHTTPClient(client *http.Client) ClientOption {
	return func(c *Client) {
		c.httpClient = client
	}
}

// WithTenantID sets the tenant ID for multi-tenant environments.
func WithTenantID(tenantID string) ClientOption {
	return func(c *Client) {
		c.tenantID = tenantID
	}
}

// NewClient creates a new RampOS API client.
func NewClient(apiKey, apiSecret string, opts ...ClientOption) *Client {
	c := &Client{
		baseURL:   DefaultBaseURL,
		apiKey:    apiKey,
		apiSecret: apiSecret,
		httpClient: &http.Client{
			Timeout: DefaultTimeout,
		},
	}

	for _, opt := range opts {
		opt(c)
	}

	c.Payins = &PayinService{client: c}
	c.Payouts = &PayoutService{client: c}
	c.Users = &UsersService{client: c}
	c.Ledger = &LedgerService{client: c}
	c.Compliance = &ComplianceService{client: c}
	c.AA = &AAService{client: c}

	return c
}

// WithAPIKey sets the API key.
func WithAPIKey(apiKey string) ClientOption {
	return func(c *Client) {
		c.apiKey = apiKey
	}
}

// WithAPISecret sets the API secret.
func WithAPISecret(apiSecret string) ClientOption {
	return func(c *Client) {
		c.apiSecret = apiSecret
	}
}

// doRequest performs an HTTP request with authentication.
func (c *Client) doRequest(ctx context.Context, method, path string, body interface{}, result interface{}) error {
	var bodyReader io.Reader
	var bodyBytes []byte

	if body != nil {
		var err error
		bodyBytes, err = json.Marshal(body)
		if err != nil {
			return fmt.Errorf("failed to marshal request body: %w", err)
		}
		bodyReader = bytes.NewReader(bodyBytes)
	}

	url := c.baseURL + path
	req, err := http.NewRequestWithContext(ctx, method, url, bodyReader)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	// Set headers
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Authorization", "Bearer "+c.apiKey)

	// Add HMAC signature
	timestamp := time.Now().Unix()
	req.Header.Set("X-Timestamp", fmt.Sprintf("%d", timestamp))

	signature := c.signRequest(method, path, timestamp, bodyBytes)
	req.Header.Set("X-Signature", signature)

	if c.tenantID != "" {
		req.Header.Set("X-Tenant-ID", c.tenantID)
	}

	// Execute request
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	// Read response body
	// Limit response size to 10MB to prevent DoS
	limitedReader := io.LimitReader(resp.Body, 10*1024*1024)
	respBody, err := io.ReadAll(limitedReader)
	if err != nil {
		return fmt.Errorf("failed to read response: %w", err)
	}

	// Check for errors
	if resp.StatusCode >= 400 {
		var apiErr APIError
		if err := json.Unmarshal(respBody, &apiErr); err != nil {
			return fmt.Errorf("request failed with status %d: %s", resp.StatusCode, string(respBody))
		}
		apiErr.StatusCode = resp.StatusCode
		return &apiErr
	}

	// Parse result
	if result != nil && len(respBody) > 0 {
		if err := json.Unmarshal(respBody, result); err != nil {
			return fmt.Errorf("failed to parse response: %w", err)
		}
	}

	return nil
}

// signRequest creates an HMAC-SHA256 signature for the request.
func (c *Client) signRequest(method, path string, timestamp int64, body []byte) string {
	message := fmt.Sprintf("%s\n%s\n%d\n%s", method, path, timestamp, string(body))
	mac := hmac.New(sha256.New, []byte(c.apiSecret))
	mac.Write([]byte(message))
	return hex.EncodeToString(mac.Sum(nil))
}

// APIError represents an API error response.
type APIError struct {
	StatusCode int    `json:"-"`
	Code       string `json:"code"`
	Message    string `json:"message"`
}

func (e *APIError) Error() string {
	return fmt.Sprintf("RampOS API error [%d] %s: %s", e.StatusCode, e.Code, e.Message)
}
