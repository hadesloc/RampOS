# RampOS Go SDK

[![SDK CI](../../actions/workflows/sdk-generate.yml/badge.svg)](../../actions/workflows/sdk-generate.yml)
[![SDK Tests](../../actions/workflows/sdk-ci.yml/badge.svg)](../../actions/workflows/sdk-ci.yml)

Official Go SDK for interacting with the RampOS API. Idiomatic Go, stdlib only (net/http, crypto, encoding/json).

## Installation

```bash
go get github.com/rampos/sdk-go
```

## Quick Start

```go
package main

import (
	"context"
	"fmt"
	"log"

	rampos "github.com/rampos/sdk-go"
)

func main() {
	client := rampos.NewClient(
		"your-api-key",
		"your-api-secret",
		rampos.WithBaseURL("https://api.rampos.io"),
		rampos.WithTenantID("tenant_123"),
		rampos.WithRetry(rampos.DefaultRetryConfig()),
	)

	ctx := context.Background()

	// Create a Pay-In Intent
	payin, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
		UserID:        "user_123",
		AmountVND:     1000000,
		RailsProvider: "vietqr",
	})
	if err != nil {
		log.Fatalf("Failed to create payin: %v", err)
	}

	fmt.Printf("Created Pay-In: %s, Ref: %s\n", payin.IntentID, payin.ReferenceCode)
}
```

## Features

- **Idiomatic Go**: `context.Context` on all methods, error returns, functional options
- **HMAC-SHA256 Request Signing**: Automatic signature generation for every request
- **Retry with Exponential Backoff + Jitter**: Configurable retry for transient failures
- **Custom Error Types**: `APIError` with status codes for programmatic error handling
- **Webhook Verification**: HMAC v1 (`sha256=<hex>`) and legacy timestamp-based formats
- **Full API Coverage**: Intents, Users, Ledger, Compliance, Account Abstraction, Passkeys
- **Zero Dependencies**: Uses only Go stdlib (`net/http`, `crypto`, `encoding/json`)

## Contract-Driven Note

RampOS keeps the public SDKs aligned to the OpenAPI contract and uses the thin `rampos-cli` preview for bounded admin/operator flows that are not yet first-class SDK namespaces.

For example, the reconciliation workbench and evidence export surface currently live behind:

```bash
python scripts/rampos-cli.py reconciliation workbench
python scripts/rampos-cli.py reconciliation evidence --discrepancy-id <id>
```

Use `scripts/validate-openapi.sh` and `scripts/test-rampos-cli.sh` together when validating SDK/CLI drift locally.

## Authentication

The SDK uses API Key + Secret for authentication. HMAC-SHA256 signatures are automatically generated.

```go
client := rampos.NewClient("your-api-key", "your-api-secret")
```

### Client Options

```go
client := rampos.NewClient("key", "secret",
	rampos.WithBaseURL("https://custom.api.com"),       // Custom API URL
	rampos.WithTenantID("tenant_123"),                  // Multi-tenant
	rampos.WithHTTPClient(&http.Client{Timeout: 60*time.Second}), // Custom HTTP client
	rampos.WithRetry(rampos.RetryConfig{                // Retry config
		MaxRetries: 5,
		BaseDelay:  500 * time.Millisecond,
		MaxDelay:   10 * time.Second,
		RetryableStatusCodes: []int{429, 500, 502, 503, 504},
	}),
)
```

## API Reference

### Intents (Pay-In / Pay-Out)

