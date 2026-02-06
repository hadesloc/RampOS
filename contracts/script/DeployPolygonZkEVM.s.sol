// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Script, console } from "forge-std/Script.sol";
import { VNDToken } from "../src/VNDToken.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title DeployPolygonZkEVM
 * @notice Deploy RampOS contracts to Polygon zkEVM mainnet and testnet
 * @dev Polygon zkEVM has specific considerations:
 *   - Uses ETH as native token (bridged from Ethereum)
 *   - EIP-4337 EntryPoint v0.6 is deployed at standard address
 *   - Gas estimation may differ due to zk-proof overhead
 *   - Transaction finality is faster than Ethereum L1
 *
 * Required environment variables:
 *  - DEPLOYER_PRIVATE_KEY: Private key for deployment
 *  - POLYGON_ZKEVM_RPC_URL: RPC URL for Polygon zkEVM
 *
 * Usage:
 *  # Polygon zkEVM Mainnet
 *  forge script script/DeployPolygonZkEVM.s.sol:DeployPolygonZkEVMMainnet \
 *    --rpc-url $POLYGON_ZKEVM_RPC_URL --broadcast --verify \
 *    --verifier blockscout --verifier-url https://zkevm.polygonscan.com/api
 *
 *  # Polygon zkEVM Cardona Testnet
 *  forge script script/DeployPolygonZkEVM.s.sol:DeployPolygonZkEVMTestnet \
 *    --rpc-url $POLYGON_ZKEVM_CARDONA_RPC_URL --broadcast --verify \
 *    --verifier blockscout --verifier-url https://cardona-zkevm.polygonscan.com/api
 */

/// @notice Polygon zkEVM specific configuration
library PolygonZkEVMConfig {
    // Chain IDs
    uint256 constant MAINNET_CHAIN_ID = 1101;
    uint256 constant CARDONA_TESTNET_CHAIN_ID = 2442;

    // ERC-4337 EntryPoint v0.6 (same on all EVM chains)
    address constant ENTRY_POINT_V06 = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;

    // Polygon zkEVM Bundlers (community bundlers)
    // Primary: Pimlico
    string constant PIMLICO_BUNDLER_MAINNET = "https://api.pimlico.io/v2/polygon-zkevm/rpc";
    string constant PIMLICO_BUNDLER_TESTNET = "https://api.pimlico.io/v2/polygon-zkevm-cardona/rpc";

    // Alternative: Stackup
    string constant STACKUP_BUNDLER_MAINNET = "https://api.stackup.sh/v1/node/polygon-zkevm";

    // RPC URLs
    string constant MAINNET_RPC = "https://zkevm-rpc.com";
    string constant CARDONA_RPC = "https://rpc.cardona.zkevm-rpc.com";

    // Block Explorer
    string constant MAINNET_EXPLORER = "https://zkevm.polygonscan.com";
    string constant CARDONA_EXPLORER = "https://cardona-zkevm.polygonscan.com";

    // Gas estimation multiplier for zkEVM (1.2x due to zk-proof overhead)
    uint256 constant GAS_MULTIPLIER = 120; // 120%
    uint256 constant GAS_DIVISOR = 100;

    /// @notice Get recommended gas limit with zkEVM overhead
    function getZkEvmGasLimit(uint256 baseGas) internal pure returns (uint256) {
        return (baseGas * GAS_MULTIPLIER) / GAS_DIVISOR;
    }
}

/**
 * @title DeployPolygonZkEVMMainnet
 * @notice Deploy to Polygon zkEVM Mainnet (Chain ID: 1101)
 */
