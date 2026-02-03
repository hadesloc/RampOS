package rampos

import (
	"context"
	"fmt"
	"net/url"
	"strconv"
	"time"
)

// ============================================================================
// Compliance Service
// ============================================================================

// ComplianceService handles compliance-related API operations.
type ComplianceService struct {
	client *Client
}

// ============================================================================
// Compliance Case Types
// ============================================================================

// ComplianceCase represents a compliance case.
type ComplianceCase struct {
	ID            string                 `json:"id"`
	TenantID      string                 `json:"tenantId"`
	UserID        string                 `json:"userId"`
	CaseType      string                 `json:"caseType"`
	Status        string                 `json:"status"`
	Severity      string                 `json:"severity"`
	RuleID        string                 `json:"ruleId,omitempty"`
	RuleName      string                 `json:"ruleName,omitempty"`
	Description   string                 `json:"description"`
	Evidence      []CaseEvidence         `json:"evidence,omitempty"`
	Resolution    *CaseResolution        `json:"resolution,omitempty"`
	AssignedTo    string                 `json:"assignedTo,omitempty"`
	Metadata      map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt     time.Time              `json:"createdAt"`
	UpdatedAt     time.Time              `json:"updatedAt"`
	ResolvedAt    *time.Time             `json:"resolvedAt,omitempty"`
}

// CaseEvidence represents evidence attached to a compliance case.
type CaseEvidence struct {
	Type        string                 `json:"type"`
	Description string                 `json:"description"`
	Data        map[string]interface{} `json:"data,omitempty"`
	CreatedAt   time.Time              `json:"createdAt"`
}

// CaseResolution represents the resolution of a compliance case.
type CaseResolution struct {
	Status     string    `json:"status"`
	Notes      string    `json:"notes"`
	ResolvedBy string    `json:"resolvedBy"`
	ResolvedAt time.Time `json:"resolvedAt"`
}

// ListCasesParams represents parameters for listing compliance cases.
type ListCasesParams struct {
	UserID     *string    `json:"userId,omitempty"`
	CaseType   *string    `json:"caseType,omitempty"`
	Status     *string    `json:"status,omitempty"`
	Severity   *string    `json:"severity,omitempty"`
	RuleID     *string    `json:"ruleId,omitempty"`
	AssignedTo *string    `json:"assignedTo,omitempty"`
	StartDate  *time.Time `json:"startDate,omitempty"`
	EndDate    *time.Time `json:"endDate,omitempty"`
	Limit      int        `json:"limit,omitempty"`
	Offset     int        `json:"offset,omitempty"`
}

// ListCasesResponse represents a paginated list of compliance cases.
type ListCasesResponse struct {
	Data       []ComplianceCase `json:"data"`
	Pagination PaginationInfo   `json:"pagination"`
}

// ============================================================================
// Compliance Rule Types
// ============================================================================

// ComplianceRule represents a compliance rule.
type ComplianceRule struct {
	ID          string                 `json:"id"`
	TenantID    string                 `json:"tenantId"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	RuleType    string                 `json:"ruleType"`
	Severity    string                 `json:"severity"`
	Enabled     bool                   `json:"enabled"`
	Conditions  []RuleCondition        `json:"conditions"`
	Actions     []RuleAction           `json:"actions"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt   time.Time              `json:"createdAt"`
	UpdatedAt   time.Time              `json:"updatedAt"`
}

// RuleCondition represents a condition in a compliance rule.
type RuleCondition struct {
	Field    string      `json:"field"`
	Operator string      `json:"operator"`
	Value    interface{} `json:"value"`
}

// RuleAction represents an action to take when a rule is triggered.
type RuleAction struct {
	Type   string                 `json:"type"`
	Params map[string]interface{} `json:"params,omitempty"`
}

// ListRulesResponse represents a list of compliance rules.
type ListRulesResponse struct {
	Data       []ComplianceRule `json:"data"`
	Pagination PaginationInfo   `json:"pagination"`
}

// CreateRuleRequest represents a request to create a compliance rule.
type CreateRuleRequest struct {
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	RuleType    string                 `json:"ruleType"`
	Severity    string                 `json:"severity"`
	Enabled     bool                   `json:"enabled"`
	Conditions  []RuleCondition        `json:"conditions"`
	Actions     []RuleAction           `json:"actions"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// CreateRuleResponse represents a response after creating a compliance rule.
type CreateRuleResponse struct {
	Rule ComplianceRule `json:"rule"`
}

// ============================================================================
// Compliance Service Methods
// ============================================================================

// ListCases retrieves compliance cases with optional filtering.
func (s *ComplianceService) ListCases(ctx context.Context, params ListCasesParams) (*ListCasesResponse, error) {
	path := "/v1/compliance/cases"
	queryParams := url.Values{}

	if params.UserID != nil {
		queryParams.Set("userId", *params.UserID)
	}
	if params.CaseType != nil {
		queryParams.Set("caseType", *params.CaseType)
	}
	if params.Status != nil {
		queryParams.Set("status", *params.Status)
	}
	if params.Severity != nil {
		queryParams.Set("severity", *params.Severity)
	}
	if params.RuleID != nil {
		queryParams.Set("ruleId", *params.RuleID)
	}
	if params.AssignedTo != nil {
		queryParams.Set("assignedTo", *params.AssignedTo)
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

	var resp ListCasesResponse
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetCase retrieves a compliance case by ID.
func (s *ComplianceService) GetCase(ctx context.Context, caseID string) (*ComplianceCase, error) {
	var resp ComplianceCase
	path := "/v1/compliance/cases/" + url.PathEscape(caseID)
	err := s.client.doRequest(ctx, "GET", path, nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// ListRules retrieves all compliance rules.
func (s *ComplianceService) ListRules(ctx context.Context) (*ListRulesResponse, error) {
	var resp ListRulesResponse
	err := s.client.doRequest(ctx, "GET", "/v1/compliance/rules", nil, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// CreateRule creates a new compliance rule.
func (s *ComplianceService) CreateRule(ctx context.Context, req CreateRuleRequest) (*CreateRuleResponse, error) {
	var resp CreateRuleResponse
	err := s.client.doRequest(ctx, "POST", "/v1/compliance/rules", req, &resp)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}
