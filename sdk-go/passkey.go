package rampos

import (
	"context"
	"net/url"
)

// ============================================================================
// Passkey Wallet Service
// ============================================================================

// PasskeyService handles passkey-native smart account operations.
type PasskeyService struct {
	client *Client
}

// ============================================================================
// Passkey Types
// ============================================================================

// PasskeyCredential represents a registered WebAuthn passkey credential.
type PasskeyCredential struct {
	CredentialID        string  `json:"credentialId"`
	UserID              string  `json:"userId"`
	PublicKeyX          string  `json:"publicKeyX"`
	PublicKeyY          string  `json:"publicKeyY"`
	SmartAccountAddress *string `json:"smartAccountAddress,omitempty"`
	DisplayName         string  `json:"displayName"`
	IsActive            bool    `json:"isActive"`
	CreatedAt           string  `json:"createdAt"`
	LastUsedAt          *string `json:"lastUsedAt,omitempty"`
}

// RegisterPasskeyParams is the input for registering a passkey credential.
type RegisterPasskeyParams struct {
	UserID       string `json:"userId"`
	CredentialID string `json:"credentialId"`
	PublicKeyX   string `json:"publicKeyX"`
	PublicKeyY   string `json:"publicKeyY"`
	DisplayName  string `json:"displayName"`
}

// RegisterPasskeyResponse is returned after registering a passkey.
type RegisterPasskeyResponse struct {
	CredentialID        string  `json:"credentialId"`
	SmartAccountAddress *string `json:"smartAccountAddress,omitempty"`
	CreatedAt           string  `json:"createdAt"`
}

// CreatePasskeyWalletParams is the input for creating a passkey wallet.
type CreatePasskeyWalletParams struct {
	UserID       string `json:"userId"`
	CredentialID string `json:"credentialId"`
	PublicKeyX   string `json:"publicKeyX"`
	PublicKeyY   string `json:"publicKeyY"`
	DisplayName  string `json:"displayName"`
	OwnerAddress string `json:"ownerAddress,omitempty"`
	Salt         string `json:"salt,omitempty"`
}

// CreatePasskeyWalletResponse is returned after creating a passkey wallet.
type CreatePasskeyWalletResponse struct {
	CredentialID        string `json:"credentialId"`
	SmartAccountAddress string `json:"smartAccountAddress"`
	PublicKeyX          string `json:"publicKeyX"`
	PublicKeyY          string `json:"publicKeyY"`
	IsDeployed          bool   `json:"isDeployed"`
	CreatedAt           string `json:"createdAt"`
}

// LinkSmartAccountParams links a passkey credential to a smart account.
type LinkSmartAccountParams struct {
	UserID              string `json:"userId"`
	CredentialID        string `json:"credentialId"`
	SmartAccountAddress string `json:"smartAccountAddress"`
}

// PasskeySignature contains r,s components of a P256 signature.
type PasskeySignature struct {
	R string `json:"r"`
	S string `json:"s"`
}

// WebAuthnAssertion contains the WebAuthn assertion data for signing.
type WebAuthnAssertion struct {
	AuthenticatorData string           `json:"authenticatorData"`
	ClientDataJSON    string           `json:"clientDataJSON"`
	Signature         PasskeySignature `json:"signature"`
	CredentialID      string           `json:"credentialId"`
}

// SignTransactionUserOp is the user operation part of a sign transaction request.
type SignTransactionUserOp struct {
	Sender               string `json:"sender"`
	Nonce                string `json:"nonce"`
	CallData             string `json:"callData"`
	CallGasLimit         string `json:"callGasLimit,omitempty"`
	VerificationGasLimit string `json:"verificationGasLimit,omitempty"`
	PreVerificationGas   string `json:"preVerificationGas,omitempty"`
	MaxFeePerGas         string `json:"maxFeePerGas,omitempty"`
	MaxPriorityFeePerGas string `json:"maxPriorityFeePerGas,omitempty"`
}

