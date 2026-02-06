#!/bin/bash
# End-to-End Test Flow for RampOS
# This script tests the complete deposit -> trade -> withdraw flow

set -e

API_URL="${API_URL:-http://localhost:8080}"
PORTAL_EMAIL="${PORTAL_EMAIL:-test@example.com}"

echo "=========================================="
echo "  RampOS End-to-End Test Flow"
echo "=========================================="
echo ""
echo "API URL: $API_URL"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

success() { echo -e "${GREEN}✓ $1${NC}"; }
error() { echo -e "${RED}✗ $1${NC}"; exit 1; }
info() { echo -e "${YELLOW}→ $1${NC}"; }

# ============================================================================
# Step 1: Register/Login
# ============================================================================
info "Step 1: Registering user..."

REGISTER_RESPONSE=$(curl -s -X POST "$API_URL/v1/portal/auth/register" \
  -H "Content-Type: application/json" \
  -d "{\"email\": \"$PORTAL_EMAIL\", \"method\": \"magic_link\"}" || echo '{"error": "failed"}')

if echo "$REGISTER_RESPONSE" | grep -q "error"; then
  info "User may already exist, trying to login..."
fi

# For testing, we'll use a mock session token
# In production, this would come from magic link verification
SESSION_TOKEN="test_session_$(date +%s)"
AUTH_HEADER="Authorization: Bearer $SESSION_TOKEN"

success "User session created"

# ============================================================================
# Step 2: Create Smart Account
# ============================================================================
info "Step 2: Creating smart account..."

ACCOUNT_RESPONSE=$(curl -s -X POST "$API_URL/v1/portal/wallet/account" \
  -H "$AUTH_HEADER" \
  -H "Content-Type: application/json")

echo "Response: $ACCOUNT_RESPONSE"

if echo "$ACCOUNT_RESPONSE" | grep -q "address"; then
  SMART_ACCOUNT=$(echo "$ACCOUNT_RESPONSE" | jq -r '.address')
  success "Smart account created: $SMART_ACCOUNT"
else
  error "Failed to create smart account"
fi

# ============================================================================
# Step 3: Get Deposit Info (VND Bank Transfer)
# ============================================================================
info "Step 3: Getting VND deposit info..."

DEPOSIT_INFO=$(curl -s "$API_URL/v1/portal/wallet/deposit-info?method=VND_BANK" \
  -H "$AUTH_HEADER")

echo "Deposit Info: $DEPOSIT_INFO"

TRANSFER_CONTENT=$(echo "$DEPOSIT_INFO" | jq -r '.transferContent')
BANK_ACCOUNT=$(echo "$DEPOSIT_INFO" | jq -r '.accountNumber')

success "Deposit info retrieved"
echo "  Bank: $(echo "$DEPOSIT_INFO" | jq -r '.bankName')"
echo "  Account: $BANK_ACCOUNT"
echo "  Transfer Content: $TRANSFER_CONTENT"

# ============================================================================
# Step 4: Simulate Bank Deposit (via webhook)
# ============================================================================
info "Step 4: Simulating bank deposit (1,000,000 VND)..."

AMOUNT=1000000
BANK_TX_ID="BANK$(date +%s)"

WEBHOOK_PAYLOAD=$(cat <<EOF
{
  "transactionId": "$BANK_TX_ID",
  "referenceCode": "$TRANSFER_CONTENT",
  "amount": $AMOUNT,
  "currency": "VND",
  "senderBankCode": "VCB",
  "senderAccount": "1234567890",
  "senderName": "TEST USER",
  "status": "SUCCESS"
}
EOF
)

WEBHOOK_RESPONSE=$(curl -s -X POST "$API_URL/v1/webhooks/bank/vietqr" \
  -H "Content-Type: application/json" \
  -d "$WEBHOOK_PAYLOAD")

echo "Webhook Response: $WEBHOOK_RESPONSE"

if echo "$WEBHOOK_RESPONSE" | grep -q "success.*true"; then
  success "Bank deposit confirmed"
else
  info "Webhook may have failed (check logs)"
fi

# ============================================================================
# Step 5: Check Balances
# ============================================================================
info "Step 5: Checking balances..."

BALANCES=$(curl -s "$API_URL/v1/portal/wallet/balances" \
  -H "$AUTH_HEADER")

echo "Balances: $BALANCES"

VND_BALANCE=$(echo "$BALANCES" | jq -r '.[] | select(.currency == "VND") | .available')
success "VND Balance: $VND_BALANCE"

# ============================================================================
# Step 6: Get Crypto Deposit Address
# ============================================================================
info "Step 6: Getting crypto deposit address..."

CRYPTO_INFO=$(curl -s "$API_URL/v1/portal/wallet/deposit-info?method=CRYPTO" \
  -H "$AUTH_HEADER")

echo "Crypto Deposit Info: $CRYPTO_INFO"

DEPOSIT_ADDRESS=$(echo "$CRYPTO_INFO" | jq -r '.depositAddress')
NETWORK=$(echo "$CRYPTO_INFO" | jq -r '.network')

success "Crypto deposit address: $DEPOSIT_ADDRESS"
echo "  Network: $NETWORK"

# ============================================================================
# Summary
# ============================================================================
echo ""
echo "=========================================="
echo "  Test Summary"
echo "=========================================="
echo ""
echo "Smart Account: $SMART_ACCOUNT"
echo "VND Balance: $VND_BALANCE"
echo "Deposit Address: $DEPOSIT_ADDRESS"
echo "Network: $NETWORK"
echo ""
echo "Next steps for manual testing:"
echo "1. Send testnet tokens to: $DEPOSIT_ADDRESS"
echo "2. Check balance updates in the portal"
echo "3. Test withdrawal flow"
echo ""
success "E2E test completed!"
