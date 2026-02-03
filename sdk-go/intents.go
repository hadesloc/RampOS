package rampos

import (
	"context"
	"net/url"
	"strconv"
	"time"
)

// ============================================================================
// Pay-in Types
// ============================================================================

// CreatePayinRequest represents a request to create a pay-in intent.
type CreatePayinRequest struct {
	UserID        string                 `json:"userId"`
	AmountVND     int64                  `json:"amountVnd"`
	RailsProvider string                 `json:"railsProvider"`
	Metadata      map[string]interface{} `json:"metadata,omitempty"`
}

// CreatePayinResponse represents a pay-in creation response.
type CreatePayinResponse struct {
	IntentID       string          `json:"intentId"`
	ReferenceCode  string          `json:"referenceCode"`
	VirtualAccount *VirtualAccount `json:"virtualAccount,omitempty"`
	ExpiresAt      time.Time       `json:"expiresAt"`
	Status         string          `json:"status"`
}

// VirtualAccount represents a virtual bank account for receiving payments.
type VirtualAccount struct {
	Bank          string `json:"bank"`
	AccountNumber string `json:"accountNumber"`
	AccountName   string `json:"accountName"`
}

// ConfirmPayinRequest represents a bank confirmation for a pay-in.
type ConfirmPayinRequest struct {
	ReferenceCode  string    `json:"referenceCode"`
	Status         string    `json:"status"` // "FUNDS_CONFIRMED"
	BankTxID       string    `json:"bankTxId"`
	AmountVND      int64     `json:"amountVnd"`
	SettledAt      time.Time `json:"settledAt"`
	RawPayloadHash string    `json:"rawPayloadHash"`
}

// ConfirmPayinResponse represents a pay-in confirmation response.
type ConfirmPayinResponse struct {
	IntentID string `json:"intentId"`
	Status   string `json:"status"`
}

