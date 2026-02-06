#!/bin/bash
# Deploy RampOS contracts to Base Sepolia testnet
# Usage: ./deploy-base-sepolia.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTRACTS_DIR="$(dirname "$SCRIPT_DIR")/contracts"

echo "=== RampOS Contract Deployment ==="
echo "Network: Base Sepolia (Chain ID: 84532)"
echo ""

# Check if .env exists
if [ ! -f "$CONTRACTS_DIR/.env" ]; then
    echo "Error: $CONTRACTS_DIR/.env not found!"
    echo "Copy .env.example to .env and fill in your values:"
    echo "  cp $CONTRACTS_DIR/.env.example $CONTRACTS_DIR/.env"
    exit 1
fi

# Load environment
source "$CONTRACTS_DIR/.env"

# Check required vars
if [ -z "$DEPLOYER_PRIVATE_KEY" ] || [ "$DEPLOYER_PRIVATE_KEY" == "0x..." ]; then
    echo "Error: DEPLOYER_PRIVATE_KEY not set in .env"
    echo ""
    echo "To get testnet ETH on Base Sepolia:"
    echo "1. Create a new wallet (MetaMask, etc.)"
    echo "2. Get the private key"
    echo "3. Get testnet ETH from: https://www.coinbase.com/faucets/base-ethereum-sepolia-faucet"
    echo "4. Add to .env: DEPLOYER_PRIVATE_KEY=0x..."
    exit 1
fi

cd "$CONTRACTS_DIR"

echo "Building contracts..."
forge build

echo ""
echo "Deploying to Base Sepolia..."
forge script script/DeployAll.s.sol:DeployAllScript \
    --rpc-url https://sepolia.base.org \
    --broadcast \
    --verify \
    -vvv

echo ""
echo "=== Deployment Complete ==="
echo ""
echo "Update your .env files with the deployed addresses!"
echo "Check: $CONTRACTS_DIR/broadcast/DeployAll.s.sol/84532/run-latest.json"
