package rampos_test

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	rampos "github.com/rampos/sdk-go"
)

// ============================================================================
// Client Tests
// ============================================================================

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

func TestClientWithRetry(t *testing.T) {
	client := rampos.NewClient(
		"test-key",
		"test-secret",
		rampos.WithRetry(rampos.RetryConfig{
			MaxRetries: 5,
			BaseDelay:  100 * time.Millisecond,
			MaxDelay:   2 * time.Second,
		}),
	)

	if client == nil {
		t.Fatal("expected client to be non-nil")
	}
}

func TestHMACSignatureInRequest(t *testing.T) {
	var receivedHeaders http.Header

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedHeaders = r.Header.Clone()
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"id":"intent_1","intentType":"PAYIN","state":"CREATED","amount":"100","currency":"VND","stateHistory":[],"createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("my-key", "my-secret", rampos.WithBaseURL(server.URL))
	_, err := client.GetIntent(context.Background(), "intent_1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedHeaders.Get("Authorization") != "Bearer my-key" {
		t.Errorf("expected Authorization header, got %q", receivedHeaders.Get("Authorization"))
	}
	if receivedHeaders.Get("X-Signature") == "" {
		t.Error("expected X-Signature header to be set")
	}
	if receivedHeaders.Get("X-Timestamp") == "" {
		t.Error("expected X-Timestamp header to be set")
	}
}

func TestTenantIDHeader(t *testing.T) {
	var receivedHeaders http.Header

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedHeaders = r.Header.Clone()
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"id":"i1","intentType":"PAYIN","state":"CREATED","amount":"100","currency":"VND","stateHistory":[],"createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL), rampos.WithTenantID("tenant_abc"))
	_, _ = client.GetIntent(context.Background(), "i1")

	if receivedHeaders.Get("X-Tenant-ID") != "tenant_abc" {
		t.Errorf("expected X-Tenant-ID=tenant_abc, got %q", receivedHeaders.Get("X-Tenant-ID"))
	}
}

// ============================================================================
// Intent / PayIn / PayOut Tests
// ============================================================================

func TestCreatePayin(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/v1/intents/payin" {
			t.Errorf("expected /v1/intents/payin, got %s", r.URL.Path)
		}

		// Verify request body
		body, _ := io.ReadAll(r.Body)
		var req map[string]interface{}
		json.Unmarshal(body, &req)
		if req["userId"] != "user_123" {
			t.Errorf("expected userId=user_123, got %v", req["userId"])
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
	if resp.Status != "PENDING_BANK" {
		t.Errorf("expected PENDING_BANK, got %s", resp.Status)
	}
}

