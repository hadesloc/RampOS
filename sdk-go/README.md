# RampOS Go SDK

Official Go SDK for interacting with the RampOS API.

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
		rampos.WithBaseURL("https://api.rampos.io"), // Optional
	)

	ctx := context.Background()

	// Create a Pay-In Intent
	payin, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
		UserID:        "user_123",
		AmountVND:     1000000,
		RailsProvider: "mock",
		Metadata: map[string]interface{}{
			"order_id": "ord_123",
		},
	})
	if err != nil {
		log.Fatalf("Failed to create payin: %v", err)
	}

	fmt.Printf("Created Pay-In: %s, Ref: %s\n", payin.IntentID, payin.ReferenceCode)
}
```

## Authentication

The SDK uses an API Key and Secret for authentication. Signatures are automatically generated for each request.

```go
client := rampos.NewClient("your-api-key", "your-api-secret")
```

## API Reference

### Examples

Complete examples are available in the [examples](./examples) directory.

#### Create Pay-In
```go
client := rampos.NewClient(
    rampos.WithAPIKey("your-api-key"),
    rampos.WithAPISecret("your-api-secret"),
)

intent, err := client.Payins.Create(context.Background(), &rampos.CreatePayinRequest{
    UserID:    "usr_123",
    AmountVND: 1000000,
})
```

#### Create Pay-Out
```go
intent, err := client.Payouts.Create(context.Background(), &rampos.CreatePayoutRequest{
    UserID:    "usr_123",
    AmountVND: 1000000,
    BankAccount: rampos.BankAccount{
        BankCode:      "970415",
        AccountNumber: "101000000000",
        AccountName:   "NGUYEN VAN A",
    },
})
```

### Intents

#### Create Pay-In
Create a new intent to receive funds from a user.

```go
payin, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
    UserID:        "user_123",
    AmountVND:     500000,
    RailsProvider: "vietqr",
})
```

#### Confirm Pay-In
Confirm that funds have been received.

```go
confirmed, err := client.ConfirmPayin(ctx, rampos.ConfirmPayinRequest{
    ReferenceCode: "PAYIN_REF_123",
    Status:        "FUNDS_CONFIRMED",
    BankTxID:      "BANK_TX_999",
    AmountVND:     500000,
})
```

#### Create Pay-Out
Create a new intent to send funds to a user.

```go
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
```

#### Get Intent
Retrieve details of an existing intent.

```go
intent, err := client.GetIntent(ctx, "intent_id_123")
```

#### List Intents
List intents with filtering options.

```go
state := "COMPLETED"
intents, err := client.ListIntents(ctx, rampos.ListIntentsRequest{
    UserID: nil, // optional
    State:  &state,
    Limit:  10,
})
```

### Users

#### Get User
Retrieve details of a specific user.

```go
user, err := client.Users.Get(ctx, "user_123")
if err != nil {
    log.Fatalf("Failed to get user: %v", err)
}
fmt.Printf("User: %s, KYC Status: %s\n", user.ID, user.KYCStatus)
```

#### List Users
List users with filtering options.

```go
status := "ACTIVE"
users, err := client.Users.List(ctx, rampos.ListUsersParams{
    Status: &status,
    Limit:  50,
    Offset: 0,
})
if err != nil {
    log.Fatalf("Failed to list users: %v", err)
}
for _, u := range users.Data {
    fmt.Printf("User: %s, Status: %s\n", u.ID, u.Status)
}
```

#### Get User Balances
Check a user's balances.

```go
balances, err := client.Users.GetBalances(ctx, "user_123")
if err != nil {
    log.Fatalf("Failed to get balances: %v", err)
}
for _, b := range balances.Balances {
    fmt.Printf("%s %s: Balance=%s, Available=%s\n", b.AccountType, b.Currency, b.Balance, b.Available)
}
```

### Ledger

#### Get Ledger Entries
Retrieve ledger entries with optional filtering.

```go
userID := "user_123"
entries, err := client.Ledger.GetEntries(ctx, rampos.LedgerEntriesParams{
    UserID: &userID,
    Limit:  100,
})
if err != nil {
    log.Fatalf("Failed to get ledger entries: %v", err)
}
for _, entry := range entries.Data {
    fmt.Printf("Entry: %s %s %s %s\n", entry.ID, entry.Direction, entry.Amount, entry.Currency)
}
```

#### Get Ledger Balances
Query ledger balances with filtering.

```go
currency := "VND"
balances, err := client.Ledger.GetBalances(ctx, rampos.LedgerBalancesParams{
    Currency: &currency,
    Limit:    50,
})
if err != nil {
    log.Fatalf("Failed to get ledger balances: %v", err)
}
for _, b := range balances.Data {
    fmt.Printf("User %s: %s %s\n", b.UserID, b.Balance, b.Currency)
}
```

### Compliance

#### List Compliance Cases
List compliance cases with filtering.

```go
status := "OPEN"
cases, err := client.Compliance.ListCases(ctx, rampos.ListCasesParams{
    Status: &status,
    Limit:  20,
})
if err != nil {
    log.Fatalf("Failed to list cases: %v", err)
}
for _, c := range cases.Data {
    fmt.Printf("Case %s: %s - %s\n", c.ID, c.CaseType, c.Status)
}
```

#### Get Compliance Case
Retrieve details of a specific compliance case.

```go
caseDetails, err := client.Compliance.GetCase(ctx, "case_123")
if err != nil {
    log.Fatalf("Failed to get case: %v", err)
}
fmt.Printf("Case: %s, User: %s, Severity: %s\n", caseDetails.ID, caseDetails.UserID, caseDetails.Severity)
```

#### List Compliance Rules
Retrieve all compliance rules.

```go
rules, err := client.Compliance.ListRules(ctx)
if err != nil {
    log.Fatalf("Failed to list rules: %v", err)
}
for _, r := range rules.Data {
    fmt.Printf("Rule %s: %s (enabled: %v)\n", r.ID, r.Name, r.Enabled)
}
```

#### Create Compliance Rule
Create a new compliance rule.

```go
rule, err := client.Compliance.CreateRule(ctx, rampos.CreateRuleRequest{
    Name:        "High Value Transaction Alert",
    Description: "Alert on transactions over 100M VND",
    RuleType:    "TRANSACTION",
    Severity:    "HIGH",
    Enabled:     true,
    Conditions: []rampos.RuleCondition{
        {Field: "amount", Operator: "gt", Value: 100000000},
    },
    Actions: []rampos.RuleAction{
        {Type: "CREATE_CASE", Params: map[string]interface{}{"autoAssign": true}},
    },
})
if err != nil {
    log.Fatalf("Failed to create rule: %v", err)
}
fmt.Printf("Created rule: %s\n", rule.Rule.ID)
```

### Account Abstraction (AA)

#### Create Smart Account
Create an ERC-4337 smart account for a user.

```go
account, err := client.AA.CreateAccount(ctx, rampos.CreateAccountParams{
    UserID:       "user_123",
    OwnerAddress: "0x1234567890abcdef1234567890abcdef12345678",
    ChainID:      1, // Ethereum mainnet
    AccountType:  "simple",
})
if err != nil {
    log.Fatalf("Failed to create account: %v", err)
}
fmt.Printf("Created smart account: %s\n", account.Account.Address)
```

#### Get Smart Account
Retrieve a smart account by address.

```go
account, err := client.AA.GetAccount(ctx, "0xabcdef1234567890abcdef1234567890abcdef12")
if err != nil {
    log.Fatalf("Failed to get account: %v", err)
}
fmt.Printf("Account: %s, Deployed: %v\n", account.Address, account.IsDeployed)
```

#### Create User Operation
Create a new ERC-4337 user operation.

```go
userOp, err := client.AA.CreateUserOperation(ctx, rampos.UserOpParams{
    Sender:   "0xabcdef1234567890abcdef1234567890abcdef12",
    ChainID:  1,
    CallData: "0x...", // Encoded call data
})
if err != nil {
    log.Fatalf("Failed to create user operation: %v", err)
}
fmt.Printf("UserOp Hash: %s\n", userOp.UserOpHash)
```

#### Estimate Gas
Estimate gas for a user operation.

```go
estimate, err := client.AA.EstimateGas(ctx, rampos.UserOpParams{
    Sender:   "0xabcdef1234567890abcdef1234567890abcdef12",
    ChainID:  1,
    CallData: "0x...",
})
if err != nil {
    log.Fatalf("Failed to estimate gas: %v", err)
}
fmt.Printf("Call Gas Limit: %s\n", estimate.CallGasLimit)
```

### Webhooks

Verify and parse incoming webhooks from RampOS.

```go
func handleWebhook(w http.ResponseWriter, r *http.Request) {
    verifier := rampos.NewWebhookVerifier("your-webhook-secret")

    // Read body
    body, _ := io.ReadAll(r.Body)
    signature := r.Header.Get("X-RampOS-Signature")
    timestamp := r.Header.Get("X-Timestamp")

    // Verify and parse
    event, err := verifier.VerifyAndParse(body, signature, timestamp)
    if err != nil {
        http.Error(w, "Invalid webhook", http.StatusUnauthorized)
        return
    }

    // Handle event based on type
    switch event.Type {
    case rampos.EventIntentPayinConfirmed:
        fmt.Printf("Pay-in confirmed: %s\n", event.GetIntentID())
    // ... handle other events
    }

    w.WriteHeader(http.StatusOK)
}
```

## Error Handling

The SDK returns `*rampos.APIError` for API-level errors (4xx, 5xx), which includes the status code and error message.

```go
if err != nil {
    if apiErr, ok := err.(*rampos.APIError); ok {
        fmt.Printf("API Error %d: %s\n", apiErr.StatusCode, apiErr.Message)
    } else {
        fmt.Printf("Network error: %v\n", err)
    }
}
```

## License

MIT
