// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Script, console } from "forge-std/Script.sol";
import { VNDToken } from "../src/VNDToken.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

/**
 * @title DeployArbitrum
 * @notice Deploy RampOS contracts to Arbitrum One and Arbitrum Sepolia
 * @dev Deploys Account Factory, Paymaster, and optionally VND Token
 *
 * Arbitrum Configuration:
 *  - Chain ID (Mainnet): 42161
 *  - Chain ID (Sepolia): 421614
 *  - EntryPoint v0.7: 0x0000000071727De22E5E9d8BAf0edAc6f37da032
 *  - EntryPoint v0.6: 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789
 *
 * Required environment variables:
 *  - DEPLOYER_PRIVATE_KEY: Private key for deployment
 *  - ARBITRUM_RPC_URL: Arbitrum RPC endpoint (mainnet or testnet)
 *
 * Optional environment variables:
 *  - PAYMASTER_SIGNER: Address for paymaster signatures (defaults to deployer)
 *  - USE_ENTRYPOINT_V7: Set to "true" to use v0.7 (default: v0.6)
 *
 * Usage:
 *  # Arbitrum Sepolia (testnet)
 *  forge script script/DeployArbitrum.s.sol:DeployArbitrum \
 *    --rpc-url https://sepolia-rollup.arbitrum.io/rpc \
 *    --broadcast --verify
 *
 *  # Arbitrum One (mainnet)
 *  forge script script/DeployArbitrum.s.sol:DeployArbitrum \
 *    --rpc-url https://arb1.arbitrum.io/rpc \
 *    --broadcast --verify
 */
contract DeployArbitrum is Script {
    // ERC-4337 EntryPoint addresses (same on all EVM chains)
    address constant ENTRY_POINT_V06 = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    // Arbitrum chain IDs
    uint256 constant ARBITRUM_ONE = 42161;
    uint256 constant ARBITRUM_SEPOLIA = 421614;

    // Arbitrum bundler endpoints (for reference)
    string constant BUNDLER_MAINNET = "https://api.pimlico.io/v2/42161/rpc";
    string constant BUNDLER_SEPOLIA = "https://api.pimlico.io/v2/421614/rpc";

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        // Check if using v0.7 EntryPoint
        bool useV7 = vm.envOr("USE_ENTRYPOINT_V7", false);
        address entryPoint = useV7 ? ENTRY_POINT_V07 : ENTRY_POINT_V06;

        // Paymaster signer (defaults to deployer)
        address paymasterSigner = vm.envOr("PAYMASTER_SIGNER", deployer);

        // Verify we're on Arbitrum
        require(
            block.chainid == ARBITRUM_ONE || block.chainid == ARBITRUM_SEPOLIA,
            "Not on Arbitrum chain"
        );

        string memory networkName = block.chainid == ARBITRUM_ONE ? "Arbitrum One" : "Arbitrum Sepolia";
        string memory entryPointVersion = useV7 ? "v0.7" : "v0.6";

        console.log("========================================");
        console.log("  RampOS Arbitrum Deployment");
        console.log("========================================");
        console.log("");
        console.log("Network:        ", networkName);
        console.log("Chain ID:       ", block.chainid);
        console.log("Deployer:       ", deployer);
        console.log("EntryPoint:     ", entryPoint);
        console.log("EP Version:     ", entryPointVersion);
        console.log("Paymaster Signer:", paymasterSigner);
        console.log("");

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy VND Token (UUPS proxy)
        VNDToken vndImpl = new VNDToken();
        ERC1967Proxy vndProxy = new ERC1967Proxy(
            address(vndImpl),
            abi.encodeCall(VNDToken.initialize, (deployer))
        );
        VNDToken vndToken = VNDToken(address(vndProxy));
        console.log("[1/3] VND Token deployed at:", address(vndToken));

        // 2. Deploy Account Factory
        RampOSAccountFactory factory = new RampOSAccountFactory(IEntryPoint(entryPoint));
        console.log("[2/3] Account Factory deployed at:", address(factory));
        console.log("      Account Implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));

        // 3. Deploy Paymaster
        RampOSPaymaster paymaster = new RampOSPaymaster(IEntryPoint(entryPoint), paymasterSigner);
        console.log("[3/3] Paymaster deployed at:", address(paymaster));

        vm.stopBroadcast();

        // Deployment summary
        console.log("");
        console.log("========================================");
        console.log("  Deployment Summary");
        console.log("========================================");
        console.log("VND Token:          ", address(vndToken));
        console.log("Account Factory:    ", address(factory));
        console.log("Account Impl:       ", address(factory.ACCOUNT_IMPLEMENTATION()));
        console.log("Paymaster:          ", address(paymaster));
        console.log("EntryPoint:         ", entryPoint);
        console.log("");
        console.log("========================================");
        console.log("  Post-Deployment Checklist");
        console.log("========================================");
        console.log("1. Fund Paymaster with ETH:");
        console.log("   cast send", address(paymaster), "--value 0.1ether");
        console.log("");
        console.log("2. Deposit to EntryPoint for Paymaster:");
        console.log("   Paymaster.deposit{value: 0.1 ether}()");
        console.log("");
        console.log("3. Add backend as VND Token minter");
        console.log("");
        console.log("4. Configure bundler endpoint:");
        if (block.chainid == ARBITRUM_ONE) {
            console.log("   ", BUNDLER_MAINNET);
        } else {
            console.log("   ", BUNDLER_SEPOLIA);
        }
        console.log("");
        console.log("5. Update environment variables with addresses");
    }
}

