package rampos

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"strconv"
	"testing"
	"time"
)

func TestCurrentWebhookEnvelopeParsesCreatedAt(t *testing.T) {
	secret := "whsec_current_contract"
	payload := []byte(`{"id":"evt_123","type":"intent.status.changed","created_at":"2026-03-09T01:00:00Z","data":{"intentId":"intent_123","newStatus":"FUNDS_CONFIRMED"}}`)
	timestamp := time.Now().Unix()
	timestampStr := strconv.FormatInt(timestamp, 10)

	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(timestampStr + "." + string(payload)))
	signature := "t=" + timestampStr + ",v1=" + hex.EncodeToString(mac.Sum(nil))

	verifier := NewWebhookVerifier(secret)
	event, err := verifier.VerifyAndParse(payload, signature, "")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if event.ID != "evt_123" {
		t.Fatalf("expected evt_123, got %s", event.ID)
	}
	if event.Type != EventIntentStatusChanged {
		t.Fatalf("expected %s, got %s", EventIntentStatusChanged, event.Type)
	}
	if event.CreatedAt.IsZero() {
		t.Fatal("expected created_at to be parsed")
	}
	if event.GetIntentID() != "intent_123" {
		t.Fatalf("expected intent_123, got %s", event.GetIntentID())
	}
}

func TestVerifyAndParseSupportsTimestampedV1Header(t *testing.T) {
	secret := "whsec_current_contract"
	payload := []byte(`{"id":"evt_risk_123","type":"risk.review.required","created_at":"2026-03-09T01:00:00Z","data":{"intentId":"intent_456"}}`)
	timestamp := time.Now().Unix()
	timestampStr := strconv.FormatInt(timestamp, 10)

	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(timestampStr + "." + string(payload)))
	signature := "t=" + timestampStr + ",v1=" + hex.EncodeToString(mac.Sum(nil))

	verifier := NewWebhookVerifier(secret)
	event, err := verifier.VerifyAndParse(payload, signature, "")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if event.Type != EventRiskReviewRequired {
		t.Fatalf("expected %s, got %s", EventRiskReviewRequired, event.Type)
	}
}
