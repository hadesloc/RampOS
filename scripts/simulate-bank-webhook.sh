#!/bin/bash
# Simulate bank webhook for testing
# Usage: ./simulate-bank-webhook.sh [amount] [reference_code]

API_URL="${API_URL:-http://localhost:8080}"
AMOUNT="${1:-1000000}"
REFERENCE="${2:-TENANT1_REF$(date +%s)}"
BANK_TX_ID="TEST$(date +%s%N | head -c 16)"

echo "=== Simulating VietQR Bank Webhook ==="
echo "API URL: $API_URL"
echo "Amount: $AMOUNT VND"
echo "Reference: $REFERENCE"
echo "Bank TX ID: $BANK_TX_ID"
echo ""

# VietQR webhook payload
PAYLOAD=$(cat <<EOF
{
  "transactionId": "$BANK_TX_ID",
  "referenceCode": "$REFERENCE",
  "amount": $AMOUNT,
  "currency": "VND",
  "senderBankCode": "VCB",
  "senderAccount": "1234567890",
  "senderName": "NGUYEN VAN A",
  "receiverBankCode": "TCB",
  "receiverAccount": "9876543210",
  "receiverName": "RAMP PLATFORM",
  "description": "Nap tien $REFERENCE",
  "transactionTime": $(date +%s000),
  "status": "SUCCESS"
}
EOF
)

echo "Payload:"
echo "$PAYLOAD" | jq .
echo ""

# Send webhook
echo "Sending webhook to $API_URL/v1/webhooks/bank/vietqr ..."
RESPONSE=$(curl -s -X POST "$API_URL/v1/webhooks/bank/vietqr" \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD")

echo ""
echo "Response:"
echo "$RESPONSE" | jq .

# Check success
if echo "$RESPONSE" | jq -e '.success == true' > /dev/null 2>&1; then
  echo ""
  echo "✅ Webhook processed successfully!"
  CONFIRMATION_ID=$(echo "$RESPONSE" | jq -r '.confirmationId // .confirmation_id // "N/A"')
  echo "Confirmation ID: $CONFIRMATION_ID"
else
  echo ""
  echo "❌ Webhook processing failed"
fi
