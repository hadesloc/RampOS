// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Script, console } from "forge-std/Script.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import { VNDToken } from "../src/VNDToken.sol";

/**
 * @title DeployLayers
 * @notice Unified deployment script for all L2 chains
 * @dev Handles chain detection and configuration automatically
 */
contract DeployLayers is Script {
    // Standard ERC-4337 EntryPoint v0.6 (same on all EVM chains)
    address constant ENTRY_POINT_V06 = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;

    // EntryPoint v0.7 (same on all EVM chains)
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);
        uint256 chainId = block.chainid;

        // Detect chain and configure
        string memory chainName = _getChainName(chainId);
        bool useV7 = vm.envOr("USE_ENTRYPOINT_V7", false);
        address entryPoint = useV7 ? ENTRY_POINT_V07 : ENTRY_POINT_V06;

        console.log("=== RampOS Deployment ===");
        console.log("Chain:      ", chainName);
        console.log("Chain ID:   ", chainId);
        console.log("Deployer:   ", deployer);
        console.log("EntryPoint: ", entryPoint);
        console.log("Version:    ", useV7 ? "v0.7" : "v0.6");
        console.log("");

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy VND Token (if not exists or forced)
        VNDToken vndToken = new VNDToken(deployer);
        console.log("VND Token:       ", address(vndToken));

        // 2. Deploy Factory
        RampOSAccountFactory factory = new RampOSAccountFactory(IEntryPoint(entryPoint));
        console.log("Account Factory: ", address(factory));
        console.log("  - Impl:        ", address(factory.ACCOUNT_IMPLEMENTATION()));

        // 3. Deploy Paymaster
        address paymasterSigner = vm.envOr("PAYMASTER_SIGNER", deployer);
        RampOSPaymaster paymaster = new RampOSPaymaster(IEntryPoint(entryPoint), paymasterSigner);
        console.log("Paymaster:       ", address(paymaster));
        console.log("  - Signer:      ", paymasterSigner);

        vm.stopBroadcast();

        _printNextSteps(chainName, address(vndToken), address(factory), address(paymaster));
    }

    function _getChainName(uint256 chainId) internal pure returns (string memory) {
        if (chainId == 8453) return "Base Mainnet";
        if (chainId == 84532) return "Base Sepolia";
        if (chainId == 42161) return "Arbitrum One";
        if (chainId == 421614) return "Arbitrum Sepolia";
        if (chainId == 10) return "Optimism Mainnet";
        if (chainId == 11155420) return "Optimism Sepolia";
        if (chainId == 1101) return "Polygon zkEVM Mainnet";
        if (chainId == 2442) return "Polygon zkEVM Cardona";
        return "Unknown Chain";
    }

    function _printNextSteps(string memory chainName, address vnd, address factory, address paymaster) internal view {
        console.log("");
        console.log("=== Deployment Complete ===");
        console.log("Network:  ", chainName);
        console.log("VND:      ", vnd);
        console.log("Factory:  ", factory);
        console.log("Paymaster:", paymaster);
        console.log("");
        console.log("Next Steps:");
        console.log("1. Update .env with new addresses");
        console.log("2. Verify contracts on explorer");
        console.log("3. Fund Paymaster with ETH");
        console.log("4. Add backend as VND minter");
    }
}
