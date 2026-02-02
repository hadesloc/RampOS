# RampOS Go SDK - Quickstart Guide

The RampOS Go SDK provides a type-safe client for integrating with the RampOS crypto/VND exchange infrastructure.

## Installation

```bash
go get github.com/rampos/sdk-go
```

## Requirements

- Go 1.21 or later

## Quick Start

### Initialize the Client

```go
package main

import (
    "context"
    "log"
    "os"

    rampos "github.com/rampos/sdk-go"
)

func main() {
    // Create a new client with your API credentials
    client := rampos.NewClient(
        os.Getenv("RAMPOS_API_KEY"),
        os.Getenv("RAMPOS_API_SECRET"),
    )

    // Use the client...
}
```

### Client Options

```go
// With custom base URL (for staging/testing)
client := rampos.NewClient(
    apiKey,
    apiSecret,
    rampos.WithBaseURL("https://staging-api.rampos.io"),
)

// With custom HTTP client
httpClient := &http.Client{
    Timeout: 60 * time.Second,
}
client := rampos.NewClient(
    apiKey,
    apiSecret,
    rampos.WithHTTPClient(httpClient),
)

// With tenant ID for multi-tenant environments
client := rampos.NewClient(
    apiKey,
    apiSecret,
    rampos.WithTenantID("tenant_123"),
)

// Combine multiple options
client := rampos.NewClient(
    apiKey,
    apiSecret,
    rampos.WithBaseURL("https://api.rampos.io"),
    rampos.WithTenantID("tenant_123"),
    rampos.WithHTTPClient(customClient),
)
```

## Create a Pay-In Intent

A pay-in intent initiates a fiat-to-crypto deposit flow.

```go
package main

import (
    "context"
    "fmt"
    "log"
    "time"

    rampos "github.com/rampos/sdk-go"
)

func main() {
    client := rampos.NewClient(
        "your-api-key",
        "your-api-secret",
        rampos.WithTenantID("your-tenant-id"),
    )

    ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
    defer cancel()

    // Create a pay-in intent for 1,000,000 VND
    resp, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
        UserID:        "user_123",
        AmountVND:     1000000,
        RailsProvider: "vietqr", // or "mock" for testing
        Metadata: map[string]interface{}{
            "orderId": "order_456",
        },
    })
    if err != nil {
        log.Fatalf("Failed to create pay-in: %v", err)
    }

    fmt.Printf("Intent ID: %s\n", resp.IntentID)
    fmt.Printf("Reference Code: %s\n", resp.ReferenceCode)
    fmt.Printf("Expires At: %s\n", resp.ExpiresAt)

    // Display virtual account for payment
    if resp.VirtualAccount != nil {
        fmt.Printf("Bank: %s\n", resp.VirtualAccount.Bank)
        fmt.Printf("Account Number: %s\n", resp.VirtualAccount.AccountNumber)
        fmt.Printf("Account Name: %s\n", resp.VirtualAccount.AccountName)
    }
}
```

## Confirm a Pay-In

After the bank confirms funds received, confirm the pay-in.

```go
func confirmPayIn(client *rampos.Client, referenceCode, bankTxID string) error {
    ctx := context.Background()

    resp, err := client.ConfirmPayin(ctx, rampos.ConfirmPayinRequest{
        ReferenceCode:  referenceCode,
        Status:         "FUNDS_CONFIRMED",
        BankTxID:       bankTxID,
        AmountVND:      1000000,
        SettledAt:      time.Now(),
        RawPayloadHash: "sha256-hash-of-bank-notification",
    })
    if err != nil {
        return fmt.Errorf("failed to confirm pay-in: %w", err)
    }

    fmt.Printf("Intent %s confirmed with status: %s\n", resp.IntentID, resp.Status)
    return nil
}
```

## Create a Pay-Out Intent

A pay-out intent initiates a crypto-to-fiat withdrawal.