```go
// Create Pay-In
payin, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
	UserID:        "user_123",
	AmountVND:     500000,
	RailsProvider: "vietqr",
})

// Confirm Pay-In
confirmed, err := client.ConfirmPayin(ctx, rampos.ConfirmPayinRequest{
	ReferenceCode: "PAYIN_REF_123",
	Status:        "FUNDS_CONFIRMED",
	BankTxID:      "BANK_TX_999",
	AmountVND:     500000,
})

// Create Pay-Out
payout, err := client.CreatePayout(ctx, rampos.CreatePayoutRequest{
	UserID:        "user_123",
	AmountVND:     200000,
	RailsProvider: "mock",
	BankAccount: rampos.BankAccount{
		BankCode:      "VCB",
		AccountNumber: "1234567890",
		AccountName:   "NGUYEN VAN A",
	},
})

// Get Intent by ID
intent, err := client.GetIntent(ctx, "intent_id_123")

// List Intents with filters
state := "COMPLETED"
intents, err := client.ListIntents(ctx, rampos.ListIntentsRequest{
	State: &state,
	Limit: 10,
})

// Or use sub-service syntax
payin, err := client.Payins.Create(ctx, &rampos.CreatePayinRequest{...})
payout, err := client.Payouts.Create(ctx, &rampos.CreatePayoutRequest{...})
```

### Users

```go
// Get User
user, err := client.Users.Get(ctx, "user_123")

// List Users
status := "ACTIVE"
users, err := client.Users.List(ctx, rampos.ListUsersParams{
	Status: &status,
	Limit:  50,
})

// Get User Balances
balances, err := client.Users.GetBalances(ctx, "user_123")
```

### Ledger

```go
// Get Ledger Entries
userID := "user_123"
entries, err := client.Ledger.GetEntries(ctx, rampos.LedgerEntriesParams{
	UserID: &userID,
	Limit:  100,
})

// Get Ledger Balances
currency := "VND"
balances, err := client.Ledger.GetBalances(ctx, rampos.LedgerBalancesParams{
	Currency: &currency,
})
```

### Compliance

```go
// List Cases
status := "OPEN"
cases, err := client.Compliance.ListCases(ctx, rampos.ListCasesParams{
	Status: &status,
})

// Get Case
caseDetail, err := client.Compliance.GetCase(ctx, "case_123")

// List Rules
rules, err := client.Compliance.ListRules(ctx)

// Create Rule
rule, err := client.Compliance.CreateRule(ctx, rampos.CreateRuleRequest{
	Name:     "High Value Alert",
	RuleType: "TRANSACTION",
	Severity: "HIGH",
	Enabled:  true,
	Conditions: []rampos.RuleCondition{
		{Field: "amount", Operator: "gt", Value: 100000000},
	},
	Actions: []rampos.RuleAction{
		{Type: "CREATE_CASE"},
	},
})
```

### Account Abstraction (ERC-4337)

```go
// Create Smart Account
account, err := client.AA.CreateAccount(ctx, rampos.CreateAccountParams{
	UserID:       "user_123",
	OwnerAddress: "0x1234...5678",
	ChainID:      1,
})

// Get Smart Account
account, err := client.AA.GetAccount(ctx, "0xabc...def")

// Create User Operation
userOp, err := client.AA.CreateUserOperation(ctx, rampos.UserOpParams{
	Sender:   "0xabc...def",
	ChainID:  1,
	CallData: "0x...",
})

// Estimate Gas
estimate, err := client.AA.EstimateGas(ctx, rampos.UserOpParams{
	Sender:   "0xabc...def",
	ChainID:  1,
	CallData: "0x...",
})
```

### Passkey Wallets (WebAuthn P256)

