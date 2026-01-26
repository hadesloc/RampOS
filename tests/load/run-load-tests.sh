#!/bin/bash
set -e

BASE_URL=${BASE_URL:-"http://localhost:3000/v1"}
SCENARIO=${SCENARIO:-"smoke"}

echo "Running load tests against $BASE_URL with scenario $SCENARIO"

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo "k6 could not be found. Please install it first."
    exit 1
fi

echo "--- Running Payin Tests ---"
k6 run -e BASE_URL=$BASE_URL -e SCENARIO=$SCENARIO tests/load/payin.js

echo "--- Running Payout Tests ---"
k6 run -e BASE_URL=$BASE_URL -e SCENARIO=$SCENARIO tests/load/payout.js

echo "--- Running Trade Tests ---"
k6 run -e BASE_URL=$BASE_URL -e SCENARIO=$SCENARIO tests/load/trade.js

echo "--- Running Mixed Tests ---"
k6 run -e BASE_URL=$BASE_URL -e SCENARIO=$SCENARIO tests/load/mixed.js

echo "Load tests completed successfully."
