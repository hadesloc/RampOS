#!/bin/bash
# RampOS Multi-Chain Deployment Script
# Supports: Arbitrum, Base, Optimism, Polygon zkEVM

set -e

# Load environment variables
if [ -f .env ]; then
  source .env
else
  echo "Error: .env file not found"
  exit 1
fi

# Configuration
CHAINS=("base-sepolia" "arbitrum-sepolia" "optimism-sepolia" "polygon-zkevm-cardona")
DEPLOY_SCRIPT="script/DeployLayers.s.sol"
LOG_DIR="deploy-logs"

mkdir -p $LOG_DIR

echo "=== RampOS Multi-Chain Deployment ==="
echo "Chains: ${CHAINS[@]}"
echo "Log Directory: $LOG_DIR"
echo ""

# Function to deploy to a specific chain
deploy_chain() {
    local chain=$1
    local rpc_url=""
    local verifier_args=""
    local script_contract=""

    echo ">>> Deploying to $chain..."

    case $chain in
        "base-sepolia")
            rpc_url="https://sepolia.base.org"
            script_contract="DeployBase"
            verifier_args="--verify --etherscan-api-key $BASESCAN_API_KEY"
            ;;
        "base-mainnet")
            rpc_url="https://mainnet.base.org"
            script_contract="DeployBase"
            verifier_args="--verify --etherscan-api-key $BASESCAN_API_KEY"
            ;;
        "arbitrum-sepolia")
            rpc_url="https://sepolia-rollup.arbitrum.io/rpc"
            script_contract="DeployArbitrum"
            verifier_args="--verify --etherscan-api-key $ARBISCAN_API_KEY"
            ;;
        "arbitrum-one")
            rpc_url="https://arb1.arbitrum.io/rpc"
            script_contract="DeployArbitrum"
            verifier_args="--verify --etherscan-api-key $ARBISCAN_API_KEY"
            ;;
        "optimism-sepolia")
            rpc_url="https://sepolia.optimism.io"
            script_contract="DeployOptimism"
            verifier_args="--verify --etherscan-api-key $OPTIMISM_ETHERSCAN_API_KEY"
            ;;
        "optimism-mainnet")
            rpc_url="https://mainnet.optimism.io"
            script_contract="DeployOptimism"
            verifier_args="--verify --etherscan-api-key $OPTIMISM_ETHERSCAN_API_KEY"
            ;;
        "polygon-zkevm-cardona")
            rpc_url="https://rpc.cardona.zkevm-rpc.com"
            script_contract="DeployPolygonZkEVMTestnet"
            verifier_args="--verify --verifier blockscout --verifier-url https://cardona-zkevm.polygonscan.com/api"
            ;;
        "polygon-zkevm-mainnet")
            rpc_url="https://zkevm-rpc.com"
            script_contract="DeployPolygonZkEVMMainnet"
            verifier_args="--verify --verifier blockscout --verifier-url https://zkevm.polygonscan.com/api"
            ;;
        *)
            echo "Error: Unknown chain $chain"
            return 1
            ;;
    esac

    # Determine which script file to use
    local script_file=""
    if [[ $chain == *"base"* ]]; then
        script_file="script/DeployBase.s.sol"
    elif [[ $chain == *"arbitrum"* ]]; then
        script_file="script/DeployArbitrum.s.sol"
    elif [[ $chain == *"optimism"* ]]; then
        script_file="script/DeployOptimism.s.sol"
    elif [[ $chain == *"polygon-zkevm"* ]]; then
        script_file="script/DeployPolygonZkEVM.s.sol"
    fi

    echo "Using script: $script_file:$script_contract"
    echo "RPC URL: $rpc_url"

    # Run deployment
    forge script "$script_file:$script_contract" \
        --rpc-url "$rpc_url" \
        --broadcast \
        $verifier_args \
        -vvvv | tee "$LOG_DIR/deploy-$chain.log"

    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo ">>> Deployment to $chain SUCCESSFUL"
    else
        echo ">>> Deployment to $chain FAILED"
        exit 1
    fi
    echo ""
}

# Main loop
for chain in "${CHAINS[@]}"; do
    deploy_chain "$chain"
done

echo "=== All Deployments Completed ==="
echo "Check logs in $LOG_DIR for details."
