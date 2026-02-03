package rampos

import (
	"context"
	"fmt"
	"net/url"
	"strconv"
	"time"
)

// ============================================================================
// Users Service
// ============================================================================

// UsersService handles user-related API operations.
type UsersService struct {
	client *Client
}

// ============================================================================
// User Types
// ============================================================================

// User represents a RampOS user.
type User struct {
	ID            string                 `json:"id"`
	TenantID      string                 `json:"tenantId"`
	ExternalID    string                 `json:"externalId,omitempty"`
	Email         string                 `json:"email,omitempty"`
	Phone         string                 `json:"phone,omitempty"`
	DisplayName   string                 `json:"displayName,omitempty"`
	KYCStatus     string                 `json:"kycStatus"`
	KYCLevel      int                    `json:"kycLevel"`
	RiskScore     float64                `json:"riskScore,omitempty"`
	Status        string                 `json:"status"`
	Metadata      map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt     time.Time              `json:"createdAt"`
	UpdatedAt     time.Time              `json:"updatedAt"`
	LastActiveAt  *time.Time             `json:"lastActiveAt,omitempty"`
}

// ListUsersParams represents parameters for listing users.
type ListUsersParams struct {
	Limit      int     `json:"limit,omitempty"`
	Offset     int     `json:"offset,omitempty"`
	Status     *string `json:"status,omitempty"`
	KYCStatus  *string `json:"kycStatus,omitempty"`
	Search     *string `json:"search,omitempty"`
	SortBy     *string `json:"sortBy,omitempty"`
	SortOrder  *string `json:"sortOrder,omitempty"`
}

// ListUsersResponse represents a paginated list of users.
type ListUsersResponse struct {
	Data       []User         `json:"data"`
	Pagination PaginationInfo `json:"pagination"`
}

// UserBalance represents a single balance entry for a user.
type UserBalance struct {
	AccountType string `json:"accountType"`
	Currency    string `json:"currency"`
	Balance     string `json:"balance"`
	Available   string `json:"available"`
	Locked      string `json:"locked"`
}

// UserBalancesResponse represents all balances for a user.
type UserBalancesResponse struct {
	UserID   string        `json:"userId"`
	Balances []UserBalance `json:"balances"`
}

// ============================================================================
// Users Service Methods
// ============================================================================

// Get retrieves a user by ID.
func (s *UsersService) Get(ctx context.Context, userID string) (*User, error) {
	var resp User
	path := "/v1/users/" + url.PathEscape(userID)
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// List retrieves a paginated list of users.
func (s *UsersService) List(ctx context.Context, params ListUsersParams) (*ListUsersResponse, error) {
	path := "/v1/users"
	queryParams := url.Values{}

	if params.Limit > 0 {
		queryParams.Set("limit", strconv.Itoa(params.Limit))
	}
	if params.Offset > 0 {
		queryParams.Set("offset", strconv.Itoa(params.Offset))
	}
	if params.Status != nil {
		queryParams.Set("status", *params.Status)
	}
	if params.KYCStatus != nil {
		queryParams.Set("kycStatus", *params.KYCStatus)
	}
	if params.Search != nil {
		queryParams.Set("search", *params.Search)
	}
	if params.SortBy != nil {
		queryParams.Set("sortBy", *params.SortBy)
	}
	if params.SortOrder != nil {
		queryParams.Set("sortOrder", *params.SortOrder)
	}

	if len(queryParams) > 0 {
		path = fmt.Sprintf("%s?%s", path, queryParams.Encode())
	}

	var resp ListUsersResponse
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetBalances retrieves all balances for a user.
func (s *UsersService) GetBalances(ctx context.Context, userID string) (*UserBalancesResponse, error) {
	var resp UserBalancesResponse
	path := "/v1/users/" + url.PathEscape(userID) + "/balances"
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}
