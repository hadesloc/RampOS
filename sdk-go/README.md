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

### Balances

#### Get User Balances
Check a user's balances.

```go
balances, err := client.GetUserBalances(ctx, "user_123")
for _, b := range balances.Balances {
    fmt.Printf("%s: %s\n", b.Currency, b.Balance)
}
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