func TestConfirmPayin(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/v1/intents/payin/confirm" {
			t.Errorf("expected /v1/intents/payin/confirm, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"intentId":"intent_123","status":"BANK_CONFIRMED"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.ConfirmPayin(context.Background(), rampos.ConfirmPayinRequest{
		ReferenceCode: "REF456",
		Status:        "FUNDS_CONFIRMED",
		BankTxID:      "TX999",
		AmountVND:     1000000,
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.IntentID != "intent_123" {
		t.Errorf("expected intent_123, got %s", resp.IntentID)
	}
	if resp.Status != "BANK_CONFIRMED" {
		t.Errorf("expected BANK_CONFIRMED, got %s", resp.Status)
	}
}

func TestCreatePayout(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/v1/intents/payout" {
			t.Errorf("expected /v1/intents/payout, got %s", r.URL.Path)
		}

		body, _ := io.ReadAll(r.Body)
		var req map[string]interface{}
		json.Unmarshal(body, &req)
		bankAccount := req["bankAccount"].(map[string]interface{})
		if bankAccount["bankCode"] != "VCB" {
			t.Errorf("expected bankCode=VCB, got %v", bankAccount["bankCode"])
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		w.Write([]byte(`{"intentId":"payout_456","status":"PROCESSING"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.CreatePayout(context.Background(), rampos.CreatePayoutRequest{
		UserID:        "user_123",
		AmountVND:     500000,
		RailsProvider: "mock",
		BankAccount: rampos.BankAccount{
			BankCode:      "VCB",
			AccountNumber: "1234567890",
			AccountName:   "NGUYEN VAN A",
		},
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.IntentID != "payout_456" {
		t.Errorf("expected payout_456, got %s", resp.IntentID)
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

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
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

func TestListIntents(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			t.Errorf("expected GET, got %s", r.Method)
		}
		if !strings.HasPrefix(r.URL.Path, "/v1/intents") {
			t.Errorf("expected path starting with /v1/intents, got %s", r.URL.Path)
		}
		if r.URL.Query().Get("limit") != "10" {
			t.Errorf("expected limit=10, got %s", r.URL.Query().Get("limit"))
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"data": [
				{"id":"i1","intentType":"PAYIN","state":"COMPLETED","amount":"100","currency":"VND","stateHistory":[],"createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"},
				{"id":"i2","intentType":"PAYOUT","state":"COMPLETED","amount":"200","currency":"VND","stateHistory":[],"createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}
			],
			"pagination": {"limit": 10, "offset": 0, "hasMore": false}
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	state := "COMPLETED"
	resp, err := client.ListIntents(context.Background(), rampos.ListIntentsRequest{
		State: &state,
		Limit: 10,
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(resp.Data) != 2 {
		t.Errorf("expected 2 intents, got %d", len(resp.Data))
	}
}

func TestRecordTrade(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/v1/events/trade-executed" {
			t.Errorf("expected /v1/events/trade-executed, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"intentId":"trade_001","status":"RECORDED"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.RecordTrade(context.Background(), rampos.RecordTradeRequest{
		TradeID:     "t1",
		UserID:      "u1",
		Symbol:      "BTC/VND",
		Price:       "1500000000",
		VNDDelta:    -1500000000,
		CryptoDelta: "1.0",
		Timestamp:   "2026-01-01T00:00:00Z",
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.IntentID != "trade_001" {
		t.Errorf("expected trade_001, got %s", resp.IntentID)
	}
}

// ============================================================================
// Error Handling Tests
// ============================================================================

func TestAPIError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		w.Write([]byte(`{"code":"NOT_FOUND","message":"Intent not found"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
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

func TestAPIErrorUnauthorized(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		w.Write([]byte(`{"code":"UNAUTHORIZED","message":"Invalid API key"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("bad-key", "bad-secret", rampos.WithBaseURL(server.URL))
	_, err := client.GetIntent(context.Background(), "intent_1")

	if err == nil {
		t.Fatal("expected error, got nil")
	}

	apiErr, ok := err.(*rampos.APIError)
	if !ok {
		t.Fatalf("expected APIError, got %T: %v", err, err)
	}
	if apiErr.StatusCode != 401 {
		t.Errorf("expected 401, got %d", apiErr.StatusCode)
	}
}

func TestAPIErrorBadRequest(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		w.Write([]byte(`{"code":"VALIDATION_ERROR","message":"Invalid amount"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	_, err := client.CreatePayin(context.Background(), rampos.CreatePayinRequest{})

	if err == nil {
		t.Fatal("expected error, got nil")
	}
	apiErr, ok := err.(*rampos.APIError)
	if !ok {
		t.Fatalf("expected APIError, got %T", err)
	}
	if apiErr.Code != "VALIDATION_ERROR" {
		t.Errorf("expected VALIDATION_ERROR, got %s", apiErr.Code)
	}
}

func TestAPIErrorServerError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte(`{"code":"INTERNAL_ERROR","message":"Something went wrong"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	_, err := client.GetIntent(context.Background(), "intent_1")

	if err == nil {
		t.Fatal("expected error, got nil")
	}
	apiErr, ok := err.(*rampos.APIError)
	if !ok {
		t.Fatalf("expected APIError, got %T", err)
	}
	if apiErr.StatusCode != 500 {
		t.Errorf("expected 500, got %d", apiErr.StatusCode)
	}
}

func TestContextCancellation(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(2 * time.Second)
		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	ctx, cancel := context.WithTimeout(context.Background(), 50*time.Millisecond)
	defer cancel()

	_, err := client.GetIntent(ctx, "intent_1")
	if err == nil {
		t.Fatal("expected error due to context timeout")
	}
}

// ============================================================================
// Users Service Tests
// ============================================================================

func TestUsersGet(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			t.Errorf("expected GET, got %s", r.Method)
		}
		if r.URL.Path != "/v1/users/user_456" {
			t.Errorf("expected /v1/users/user_456, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"id": "user_456",
			"tenantId": "tenant_1",
			"kycStatus": "VERIFIED",
			"kycLevel": 2,
			"status": "ACTIVE",
			"createdAt": "2026-01-01T00:00:00Z",
			"updatedAt": "2026-01-01T00:00:00Z"
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	user, err := client.Users.Get(context.Background(), "user_456")

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if user.ID != "user_456" {
		t.Errorf("expected user_456, got %s", user.ID)
	}
	if user.KYCStatus != "VERIFIED" {
		t.Errorf("expected VERIFIED, got %s", user.KYCStatus)
	}
}

func TestUsersGetBalances(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/users/user_456/balances" {
			t.Errorf("expected /v1/users/user_456/balances, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"userId": "user_456",
			"balances": [
				{"accountType": "FIAT", "currency": "VND", "balance": "5000000", "available": "5000000", "locked": "0"},
				{"accountType": "CRYPTO", "currency": "BTC", "balance": "0.5", "available": "0.5", "locked": "0"}
			]
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	balances, err := client.Users.GetBalances(context.Background(), "user_456")

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(balances.Balances) != 2 {
		t.Errorf("expected 2 balances, got %d", len(balances.Balances))
	}
	if balances.Balances[0].Currency != "VND" {
		t.Errorf("expected VND, got %s", balances.Balances[0].Currency)
	}
}

func TestUsersList(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Query().Get("status") != "ACTIVE" {
			t.Errorf("expected status=ACTIVE, got %s", r.URL.Query().Get("status"))
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"data": [{"id":"u1","tenantId":"t1","kycStatus":"VERIFIED","kycLevel":1,"status":"ACTIVE","createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}],
			"pagination": {"limit": 20, "offset": 0, "hasMore": false}
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	status := "ACTIVE"
	resp, err := client.Users.List(context.Background(), rampos.ListUsersParams{
		Status: &status,
		Limit:  20,
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(resp.Data) != 1 {
		t.Errorf("expected 1 user, got %d", len(resp.Data))
	}
}

// ============================================================================
// Ledger Service Tests
// ============================================================================

func TestLedgerGetEntries(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			t.Errorf("expected GET, got %s", r.Method)
		}
		if !strings.HasPrefix(r.URL.Path, "/v1/ledger/entries") {
			t.Errorf("expected /v1/ledger/entries, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"data": [
				{"id":"le1","tenantId":"t1","userId":"u1","accountType":"FIAT","currency":"VND","amount":"100000","balanceAfter":"5100000","entryType":"CREDIT","direction":"CREDIT","createdAt":"2026-01-01T00:00:00Z"}
			],
			"pagination": {"limit": 100, "offset": 0, "hasMore": false}
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	userID := "u1"
	resp, err := client.Ledger.GetEntries(context.Background(), rampos.LedgerEntriesParams{
		UserID: &userID,
		Limit:  100,
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(resp.Data) != 1 {
		t.Errorf("expected 1 entry, got %d", len(resp.Data))
	}
	if resp.Data[0].Amount != "100000" {
		t.Errorf("expected amount=100000, got %s", resp.Data[0].Amount)
	}
}

func TestLedgerGetBalances(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"data": [
				{"userId":"u1","accountType":"FIAT","currency":"VND","balance":"5000000","available":"4500000","locked":"500000","updatedAt":"2026-01-01"}
			],
			"pagination": {"limit": 50, "offset": 0, "hasMore": false}
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	currency := "VND"
	resp, err := client.Ledger.GetBalances(context.Background(), rampos.LedgerBalancesParams{
		Currency: &currency,
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(resp.Data) != 1 {
		t.Errorf("expected 1 balance, got %d", len(resp.Data))
	}
}

// ============================================================================
// Compliance Service Tests
// ============================================================================

func TestComplianceListCases(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !strings.HasPrefix(r.URL.Path, "/v1/compliance/cases") {
			t.Errorf("expected /v1/compliance/cases, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"data": [
				{"id":"case_1","tenantId":"t1","userId":"u1","caseType":"AML","status":"OPEN","severity":"HIGH","description":"Suspicious","createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}
			],
			"pagination": {"limit": 20, "offset": 0, "hasMore": false}
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	status := "OPEN"
	resp, err := client.Compliance.ListCases(context.Background(), rampos.ListCasesParams{
		Status: &status,
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(resp.Data) != 1 {
		t.Errorf("expected 1 case, got %d", len(resp.Data))
	}
	if resp.Data[0].Severity != "HIGH" {
		t.Errorf("expected HIGH, got %s", resp.Data[0].Severity)
	}
}

func TestComplianceGetCase(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/compliance/cases/case_1" {
			t.Errorf("expected /v1/compliance/cases/case_1, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"id":"case_1","tenantId":"t1","userId":"u1","caseType":"AML","status":"OPEN","severity":"HIGH","description":"Suspicious tx","createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.Compliance.GetCase(context.Background(), "case_1")

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.ID != "case_1" {
		t.Errorf("expected case_1, got %s", resp.ID)
	}
}

func TestComplianceCreateRule(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			t.Errorf("expected POST, got %s", r.Method)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		w.Write([]byte(`{"rule":{"id":"rule_1","tenantId":"t1","name":"High Value","description":"Alert on high value","ruleType":"TRANSACTION","severity":"HIGH","enabled":true,"conditions":[],"actions":[],"createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.Compliance.CreateRule(context.Background(), rampos.CreateRuleRequest{
		Name:        "High Value",
		Description: "Alert on high value",
		RuleType:    "TRANSACTION",
		Severity:    "HIGH",
		Enabled:     true,
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.Rule.ID != "rule_1" {
		t.Errorf("expected rule_1, got %s", resp.Rule.ID)
	}
}

// ============================================================================
// Account Abstraction (AA) Tests
// ============================================================================

func TestAACreateAccount(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" || r.URL.Path != "/v1/aa/accounts" {
			t.Errorf("unexpected %s %s", r.Method, r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		w.Write([]byte(`{"account":{"address":"0xabc","tenantId":"t1","userId":"u1","ownerAddress":"0x123","chainId":1,"factoryAddress":"0xfac","entryPoint":"0xep","accountType":"simple","isDeployed":false,"createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.AA.CreateAccount(context.Background(), rampos.CreateAccountParams{
		UserID:       "u1",
		OwnerAddress: "0x123",
		ChainID:      1,
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.Account.Address != "0xabc" {
		t.Errorf("expected 0xabc, got %s", resp.Account.Address)
	}
}

func TestAAGetAccount(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/aa/accounts/0xabc" {
			t.Errorf("expected /v1/aa/accounts/0xabc, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"address":"0xabc","tenantId":"t1","userId":"u1","ownerAddress":"0x123","chainId":1,"factoryAddress":"0xfac","entryPoint":"0xep","accountType":"simple","isDeployed":true,"createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.AA.GetAccount(context.Background(), "0xabc")

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !resp.IsDeployed {
		t.Error("expected IsDeployed=true")
	}
}

func TestAAEstimateGas(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"callGasLimit":"100000","verificationGasLimit":"200000","preVerificationGas":"50000","maxFeePerGas":"20000000000","maxPriorityFeePerGas":"1500000000"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.AA.EstimateGas(context.Background(), rampos.UserOpParams{
		Sender:   "0xabc",
		ChainID:  1,
		CallData: "0x1234",
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.CallGasLimit != "100000" {
		t.Errorf("expected 100000, got %s", resp.CallGasLimit)
	}
}

// ============================================================================
// Passkey Service Tests
// ============================================================================

func TestPasskeyCreateWallet(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" || r.URL.Path != "/v1/aa/passkey/wallets" {
			t.Errorf("unexpected %s %s", r.Method, r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		w.Write([]byte(`{
			"credentialId": "cred_123",
			"smartAccountAddress": "0xwallet",
			"publicKeyX": "0xaaa",
			"publicKeyY": "0xbbb",
			"isDeployed": false,
			"createdAt": "2026-01-01T00:00:00Z"
		}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.Passkey.CreateWallet(context.Background(), rampos.CreatePasskeyWalletParams{
		UserID:       "u1",
		CredentialID: "cred_123",
		PublicKeyX:   "0xaaa",
		PublicKeyY:   "0xbbb",
		DisplayName:  "My Passkey",
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.SmartAccountAddress != "0xwallet" {
		t.Errorf("expected 0xwallet, got %s", resp.SmartAccountAddress)
	}
	if resp.CredentialID != "cred_123" {
		t.Errorf("expected cred_123, got %s", resp.CredentialID)
	}
}

func TestPasskeyRegisterCredential(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/aa/passkey/credentials" {
			t.Errorf("expected /v1/aa/passkey/credentials, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		w.Write([]byte(`{"credentialId":"cred_1","createdAt":"2026-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.Passkey.RegisterCredential(context.Background(), rampos.RegisterPasskeyParams{
		UserID:       "u1",
		CredentialID: "cred_1",
		PublicKeyX:   "0xaaa",
		PublicKeyY:   "0xbbb",
		DisplayName:  "Test Key",
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.CredentialID != "cred_1" {
		t.Errorf("expected cred_1, got %s", resp.CredentialID)
	}
}

func TestPasskeyGetCredentials(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/aa/passkey/credentials/user_1" {
			t.Errorf("expected /v1/aa/passkey/credentials/user_1, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`[
			{"credentialId":"c1","userId":"user_1","publicKeyX":"0xa","publicKeyY":"0xb","displayName":"Key 1","isActive":true,"createdAt":"2026-01-01T00:00:00Z"},
			{"credentialId":"c2","userId":"user_1","publicKeyX":"0xc","publicKeyY":"0xd","displayName":"Key 2","isActive":true,"createdAt":"2026-01-02T00:00:00Z"}
		]`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	creds, err := client.Passkey.GetCredentials(context.Background(), "user_1")

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(creds) != 2 {
		t.Errorf("expected 2 credentials, got %d", len(creds))
	}
}

func TestPasskeyGetCredential(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/aa/passkey/credentials/user_1/cred_1" {
			t.Errorf("expected /v1/aa/passkey/credentials/user_1/cred_1, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"credentialId":"cred_1","userId":"user_1","publicKeyX":"0xa","publicKeyY":"0xb","displayName":"Key 1","isActive":true,"createdAt":"2026-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	cred, err := client.Passkey.GetCredential(context.Background(), "user_1", "cred_1")

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cred.CredentialID != "cred_1" {
		t.Errorf("expected cred_1, got %s", cred.CredentialID)
	}
	if !cred.IsActive {
		t.Error("expected IsActive=true")
	}
}

func TestPasskeySignTransaction(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/aa/passkey/sign" {
			t.Errorf("expected /v1/aa/passkey/sign, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"userOpHash":"0xhash","sender":"0xsender","nonce":"1","signature":"0xsig","status":"SUBMITTED"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.Passkey.SignTransaction(context.Background(), rampos.SignTransactionParams{
		UserID:       "u1",
		CredentialID: "c1",
		UserOperation: rampos.SignTransactionUserOp{
			Sender:   "0xsender",
			Nonce:    "1",
			CallData: "0xcalldata",
		},
		Assertion: rampos.WebAuthnAssertion{
			AuthenticatorData: "0xauth",
			ClientDataJSON:    "0xclient",
			Signature:         rampos.PasskeySignature{R: "0xr", S: "0xs"},
			CredentialID:      "c1",
		},
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.UserOpHash != "0xhash" {
		t.Errorf("expected 0xhash, got %s", resp.UserOpHash)
	}
}

func TestPasskeyLinkSmartAccount(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" || r.URL.Path != "/v1/aa/passkey/link" {
			t.Errorf("unexpected %s %s", r.Method, r.URL.Path)
		}
		w.WriteHeader(http.StatusNoContent)
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	err := client.Passkey.LinkSmartAccount(context.Background(), rampos.LinkSmartAccountParams{
		UserID:              "u1",
		CredentialID:        "c1",
		SmartAccountAddress: "0xwallet",
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestPasskeyDeactivateCredential(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "DELETE" {
			t.Errorf("expected DELETE, got %s", r.Method)
		}
		if r.URL.Path != "/v1/aa/passkey/credentials/user_1/cred_1" {
			t.Errorf("expected /v1/aa/passkey/credentials/user_1/cred_1, got %s", r.URL.Path)
		}
		w.WriteHeader(http.StatusNoContent)
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	err := client.Passkey.DeactivateCredential(context.Background(), "user_1", "cred_1")

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestPasskeyGetCounterfactualAddress(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/aa/passkey/address" {
			t.Errorf("expected /v1/aa/passkey/address, got %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"address":"0xcounterfactual","isDeployed":false}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.Passkey.GetCounterfactualAddress(context.Background(), rampos.GetCounterfactualAddressParams{
		PublicKeyX: "0xaaa",
		PublicKeyY: "0xbbb",
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.Address != "0xcounterfactual" {
		t.Errorf("expected 0xcounterfactual, got %s", resp.Address)
	}
}

// ============================================================================
// Webhook Tests
// ============================================================================

func TestWebhookVerifyV1(t *testing.T) {
	secret := "webhook-secret-v1"
	payload := `{"id":"evt_1","type":"intent.payin.confirmed","data":{"intentId":"i1"}}`

	// Compute valid v1 signature
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(payload))
	digest := hex.EncodeToString(mac.Sum(nil))
	signature := "sha256=" + digest

	verifier := rampos.NewWebhookVerifier(secret)

	if !verifier.Verify(payload, signature) {
		t.Error("expected Verify to return true for valid signature")
	}
}

func TestWebhookVerifyV1Invalid(t *testing.T) {
	verifier := rampos.NewWebhookVerifier("secret")

	if verifier.Verify(`{"test":true}`, "sha256=invalid") {
		t.Error("expected Verify to return false for invalid signature")
	}
}

func TestWebhookVerifyEmptyInputs(t *testing.T) {
	verifier := rampos.NewWebhookVerifier("secret")

	if verifier.Verify("", "sha256=abc") {
		t.Error("expected false for empty payload")
	}
	if verifier.Verify("payload", "") {
		t.Error("expected false for empty signature")
	}
}

func TestWebhookVerifyAndParse(t *testing.T) {
	secret := "test-secret"
	payload := `{"id":"evt_123","type":"intent.payin.confirmed","timestamp":"2026-01-23T10:00:00Z","data":{"intentId":"intent_123"}}`
	timestamp := fmt.Sprintf("%d", time.Now().Unix())

	// Compute legacy signature (timestamp.payload)
	message := fmt.Sprintf("%s.%s", timestamp, payload)
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(message))
	signature := hex.EncodeToString(mac.Sum(nil))

	verifier := rampos.NewWebhookVerifier(secret)
	event, err := verifier.VerifyAndParse([]byte(payload), signature, timestamp)

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if event.Type != "intent.payin.confirmed" {
		t.Errorf("expected intent.payin.confirmed, got %s", event.Type)
	}
	if event.GetIntentID() != "intent_123" {
		t.Errorf("expected intent_123, got %s", event.GetIntentID())
	}
}

func TestWebhookVerifyAndParseV1Format(t *testing.T) {
	secret := "v1-secret"
	payload := `{"id":"evt_2","type":"intent.payout.completed","data":{"intentId":"po_1","amount":500000}}`
	timestamp := fmt.Sprintf("%d", time.Now().Unix())

	// Compute v1 signature (sha256=<hex>)
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(payload))
	digest := hex.EncodeToString(mac.Sum(nil))
	signature := "sha256=" + digest

	verifier := rampos.NewWebhookVerifier(secret)
	event, err := verifier.VerifyAndParse([]byte(payload), signature, timestamp)

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if event.Type != "intent.payout.completed" {
		t.Errorf("expected intent.payout.completed, got %s", event.Type)
	}
}

func TestWebhookTimestampTooOld(t *testing.T) {
	verifier := rampos.NewWebhookVerifier("secret")
	oldTimestamp := fmt.Sprintf("%d", time.Now().Add(-10*time.Minute).Unix())

	_, err := verifier.VerifyAndParse([]byte(`{}`), "sig", oldTimestamp)
	if err == nil {
		t.Error("expected error for old timestamp")
	}
}

func TestWebhookEventTypes(t *testing.T) {
	tests := []struct {
		eventType string
		isPayin   bool
		isPayout  bool
		isCase    bool
	}{
		{rampos.EventIntentPayinCreated, true, false, false},
		{rampos.EventIntentPayinConfirmed, true, false, false},
		{rampos.EventIntentPayinExpired, true, false, false},
		{rampos.EventIntentPayinFailed, true, false, false},
		{rampos.EventIntentPayoutCreated, false, true, false},
		{rampos.EventIntentPayoutCompleted, false, true, false},
		{rampos.EventIntentPayoutFailed, false, true, false},
		{rampos.EventCaseCreated, false, false, true},
		{rampos.EventCaseResolved, false, false, true},
		{rampos.EventIntentTradeExecuted, false, false, false},
	}

	for _, tt := range tests {
		t.Run(tt.eventType, func(t *testing.T) {
			event := &rampos.WebhookEvent{Type: tt.eventType}
			if event.IsPayinEvent() != tt.isPayin {
				t.Errorf("IsPayinEvent: expected %v", tt.isPayin)
			}
			if event.IsPayoutEvent() != tt.isPayout {
				t.Errorf("IsPayoutEvent: expected %v", tt.isPayout)
			}
			if event.IsCaseEvent() != tt.isCase {
				t.Errorf("IsCaseEvent: expected %v", tt.isCase)
			}
		})
	}
}

func TestWebhookEventHelpers(t *testing.T) {
	event := &rampos.WebhookEvent{
		Type: rampos.EventIntentPayinConfirmed,
		Data: map[string]interface{}{
			"intentId": "i_123",
			"userId":   "u_456",
			"amount":   float64(1000000),
		},
	}

	if event.GetIntentID() != "i_123" {
		t.Errorf("expected i_123, got %s", event.GetIntentID())
	}
	if event.GetUserID() != "u_456" {
		t.Errorf("expected u_456, got %s", event.GetUserID())
	}
	if event.GetAmount() != 1000000 {
		t.Errorf("expected 1000000, got %d", event.GetAmount())
	}
}

func TestWebhookEventEmptyData(t *testing.T) {
	event := &rampos.WebhookEvent{
		Type: rampos.EventIntentPayinCreated,
		Data: map[string]interface{}{},
	}

	if event.GetIntentID() != "" {
		t.Error("expected empty intentId")
	}
	if event.GetUserID() != "" {
		t.Error("expected empty userId")
	}
	if event.GetAmount() != 0 {
		t.Error("expected 0 amount")
	}
}

// ============================================================================
// Retry Tests
// ============================================================================

func TestRetryDefaultConfig(t *testing.T) {
	config := rampos.DefaultRetryConfig()

	if config.MaxRetries != 3 {
		t.Errorf("expected MaxRetries=3, got %d", config.MaxRetries)
	}
	if config.BaseDelay != 1*time.Second {
		t.Errorf("expected BaseDelay=1s, got %v", config.BaseDelay)
	}
	if config.MaxDelay != 30*time.Second {
		t.Errorf("expected MaxDelay=30s, got %v", config.MaxDelay)
	}
}

func TestRetryOn500(t *testing.T) {
	callCount := 0

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		callCount++
		if callCount < 3 {
			w.WriteHeader(http.StatusInternalServerError)
			w.Write([]byte(`{"code":"INTERNAL","message":"oops"}`))
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"id":"i1","intentType":"PAYIN","state":"CREATED","amount":"100","currency":"VND","stateHistory":[],"createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret",
		rampos.WithBaseURL(server.URL),
		rampos.WithRetry(rampos.RetryConfig{
			MaxRetries:           3,
			BaseDelay:            10 * time.Millisecond,
			MaxDelay:             100 * time.Millisecond,
			RetryableStatusCodes: []int{500, 502, 503},
		}),
	)

	resp, err := client.GetIntent(context.Background(), "i1")
	if err != nil {
		t.Fatalf("expected success after retries, got error: %v", err)
	}
	if resp.ID != "i1" {
		t.Errorf("expected i1, got %s", resp.ID)
	}
	if callCount != 3 {
		t.Errorf("expected 3 calls, got %d", callCount)
	}
}

// ============================================================================
// PayinService / PayoutService alias tests
// ============================================================================

func TestPayinServiceCreate(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		w.Write([]byte(`{"intentId":"pi_via_service","referenceCode":"REF","expiresAt":"2026-01-23T12:00:00Z","status":"PENDING_BANK"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.Payins.Create(context.Background(), &rampos.CreatePayinRequest{
		UserID:        "u1",
		AmountVND:     100000,
		RailsProvider: "mock",
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.IntentID != "pi_via_service" {
		t.Errorf("expected pi_via_service, got %s", resp.IntentID)
	}
}

func TestPayoutServiceCreate(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		w.Write([]byte(`{"intentId":"po_via_service","status":"PROCESSING"}`))
	}))
	defer server.Close()

	client := rampos.NewClient("key", "secret", rampos.WithBaseURL(server.URL))
	resp, err := client.Payouts.Create(context.Background(), &rampos.CreatePayoutRequest{
		UserID:        "u1",
		AmountVND:     100000,
		RailsProvider: "mock",
		BankAccount:   rampos.BankAccount{BankCode: "VCB", AccountNumber: "123", AccountName: "A"},
	})

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.IntentID != "po_via_service" {
		t.Errorf("expected po_via_service, got %s", resp.IntentID)
	}
}
