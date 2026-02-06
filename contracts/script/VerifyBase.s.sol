// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Script, console } from "forge-std/Script.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title VerifyBaseDeployment
 * @notice Verification script for RampOS Base deployment
 * @dev Verifies all deployed contracts are working correctly
 *
 * Usage:
 *  forge script script/VerifyBase.s.sol:VerifyBaseDeployment --rpc-url base_sepolia
 */
contract VerifyBaseDeployment is Script {
    // Entry Point v0.7
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    function run() external view {
        // Load deployed addresses from environment
        address factoryAddress = vm.envAddress("FACTORY_ADDRESS");
        address paymasterAddress = vm.envAddress("PAYMASTER_ADDRESS");

        console.log("=== Verifying RampOS Base Deployment ===\n");
        console.log("Chain ID:", block.chainid);
        console.log("Factory:", factoryAddress);
        console.log("Paymaster:", paymasterAddress);
        console.log("");

        // Verify Factory
        _verifyFactory(factoryAddress);

        // Verify Paymaster
        _verifyPaymaster(paymasterAddress);

        // Verify Entry Point connection
        _verifyEntryPoint();

        console.log("\n=== All Verifications Passed ===");
    }

    function _verifyFactory(address factoryAddress) internal view {
        console.log("[1] Verifying Factory...");

        RampOSAccountFactory factory = RampOSAccountFactory(factoryAddress);

        // Check entry point
        address entryPoint = address(factory.ENTRY_POINT());
        require(entryPoint == ENTRY_POINT_V07, "Factory: Wrong entry point");
        console.log("    Entry Point: OK");

        // Check implementation
        address implementation = address(factory.ACCOUNT_IMPLEMENTATION());
        require(implementation != address(0), "Factory: No implementation");
        require(implementation.code.length > 0, "Factory: Implementation not deployed");
        console.log("    Implementation:", implementation);
        console.log("    Implementation code size:", implementation.code.length);

        // Test counterfactual address computation
        address owner = address(0x1234567890123456789012345678901234567890);
        uint256 salt = 12345;
        address computed = factory.getAddress(owner, salt);
        require(computed != address(0), "Factory: Cannot compute address");
        console.log("    Counterfactual address test: OK");
        console.log("    Factory verification: PASSED\n");
    }

    function _verifyPaymaster(address paymasterAddress) internal view {
        console.log("[2] Verifying Paymaster...");

        RampOSPaymaster paymaster = RampOSPaymaster(payable(paymasterAddress));

        // Check entry point
        address entryPoint = address(paymaster.ENTRY_POINT());
        require(entryPoint == ENTRY_POINT_V07, "Paymaster: Wrong entry point");
        console.log("    Entry Point: OK");

        // Check owner
        address owner = paymaster.owner();
        require(owner != address(0), "Paymaster: No owner");
        console.log("    Owner:", owner);

        // Check signer
        address signer = paymaster.verifyingSigner();
        require(signer != address(0), "Paymaster: No signer");
        console.log("    Signer:", signer);

        // Check deposit
        uint256 deposit = paymaster.getDeposit();
        console.log("    Deposit:", deposit, "wei");

        if (deposit == 0) {
            console.log("    WARNING: Paymaster has no deposit!");
        } else {
            console.log("    Deposit: OK");
        }

        console.log("    Paymaster verification: PASSED\n");
    }

    function _verifyEntryPoint() internal view {
        console.log("[3] Verifying Entry Point...");

        // Check entry point exists
        require(ENTRY_POINT_V07.code.length > 0, "Entry Point not deployed");
        console.log("    Entry Point deployed: OK");
        console.log("    Code size:", ENTRY_POINT_V07.code.length);
        console.log("    Entry Point verification: PASSED\n");
    }
}

/**
 * @title TestBaseUserOp
 * @notice Test script to simulate a UserOperation on Base
 */
contract TestBaseUserOp is Script {
    address constant ENTRY_POINT_V07 = 0x0000000071727De22E5E9d8BAf0edAc6f37da032;

    function run() external {
        address factoryAddress = vm.envAddress("FACTORY_ADDRESS");
        uint256 testPrivateKey = vm.envUint("TEST_PRIVATE_KEY");
        address testOwner = vm.addr(testPrivateKey);

        console.log("=== Testing UserOp Flow on Base ===\n");
        console.log("Test owner:", testOwner);

        RampOSAccountFactory factory = RampOSAccountFactory(factoryAddress);

        // 1. Compute counterfactual address
        uint256 salt = uint256(keccak256(abi.encodePacked(testOwner, block.timestamp)));
        address accountAddress = factory.getAddress(testOwner, salt);
        console.log("\n[1] Counterfactual address:", accountAddress);

        // 2. Check if account exists
        bool exists = accountAddress.code.length > 0;
        console.log("[2] Account deployed:", exists);

        if (!exists) {
            console.log("\n[3] Creating account via factory...");
            vm.startBroadcast(testPrivateKey);

            // Create account
            address created = address(factory.createAccount(testOwner, salt));
            console.log("    Account created at:", created);

            vm.stopBroadcast();

            require(created == accountAddress, "Address mismatch!");
            console.log("    Address matches counterfactual: OK");
        }

        console.log("\n=== Test Completed Successfully ===");
    }
}
