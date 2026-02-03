// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Script, console } from "forge-std/Script.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title DeployScript
 * @notice Deployment script for RampOS smart contracts
 * @dev Deploys AccountFactory and Paymaster contracts
 *
 * Required environment variables:
 *  - DEPLOYER_PRIVATE_KEY: Private key for deployment
 *  - ENTRY_POINT_ADDRESS: ERC-4337 EntryPoint contract address
 *  - PAYMASTER_SIGNER: Address authorized to sign paymaster sponsorships
 *
 * Usage:
 *  forge script script/Deploy.s.sol --rpc-url $RPC_URL --broadcast
 */
contract DeployScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address entryPoint = vm.envAddress("ENTRY_POINT_ADDRESS");
        address paymasterSigner = vm.envAddress("PAYMASTER_SIGNER");

        vm.startBroadcast(deployerPrivateKey);

        // Deploy factory (also deploys account implementation)
        RampOSAccountFactory factory = new RampOSAccountFactory(IEntryPoint(entryPoint));
        console.log("Factory deployed at:", address(factory));
        console.log("Account implementation:", address(factory.ACCOUNT_IMPLEMENTATION()));
        console.log("Entry point:", address(factory.ENTRY_POINT()));

        // Deploy paymaster
        RampOSPaymaster paymaster = new RampOSPaymaster(IEntryPoint(entryPoint), paymasterSigner);
        console.log("Paymaster deployed at:", address(paymaster));
        console.log("Paymaster signer:", paymasterSigner);

        vm.stopBroadcast();

        // Log summary
        console.log("\n=== Deployment Summary ===");
        console.log("Factory:", address(factory));
        console.log("Paymaster:", address(paymaster));
        console.log("EntryPoint:", entryPoint);
    }
}