// SignTransactionParams is the input for signing a transaction with a passkey.
type SignTransactionParams struct {
	UserID        string                `json:"userId"`
	CredentialID  string                `json:"credentialId"`
	UserOperation SignTransactionUserOp `json:"userOperation"`
	Assertion     WebAuthnAssertion     `json:"assertion"`
}

// SignTransactionResponse is returned after signing a transaction.
type SignTransactionResponse struct {
	UserOpHash string `json:"userOpHash"`
	Sender     string `json:"sender"`
	Nonce      string `json:"nonce"`
	Signature  string `json:"signature"`
	Status     string `json:"status"`
}

// GetCounterfactualAddressParams computes a CREATE2 address.
type GetCounterfactualAddressParams struct {
	PublicKeyX string `json:"publicKeyX"`
	PublicKeyY string `json:"publicKeyY"`
	Salt       string `json:"salt,omitempty"`
}

// GetCounterfactualAddressResponse is the computed counterfactual address.
type GetCounterfactualAddressResponse struct {
	Address    string `json:"address"`
	IsDeployed bool   `json:"isDeployed"`
}

// ============================================================================
// Passkey Service Methods
// ============================================================================

// CreateWallet creates a passkey wallet: registers the credential and deploys
// a smart account with the passkey set as a signer.
func (s *PasskeyService) CreateWallet(ctx context.Context, params CreatePasskeyWalletParams) (*CreatePasskeyWalletResponse, error) {
	var resp CreatePasskeyWalletResponse
	err := s.client.doRequest(ctx, "POST", "/v1/aa/passkey/wallets", params, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetCounterfactualAddress computes the CREATE2 address for a passkey wallet
// before deployment. Useful for pre-funding the account.
func (s *PasskeyService) GetCounterfactualAddress(ctx context.Context, params GetCounterfactualAddressParams) (*GetCounterfactualAddressResponse, error) {
	var resp GetCounterfactualAddressResponse
	err := s.client.doRequest(ctx, "POST", "/v1/aa/passkey/address", params, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// SignTransaction signs and submits an ERC-4337 UserOperation using a passkey.
func (s *PasskeyService) SignTransaction(ctx context.Context, params SignTransactionParams) (*SignTransactionResponse, error) {
	var resp SignTransactionResponse
	err := s.client.doRequest(ctx, "POST", "/v1/aa/passkey/sign", params, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// RegisterCredential registers a new passkey credential for a user.
func (s *PasskeyService) RegisterCredential(ctx context.Context, params RegisterPasskeyParams) (*RegisterPasskeyResponse, error) {
	var resp RegisterPasskeyResponse
	err := s.client.doRequest(ctx, "POST", "/v1/aa/passkey/credentials", params, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetCredentials gets all passkey credentials for a user.
func (s *PasskeyService) GetCredentials(ctx context.Context, userID string) ([]PasskeyCredential, error) {
	var resp []PasskeyCredential
	path := "/v1/aa/passkey/credentials/" + url.PathEscape(userID)
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return resp, nil
}

// GetCredential gets a specific passkey credential by credential ID.
func (s *PasskeyService) GetCredential(ctx context.Context, userID, credentialID string) (*PasskeyCredential, error) {
	var resp PasskeyCredential
	path := "/v1/aa/passkey/credentials/" + url.PathEscape(userID) + "/" + url.PathEscape(credentialID)
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// LinkSmartAccount links a passkey credential to an existing smart account.
func (s *PasskeyService) LinkSmartAccount(ctx context.Context, params LinkSmartAccountParams) error {
	return s.client.doRequest(ctx, "POST", "/v1/aa/passkey/link", params, nil)
}

// DeactivateCredential deactivates a passkey credential.
func (s *PasskeyService) DeactivateCredential(ctx context.Context, userID, credentialID string) error {
	path := "/v1/aa/passkey/credentials/" + url.PathEscape(userID) + "/" + url.PathEscape(credentialID)
	return s.client.doRequest(ctx, "DELETE", path, nil, nil)
}
