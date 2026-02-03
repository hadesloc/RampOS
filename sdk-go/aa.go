package rampos

import (
	"context"
	"net/url"
	"time"
)

// ============================================================================
// Account Abstraction (AA) Service
// ============================================================================

// AAService handles Account Abstraction-related API operations.
type AAService struct {
	client *Client
}

// ============================================================================
// Smart Account Types
// ============================================================================

// SmartAccount represents an ERC-4337 smart account.
type SmartAccount struct {
	Address         string                 `json:"address"`
	TenantID        string                 `json:"tenantId"`
	UserID          string                 `json:"userId"`
	OwnerAddress    string                 `json:"ownerAddress"`
	ChainID         int64                  `json:"chainId"`
	FactoryAddress  string                 `json:"factoryAddress"`
	EntryPoint      string                 `json:"entryPoint"`
	AccountType     string                 `json:"accountType"`
	IsDeployed      bool                   `json:"isDeployed"`
	Salt            string                 `json:"salt,omitempty"`
	InitCode        string                 `json:"initCode,omitempty"`
	Metadata        map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt       time.Time              `json:"createdAt"`
	UpdatedAt       time.Time              `json:"updatedAt"`
	DeployedAt      *time.Time             `json:"deployedAt,omitempty"`
}

// CreateAccountParams represents parameters for creating a smart account.
type CreateAccountParams struct {
	UserID       string                 `json:"userId"`
	OwnerAddress string                 `json:"ownerAddress"`
	ChainID      int64                  `json:"chainId"`
	AccountType  string                 `json:"accountType,omitempty"` // e.g., "simple", "multisig"
	Salt         string                 `json:"salt,omitempty"`
	Metadata     map[string]interface{} `json:"metadata,omitempty"`
}

// CreateAccountResponse represents a response after creating a smart account.
type CreateAccountResponse struct {
	Account SmartAccount `json:"account"`
}

// ============================================================================
// User Operation Types
// ============================================================================

// UserOperation represents an ERC-4337 user operation.
type UserOperation struct {
	ID                   string                 `json:"id"`
	Sender               string                 `json:"sender"`
	Nonce                string                 `json:"nonce"`
	InitCode             string                 `json:"initCode"`
	CallData             string                 `json:"callData"`
	CallGasLimit         string                 `json:"callGasLimit"`
	VerificationGasLimit string                 `json:"verificationGasLimit"`
	PreVerificationGas   string                 `json:"preVerificationGas"`
	MaxFeePerGas         string                 `json:"maxFeePerGas"`
	MaxPriorityFeePerGas string                 `json:"maxPriorityFeePerGas"`
	PaymasterAndData     string                 `json:"paymasterAndData"`
	Signature            string                 `json:"signature"`
	Status               string                 `json:"status"`
	TxHash               string                 `json:"txHash,omitempty"`
	UserOpHash           string                 `json:"userOpHash,omitempty"`
	ChainID              int64                  `json:"chainId"`
	Metadata             map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt            time.Time              `json:"createdAt"`
	UpdatedAt            time.Time              `json:"updatedAt"`
	ExecutedAt           *time.Time             `json:"executedAt,omitempty"`
}

// UserOpParams represents parameters for creating a user operation.
type UserOpParams struct {
	Sender               string                 `json:"sender"`
	ChainID              int64                  `json:"chainId"`
	CallData             string                 `json:"callData"`
	CallGasLimit         string                 `json:"callGasLimit,omitempty"`
	VerificationGasLimit string                 `json:"verificationGasLimit,omitempty"`
	PreVerificationGas   string                 `json:"preVerificationGas,omitempty"`
	MaxFeePerGas         string                 `json:"maxFeePerGas,omitempty"`
	MaxPriorityFeePerGas string                 `json:"maxPriorityFeePerGas,omitempty"`
	PaymasterAndData     string                 `json:"paymasterAndData,omitempty"`
	Signature            string                 `json:"signature,omitempty"`
	Metadata             map[string]interface{} `json:"metadata,omitempty"`
}

// CreateUserOpResponse represents a response after creating a user operation.
type CreateUserOpResponse struct {
	UserOperation UserOperation `json:"userOperation"`
	UserOpHash    string        `json:"userOpHash"`
}

// ============================================================================
// AA Service Methods
// ============================================================================

// CreateAccount creates a new smart account for a user.
func (s *AAService) CreateAccount(ctx context.Context, params CreateAccountParams) (*CreateAccountResponse, error) {
	var resp CreateAccountResponse
	err := s.client.doRequest(ctx, "POST", "/v1/aa/accounts", params, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetAccount retrieves a smart account by address.
func (s *AAService) GetAccount(ctx context.Context, address string) (*SmartAccount, error) {
	var resp SmartAccount
	path := "/v1/aa/accounts/" + url.PathEscape(address)
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// CreateUserOperation creates a new user operation.
func (s *AAService) CreateUserOperation(ctx context.Context, params UserOpParams) (*CreateUserOpResponse, error) {
	var resp CreateUserOpResponse
	err := s.client.doRequest(ctx, "POST", "/v1/aa/user-operations", params, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetUserOperation retrieves a user operation by ID.
func (s *AAService) GetUserOperation(ctx context.Context, userOpID string) (*UserOperation, error) {
	var resp UserOperation
	path := "/v1/aa/user-operations/" + url.PathEscape(userOpID)
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// EstimateGas estimates gas for a user operation.
func (s *AAService) EstimateGas(ctx context.Context, params UserOpParams) (*UserOpGasEstimate, error) {
	var resp UserOpGasEstimate
	err := s.client.doRequest(ctx, "POST", "/v1/aa/estimate-gas", params, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// UserOpGasEstimate represents gas estimates for a user operation.
type UserOpGasEstimate struct {
	CallGasLimit         string `json:"callGasLimit"`
	VerificationGasLimit string `json:"verificationGasLimit"`
	PreVerificationGas   string `json:"preVerificationGas"`
	MaxFeePerGas         string `json:"maxFeePerGas"`
	MaxPriorityFeePerGas string `json:"maxPriorityFeePerGas"`
}
