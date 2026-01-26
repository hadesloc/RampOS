package types

import "time"

type IntentType string

const (
	IntentTypePayIn  IntentType = "PAY_IN"
	IntentTypePayOut IntentType = "PAY_OUT"
	IntentTypeTrade  IntentType = "TRADE"
)

type IntentStatus string

const (
	IntentStatusCreated   IntentStatus = "CREATED"
	IntentStatusPending   IntentStatus = "PENDING"
	IntentStatusCompleted IntentStatus = "COMPLETED"
	IntentStatusFailed    IntentStatus = "FAILED"
	IntentStatusCancelled IntentStatus = "CANCELLED"
)

type Intent struct {
	ID          string                 `json:"id"`
	TenantID    string                 `json:"tenantId"`
	Type        IntentType             `json:"type"`
	Status      IntentStatus           `json:"status"`
	Amount      string                 `json:"amount"`
	Currency    string                 `json:"currency"`
	BankAccount *string                `json:"bankAccount,omitempty"`
	BankRef     *string                `json:"bankRef,omitempty"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt   time.Time              `json:"createdAt"`
	UpdatedAt   time.Time              `json:"updatedAt"`
}

type CreatePayInRequest struct {
	Amount   string                 `json:"amount"`
	Currency string                 `json:"currency"`
	Metadata map[string]interface{} `json:"metadata,omitempty"`
}

type CreatePayOutRequest struct {
	Amount      string                 `json:"amount"`
	Currency    string                 `json:"currency"`
	BankAccount string                 `json:"bankAccount"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

type IntentFilters struct {
	Type      *IntentType   `json:"type,omitempty"`
	Status    *IntentStatus `json:"status,omitempty"`
	StartDate *time.Time    `json:"startDate,omitempty"`
	EndDate   *time.Time    `json:"endDate,omitempty"`
	Limit     *int          `json:"limit,omitempty"`
	Offset    *int          `json:"offset,omitempty"`
}
