// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { RampOSAccountFactory } from "../../src/RampOSAccountFactory.sol";
import { RampOSAccount } from "../../src/RampOSAccount.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title FactoryFuzz
 * @notice Comprehensive fuzz tests for RampOSAccountFactory
 * @dev Tests deterministic address generation and deployment edge cases
 */
contract FactoryFuzz is Test {
    RampOSAccountFactory factory;
    IEntryPoint entryPoint;

    function setUp() public {
        // Use canonical ERC-4337 entry point address
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));
        factory = new RampOSAccountFactory(entryPoint);
    }

    // ============ Address Prediction Fuzz Tests ============

    /**
     * @notice Fuzz test: getAddress returns correct predicted address
     * @dev Ensures CREATE2 address prediction is deterministic
     */
    function testFuzz_GetAddressDeterministic(address owner, uint256 salt) public {
        vm.assume(owner != address(0));

        address predicted1 = factory.getAddress(owner, salt);
        address predicted2 = factory.getAddress(owner, salt);

        assertEq(predicted1, predicted2, "Same inputs should give same prediction");
    }

    /**
     * @notice Fuzz test: Different salts produce different addresses
     * @dev Ensures salt affects address generation
     */
    function testFuzz_DifferentSaltsDifferentAddresses(address owner, uint256 salt1, uint256 salt2) public {
        vm.assume(owner != address(0));
        vm.assume(salt1 != salt2);

        address addr1 = factory.getAddress(owner, salt1);
        address addr2 = factory.getAddress(owner, salt2);

        assertTrue(addr1 != addr2, "Different salts should produce different addresses");
    }

    /**
     * @notice Fuzz test: Different owners produce different addresses
     * @dev Ensures owner affects address generation
     */
    function testFuzz_DifferentOwnersDifferentAddresses(
        address owner1,
        address owner2,
        uint256 salt
    ) public {
        vm.assume(owner1 != address(0) && owner2 != address(0));
        vm.assume(owner1 != owner2);

        address addr1 = factory.getAddress(owner1, salt);
        address addr2 = factory.getAddress(owner2, salt);

        assertTrue(addr1 != addr2, "Different owners should produce different addresses");
    }

    /**
     * @notice Fuzz test: Predicted address matches deployed address
     * @dev Core CREATE2 verification
     */
    function testFuzz_PredictionMatchesDeployment(address owner, uint256 salt) public {
        vm.assume(owner != address(0));

        address predicted = factory.getAddress(owner, salt);
        RampOSAccount account = factory.createAccount(owner, salt);

        assertEq(address(account), predicted, "Deployed address should match prediction");
    }

    // ============ Account Creation Fuzz Tests ============

    /**
     * @notice Fuzz test: Create account with any valid owner
     * @dev Ensures account creation works for any non-zero address
     */
    function testFuzz_CreateAccountAnyOwner(address owner, uint256 salt) public {
        vm.assume(owner != address(0));

        RampOSAccount account = factory.createAccount(owner, salt);

        assertEq(account.owner(), owner, "Account owner should match");
        assertEq(address(account.entryPoint()), address(entryPoint), "Entry point should match");
    }

    /**
     * @notice Fuzz test: Create account is idempotent
     * @dev Same parameters should return same account without reverting
     */
    function testFuzz_CreateAccountIdempotent(address owner, uint256 salt) public {
        vm.assume(owner != address(0));

        RampOSAccount account1 = factory.createAccount(owner, salt);
        RampOSAccount account2 = factory.createAccount(owner, salt);

        assertEq(address(account1), address(account2), "Same params should return same account");
    }

    /**
     * @notice Fuzz test: Multiple accounts for same owner with different salts
     * @dev Tests account differentiation by salt
     */
    function testFuzz_MultipleAccountsSameOwner(address owner, uint256 salt1, uint256 salt2) public {
        vm.assume(owner != address(0));
        vm.assume(salt1 != salt2);

        RampOSAccount account1 = factory.createAccount(owner, salt1);
        RampOSAccount account2 = factory.createAccount(owner, salt2);

        assertTrue(address(account1) != address(account2), "Different salts should create different accounts");
        assertEq(account1.owner(), owner, "First account owner should match");
        assertEq(account2.owner(), owner, "Second account owner should match");
    }

    /**
     * @notice Fuzz test: Zero address owner should revert
     * @dev Ensures invalid owner is rejected
     */
    function testFuzz_RevertZeroOwner(uint256 salt) public {
        vm.expectRevert("Invalid owner");
        factory.createAccount(address(0), salt);
    }

    // ============ Edge Case Tests ============

    /**
     * @notice Fuzz test: Extreme salt values
     * @dev Tests boundary salt values
     */
    function testFuzz_ExtremeSaltValues(address owner) public {
        vm.assume(owner != address(0));

        // Test with 0
        RampOSAccount account0 = factory.createAccount(owner, 0);
        assertEq(account0.owner(), owner);

        // Test with max uint256
        RampOSAccount accountMax = factory.createAccount(owner, type(uint256).max);
        assertEq(accountMax.owner(), owner);

        // Ensure they're different
        assertTrue(address(account0) != address(accountMax), "Different salts should be different");
    }

    /**
     * @notice Fuzz test: Sequential salt values
     * @dev Tests that sequential salts produce unique addresses
     */
    function testFuzz_SequentialSalts(address owner, uint256 baseSalt) public {
        vm.assume(owner != address(0));
        vm.assume(baseSalt < type(uint256).max - 10);

        address[] memory addresses = new address[](10);

        for (uint256 i = 0; i < 10; i++) {
            addresses[i] = factory.getAddress(owner, baseSalt + i);
        }

        // Verify all addresses are unique
        for (uint256 i = 0; i < 10; i++) {
            for (uint256 j = i + 1; j < 10; j++) {
                assertTrue(addresses[i] != addresses[j], "All addresses should be unique");
            }
        }
    }

    /**
     * @notice Fuzz test: Create many accounts stress test
     * @dev Tests factory can handle multiple deployments
     */
    function testFuzz_CreateManyAccounts(uint256 seed) public {
        // Bound seed to prevent overflow in address calculations
        seed = bound(seed, 1, type(uint128).max);
        uint256 count = (seed % 20) + 1; // 1-20 accounts

        for (uint256 i = 0; i < count; i++) {
            // Use keccak to generate addresses safely
            address generatedOwner = address(uint160(uint256(keccak256(abi.encode(seed, i)))));
            if (generatedOwner == address(0)) continue;

            uint256 salt = uint256(keccak256(abi.encode(seed, i, "salt")));
            RampOSAccount account = factory.createAccount(generatedOwner, salt);

            assertEq(account.owner(), generatedOwner, "Each account owner should match");
        }
    }

    /**
     * @notice Fuzz test: Re-initialization attack prevention
     * @dev Ensures deployed accounts cannot be re-initialized
     */
    function testFuzz_ReInitializationPrevented(address owner, uint256 salt, address attacker) public {
        vm.assume(owner != address(0));
        vm.assume(attacker != address(0));
        vm.assume(owner != attacker);

        RampOSAccount account = factory.createAccount(owner, salt);

        // Try to reinitialize
        vm.expectRevert();
        account.initialize(attacker);

        // Owner should still be original
        assertEq(account.owner(), owner, "Owner should not change");
    }

    /**
     * @notice Fuzz test: Factory immutables are consistent
     * @dev Ensures immutable values don't change
     */
    function testFuzz_ImmutablesConsistent(uint256 iterations) public {
        iterations = iterations % 100 + 1;

        address storedImplementation = address(factory.ACCOUNT_IMPLEMENTATION());
        address storedEntryPoint = address(factory.ENTRY_POINT());

        for (uint256 i = 0; i < iterations; i++) {
            assertEq(
                address(factory.ACCOUNT_IMPLEMENTATION()),
                storedImplementation,
                "Implementation should be constant"
            );
            assertEq(
                address(factory.ENTRY_POINT()),
                storedEntryPoint,
                "Entry point should be constant"
            );
        }
    }

    /**
     * @notice Fuzz test: Address prediction before deployment matches after
     * @dev Full cycle test: predict -> create -> verify
     */
    function testFuzz_FullCyclePredictCreateVerify(address owner, uint256 salt) public {
        vm.assume(owner != address(0));

        // Step 1: Predict
        address predicted = factory.getAddress(owner, salt);

        // Verify no code at predicted address
        assertEq(predicted.code.length, 0, "No code should exist before deployment");

        // Step 2: Create
        RampOSAccount account = factory.createAccount(owner, salt);

        // Step 3: Verify
        assertEq(address(account), predicted, "Address should match prediction");
        assertTrue(address(account).code.length > 0, "Code should exist after deployment");
        assertEq(account.owner(), owner, "Owner should be set");

        // Step 4: Verify prediction still returns same address
        assertEq(factory.getAddress(owner, salt), predicted, "Prediction should be stable");
    }

    /**
     * @notice Fuzz test: Multiple factories produce different addresses
     * @dev Tests factory address isolation
     */
    function testFuzz_DifferentFactoriesDifferentAddresses(address owner, uint256 salt) public {
        vm.assume(owner != address(0));

        // Create a second factory
        RampOSAccountFactory factory2 = new RampOSAccountFactory(entryPoint);

        address addr1 = factory.getAddress(owner, salt);
        address addr2 = factory2.getAddress(owner, salt);

        // Addresses should be different because factory address is part of CREATE2
        assertTrue(addr1 != addr2, "Different factories should produce different addresses");
    }

    /**
     * @notice Fuzz test: Account code is minimal proxy
     * @dev Verifies deployed bytecode is EIP-1167 minimal proxy
     */
    function testFuzz_DeployedCodeIsMinimalProxy(address owner, uint256 salt) public {
        vm.assume(owner != address(0));

        RampOSAccount account = factory.createAccount(owner, salt);
        bytes memory code = address(account).code;

        // EIP-1167 minimal proxy is exactly 45 bytes
        // Format: 0x363d3d373d3d3d363d73<impl>5af43d82803e903d91602b57fd5bf3
        assertEq(code.length, 45, "Minimal proxy should be 45 bytes");

        // Check EIP-1167 prefix
        assertEq(uint8(code[0]), 0x36, "Should start with CALLDATASIZE");
        assertEq(uint8(code[1]), 0x3d, "Second byte should be RETURNDATASIZE");
    }

    /**
     * @notice Fuzz test: Created account has correct entry point
     * @dev Verifies entry point is properly set
     */
    function testFuzz_AccountHasCorrectEntryPoint(address owner, uint256 salt) public {
        vm.assume(owner != address(0));

        RampOSAccount account = factory.createAccount(owner, salt);

        assertEq(
            address(account.entryPoint()),
            address(entryPoint),
            "Account entry point should match factory's"
        );
    }
}