```go
// Create Passkey Wallet
wallet, err := client.Passkey.CreateWallet(ctx, rampos.CreatePasskeyWalletParams{
	UserID:       "user_123",
	CredentialID: "cred_abc",
	PublicKeyX:   "0x...",
	PublicKeyY:   "0x...",
	DisplayName:  "My Passkey",
})

// Register Credential
cred, err := client.Passkey.RegisterCredential(ctx, rampos.RegisterPasskeyParams{
	UserID:       "user_123",
	CredentialID: "cred_abc",
	PublicKeyX:   "0x...",
	PublicKeyY:   "0x...",
	DisplayName:  "My Key",
})

// Get Credentials
creds, err := client.Passkey.GetCredentials(ctx, "user_123")

// Sign Transaction
signed, err := client.Passkey.SignTransaction(ctx, rampos.SignTransactionParams{
	UserID:       "user_123",
	CredentialID: "cred_abc",
	UserOperation: rampos.SignTransactionUserOp{
		Sender:   "0x...",
		Nonce:    "1",
		CallData: "0x...",
	},
	Assertion: rampos.WebAuthnAssertion{
		AuthenticatorData: "0x...",
		ClientDataJSON:    "0x...",
		Signature:         rampos.PasskeySignature{R: "0x...", S: "0x..."},
		CredentialID:      "cred_abc",
	},
})

// Link Smart Account
err := client.Passkey.LinkSmartAccount(ctx, rampos.LinkSmartAccountParams{
	UserID:              "user_123",
	CredentialID:        "cred_abc",
	SmartAccountAddress: "0x...",
})

// Get Counterfactual Address
addr, err := client.Passkey.GetCounterfactualAddress(ctx, rampos.GetCounterfactualAddressParams{
	PublicKeyX: "0x...",
	PublicKeyY: "0x...",
})

// Deactivate Credential
err := client.Passkey.DeactivateCredential(ctx, "user_123", "cred_abc")
```

### Webhooks

```go
func handleWebhook(w http.ResponseWriter, r *http.Request) {
	verifier := rampos.NewWebhookVerifier("your-webhook-secret")

	body, _ := io.ReadAll(r.Body)
	signature := r.Header.Get("X-RampOS-Signature")
	timestamp := r.Header.Get("X-Timestamp")

	// Option 1: Verify + parse in one step
	event, err := verifier.VerifyAndParse(body, signature, timestamp)
	if err != nil {
		http.Error(w, "Invalid webhook", http.StatusUnauthorized)
		return
	}

	// Option 2: Simple signature verification (v1 sha256= format)
	if !verifier.Verify(string(body), signature) {
		http.Error(w, "Bad signature", http.StatusUnauthorized)
		return
	}

	switch event.Type {
	case rampos.EventIntentPayinConfirmed:
		fmt.Printf("Pay-in confirmed: %s\n", event.GetIntentID())
	case rampos.EventIntentPayoutCompleted:
		fmt.Printf("Pay-out completed: %s\n", event.GetIntentID())
	case rampos.EventCaseCreated:
		fmt.Printf("Compliance case: %s\n", event.GetIntentID())
	}

	w.WriteHeader(http.StatusOK)
}
```

## Error Handling

```go
if err != nil {
	if apiErr, ok := err.(*rampos.APIError); ok {
		fmt.Printf("API Error [%d] %s: %s\n", apiErr.StatusCode, apiErr.Code, apiErr.Message)
	} else {
		fmt.Printf("Network error: %v\n", err)
	}
}
```

## Retry Configuration

The SDK supports configurable retry with exponential backoff and jitter:

```go
client := rampos.NewClient("key", "secret",
	rampos.WithRetry(rampos.RetryConfig{
		MaxRetries:           3,                              // Max retry attempts
		BaseDelay:            1 * time.Second,                // Initial delay
		MaxDelay:             30 * time.Second,               // Max delay cap
		RetryableStatusCodes: []int{429, 500, 502, 503, 504}, // Status codes to retry
	}),
)

// Or use defaults:
client := rampos.NewClient("key", "secret",
	rampos.WithRetry(rampos.DefaultRetryConfig()),
)
```

## License

MIT

## SDK Generation & Drift Detection

This SDK is generated from the OpenAPI spec defined in `crates/ramp-api/src/openapi.rs`.

**CI Pipeline**: The `sdk-generate.yml` workflow automatically:
- Detects changes to the OpenAPI spec (openapi.rs, DTOs, handlers)
- Runs SDK tests across Go 1.21 and 1.22
- Fails if SDK code is stale relative to the spec

**Local validation**:
```bash
# Run drift detection locally
bash scripts/validate-openapi.sh
```

**When updating the API**:
1. Modify `crates/ramp-api/src/openapi.rs` with new endpoints/schemas
2. Update Go SDK types and client methods
3. Add/update tests
4. Run `bash scripts/validate-openapi.sh` to verify
5. Commit all changes together
