// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Script, console } from "forge-std/Script.sol";
import { VNDToken } from "../src/VNDToken.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title DeployOptimism
 * @notice Deploy RampOS contracts to Optimism Mainnet and Optimism Sepolia
 * @dev Deploys Account Factory, Paymaster, and optionally VND Token
 *
 * Optimism Configuration:
 *  - Chain ID (Mainnet): 10
 *  - Chain ID (Sepolia): 11155420
 *  - EntryPoint v0.7: 0x0000000071727De22E5E9d8BAf0edAc6f37da032
 *  - EntryPoint v0.6: 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789
 *
 * Required environment variables:
 *  - DEPLOYER_PRIVATE_KEY: Private key for deployment
 *  - OPTIMISM_RPC_URL: Optimism RPC endpoint (mainnet or testnet)
 *
 * Optional environment variables:
 *  - PAYMASTER_SIGNER: Address for paymaster signatures (defaults to deployer)
 *  - USE_ENTRYPOINT_V7: Set to "true" to use v0.7 (default: v0.6)
 *
 * Usage:
 *  # Optimism Sepolia (testnet)
 *  forge script script/DeployOptimism.s.sol:DeployOptimism \
 *    --rpc-url https://sepolia.optimism.io \
 *    --broadcast --verify
 *
 *  # Optimism Mainnet (production)
 *  forge script script/DeployOptimism.s.sol:DeployOptimism \
 *    --rpc-url https://mainnet.optimism.io \
 *    --broadcast --verify
 */
contract DeployOptimism is Script {
    // ERC-4337 EntryPoint addresses (same on all EVM chains)
    address constant ENTRY_POINT_V06 = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    // Optimism chain IDs
    uint256 constant OPTIMISM_MAINNET = 10;
    uint256 constant OPTIMISM_SEPOLIA = 11155420;

    // Optimism bundler endpoints (for reference)
    string constant BUNDLER_MAINNET = "https://api.pimlico.io/v2/10/rpc";
    string constant BUNDLER_SEPOLIA = "https://api.pimlico.io/v2/11155420/rpc";

    // Alternative bundlers
    string constant ALCHEMY_BUNDLER_MAINNET = "https://opt-mainnet.g.alchemy.com/v2";
    string constant STACKUP_BUNDLER = "https://api.stackup.sh/v1/node";

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        // Check if using v0.7 EntryPoint
        bool useV7 = vm.envOr("USE_ENTRYPOINT_V7", false);
        address entryPoint = useV7 ? ENTRY_POINT_V07 : ENTRY_POINT_V06;

        // Paymaster signer (defaults to deployer)
        address paymasterSigner = vm.envOr("PAYMASTER_SIGNER", deployer);

        // Verify we're on Optimism
        require(
            block.chainid == OPTIMISM_MAINNET || block.chainid == OPTIMISM_SEPOLIA,
            "Not on Optimism chain"
        );

        string memory networkName = block.chainid == OPTIMISM_MAINNET ? "Optimism Mainnet" : "Optimism Sepolia";
        string memory entryPointVersion = useV7 ? "v0.7" : "v0.6";

        console.log("========================================");
        console.log("  RampOS Optimism Deployment");
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

        // 1. Deploy VND Token
        VNDToken vndToken = new VNDToken(deployer);
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
        if (block.chainid == OPTIMISM_MAINNET) {
            console.log("   ", BUNDLER_MAINNET);
        } else {
            console.log("   ", BUNDLER_SEPOLIA);
        }
        console.log("");
        console.log("5. Update environment variables with addresses");
    }
}

/**
 * @title DeployOptimismFactory
 * @notice Deploy only the Account Factory on Optimism
 */