```go
func createPayOut(client *rampos.Client, userID string, amount int64) error {
    ctx := context.Background()

    resp, err := client.CreatePayout(ctx, rampos.CreatePayoutRequest{
        UserID:        userID,
        AmountVND:     amount,
        RailsProvider: "vietqr",
        BankAccount: rampos.BankAccount{
            BankCode:      "VCB",
            AccountNumber: "1234567890",
            AccountName:   "NGUYEN VAN A",
        },
        Metadata: map[string]interface{}{
            "withdrawalId": "wd_789",
        },
    })
    if err != nil {
        return fmt.Errorf("failed to create pay-out: %w", err)
    }

    fmt.Printf("Pay-out intent created: %s\n", resp.IntentID)
    fmt.Printf("Status: %s\n", resp.Status)
    return nil
}
```

## Get Intent Status

```go
func getIntent(client *rampos.Client, intentID string) (*rampos.Intent, error) {
    ctx := context.Background()

    intent, err := client.GetIntent(ctx, intentID)
    if err != nil {
        return nil, fmt.Errorf("failed to get intent: %w", err)
    }

    fmt.Printf("Intent: %s\n", intent.ID)
    fmt.Printf("Type: %s\n", intent.IntentType)
    fmt.Printf("State: %s\n", intent.State)
    fmt.Printf("Amount: %s %s\n", intent.Amount, intent.Currency)

    // Check state history
    fmt.Println("State History:")
    for _, h := range intent.StateHistory {
        reason := ""
        if h.Reason != nil {
            reason = " - " + *h.Reason
        }
        fmt.Printf("  %s: %s%s\n", h.Timestamp, h.State, reason)
    }

    return intent, nil
}
```

## List Intents with Filters

```go
func listIntents(client *rampos.Client, userID string) error {
    ctx := context.Background()

    intentType := "PAY_IN"
    state := "COMPLETED"

    resp, err := client.ListIntents(ctx, rampos.ListIntentsRequest{
        UserID:     &userID,
        IntentType: &intentType,
        State:      &state,
        Limit:      50,
        Offset:     0,
    })
    if err != nil {
        return fmt.Errorf("failed to list intents: %w", err)
    }

    fmt.Printf("Found %d intents\n", len(resp.Data))
    for _, intent := range resp.Data {
        fmt.Printf("  %s: %s %s - %s\n",
            intent.ID, intent.Amount, intent.Currency, intent.State)
    }

    if resp.Pagination.HasMore {
        fmt.Println("More results available...")
    }

    return nil
}
```

## Get User Balances

```go
func getUserBalances(client *rampos.Client, userID string) error {
    ctx := context.Background()

    balances, err := client.GetUserBalances(ctx, userID)
    if err != nil {
        return fmt.Errorf("failed to get balances: %w", err)
    }

    fmt.Printf("User %s balances:\n", userID)
    for _, b := range balances.Balances {
        fmt.Printf("  %s (%s): %s\n", b.Currency, b.AccountType, b.Balance)
    }

    return nil
}
```

## Record a Trade

```go
func recordTrade(client *rampos.Client) error {
    ctx := context.Background()

    resp, err := client.RecordTrade(ctx, rampos.RecordTradeRequest{
        TradeID:     "trade_123",
        UserID:      "user_456",
        Symbol:      "BTC/VND",
        Price:       "1500000000", // 1.5 billion VND per BTC
        VNDDelta:    -1000000,     // User paid 1M VND
        CryptoDelta: "0.00066667", // Received ~0.00067 BTC
        Timestamp:   time.Now().Format(time.RFC3339),
    })
    if err != nil {
        return fmt.Errorf("failed to record trade: %w", err)
    }

    fmt.Printf("Trade recorded as intent: %s\n", resp.IntentID)
    return nil
}
```

## Handling Webhooks

### Setting Up the Webhook Handler

