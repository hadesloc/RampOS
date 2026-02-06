// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Script, console } from "forge-std/Script.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title DeployBase
 * @notice Deployment script for RampOS smart contracts on Base
 * @dev Deploys AccountFactory, Paymaster, and Session Key Validator
 *
 * Base Chain Configuration:
 *  - Chain ID: 8453 (mainnet), 84532 (sepolia)
 *  - Entry Point v0.7: 0x0000000071727De22E5E9d8BAf0edAc6f37da032
 *  - Bundler: Pimlico or Stackup
 *
 * Required environment variables:
 *  - DEPLOYER_PRIVATE_KEY: Private key for deployment
 *  - PAYMASTER_SIGNER: Address authorized to sign paymaster sponsorships
 *  - PAYMASTER_DEPOSIT: Initial ETH deposit for paymaster (in wei)
 *
 * Usage:
 *  # Testnet (Base Sepolia)
 *  forge script script/DeployBase.s.sol:DeployBase --rpc-url base_sepolia --broadcast --verify
 *
 *  # Mainnet
 *  forge script script/DeployBase.s.sol:DeployBase --rpc-url base --broadcast --verify
 */
contract DeployBase is Script {
    // Entry Point v0.7 (canonical address on all chains)
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    // Base chain IDs
    uint256 constant BASE_MAINNET = 8453;
    uint256 constant BASE_SEPOLIA = 84532;

    // Bundler endpoints
    string constant PIMLICO_BASE = "https://api.pimlico.io/v2/base/rpc";
    string constant STACKUP_BASE = "https://api.stackup.sh/v1/node/base";

    // Deployed contract addresses (to be logged)
    RampOSAccountFactory public factory;
    RampOSPaymaster public paymaster;

    function run() external {
        // Validate chain ID
        uint256 chainId = block.chainid;
        require(
            chainId == BASE_MAINNET || chainId == BASE_SEPOLIA,
            "DeployBase: Must deploy on Base mainnet or sepolia"
        );

        bool isMainnet = chainId == BASE_MAINNET;
        string memory networkName = isMainnet ? "Base Mainnet" : "Base Sepolia";

        console.log("=== RampOS Base Deployment ===");
        console.log("Network:", networkName);
        console.log("Chain ID:", chainId);
        console.log("Entry Point v0.7:", ENTRY_POINT_V07);

        // Load configuration
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address paymasterSigner = vm.envAddress("PAYMASTER_SIGNER");
        uint256 paymasterDeposit = vm.envOr("PAYMASTER_DEPOSIT", uint256(0.1 ether));

        address deployer = vm.addr(deployerPrivateKey);
        console.log("Deployer:", deployer);
        console.log("Paymaster Signer:", paymasterSigner);
        console.log("Initial Paymaster Deposit:", paymasterDeposit);

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy Account Factory
        factory = new RampOSAccountFactory(IEntryPoint(ENTRY_POINT_V07));
        console.log("\n[1/3] Factory deployed at:", address(factory));
        console.log("      Account Implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));

        // 2. Deploy Paymaster
        paymaster = new RampOSPaymaster(IEntryPoint(ENTRY_POINT_V07), paymasterSigner);
        console.log("\n[2/3] Paymaster deployed at:", address(paymaster));

        // 3. Fund Paymaster with initial deposit
        if (paymasterDeposit > 0) {
            paymaster.deposit{ value: paymasterDeposit }();
            console.log("\n[3/3] Paymaster funded with:", paymasterDeposit, "wei");
        }

        vm.stopBroadcast();

        // Log deployment summary
        _logDeploymentSummary(networkName, chainId);

        // Log bundler configuration
        _logBundlerConfig(isMainnet);
    }

    function _logDeploymentSummary(string memory networkName, uint256 chainId) internal view {
        console.log("\n========================================");
        console.log("        DEPLOYMENT SUMMARY");
        console.log("========================================");
        console.log("Network:              ", networkName);
        console.log("Chain ID:             ", chainId);
        console.log("----------------------------------------");
        console.log("Entry Point:          ", ENTRY_POINT_V07);
        console.log("Account Factory:      ", address(factory));
        console.log("Account Implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));
        console.log("Paymaster:            ", address(paymaster));
        console.log("----------------------------------------");
        console.log("Paymaster Balance:    ", address(paymaster).balance);
        console.log("========================================\n");
    }

    function _logBundlerConfig(bool isMainnet) internal pure {
        console.log("========================================");
        console.log("        BUNDLER CONFIGURATION");
        console.log("========================================");

        if (isMainnet) {
            console.log("Pimlico (recommended):");
            console.log("  Endpoint: https://api.pimlico.io/v2/8453/rpc?apikey=YOUR_KEY");
            console.log("");
            console.log("Stackup:");
            console.log("  Endpoint: https://api.stackup.sh/v1/node/base?apiKey=YOUR_KEY");
        } else {
            console.log("Pimlico (recommended):");
            console.log("  Endpoint: https://api.pimlico.io/v2/84532/rpc?apikey=YOUR_KEY");
            console.log("");
            console.log("Stackup:");
            console.log("  Endpoint: https://api.stackup.sh/v1/node/base-sepolia?apiKey=YOUR_KEY");
        }

        console.log("========================================\n");
    }
}

/**
 * @title DeployBaseTestnet
 * @notice Quick deployment to Base Sepolia for testing
 */
contract DeployBaseTestnet is DeployBase {
    function setUp() public {
        // Pre-check for testnet
        require(block.chainid == 84532, "Must be on Base Sepolia");
    }
}

/**
 * @title DeployBaseMainnet
 * @notice Production deployment to Base Mainnet
 */
contract DeployBaseMainnet is DeployBase {
    function setUp() public {
        // Pre-check for mainnet
        require(block.chainid == 8453, "Must be on Base Mainnet");
    }
}