/**
 * @title DeployArbitrumFactory
 * @notice Deploy only the Account Factory on Arbitrum
 */
contract DeployArbitrumFactory is Script {
    address constant ENTRY_POINT_V06 = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        bool useV7 = vm.envOr("USE_ENTRYPOINT_V7", false);
        address entryPoint = useV7 ? ENTRY_POINT_V07 : ENTRY_POINT_V06;

        console.log("Deploying Account Factory on Arbitrum...");
        console.log("Deployer:", deployer);
        console.log("EntryPoint:", entryPoint);

        vm.startBroadcast(deployerPrivateKey);

        RampOSAccountFactory factory = new RampOSAccountFactory(IEntryPoint(entryPoint));

        vm.stopBroadcast();

        console.log("");
        console.log("Account Factory:", address(factory));
        console.log("Account Implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));
    }
}

/**
 * @title DeployArbitrumPaymaster
 * @notice Deploy only the Paymaster on Arbitrum
 */
contract DeployArbitrumPaymaster is Script {
    address constant ENTRY_POINT_V06 = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        bool useV7 = vm.envOr("USE_ENTRYPOINT_V7", false);
        address entryPoint = useV7 ? ENTRY_POINT_V07 : ENTRY_POINT_V06;
        address paymasterSigner = vm.envOr("PAYMASTER_SIGNER", deployer);

        console.log("Deploying Paymaster on Arbitrum...");
        console.log("Deployer:", deployer);
        console.log("EntryPoint:", entryPoint);
        console.log("Paymaster Signer:", paymasterSigner);

        vm.startBroadcast(deployerPrivateKey);

        RampOSPaymaster paymaster = new RampOSPaymaster(IEntryPoint(entryPoint), paymasterSigner);

        vm.stopBroadcast();

        console.log("");
        console.log("Paymaster:", address(paymaster));
        console.log("");
        console.log("Next: Fund paymaster and deposit to EntryPoint");
    }
}

/**
 * @title VerifyArbitrum
 * @notice Verify deployed contracts on Arbitrum
 * @dev Run after deployment to verify contracts on Arbiscan
 *
 * Usage:
 *  forge verify-contract <FACTORY_ADDRESS> RampOSAccountFactory \
 *    --chain arbitrum-one \
 *    --constructor-args $(cast abi-encode "constructor(address)" 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)
 */
contract VerifyArbitrum is Script {
    function run() external view {
        console.log("========================================");
        console.log("  Contract Verification Commands");
        console.log("========================================");
        console.log("");
        console.log("Run these commands to verify on Arbiscan:");
        console.log("");
        console.log("1. Verify Account Factory:");
        console.log("   forge verify-contract <FACTORY_ADDRESS> RampOSAccountFactory \\");
        console.log("     --chain arbitrum-one \\");
        console.log("     --constructor-args $(cast abi-encode 'constructor(address)' 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)");
        console.log("");
        console.log("2. Verify Paymaster:");
        console.log("   forge verify-contract <PAYMASTER_ADDRESS> RampOSPaymaster \\");
        console.log("     --chain arbitrum-one \\");
        console.log("     --constructor-args $(cast abi-encode 'constructor(address,address)' 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 <SIGNER_ADDRESS>)");
        console.log("");
        console.log("3. Verify VND Token:");
        console.log("   forge verify-contract <VND_ADDRESS> VNDToken \\");
        console.log("     --chain arbitrum-one \\");
        console.log("     --constructor-args $(cast abi-encode 'constructor(address)' <ADMIN_ADDRESS>)");
        console.log("");
        console.log("For Arbitrum Sepolia, use --chain arbitrum-sepolia");
    }
}

/**
 * @title FundPaymaster
 * @notice Fund the Paymaster contract on Arbitrum
 */
contract FundPaymaster is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address paymasterAddress = vm.envAddress("PAYMASTER_ADDRESS");
        uint256 fundAmount = vm.envOr("FUND_AMOUNT", uint256(0.1 ether));

        console.log("Funding Paymaster on Arbitrum...");
        console.log("Paymaster:", paymasterAddress);
        console.log("Amount:", fundAmount);

        vm.startBroadcast(deployerPrivateKey);

        // Send ETH directly to paymaster
        (bool success,) = paymasterAddress.call{value: fundAmount}("");
        require(success, "Failed to fund paymaster");

        // Also deposit to EntryPoint
        RampOSPaymaster paymaster = RampOSPaymaster(payable(paymasterAddress));
        paymaster.deposit{value: fundAmount}();

        vm.stopBroadcast();

        console.log("Paymaster funded successfully");
        console.log("EntryPoint deposit:", paymaster.getDeposit());
    }
}