```go
package main

import (
    "encoding/json"
    "io"
    "log"
    "net/http"

    rampos "github.com/rampos/sdk-go"
)

func main() {
    // Create webhook verifier with your secret
    verifier := rampos.NewWebhookVerifier("your-webhook-secret")

    http.HandleFunc("/webhooks/rampos", func(w http.ResponseWriter, r *http.Request) {
        // Read the raw body
        body, err := io.ReadAll(r.Body)
        if err != nil {
            http.Error(w, "Failed to read body", http.StatusBadRequest)
            return
        }

        // Get headers
        signature := r.Header.Get("X-Signature")
        timestamp := r.Header.Get("X-Timestamp")

        // Verify and parse the event
        event, err := verifier.VerifyAndParse(body, signature, timestamp)
        if err != nil {
            log.Printf("Invalid webhook: %v", err)
            http.Error(w, "Invalid signature", http.StatusUnauthorized)
            return
        }

        // Handle the event based on type
        handleWebhookEvent(event)

        w.WriteHeader(http.StatusOK)
        w.Write([]byte("OK"))
    })

    log.Println("Webhook server listening on :8080")
    log.Fatal(http.ListenAndServe(":8080", nil))
}

func handleWebhookEvent(event *rampos.WebhookEvent) {
    log.Printf("Received event: %s (ID: %s)", event.Type, event.ID)

    switch event.Type {
    case rampos.EventIntentPayinCreated:
        log.Printf("Pay-in created: %s", event.GetIntentID())

    case rampos.EventIntentPayinConfirmed:
        log.Printf("Pay-in confirmed: %s for user %s, amount: %d",
            event.GetIntentID(), event.GetUserID(), event.GetAmount())
        // Credit user's account

    case rampos.EventIntentPayinExpired:
        log.Printf("Pay-in expired: %s", event.GetIntentID())

    case rampos.EventIntentPayinFailed:
        log.Printf("Pay-in failed: %s", event.GetIntentID())

    case rampos.EventIntentPayoutCreated:
        log.Printf("Pay-out created: %s", event.GetIntentID())

    case rampos.EventIntentPayoutCompleted:
        log.Printf("Pay-out completed: %s", event.GetIntentID())

    case rampos.EventIntentPayoutFailed:
        log.Printf("Pay-out failed: %s", event.GetIntentID())

    case rampos.EventIntentTradeExecuted:
        log.Printf("Trade executed: %s", event.GetIntentID())

    case rampos.EventCaseCreated:
        log.Printf("Compliance case created")

    case rampos.EventCaseResolved:
        log.Printf("Compliance case resolved")

    default:
        log.Printf("Unknown event type: %s", event.Type)
    }
}
```

### Using Event Helper Methods

```go
func handleEvent(event *rampos.WebhookEvent) {
    // Check event category
    if event.IsPayinEvent() {
        log.Println("This is a pay-in related event")
    }

    if event.IsPayoutEvent() {
        log.Println("This is a pay-out related event")
    }

    if event.IsCaseEvent() {
        log.Println("This is a compliance case event")
    }

    // Extract common fields
    intentID := event.GetIntentID()
    userID := event.GetUserID()
    amount := event.GetAmount()

    log.Printf("Intent: %s, User: %s, Amount: %d", intentID, userID, amount)
}
```

## Error Handling

The SDK returns typed errors for API failures.

```go
func handleErrors(client *rampos.Client) {
    ctx := context.Background()

    intent, err := client.GetIntent(ctx, "non-existent-id")
    if err != nil {
        // Check if it's an API error
        if apiErr, ok := err.(*rampos.APIError); ok {
            switch apiErr.StatusCode {
            case 404:
                log.Println("Intent not found")
            case 401:
                log.Println("Authentication failed - check your API credentials")
            case 403:
                log.Println("Access denied - check your permissions")
            case 429:
                log.Println("Rate limited - slow down your requests")
            default:
                log.Printf("API error [%d] %s: %s",
                    apiErr.StatusCode, apiErr.Code, apiErr.Message)
            }
            return
        }

        // Other errors (network, timeout, etc.)
        log.Printf("Request failed: %v", err)
        return
    }

    // Success
    log.Printf("Found intent: %s", intent.ID)
}
```

## Full Integration Example

Here is a complete example of a pay-in flow:

