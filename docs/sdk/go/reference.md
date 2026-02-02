# RampOS Go SDK - API Reference

Complete API reference for the RampOS Go SDK.

## Table of Contents

- [Client](#client)
- [Pay-In Operations](#pay-in-operations)
- [Pay-Out Operations](#pay-out-operations)
- [Intent Operations](#intent-operations)
- [User Operations](#user-operations)
- [Trade Operations](#trade-operations)
- [Webhook Handling](#webhook-handling)
- [Type Definitions](#type-definitions)
- [Error Handling](#error-handling)

---

## Client

### NewClient

Creates a new RampOS API client.

```go
func NewClient(apiKey, apiSecret string, opts ...ClientOption) *Client
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `apiKey` | `string` | Your RampOS API key |
| `apiSecret` | `string` | Your RampOS API secret |
| `opts` | `...ClientOption` | Optional configuration functions |

**Returns:** `*Client` - The configured client

**Example:**

```go
client := rampos.NewClient(
    "your-api-key",
    "your-api-secret",
)
```

### ClientOption Functions

#### WithBaseURL

Sets a custom base URL for the API.

```go
func WithBaseURL(url string) ClientOption
```

**Example:**

```go
client := rampos.NewClient(
    apiKey,
    apiSecret,
    rampos.WithBaseURL("https://staging-api.rampos.io"),
)
```

#### WithHTTPClient

Sets a custom HTTP client.

```go
func WithHTTPClient(client *http.Client) ClientOption
```

**Example:**

```go
httpClient := &http.Client{
    Timeout: 60 * time.Second,
    Transport: &http.Transport{
        MaxIdleConns: 100,
    },
}

client := rampos.NewClient(
    apiKey,
    apiSecret,
    rampos.WithHTTPClient(httpClient),
)
```

#### WithTenantID

Sets the tenant ID for multi-tenant environments.

```go
func WithTenantID(tenantID string) ClientOption
```

**Example:**

```go
client := rampos.NewClient(
    apiKey,
    apiSecret,
    rampos.WithTenantID("tenant_123"),
)
```

### Client Struct

```go
type Client struct {
    // unexported fields
}
```

### Constants

```go
const (
    // DefaultBaseURL is the default RampOS API base URL.
    DefaultBaseURL = "https://api.rampos.io"

    // DefaultTimeout is the default HTTP client timeout.
    DefaultTimeout = 30 * time.Second
)
```

---

## Pay-In Operations

### CreatePayin

Creates a new pay-in intent for fiat-to-crypto deposits.

```go
func (c *Client) CreatePayin(ctx context.Context, req CreatePayinRequest) (*CreatePayinResponse, error)
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ctx` | `context.Context` | Request context |
| `req` | `CreatePayinRequest` | Pay-in creation request |

**CreatePayinRequest:**

| Field | Type | JSON | Required | Description |
|-------|------|------|----------|-------------|
| `UserID` | `string` | `userId` | Yes | User identifier |
| `AmountVND` | `int64` | `amountVnd` | Yes | Amount in VND |
| `RailsProvider` | `string` | `railsProvider` | Yes | Payment rails provider |
| `Metadata` | `map[string]interface{}` | `metadata` | No | Custom metadata |

**Returns:**

| Type | Description |
|------|-------------|
| `*CreatePayinResponse` | Pay-in creation response |
| `error` | Error if request fails |

**CreatePayinResponse:**

| Field | Type | JSON | Description |
|-------|------|------|-------------|
| `IntentID` | `string` | `intentId` | Created intent ID |
| `ReferenceCode` | `string` | `referenceCode` | Bank reference code |
| `VirtualAccount` | `*VirtualAccount` | `virtualAccount` | Virtual account for payment |
| `ExpiresAt` | `time.Time` | `expiresAt` | Intent expiration time |
| `Status` | `string` | `status` | Initial status |

**Example:**

```go
resp, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
    UserID:        "user_123",
    AmountVND:     1000000,
    RailsProvider: "vietqr",
    Metadata: map[string]interface{}{
        "orderId": "order_456",
    },
})
if err != nil {
    log.Fatal(err)
}
fmt.Println("Intent:", resp.IntentID)
```

---

### ConfirmPayin

Confirms a pay-in after bank receives funds.

```go
func (c *Client) ConfirmPayin(ctx context.Context, req ConfirmPayinRequest) (*ConfirmPayinResponse, error)
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ctx` | `context.Context` | Request context |
| `req` | `ConfirmPayinRequest` | Confirmation request |

**ConfirmPayinRequest:**

| Field | Type | JSON | Required | Description |
|-------|------|------|----------|-------------|
| `ReferenceCode` | `string` | `referenceCode` | Yes | Intent reference code |
| `Status` | `string` | `status` | Yes | Confirmation status |
| `BankTxID` | `string` | `bankTxId` | Yes | Bank transaction ID |
| `AmountVND` | `int64` | `amountVnd` | Yes | Confirmed amount |
| `SettledAt` | `time.Time` | `settledAt` | Yes | Settlement time |
| `RawPayloadHash` | `string` | `rawPayloadHash` | Yes | Hash of bank payload |

**Returns:**

| Type | Description |
|------|-------------|
| `*ConfirmPayinResponse` | Confirmation response |
| `error` | Error if request fails |

**Example:**

```go
resp, err := client.ConfirmPayin(ctx, rampos.ConfirmPayinRequest{
    ReferenceCode:  "REF123",
    Status:         "FUNDS_CONFIRMED",
    BankTxID:       "BANK_TX_456",
    AmountVND:      1000000,
    SettledAt:      time.Now(),
    RawPayloadHash: "sha256-hash",
})
```

---

## Pay-Out Operations

### CreatePayout

Creates a new pay-out intent for crypto-to-fiat withdrawals.

```go
func (c *Client) CreatePayout(ctx context.Context, req CreatePayoutRequest) (*CreatePayoutResponse, error)
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ctx` | `context.Context` | Request context |
| `req` | `CreatePayoutRequest` | Pay-out creation request |

**CreatePayoutRequest:**

| Field | Type | JSON | Required | Description |
|-------|------|------|----------|-------------|
| `UserID` | `string` | `userId` | Yes | User identifier |
| `AmountVND` | `int64` | `amountVnd` | Yes | Amount in VND |
| `RailsProvider` | `string` | `railsProvider` | Yes | Payment rails provider |
| `BankAccount` | `BankAccount` | `bankAccount` | Yes | Destination bank account |
| `Metadata` | `map[string]interface{}` | `metadata` | No | Custom metadata |

**BankAccount:**

| Field | Type | JSON | Description |
|-------|------|------|-------------|
| `BankCode` | `string` | `bankCode` | Bank code (e.g., "VCB") |
| `AccountNumber` | `string` | `accountNumber` | Bank account number |
| `AccountName` | `string` | `accountName` | Account holder name |

**Returns:**

| Type | Description |
|------|-------------|
| `*CreatePayoutResponse` | Pay-out creation response |
| `error` | Error if request fails |

**Example:**

```go
resp, err := client.CreatePayout(ctx, rampos.CreatePayoutRequest{
    UserID:        "user_123",
    AmountVND:     500000,
    RailsProvider: "vietqr",
    BankAccount: rampos.BankAccount{
        BankCode:      "VCB",
        AccountNumber: "1234567890",
        AccountName:   "NGUYEN VAN A",
    },
})
```

---

## Intent Operations

### GetIntent

Retrieves an intent by ID.

```go
func (c *Client) GetIntent(ctx context.Context, intentID string) (*Intent, error)
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ctx` | `context.Context` | Request context |
| `intentID` | `string` | Intent identifier |

**Returns:**

| Type | Description |
|------|-------------|
| `*Intent` | Intent details |
| `error` | Error if request fails |

**Intent:**

| Field | Type | JSON | Description |
|-------|------|------|-------------|
| `ID` | `string` | `id` | Intent ID |
| `IntentType` | `string` | `intentType` | Type (PAY_IN, PAY_OUT, TRADE) |
| `State` | `string` | `state` | Current state |
| `Amount` | `string` | `amount` | Transaction amount |
| `Currency` | `string` | `currency` | Currency code |
| `ActualAmount` | `*string` | `actualAmount` | Actual settled amount |
| `ReferenceCode` | `*string` | `referenceCode` | Bank reference |
| `BankTxID` | `*string` | `bankTxId` | Bank transaction ID |
| `ChainID` | `*string` | `chainId` | Blockchain chain ID |
| `TxHash` | `*string` | `txHash` | Blockchain transaction hash |
| `StateHistory` | `[]StateHistoryEntry` | `stateHistory` | State transition history |
| `CreatedAt` | `string` | `createdAt` | Creation timestamp |
| `UpdatedAt` | `string` | `updatedAt` | Last update timestamp |
| `ExpiresAt` | `*string` | `expiresAt` | Expiration timestamp |
| `CompletedAt` | `*string` | `completedAt` | Completion timestamp |
| `Metadata` | `map[string]interface{}` | `metadata` | Custom metadata |

**Example:**

```go
intent, err := client.GetIntent(ctx, "intent_123")
if err != nil {
    log.Fatal(err)
}
fmt.Printf("State: %s, Amount: %s %s\n", intent.State, intent.Amount, intent.Currency)
```

---

### ListIntents

Retrieves a paginated list of intents.

```go
func (c *Client) ListIntents(ctx context.Context, req ListIntentsRequest) (*ListIntentsResponse, error)
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ctx` | `context.Context` | Request context |
| `req` | `ListIntentsRequest` | Query parameters |

**ListIntentsRequest:**

| Field | Type | JSON | Required | Description |
|-------|------|------|----------|-------------|
| `UserID` | `*string` | `userId` | No | Filter by user |
| `IntentType` | `*string` | `intentType` | No | Filter by type |
| `State` | `*string` | `state` | No | Filter by state |
| `Limit` | `int` | `limit` | No | Max results (default 50) |
| `Offset` | `int` | `offset` | No | Pagination offset |

**Returns:**

| Type | Description |
|------|-------------|
| `*ListIntentsResponse` | Paginated intent list |
| `error` | Error if request fails |

**ListIntentsResponse:**

| Field | Type | JSON | Description |
|-------|------|------|-------------|
| `Data` | `[]Intent` | `data` | Array of intents |
| `Pagination` | `PaginationInfo` | `pagination` | Pagination metadata |

**PaginationInfo:**

| Field | Type | JSON | Description |
|-------|------|------|-------------|
| `Limit` | `int` | `limit` | Page size |
| `Offset` | `int` | `offset` | Current offset |
| `HasMore` | `bool` | `hasMore` | More results available |

**Example:**

```go
intentType := "PAY_IN"
state := "COMPLETED"

resp, err := client.ListIntents(ctx, rampos.ListIntentsRequest{
    IntentType: &intentType,
    State:      &state,
    Limit:      50,
})
if err != nil {
    log.Fatal(err)
}

for _, intent := range resp.Data {
    fmt.Printf("%s: %s %s\n", intent.ID, intent.Amount, intent.Currency)
}
```

---

## User Operations

### GetUserBalances

Retrieves all balances for a user.

```go
func (c *Client) GetUserBalances(ctx context.Context, userID string) (*UserBalances, error)
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ctx` | `context.Context` | Request context |
| `userID` | `string` | User identifier |

**Returns:**

| Type | Description |
|------|-------------|
| `*UserBalances` | User balances |
| `error` | Error if request fails |

**UserBalances:**

| Field | Type | JSON | Description |
|-------|------|------|-------------|
| `Balances` | `[]Balance` | `balances` | Array of balances |

**Balance:**

| Field | Type | JSON | Description |
|-------|------|------|-------------|
| `AccountType` | `string` | `accountType` | Account type |
| `Currency` | `string` | `currency` | Currency code |
| `Balance` | `string` | `balance` | Current balance |

**Example:**

```go
balances, err := client.GetUserBalances(ctx, "user_123")
if err != nil {
    log.Fatal(err)
}

for _, b := range balances.Balances {
    fmt.Printf("%s (%s): %s\n", b.Currency, b.AccountType, b.Balance)
}
```

---

## Trade Operations

### RecordTrade

Records a trade execution event.

```go
func (c *Client) RecordTrade(ctx context.Context, req RecordTradeRequest) (*RecordTradeResponse, error)
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ctx` | `context.Context` | Request context |
| `req` | `RecordTradeRequest` | Trade details |

**RecordTradeRequest:**

| Field | Type | JSON | Required | Description |
|-------|------|------|----------|-------------|
| `TradeID` | `string` | `tradeId` | Yes | Unique trade ID |
| `UserID` | `string` | `userId` | Yes | User identifier |
| `Symbol` | `string` | `symbol` | Yes | Trading pair (e.g., "BTC/VND") |
| `Price` | `string` | `price` | Yes | Execution price |
| `VNDDelta` | `int64` | `vndDelta` | Yes | VND change (negative = paid) |
| `CryptoDelta` | `string` | `cryptoDelta` | Yes | Crypto amount change |
| `Timestamp` | `string` | `ts` | Yes | Trade timestamp (RFC3339) |

**Returns:**

| Type | Description |
|------|-------------|
| `*RecordTradeResponse` | Trade recording response |
| `error` | Error if request fails |

**Example:**

```go
resp, err := client.RecordTrade(ctx, rampos.RecordTradeRequest{
    TradeID:     "trade_123",
    UserID:      "user_456",
    Symbol:      "BTC/VND",
    Price:       "1500000000",
    VNDDelta:    -1000000,     // User paid 1M VND
    CryptoDelta: "0.00066667", // Received BTC
    Timestamp:   time.Now().Format(time.RFC3339),
})
```

---

## Webhook Handling

### NewWebhookVerifier

Creates a new webhook signature verifier.

```go
func NewWebhookVerifier(secret string) *WebhookVerifier
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `secret` | `string` | Webhook signing secret |

**Returns:** `*WebhookVerifier` - Configured verifier

**Example:**

```go
verifier := rampos.NewWebhookVerifier("your-webhook-secret")
```

---

### WithTimestampTolerance

Sets the tolerance for timestamp validation.

```go
func (v *WebhookVerifier) WithTimestampTolerance(d time.Duration) *WebhookVerifier
```

**Default:** 5 minutes

**Example:**

```go
verifier := rampos.NewWebhookVerifier("secret").
    WithTimestampTolerance(10 * time.Minute)
```

---

### VerifyAndParse

Verifies the webhook signature and parses the event.

```go
func (v *WebhookVerifier) VerifyAndParse(payload []byte, signature string, timestamp string) (*WebhookEvent, error)
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `payload` | `[]byte` | Raw request body |
| `signature` | `string` | X-Signature header value |
| `timestamp` | `string` | X-Timestamp header value |

**Returns:**

| Type | Description |
|------|-------------|
| `*WebhookEvent` | Parsed webhook event |
| `error` | Error if verification fails |

**WebhookEvent:**

| Field | Type | JSON | Description |
|-------|------|------|-------------|
| `ID` | `string` | `id` | Event ID |
| `Type` | `string` | `type` | Event type |
| `Timestamp` | `time.Time` | `timestamp` | Event timestamp |
| `Data` | `map[string]interface{}` | `data` | Event payload |

**Example:**

```go
event, err := verifier.VerifyAndParse(body, signature, timestamp)
if err != nil {
    log.Printf("Invalid webhook: %v", err)
    return
}
fmt.Printf("Event type: %s\n", event.Type)
```

---

### WebhookEvent Methods

#### IsPayinEvent

```go
func (e *WebhookEvent) IsPayinEvent() bool
```

Returns true if the event is pay-in related.

#### IsPayoutEvent

```go
func (e *WebhookEvent) IsPayoutEvent() bool
```

Returns true if the event is pay-out related.

#### IsCaseEvent

```go
func (e *WebhookEvent) IsCaseEvent() bool
```

Returns true if the event is a compliance case event.

#### GetIntentID

```go
func (e *WebhookEvent) GetIntentID() string
```

Extracts the intent ID from event data.

#### GetUserID

```go
func (e *WebhookEvent) GetUserID() string
```

Extracts the user ID from event data.

#### GetAmount

```go
func (e *WebhookEvent) GetAmount() int64
```

Extracts the amount from event data.

---

### Webhook Event Types

```go
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
```

---

## Type Definitions

### VirtualAccount

```go
type VirtualAccount struct {
    Bank          string `json:"bank"`
    AccountNumber string `json:"accountNumber"`
    AccountName   string `json:"accountName"`
}
```

### StateHistoryEntry

```go
type StateHistoryEntry struct {
    State     string  `json:"state"`
    Timestamp string  `json:"timestamp"`
    Reason    *string `json:"reason,omitempty"`
}
```

### Intent States

Common intent states:

| State | Description |
|-------|-------------|
| `CREATED` | Intent created, awaiting action |
| `PENDING` | Processing in progress |
| `AWAITING_FUNDS` | Waiting for bank transfer |
| `FUNDS_CONFIRMED` | Bank confirmed receipt |
| `PROCESSING` | Being processed |
| `COMPLETED` | Successfully completed |
| `FAILED` | Failed with error |
| `CANCELLED` | Cancelled by user/system |
| `EXPIRED` | Expired before completion |

---

## Error Handling

### APIError

```go
type APIError struct {
    StatusCode int    `json:"-"`
    Code       string `json:"code"`
    Message    string `json:"message"`
}

func (e *APIError) Error() string
```

The SDK returns `*APIError` for API-level errors.

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `StatusCode` | `int` | HTTP status code |
| `Code` | `string` | Error code from API |
| `Message` | `string` | Human-readable message |

**Example:**

```go
intent, err := client.GetIntent(ctx, "invalid-id")
if err != nil {
    if apiErr, ok := err.(*rampos.APIError); ok {
        switch apiErr.StatusCode {
        case 400:
            log.Printf("Bad request: %s", apiErr.Message)
        case 401:
            log.Printf("Unauthorized: check your API credentials")
        case 403:
            log.Printf("Forbidden: insufficient permissions")
        case 404:
            log.Printf("Intent not found")
        case 429:
            log.Printf("Rate limited: slow down")
        case 500:
            log.Printf("Server error: %s", apiErr.Message)
        default:
            log.Printf("API error [%d]: %s", apiErr.StatusCode, apiErr.Message)
        }
    } else {
        // Network error, timeout, etc.
        log.Printf("Request failed: %v", err)
    }
    return
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `INVALID_REQUEST` | Request validation failed |
| `INVALID_AMOUNT` | Amount is invalid or out of range |
| `INVALID_CURRENCY` | Currency not supported |
| `INTENT_NOT_FOUND` | Intent does not exist |
| `INTENT_EXPIRED` | Intent has expired |
| `INTENT_ALREADY_CONFIRMED` | Intent already confirmed |
| `INSUFFICIENT_BALANCE` | User has insufficient balance |
| `USER_NOT_FOUND` | User does not exist |
| `USER_NOT_VERIFIED` | User KYC not verified |
| `RATE_LIMITED` | Too many requests |
| `INTERNAL_ERROR` | Server error |

---

## Best Practices

### Use Context with Timeout

```go
ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
defer cancel()

intent, err := client.GetIntent(ctx, intentID)
```

### Handle Pointer Fields

Many response fields are pointers (optional fields):

```go
intent, _ := client.GetIntent(ctx, "intent_123")

if intent.ReferenceCode != nil {
    fmt.Printf("Reference: %s\n", *intent.ReferenceCode)
}

if intent.TxHash != nil {
    fmt.Printf("Transaction: %s\n", *intent.TxHash)
}
```

### Idempotency

Use unique IDs in metadata for idempotent operations:

```go
resp, err := client.CreatePayin(ctx, rampos.CreatePayinRequest{
    UserID:    "user_123",
    AmountVND: 1000000,
    Metadata: map[string]interface{}{
        "idempotencyKey": "unique-request-id-123",
    },
})
```

### Logging and Monitoring

```go
import "log/slog"

func createPayinWithLogging(client *rampos.Client, req rampos.CreatePayinRequest) (*rampos.CreatePayinResponse, error) {
    start := time.Now()

    resp, err := client.CreatePayin(context.Background(), req)

    duration := time.Since(start)

    if err != nil {
        slog.Error("pay-in creation failed",
            "user_id", req.UserID,
            "amount", req.AmountVND,
            "duration_ms", duration.Milliseconds(),
            "error", err,
        )
        return nil, err
    }

    slog.Info("pay-in created",
        "intent_id", resp.IntentID,
        "reference", resp.ReferenceCode,
        "duration_ms", duration.Milliseconds(),
    )

    return resp, nil
}
```