contract DeployPolygonZkEVMMainnet is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        console.log("=== RampOS Polygon zkEVM Mainnet Deployment ===");
        console.log("Deployer:", deployer);
        console.log("Chain ID:", block.chainid);
        console.log("Expected Chain ID:", PolygonZkEVMConfig.MAINNET_CHAIN_ID);
        console.log("");

        require(
            block.chainid == PolygonZkEVMConfig.MAINNET_CHAIN_ID,
            "Wrong chain! Expected Polygon zkEVM Mainnet (1101)"
        );

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy VND Token
        VNDToken vndToken = new VNDToken(deployer);
        console.log("[1/3] VND Token deployed at:", address(vndToken));

        // 2. Deploy Account Factory
        RampOSAccountFactory factory = new RampOSAccountFactory(
            IEntryPoint(PolygonZkEVMConfig.ENTRY_POINT_V06)
        );
        console.log("[2/3] Account Factory deployed at:", address(factory));
        console.log("      - Account Implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));

        // 3. Deploy Paymaster
        RampOSPaymaster paymaster = new RampOSPaymaster(
            IEntryPoint(PolygonZkEVMConfig.ENTRY_POINT_V06),
            deployer
        );
        console.log("[3/3] Paymaster deployed at:", address(paymaster));

        vm.stopBroadcast();

        _printSummary(address(vndToken), address(factory), address(paymaster), deployer);
        _printZkEvmConfig();
    }

    function _printSummary(
        address vndToken,
        address factory,
        address paymaster,
        address deployer
    ) internal pure {
        console.log("");
        console.log("=== Deployment Summary ===");
        console.log("Network:          Polygon zkEVM Mainnet");
        console.log("Chain ID:         1101");
        console.log("VND Token:        ", vndToken);
        console.log("Account Factory:  ", factory);
        console.log("Paymaster:        ", paymaster);
        console.log("Entry Point:      ", PolygonZkEVMConfig.ENTRY_POINT_V06);
        console.log("Owner/Signer:     ", deployer);
    }

    function _printZkEvmConfig() internal pure {
        console.log("");
        console.log("=== Polygon zkEVM Configuration ===");
        console.log("Bundler (Pimlico):", PolygonZkEVMConfig.PIMLICO_BUNDLER_MAINNET);
        console.log("Bundler (Stackup):", PolygonZkEVMConfig.STACKUP_BUNDLER_MAINNET);
        console.log("Explorer:         ", PolygonZkEVMConfig.MAINNET_EXPLORER);
        console.log("Gas Multiplier:    1.2x (for zk-proof overhead)");
        console.log("");
        console.log("=== Next Steps ===");
        console.log("1. Fund Paymaster with ETH for gas sponsorship");
        console.log("2. Add backend address as VND Token minter");
        console.log("3. Configure bundler with Pimlico or Stackup API key");
        console.log("4. Update .env with contract addresses");
        console.log("5. Test userOp submission through bundler");
    }
}

/**
 * @title DeployPolygonZkEVMTestnet
 * @notice Deploy to Polygon zkEVM Cardona Testnet (Chain ID: 2442)
 */
contract DeployPolygonZkEVMTestnet is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        console.log("=== RampOS Polygon zkEVM Cardona Testnet Deployment ===");
        console.log("Deployer:", deployer);
        console.log("Chain ID:", block.chainid);
        console.log("Expected Chain ID:", PolygonZkEVMConfig.CARDONA_TESTNET_CHAIN_ID);
        console.log("");

        require(
            block.chainid == PolygonZkEVMConfig.CARDONA_TESTNET_CHAIN_ID,
            "Wrong chain! Expected Polygon zkEVM Cardona Testnet (2442)"
        );

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy VND Token
        VNDToken vndToken = new VNDToken(deployer);
        console.log("[1/3] VND Token deployed at:", address(vndToken));

        // 2. Deploy Account Factory
        RampOSAccountFactory factory = new RampOSAccountFactory(
            IEntryPoint(PolygonZkEVMConfig.ENTRY_POINT_V06)
        );
        console.log("[2/3] Account Factory deployed at:", address(factory));
        console.log("      - Account Implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));

        // 3. Deploy Paymaster
        RampOSPaymaster paymaster = new RampOSPaymaster(
            IEntryPoint(PolygonZkEVMConfig.ENTRY_POINT_V06),
            deployer
        );
        console.log("[3/3] Paymaster deployed at:", address(paymaster));

        vm.stopBroadcast();

        console.log("");
        console.log("=== Deployment Summary ===");
        console.log("Network:          Polygon zkEVM Cardona Testnet");
        console.log("Chain ID:         2442");
        console.log("VND Token:        ", address(vndToken));
        console.log("Account Factory:  ", address(factory));
        console.log("Paymaster:        ", address(paymaster));
        console.log("Entry Point:      ", PolygonZkEVMConfig.ENTRY_POINT_V06);
        console.log("Owner/Signer:     ", deployer);
        console.log("");
        console.log("=== Testnet Configuration ===");
        console.log("Bundler (Pimlico):", PolygonZkEVMConfig.PIMLICO_BUNDLER_TESTNET);
        console.log("Explorer:         ", PolygonZkEVMConfig.CARDONA_EXPLORER);
        console.log("Faucet:            https://faucet.polygon.technology/");
        console.log("");
        console.log("=== Next Steps ===");
        console.log("1. Get testnet ETH from Polygon faucet");
        console.log("2. Fund Paymaster with testnet ETH");
        console.log("3. Test deployment before mainnet");
    }
}