```go
package main

import (
    "context"
    "fmt"
    "log"
    "os"
    "time"

    rampos "github.com/rampos/sdk-go"
)

func main() {
    // Initialize client
    client := rampos.NewClient(
        os.Getenv("RAMPOS_API_KEY"),
        os.Getenv("RAMPOS_API_SECRET"),
        rampos.WithTenantID(os.Getenv("RAMPOS_TENANT_ID")),
    )

    ctx := context.Background()

    // Step 1: Create a pay-in intent
    fmt.Println("Creating pay-in intent...")
    payinResp, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
        UserID:        "user_123",
        AmountVND:     1000000,
        RailsProvider: "vietqr",
        Metadata: map[string]interface{}{
            "orderId": "order_456",
            "source":  "mobile_app",
        },
    })
    if err != nil {
        log.Fatalf("Failed to create pay-in: %v", err)
    }

    fmt.Printf("Intent created: %s\n", payinResp.IntentID)
    fmt.Printf("Reference: %s\n", payinResp.ReferenceCode)

    if payinResp.VirtualAccount != nil {
        fmt.Printf("Transfer to: %s - %s (%s)\n",
            payinResp.VirtualAccount.Bank,
            payinResp.VirtualAccount.AccountNumber,
            payinResp.VirtualAccount.AccountName,
        )
    }

    // Step 2: Wait for bank confirmation
    // In production, this would be triggered by a webhook
    fmt.Println("Waiting for bank confirmation...")
    time.Sleep(3 * time.Second)

    // Step 3: Confirm the payment (simulating bank callback)
    fmt.Println("Confirming payment...")
    confirmResp, err := client.ConfirmPayin(ctx, rampos.ConfirmPayinRequest{
        ReferenceCode:  payinResp.ReferenceCode,
        Status:         "FUNDS_CONFIRMED",
        BankTxID:       "BANK_TX_" + time.Now().Format("20060102150405"),
        AmountVND:      1000000,
        SettledAt:      time.Now(),
        RawPayloadHash: "sha256-hash-of-bank-notification",
    })
    if err != nil {
        log.Fatalf("Failed to confirm pay-in: %v", err)
    }

    fmt.Printf("Payment confirmed: %s\n", confirmResp.Status)

    // Step 4: Verify final status
    fmt.Println("Checking final status...")
    intent, err := client.GetIntent(ctx, payinResp.IntentID)
    if err != nil {
        log.Fatalf("Failed to get intent: %v", err)
    }

    fmt.Printf("Final state: %s\n", intent.State)
    fmt.Printf("Amount: %s %s\n", intent.Amount, intent.Currency)

    if intent.State == "COMPLETED" {
        fmt.Println("Pay-in completed successfully!")
    }

    // Step 5: Check user balance
    balances, err := client.GetUserBalances(ctx, "user_123")
    if err != nil {
        log.Printf("Warning: Failed to get balances: %v", err)
    } else {
        fmt.Println("Updated balances:")
        for _, b := range balances.Balances {
            fmt.Printf("  %s: %s\n", b.Currency, b.Balance)
        }
    }
}
```

## Best Practices

### Context and Timeouts

Always use context with timeouts for API calls:

```go
ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
defer cancel()

intent, err := client.GetIntent(ctx, intentID)
```

### Retry Logic

Implement retry logic for transient failures:

```go
func getIntentWithRetry(client *rampos.Client, intentID string, maxRetries int) (*rampos.Intent, error) {
    var lastErr error

    for i := 0; i < maxRetries; i++ {
        ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
        defer cancel()

        intent, err := client.GetIntent(ctx, intentID)
        if err == nil {
            return intent, nil
        }

        lastErr = err

        // Don't retry on client errors (4xx)
        if apiErr, ok := err.(*rampos.APIError); ok {
            if apiErr.StatusCode >= 400 && apiErr.StatusCode < 500 {
                return nil, err
            }
        }

        // Exponential backoff
        time.Sleep(time.Duration(1<<i) * time.Second)
    }

    return nil, fmt.Errorf("max retries exceeded: %w", lastErr)
}
```

### Logging

Add structured logging for debugging:

```go
func loggedCreatePayin(client *rampos.Client, req rampos.CreatePayinRequest) (*rampos.CreatePayinResponse, error) {
    log.Printf("Creating pay-in for user %s, amount %d VND", req.UserID, req.AmountVND)

    ctx := context.Background()
    resp, err := client.CreatePayin(ctx, req)

    if err != nil {
        log.Printf("Pay-in creation failed: %v", err)
        return nil, err
    }

    log.Printf("Pay-in created: %s (ref: %s)", resp.IntentID, resp.ReferenceCode)
    return resp, nil
}
```

## Next Steps

- Read the [API Reference](./reference.md) for complete method documentation
- Learn about [webhook event types](./reference.md#webhook-event-types)
- Review [error handling patterns](./reference.md#error-handling)
