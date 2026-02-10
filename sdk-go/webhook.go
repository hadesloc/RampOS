package rampos

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"
)

// WebhookEvent represents an incoming webhook event.
type WebhookEvent struct {
	ID        string                 `json:"id"`
	Type      string                 `json:"type"`
	Timestamp time.Time              `json:"timestamp"`
	Data      map[string]interface{} `json:"data"`
}

// WebhookVerifier verifies webhook signatures.
type WebhookVerifier struct {
	secret             string
	timestampTolerance time.Duration
}

// NewWebhookVerifier creates a new webhook signature verifier.
func NewWebhookVerifier(secret string) *WebhookVerifier {
	return &WebhookVerifier{
		secret:             secret,
		timestampTolerance: 5 * time.Minute,
	}
}

// WithTimestampTolerance sets the tolerance for timestamp validation.
func (v *WebhookVerifier) WithTimestampTolerance(d time.Duration) *WebhookVerifier {
	v.timestampTolerance = d
	return v
}

// Verify verifies the HMAC-SHA256 signature of a webhook payload.
// Matches the TypeScript SDK pattern: sha256=<hex digest>.
// This is the v1 signature format.
func (v *WebhookVerifier) Verify(payload string, signature string) bool {
	if payload == "" || signature == "" || v.secret == "" {
		return false
	}

	mac := hmac.New(sha256.New, []byte(v.secret))
	mac.Write([]byte(payload))
	digest := hex.EncodeToString(mac.Sum(nil))
	expected := "sha256=" + digest

	return hmac.Equal([]byte(signature), []byte(expected))
}

// VerifyAndParse verifies the webhook signature and parses the event.
func (v *WebhookVerifier) VerifyAndParse(payload []byte, signature string, timestamp string) (*WebhookEvent, error) {
	// Validate timestamp
	ts, err := strconv.ParseInt(timestamp, 10, 64)
	if err != nil {
		return nil, fmt.Errorf("invalid timestamp: %w", err)
	}

	eventTime := time.Unix(ts, 0)
	now := time.Now()

	if now.Sub(eventTime) > v.timestampTolerance {
		return nil, fmt.Errorf("timestamp too old: %v", eventTime)
	}

	if eventTime.Sub(now) > v.timestampTolerance {
		return nil, fmt.Errorf("timestamp too far in future: %v", eventTime)
	}

	// Verify signature - support both formats
	if strings.HasPrefix(signature, "sha256=") {
		// v1 format: sha256=<hex>
		if !v.Verify(string(payload), signature) {
			return nil, fmt.Errorf("signature mismatch")
		}
	} else {
		// Legacy format: timestamp.payload
		expectedSig := v.computeSignature(payload, timestamp)
		if !hmac.Equal([]byte(signature), []byte(expectedSig)) {
			return nil, fmt.Errorf("signature mismatch")
		}
	}

	// Parse event
	var event WebhookEvent
	if err := json.Unmarshal(payload, &event); err != nil {
		return nil, fmt.Errorf("failed to parse webhook event: %w", err)
	}

	return &event, nil
}

// computeSignature computes the expected HMAC-SHA256 signature (legacy format).
func (v *WebhookVerifier) computeSignature(payload []byte, timestamp string) string {
	message := fmt.Sprintf("%s.%s", timestamp, string(payload))
	mac := hmac.New(sha256.New, []byte(v.secret))
	mac.Write([]byte(message))
	return hex.EncodeToString(mac.Sum(nil))
}

// WebhookEventTypes defines known webhook event types.
const (
	EventIntentPayinCreated    = "intent.payin.created"
	EventIntentPayinConfirmed  = "intent.payin.confirmed"
	EventIntentPayinExpired    = "intent.payin.expired"
	EventIntentPayinFailed     = "intent.payin.failed"
	EventIntentPayoutCreated   = "intent.payout.created"
	EventIntentPayoutCompleted = "intent.payout.completed"
	EventIntentPayoutFailed    = "intent.payout.failed"
	EventIntentTradeExecuted   = "intent.trade.executed"
	EventCaseCreated           = "case.created"
	EventCaseResolved          = "case.resolved"
)

// IsPayinEvent returns true if the event is a pay-in related event.
func (e *WebhookEvent) IsPayinEvent() bool {
	switch e.Type {
	case EventIntentPayinCreated, EventIntentPayinConfirmed,
		EventIntentPayinExpired, EventIntentPayinFailed:
		return true
	}
	return false
}

// IsPayoutEvent returns true if the event is a pay-out related event.
func (e *WebhookEvent) IsPayoutEvent() bool {
	switch e.Type {
	case EventIntentPayoutCreated, EventIntentPayoutCompleted, EventIntentPayoutFailed:
		return true
	}
	return false
}

// IsCaseEvent returns true if the event is a compliance case event.
func (e *WebhookEvent) IsCaseEvent() bool {
	switch e.Type {
	case EventCaseCreated, EventCaseResolved:
		return true
	}
	return false
}

// GetIntentID extracts the intent ID from the event data.
func (e *WebhookEvent) GetIntentID() string {
	if id, ok := e.Data["intentId"].(string); ok {
		return id
	}
	return ""
}

// GetUserID extracts the user ID from the event data.
func (e *WebhookEvent) GetUserID() string {
	if id, ok := e.Data["userId"].(string); ok {
		return id
	}
	return ""
}

// GetAmount extracts the amount from the event data.
func (e *WebhookEvent) GetAmount() int64 {
	if amount, ok := e.Data["amount"].(float64); ok {
		return int64(amount)
	}
	return 0
}
