// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Script, console } from "forge-std/Script.sol";
import { VNDToken } from "../src/VNDToken.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title DeployAllScript
 * @notice Deploy all RampOS contracts to Base Sepolia testnet
 * @dev Deploys VND Token, Account Factory, and Paymaster
 *
 * Required environment variables:
 *  - DEPLOYER_PRIVATE_KEY: Private key for deployment
 *  - ENTRY_POINT_ADDRESS: ERC-4337 EntryPoint (0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)
 *
 * Usage:
 *  # Base Sepolia (testnet)
 *  forge script script/DeployAll.s.sol --rpc-url https://sepolia.base.org --broadcast --verify
 *
 *  # Base Mainnet (production)
 *  forge script script/DeployAll.s.sol --rpc-url https://mainnet.base.org --broadcast --verify
 */
contract DeployAllScript is Script {
    // ERC-4337 EntryPoint v0.6 (same on all chains)
    address constant ENTRY_POINT = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        console.log("=== RampOS Deployment ===");
        console.log("Deployer:", deployer);
        console.log("Chain ID:", block.chainid);
        console.log("");

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy VND Token
        VNDToken vndToken = new VNDToken(deployer);
        console.log("VND Token deployed at:", address(vndToken));

        // 2. Deploy Account Factory
        RampOSAccountFactory factory = new RampOSAccountFactory(IEntryPoint(ENTRY_POINT));
        console.log("Account Factory deployed at:", address(factory));
        console.log("  - Account Implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));

        // 3. Deploy Paymaster (deployer is also signer for testing)
        RampOSPaymaster paymaster = new RampOSPaymaster(IEntryPoint(ENTRY_POINT), deployer);
        console.log("Paymaster deployed at:", address(paymaster));

        // 4. Add backend as minter (deployer for now, change in production)
        // vndToken.addMinter(backendAddress);

        vm.stopBroadcast();

        // Summary
        console.log("");
        console.log("=== Deployment Summary ===");
        console.log("VND Token:       ", address(vndToken));
        console.log("Account Factory: ", address(factory));
        console.log("Paymaster:       ", address(paymaster));
        console.log("Entry Point:     ", ENTRY_POINT);
        console.log("");
        console.log("Next steps:");
        console.log("1. Fund Paymaster with ETH for gas sponsorship");
        console.log("2. Add backend address as VND Token minter");
        console.log("3. Update .env with contract addresses");
    }
}

/**
 * @title DeployVNDOnly
 * @notice Deploy only VND Token (for adding to existing deployment)
 */
contract DeployVNDOnly is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        console.log("Deploying VND Token...");
        console.log("Deployer:", deployer);

        vm.startBroadcast(deployerPrivateKey);

        VNDToken vndToken = new VNDToken(deployer);

        vm.stopBroadcast();

        console.log("VND Token deployed at:", address(vndToken));
    }
}
