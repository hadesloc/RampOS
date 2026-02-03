package rampos

import (
	"context"
	"fmt"
	"net/url"
	"strconv"
	"time"
)

// ============================================================================
// Ledger Service
// ============================================================================

// LedgerService handles ledger-related API operations.
type LedgerService struct {
	client *Client
}

// ============================================================================
// Ledger Entry Types
// ============================================================================

// LedgerEntry represents a single ledger entry.
type LedgerEntry struct {
	ID            string                 `json:"id"`
	TenantID      string                 `json:"tenantId"`
	UserID        string                 `json:"userId"`
	AccountType   string                 `json:"accountType"`
	Currency      string                 `json:"currency"`
	Amount        string                 `json:"amount"`
	BalanceAfter  string                 `json:"balanceAfter"`
	EntryType     string                 `json:"entryType"`
	Direction     string                 `json:"direction"` // "DEBIT" or "CREDIT"
	ReferenceID   string                 `json:"referenceId,omitempty"`
	ReferenceType string                 `json:"referenceType,omitempty"`
	Description   string                 `json:"description,omitempty"`
	Metadata      map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt     time.Time              `json:"createdAt"`
}

// LedgerEntriesParams represents parameters for listing ledger entries.
type LedgerEntriesParams struct {
	UserID        *string    `json:"userId,omitempty"`
	AccountType   *string    `json:"accountType,omitempty"`
	Currency      *string    `json:"currency,omitempty"`
	EntryType     *string    `json:"entryType,omitempty"`
	Direction     *string    `json:"direction,omitempty"`
	ReferenceID   *string    `json:"referenceId,omitempty"`
	ReferenceType *string    `json:"referenceType,omitempty"`
	StartDate     *time.Time `json:"startDate,omitempty"`
	EndDate       *time.Time `json:"endDate,omitempty"`
	Limit         int        `json:"limit,omitempty"`
	Offset        int        `json:"offset,omitempty"`
}

// LedgerEntriesResponse represents a paginated list of ledger entries.
type LedgerEntriesResponse struct {
	Data       []LedgerEntry  `json:"data"`
	Pagination PaginationInfo `json:"pagination"`
}

// ============================================================================
// Ledger Balance Types
// ============================================================================

// LedgerBalance represents an account balance in the ledger.
type LedgerBalance struct {
	UserID      string `json:"userId"`
	AccountType string `json:"accountType"`
	Currency    string `json:"currency"`
	Balance     string `json:"balance"`
	Available   string `json:"available"`
	Locked      string `json:"locked"`
	UpdatedAt   string `json:"updatedAt"`
}

// LedgerBalancesParams represents parameters for querying ledger balances.
type LedgerBalancesParams struct {
	UserID      *string `json:"userId,omitempty"`
	AccountType *string `json:"accountType,omitempty"`
	Currency    *string `json:"currency,omitempty"`
	Limit       int     `json:"limit,omitempty"`
	Offset      int     `json:"offset,omitempty"`
}

// LedgerBalancesResponse represents a paginated list of ledger balances.
type LedgerBalancesResponse struct {
	Data       []LedgerBalance `json:"data"`
	Pagination PaginationInfo  `json:"pagination"`
}

// ============================================================================
// Ledger Service Methods
// ============================================================================

// GetEntries retrieves ledger entries with optional filtering.
func (s *LedgerService) GetEntries(ctx context.Context, params LedgerEntriesParams) (*LedgerEntriesResponse, error) {
	path := "/v1/ledger/entries"
	queryParams := url.Values{}

	if params.UserID != nil {
		queryParams.Set("userId", *params.UserID)
	}
	if params.AccountType != nil {
		queryParams.Set("accountType", *params.AccountType)
	}
	if params.Currency != nil {
		queryParams.Set("currency", *params.Currency)
	}
	if params.EntryType != nil {
		queryParams.Set("entryType", *params.EntryType)
	}
	if params.Direction != nil {
		queryParams.Set("direction", *params.Direction)
	}
	if params.ReferenceID != nil {
		queryParams.Set("referenceId", *params.ReferenceID)
	}
	if params.ReferenceType != nil {
		queryParams.Set("referenceType", *params.ReferenceType)
	}
	if params.StartDate != nil {
		queryParams.Set("startDate", params.StartDate.Format(time.RFC3339))
	}
	if params.EndDate != nil {
		queryParams.Set("endDate", params.EndDate.Format(time.RFC3339))
	}
	if params.Limit > 0 {
		queryParams.Set("limit", strconv.Itoa(params.Limit))
	}
	if params.Offset > 0 {
		queryParams.Set("offset", strconv.Itoa(params.Offset))
	}

	if len(queryParams) > 0 {
		path = fmt.Sprintf("%s?%s", path, queryParams.Encode())
	}

	var resp LedgerEntriesResponse
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetBalances retrieves ledger balances with optional filtering.
func (s *LedgerService) GetBalances(ctx context.Context, params LedgerBalancesParams) (*LedgerBalancesResponse, error) {
	path := "/v1/ledger/balances"
	queryParams := url.Values{}

	if params.UserID != nil {
		queryParams.Set("userId", *params.UserID)
	}
	if params.AccountType != nil {
		queryParams.Set("accountType", *params.AccountType)
	}
	if params.Currency != nil {
		queryParams.Set("currency", *params.Currency)
	}
	if params.Limit > 0 {
		queryParams.Set("limit", strconv.Itoa(params.Limit))
	}
	if params.Offset > 0 {
		queryParams.Set("offset", strconv.Itoa(params.Offset))
	}

	if len(queryParams) > 0 {
		path = fmt.Sprintf("%s?%s", path, queryParams.Encode())
	}

	var resp LedgerBalancesResponse
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}