// CreatePayin creates a new pay-in intent.
func (c *Client) CreatePayin(ctx context.Context, req CreatePayinRequest) (*CreatePayinResponse, error) {
	var resp CreatePayinResponse
	err := c.doRequest(ctx, "POST", "/v1/intents/payin", req, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// ConfirmPayin confirms a pay-in after bank receives funds.
func (c *Client) ConfirmPayin(ctx context.Context, req ConfirmPayinRequest) (*ConfirmPayinResponse, error) {
	var resp ConfirmPayinResponse
	err := c.doRequest(ctx, "POST", "/v1/intents/payin/confirm", req, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// ============================================================================
// Pay-out Types
// ============================================================================

// CreatePayoutRequest represents a request to create a pay-out intent.
type CreatePayoutRequest struct {
	UserID        string                 `json:"userId"`
	AmountVND     int64                  `json:"amountVnd"`
	RailsProvider string                 `json:"railsProvider"`
	BankAccount   BankAccount            `json:"bankAccount"`
	Metadata      map[string]interface{} `json:"metadata,omitempty"`
}

// BankAccount represents a destination bank account.
type BankAccount struct {
	BankCode      string `json:"bankCode"`
	AccountNumber string `json:"accountNumber"`
	AccountName   string `json:"accountName"`
}

// CreatePayoutResponse represents a pay-out creation response.
type CreatePayoutResponse struct {
	IntentID string `json:"intentId"`
	Status   string `json:"status"`
}

// CreatePayout creates a new pay-out intent.
func (c *Client) CreatePayout(ctx context.Context, req CreatePayoutRequest) (*CreatePayoutResponse, error) {
	var resp CreatePayoutResponse
	err := c.doRequest(ctx, "POST", "/v1/intents/payout", req, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// ============================================================================
// Intent Query Types
// ============================================================================

// Intent represents a transaction intent.
type Intent struct {
	ID            string                 `json:"id"`
	IntentType    string                 `json:"intentType"`
	State         string                 `json:"state"`
	Amount        string                 `json:"amount"`
	Currency      string                 `json:"currency"`
	ActualAmount  *string                `json:"actualAmount,omitempty"`
	ReferenceCode *string                `json:"referenceCode,omitempty"`
	BankTxID      *string                `json:"bankTxId,omitempty"`
	ChainID       *string                `json:"chainId,omitempty"`
	TxHash        *string                `json:"txHash,omitempty"`
	StateHistory  []StateHistoryEntry    `json:"stateHistory"`
	CreatedAt     string                 `json:"createdAt"`
	UpdatedAt     string                 `json:"updatedAt"`
	ExpiresAt     *string                `json:"expiresAt,omitempty"`
	CompletedAt   *string                `json:"completedAt,omitempty"`
	Metadata      map[string]interface{} `json:"metadata,omitempty"`
}

// StateHistoryEntry represents a state transition.
type StateHistoryEntry struct {
	State     string  `json:"state"`
	Timestamp string  `json:"timestamp"`
	Reason    *string `json:"reason,omitempty"`
}

// ListIntentsRequest represents parameters for listing intents.
type ListIntentsRequest struct {
	UserID     *string `json:"userId,omitempty"`
	IntentType *string `json:"intentType,omitempty"`
	State      *string `json:"state,omitempty"`
	Limit      int     `json:"limit,omitempty"`
	Offset     int     `json:"offset,omitempty"`
}

// ListIntentsResponse represents a paginated list of intents.
type ListIntentsResponse struct {
	Data       []Intent       `json:"data"`
	Pagination PaginationInfo `json:"pagination"`
}

// PaginationInfo contains pagination metadata.
type PaginationInfo struct {
	Limit   int  `json:"limit"`
	Offset  int  `json:"offset"`
	HasMore bool `json:"hasMore"`
}

// GetIntent retrieves an intent by ID.
func (c *Client) GetIntent(ctx context.Context, intentID string) (*Intent, error) {
	var resp Intent
	err := c.doRequest(ctx, "GET", "/v1/intents/"+intentID, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// ListIntents retrieves a list of intents.
func (c *Client) ListIntents(ctx context.Context, req ListIntentsRequest) (*ListIntentsResponse, error) {
	// Build query string
	path := "/v1/intents"
	q := url.Values{}

	if req.UserID != nil {
		q.Set("userId", *req.UserID)
	}
	if req.IntentType != nil {
		q.Set("intentType", *req.IntentType)
	}
	if req.State != nil {
		q.Set("state", *req.State)
	}
	if req.Limit > 0 {
		q.Set("limit", strconv.Itoa(req.Limit))
	}
	if req.Offset > 0 {
		q.Set("offset", strconv.Itoa(req.Offset))
	}

	if encoded := q.Encode(); encoded != "" {
		path = path + "?" + encoded
	}

	var resp ListIntentsResponse
	err := c.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// ============================================================================
// Balance Types
// ============================================================================

// Balance represents a user's balance in a currency.
type Balance struct {
	AccountType string `json:"accountType"`
	Currency    string `json:"currency"`
	Balance     string `json:"balance"`
}

// UserBalances represents all balances for a user.
type UserBalances struct {
	Balances []Balance `json:"balances"`
}

// GetUserBalances retrieves a user's balances.
func (c *Client) GetUserBalances(ctx context.Context, userID string) (*UserBalances, error) {
	var resp UserBalances
	path := "/v1/users/" + c.tenantID + "/" + userID + "/balances"
	err := c.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// ============================================================================
// Trade Types
// ============================================================================

// RecordTradeRequest represents a trade execution event.
type RecordTradeRequest struct {
	TradeID     string  `json:"tradeId"`
	UserID      string  `json:"userId"`
	Symbol      string  `json:"symbol"` // e.g., "BTC/VND"
	Price       string  `json:"price"`
	VNDDelta    int64   `json:"vndDelta"`    // negative = user paid, positive = user received
	CryptoDelta string  `json:"cryptoDelta"` // amount of crypto
	Timestamp   string  `json:"ts"`
}

// RecordTradeResponse represents a trade recording response.
type RecordTradeResponse struct {
	IntentID string `json:"intentId"`
	Status   string `json:"status"`
}

// RecordTrade records a trade execution event.
func (c *Client) RecordTrade(ctx context.Context, req RecordTradeRequest) (*RecordTradeResponse, error) {
	var resp RecordTradeResponse
	err := c.doRequest(ctx, "POST", "/v1/events/trade-executed", req, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}