/**
 * @title ZkEVMGasEstimator
 * @notice Helper contract for zkEVM-specific gas estimation
 * @dev Polygon zkEVM has different gas costs due to zk-proof overhead
 */
contract ZkEVMGasEstimator is Script {
    /// @notice Estimate gas for a userOp on Polygon zkEVM
    /// @dev Adds 20% overhead for zk-proof generation
    function estimateUserOpGas(
        uint256 callGasLimit,
        uint256 verificationGasLimit,
        uint256 preVerificationGas
    ) public pure returns (
        uint256 adjustedCallGas,
        uint256 adjustedVerificationGas,
        uint256 adjustedPreVerificationGas
    ) {
        adjustedCallGas = PolygonZkEVMConfig.getZkEvmGasLimit(callGasLimit);
        adjustedVerificationGas = PolygonZkEVMConfig.getZkEvmGasLimit(verificationGasLimit);
        // PreVerificationGas has higher overhead on zkEVM (1.5x)
        adjustedPreVerificationGas = (preVerificationGas * 150) / 100;
    }

    /// @notice Get recommended gas limits for common operations
    function getRecommendedGasLimits() public pure returns (
        uint256 simpleTransfer,
        uint256 tokenTransfer,
        uint256 accountCreation,
        uint256 batchExecution
    ) {
        // Base limits for Polygon zkEVM
        simpleTransfer = 100_000;      // Simple ETH transfer
        tokenTransfer = 150_000;       // ERC20 transfer
        accountCreation = 500_000;     // Create new smart account
        batchExecution = 300_000;      // Execute batch of 3 txs
    }
}

/**
 * @title VerifyPolygonZkEVMDeployment
 * @notice Verify existing deployment on Polygon zkEVM
 */
contract VerifyPolygonZkEVMDeployment is Script {
    function run() external view {
        address factory = vm.envAddress("FACTORY_ADDRESS");
        address paymaster = vm.envAddress("PAYMASTER_ADDRESS");
        address vndToken = vm.envAddress("VND_TOKEN_ADDRESS");

        console.log("=== Verifying Polygon zkEVM Deployment ===");
        console.log("Chain ID:", block.chainid);
        console.log("");

        // Check factory
        console.log("Factory:", factory);
        uint256 factoryCode = factory.code.length;
        console.log("  - Code size:", factoryCode);
        require(factoryCode > 0, "Factory not deployed!");

        // Check paymaster
        console.log("Paymaster:", paymaster);
        uint256 paymasterCode = paymaster.code.length;
        console.log("  - Code size:", paymasterCode);
        require(paymasterCode > 0, "Paymaster not deployed!");

        // Check VND Token
        console.log("VND Token:", vndToken);
        uint256 tokenCode = vndToken.code.length;
        console.log("  - Code size:", tokenCode);
        require(tokenCode > 0, "VND Token not deployed!");

        // Check EntryPoint
        console.log("EntryPoint:", PolygonZkEVMConfig.ENTRY_POINT_V06);
        uint256 entryPointCode = PolygonZkEVMConfig.ENTRY_POINT_V06.code.length;
        console.log("  - Code size:", entryPointCode);
        require(entryPointCode > 0, "EntryPoint not available on this chain!");

        console.log("");
        console.log("All contracts verified successfully!");
    }
}
