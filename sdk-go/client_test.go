package rampos_test

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	rampos "github.com/rampos/sdk-go"
)

func TestNewClient(t *testing.T) {
	client := rampos.NewClient("test-key", "test-secret")
	if client == nil {
		t.Fatal("expected client to be non-nil")
	}
}

func TestClientWithOptions(t *testing.T) {
	customHTTPClient := &http.Client{Timeout: 60 * time.Second}

	client := rampos.NewClient(
		"test-key",
		"test-secret",
		rampos.WithBaseURL("https://custom.api.com"),
		rampos.WithHTTPClient(customHTTPClient),
		rampos.WithTenantID("tenant_123"),
	)

	if client == nil {
		t.Fatal("expected client to be non-nil")
	}
}

func TestCreatePayin(t *testing.T) {
	// Create a test server
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/v1/intents/payin" {
			t.Errorf("expected /v1/intents/payin, got %s", r.URL.Path)
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		w.Write([]byte(`{
			"intentId": "intent_123",
			"referenceCode": "REF456",
			"expiresAt": "2026-01-23T12:00:00Z",
			"status": "PENDING_BANK"
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("test-key", "test-secret", rampos.WithBaseURL(server.URL))

	resp, err := client.CreatePayin(context.Background(), rampos.CreatePayinRequest{
		UserID:        "user_123",
		AmountVND:     1000000,
		RailsProvider: "mock",
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if resp.IntentID != "intent_123" {
		t.Errorf("expected intent_123, got %s", resp.IntentID)
	}

	if resp.ReferenceCode != "REF456" {
		t.Errorf("expected REF456, got %s", resp.ReferenceCode)
	}
}

func TestGetIntent(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			t.Errorf("expected GET, got %s", r.Method)
		}
		if r.URL.Path != "/v1/intents/intent_123" {
			t.Errorf("expected /v1/intents/intent_123, got %s", r.URL.Path)
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"id": "intent_123",
			"intentType": "PAYIN",
			"state": "BANK_CONFIRMED",
			"amount": "1000000",
			"currency": "VND",
			"stateHistory": [
				{"state": "CREATED", "timestamp": "2026-01-23T10:00:00Z"},
				{"state": "PENDING_BANK", "timestamp": "2026-01-23T10:01:00Z"},
				{"state": "BANK_CONFIRMED", "timestamp": "2026-01-23T10:05:00Z"}
			],
			"createdAt": "2026-01-23T10:00:00Z",
			"updatedAt": "2026-01-23T10:05:00Z"
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("test-key", "test-secret", rampos.WithBaseURL(server.URL))

	resp, err := client.GetIntent(context.Background(), "intent_123")

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if resp.ID != "intent_123" {
		t.Errorf("expected intent_123, got %s", resp.ID)
	}

	if resp.State != "BANK_CONFIRMED" {
		t.Errorf("expected BANK_CONFIRMED, got %s", resp.State)
	}

	if len(resp.StateHistory) != 3 {
		t.Errorf("expected 3 state history entries, got %d", len(resp.StateHistory))
	}
}

func TestAPIError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		w.Write([]byte(`{
			"code": "NOT_FOUND",
			"message": "Intent not found"
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("test-key", "test-secret", rampos.WithBaseURL(server.URL))

	_, err := client.GetIntent(context.Background(), "nonexistent")

	if err == nil {
		t.Fatal("expected error, got nil")
	}

	apiErr, ok := err.(*rampos.APIError)
	if !ok {
		t.Fatalf("expected APIError, got %T", err)
	}

	if apiErr.StatusCode != 404 {
		t.Errorf("expected 404, got %d", apiErr.StatusCode)
	}

	if apiErr.Code != "NOT_FOUND" {
		t.Errorf("expected NOT_FOUND, got %s", apiErr.Code)
	}
}

func TestWebhookVerifier(t *testing.T) {
	verifier := rampos.NewWebhookVerifier("test-secret")

	// Create a valid payload
	payload := []byte(`{"id":"evt_123","type":"intent.payin.confirmed","timestamp":"2026-01-23T10:00:00Z","data":{"intentId":"intent_123"}}`)
	timestamp := "1737626400" // Some timestamp

	// This test won't pass without a valid signature, but it tests the structure
	_, err := verifier.VerifyAndParse(payload, "invalid-sig", timestamp)
	if err == nil {
		t.Error("expected error for invalid signature")
	}
}

func TestWebhookEventTypes(t *testing.T) {
	event := &rampos.WebhookEvent{Type: rampos.EventIntentPayinConfirmed}

	if !event.IsPayinEvent() {
		t.Error("expected IsPayinEvent to return true")
	}

	if event.IsPayoutEvent() {
		t.Error("expected IsPayoutEvent to return false")
	}

	if event.IsCaseEvent() {
		t.Error("expected IsCaseEvent to return false")
	}
}