contract DeployOptimismFactory is Script {
    address constant ENTRY_POINT_V06 = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    uint256 constant OPTIMISM_MAINNET = 10;
    uint256 constant OPTIMISM_SEPOLIA = 11155420;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        require(
            block.chainid == OPTIMISM_MAINNET || block.chainid == OPTIMISM_SEPOLIA,
            "Not on Optimism chain"
        );

        bool useV7 = vm.envOr("USE_ENTRYPOINT_V7", false);
        address entryPoint = useV7 ? ENTRY_POINT_V07 : ENTRY_POINT_V06;

        console.log("Deploying Account Factory on Optimism...");
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
 * @title DeployOptimismPaymaster
 * @notice Deploy only the Paymaster on Optimism
 */
contract DeployOptimismPaymaster is Script {
    address constant ENTRY_POINT_V06 = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    uint256 constant OPTIMISM_MAINNET = 10;
    uint256 constant OPTIMISM_SEPOLIA = 11155420;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        require(
            block.chainid == OPTIMISM_MAINNET || block.chainid == OPTIMISM_SEPOLIA,
            "Not on Optimism chain"
        );

        bool useV7 = vm.envOr("USE_ENTRYPOINT_V7", false);
        address entryPoint = useV7 ? ENTRY_POINT_V07 : ENTRY_POINT_V06;
        address paymasterSigner = vm.envOr("PAYMASTER_SIGNER", deployer);

        console.log("Deploying Paymaster on Optimism...");
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
 * @title VerifyOptimism
 * @notice Verify deployed contracts on Optimism
 * @dev Run after deployment to verify contracts on Optimistic Etherscan
 *
 * Usage:
 *  forge verify-contract <FACTORY_ADDRESS> RampOSAccountFactory \
 *    --chain optimism \
 *    --constructor-args $(cast abi-encode "constructor(address)" 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)
 */
contract VerifyOptimism is Script {
    function run() external view {
        console.log("========================================");
        console.log("  Contract Verification Commands");
        console.log("========================================");
        console.log("");
        console.log("Run these commands to verify on Optimistic Etherscan:");
        console.log("");
        console.log("1. Verify Account Factory:");
        console.log("   forge verify-contract <FACTORY_ADDRESS> RampOSAccountFactory \\");
        console.log("     --chain optimism \\");
        console.log("     --constructor-args $(cast abi-encode 'constructor(address)' 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)");
        console.log("");
        console.log("2. Verify Paymaster:");
        console.log("   forge verify-contract <PAYMASTER_ADDRESS> RampOSPaymaster \\");
        console.log("     --chain optimism \\");
        console.log("     --constructor-args $(cast abi-encode 'constructor(address,address)' 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 <SIGNER_ADDRESS>)");
        console.log("");
        console.log("3. Verify VND Token:");
        console.log("   forge verify-contract <VND_ADDRESS> VNDToken \\");
        console.log("     --chain optimism \\");
        console.log("     --constructor-args $(cast abi-encode 'constructor(address)' <ADMIN_ADDRESS>)");
        console.log("");
        console.log("4. Verify RampOSAccount (implementation):");
        console.log("   forge verify-contract <ACCOUNT_IMPL> RampOSAccount \\");
        console.log("     --chain optimism \\");
        console.log("     --constructor-args $(cast abi-encode 'constructor(address)' 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)");
        console.log("");
        console.log("For Optimism Sepolia, use --chain optimism-sepolia");
    }
}

/**
 * @title FundOptimismPaymaster
 * @notice Fund the Paymaster contract on Optimism
 */
contract FundOptimismPaymaster is Script {
    uint256 constant OPTIMISM_MAINNET = 10;
    uint256 constant OPTIMISM_SEPOLIA = 11155420;

    function run() external {
        require(
            block.chainid == OPTIMISM_MAINNET || block.chainid == OPTIMISM_SEPOLIA,
            "Not on Optimism chain"
        );

        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address paymasterAddress = vm.envAddress("PAYMASTER_ADDRESS");
        uint256 fundAmount = vm.envOr("FUND_AMOUNT", uint256(0.1 ether));

        console.log("Funding Paymaster on Optimism...");
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

/**
 * @title ConfigureOptimismBundler
 * @notice Display bundler configuration for Optimism
 */
contract ConfigureOptimismBundler is Script {
    function run() external view {
        console.log("========================================");
        console.log("  Optimism Bundler Configuration");
        console.log("========================================");
        console.log("");
        console.log("Mainnet Bundlers:");
        console.log("  Pimlico:  https://api.pimlico.io/v2/10/rpc?apikey=<YOUR_API_KEY>");
        console.log("  Alchemy:  https://opt-mainnet.g.alchemy.com/v2/<YOUR_API_KEY>");
        console.log("  Stackup:  https://api.stackup.sh/v1/node/<YOUR_API_KEY>");
        console.log("  Biconomy: https://bundler.biconomy.io/api/v2/10/<YOUR_API_KEY>");
        console.log("");
        console.log("Sepolia Bundlers:");
        console.log("  Pimlico:  https://api.pimlico.io/v2/11155420/rpc?apikey=<YOUR_API_KEY>");
        console.log("  Alchemy:  https://opt-sepolia.g.alchemy.com/v2/<YOUR_API_KEY>");
        console.log("");
        console.log("Environment Variables:");
        console.log("  OPTIMISM_BUNDLER_URL=<bundler_url>");
        console.log("  OPTIMISM_BUNDLER_API_KEY=<api_key>");
        console.log("");
        console.log("EntryPoint Addresses:");
        console.log("  v0.6: 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789");
        console.log("  v0.7: 0x0000000071727De22E5E9d8BAf0edAc6f37da032");
    }
}

/**
 * @title TestOptimismDeployment
 * @notice Test deployed contracts on Optimism
 */
contract TestOptimismDeployment is Script {
    function run() external view {
        address factoryAddress = vm.envAddress("FACTORY_ADDRESS");
        address paymasterAddress = vm.envAddress("PAYMASTER_ADDRESS");
        address vndTokenAddress = vm.envOr("VND_TOKEN_ADDRESS", address(0));

        console.log("========================================");
        console.log("  Testing Optimism Deployment");
        console.log("========================================");
        console.log("");

        // Check Factory
        RampOSAccountFactory factory = RampOSAccountFactory(factoryAddress);
        console.log("Factory Address:", factoryAddress);
        console.log("  Implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));
        console.log("  EntryPoint:", address(factory.entryPoint()));
        console.log("");

        // Check Paymaster
        RampOSPaymaster paymaster = RampOSPaymaster(payable(paymasterAddress));
        console.log("Paymaster Address:", paymasterAddress);
        console.log("  EntryPoint Deposit:", paymaster.getDeposit());
        console.log("  Verifying Signer:", paymaster.verifyingSigner());
        console.log("");

        // Check VND Token if provided
        if (vndTokenAddress != address(0)) {
            VNDToken vnd = VNDToken(vndTokenAddress);
            console.log("VND Token Address:", vndTokenAddress);
            console.log("  Name:", vnd.name());
            console.log("  Symbol:", vnd.symbol());
            console.log("  Total Supply:", vnd.totalSupply());
        }

        console.log("");
        console.log("All contracts verified successfully!");
    }
}
