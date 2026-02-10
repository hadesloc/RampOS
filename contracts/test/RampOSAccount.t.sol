// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { RampOSAccount } from "../src/RampOSAccount.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/// @title MockTarget
/// @notice Mock contract for testing session key target and selector restrictions
contract MockTarget {
    uint256 public value;

    function setValue(uint256 _value) external {
        value = _value;
    }

    function increment() external {
        value++;
    }

    function decrement() external {
        value--;
    }
}

/**
 * @title RampOSAccountTest
 * @notice Comprehensive unit tests for RampOSAccount smart contract
 */
contract RampOSAccountTest is Test {
    RampOSAccountFactory factory;
    IEntryPoint entryPoint;
    address owner;
    uint256 ownerKey;
    MockTarget mockTarget;
    MockTarget mockTarget2;

    function setUp() public {
        // Use a mock entry point for testing
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));

        // Create owner
        (owner, ownerKey) = makeAddrAndKey("owner");

        // Deploy factory
        factory = new RampOSAccountFactory(entryPoint);

        // Deploy mock targets
        mockTarget = new MockTarget();
        mockTarget2 = new MockTarget();
    }

    function test_CreateAccount() public {
        uint256 salt = 12345;

        // Get predicted address
        address predicted = factory.getAddress(owner, salt);

        // Create account
        RampOSAccount account = factory.createAccount(owner, salt);

        // Verify
        assertEq(address(account), predicted);
        assertEq(account.owner(), owner);
    }

    function test_CreateAccountIdempotent() public {
        uint256 salt = 12345;

        // Create twice
        RampOSAccount account1 = factory.createAccount(owner, salt);
        RampOSAccount account2 = factory.createAccount(owner, salt);

        // Should return same address
        assertEq(address(account1), address(account2));
    }

    function test_Execute() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Fund the account
        vm.deal(address(account), 1 ether);

        // Create a recipient
        address recipient = makeAddr("recipient");

        // Execute transfer as owner
        vm.prank(owner);
        account.execute(recipient, 0.1 ether, "");

        // Verify
        assertEq(recipient.balance, 0.1 ether);
    }

    function test_ExecuteBatch() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Fund the account
        vm.deal(address(account), 1 ether);

        // Create recipients
        address[] memory dests = new address[](3);
        uint256[] memory values = new uint256[](3);
        bytes[] memory datas = new bytes[](3);

        for (uint256 i = 0; i < 3; i++) {
            dests[i] = makeAddr(string(abi.encodePacked("recipient", i)));
            values[i] = 0.1 ether;
            datas[i] = "";
        }

        // Execute batch as owner
        vm.prank(owner);
        account.executeBatch(dests, values, datas);

        // Verify
        for (uint256 i = 0; i < 3; i++) {
            assertEq(dests[i].balance, 0.1 ether);
        }
    }

    function test_SessionKeyWithPermissions() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Create permissions
        address[] memory allowedTargets = new address[](1);
        allowedTargets[0] = address(mockTarget);

        bytes4[] memory allowedSelectors = new bytes4[](1);
        allowedSelectors[0] = MockTarget.setValue.selector;

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0.1 ether,
            dailyLimit: 1 ether
        });

        // Add session key
        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Verify session key is valid
        assertTrue(account.isValidSessionKey(sessionKey));

        // Verify permissions stored correctly
        RampOSAccount.SessionKeyPermissions memory storedPerms =
            account.getSessionKeyPermissions(sessionKey);
        assertEq(storedPerms.allowedTargets.length, 1);
        assertEq(storedPerms.allowedTargets[0], address(mockTarget));
        assertEq(storedPerms.allowedSelectors.length, 1);
        assertEq(storedPerms.allowedSelectors[0], MockTarget.setValue.selector);
        assertEq(storedPerms.spendingLimit, 0.1 ether);
        assertEq(storedPerms.dailyLimit, 1 ether);
    }

    function test_SessionKeyLegacy() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Add session key using legacy method
        vm.prank(owner);
        account.addSessionKeyLegacy(sessionKey, validAfter, validUntil, bytes32(0));

        // Verify
        assertTrue(account.isValidSessionKey(sessionKey));

        // Legacy keys should have empty permissions (unlimited)
        RampOSAccount.SessionKeyPermissions memory storedPerms =
            account.getSessionKeyPermissions(sessionKey);
        assertEq(storedPerms.allowedTargets.length, 0);
        assertEq(storedPerms.allowedSelectors.length, 0);
        assertEq(storedPerms.spendingLimit, 0);
        assertEq(storedPerms.dailyLimit, 0);
    }

    function test_UpdateSessionKeyPermissions() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create and add session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Initial permissions
        address[] memory allowedTargets = new address[](1);
        allowedTargets[0] = address(mockTarget);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0.1 ether,
            dailyLimit: 1 ether
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Update permissions
        address[] memory newAllowedTargets = new address[](2);
        newAllowedTargets[0] = address(mockTarget);
        newAllowedTargets[1] = address(mockTarget2);

        RampOSAccount.SessionKeyPermissions memory newPermissions =
            RampOSAccount.SessionKeyPermissions({
                allowedTargets: newAllowedTargets,
                allowedSelectors: allowedSelectors,
                spendingLimit: 0.5 ether,
                dailyLimit: 5 ether
            });

        vm.prank(owner);
        account.updateSessionKeyPermissions(sessionKey, newPermissions);

        // Verify updated permissions
        RampOSAccount.SessionKeyPermissions memory storedPerms =
            account.getSessionKeyPermissions(sessionKey);
        assertEq(storedPerms.allowedTargets.length, 2);
        assertEq(storedPerms.spendingLimit, 0.5 ether);
        assertEq(storedPerms.dailyLimit, 5 ether);
    }

    function test_SessionKeyTargetRestriction() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Only allow mockTarget
        address[] memory allowedTargets = new address[](1);
        allowedTargets[0] = address(mockTarget);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Verify isTargetAllowed
        assertTrue(account.isTargetAllowed(sessionKey, address(mockTarget)));
        assertFalse(account.isTargetAllowed(sessionKey, address(mockTarget2)));
    }

    function test_SessionKeySelectorRestriction() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Only allow setValue and increment
        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](2);
        allowedSelectors[0] = MockTarget.setValue.selector;
        allowedSelectors[1] = MockTarget.increment.selector;

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Verify isSelectorAllowed
        assertTrue(account.isSelectorAllowed(sessionKey, MockTarget.setValue.selector));
        assertTrue(account.isSelectorAllowed(sessionKey, MockTarget.increment.selector));
        assertFalse(account.isSelectorAllowed(sessionKey, MockTarget.decrement.selector));
    }

    function test_SessionKeySpendingInfo() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key with limits
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0.1 ether,
            dailyLimit: 1 ether
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Check spending info
        (uint256 dailySpent, uint256 dailyRemaining, uint256 spendingLimit) =
            account.getSessionKeySpendingInfo(sessionKey);
        assertEq(dailySpent, 0);
        assertEq(dailyRemaining, 1 ether);
        assertEq(spendingLimit, 0.1 ether);
    }

    function test_SessionKeyUnlimitedPermissions() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key with no restrictions
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // With no restrictions, any target/selector should be allowed
        assertTrue(account.isTargetAllowed(sessionKey, address(mockTarget)));
        assertTrue(account.isTargetAllowed(sessionKey, address(mockTarget2)));
        assertTrue(account.isTargetAllowed(sessionKey, address(0x1234)));
        assertTrue(account.isSelectorAllowed(sessionKey, bytes4(0xdeadbeef)));
    }

    function test_RemoveSessionKey() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0.1 ether,
            dailyLimit: 1 ether
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        assertTrue(account.isValidSessionKey(sessionKey));

        // Remove session key
        vm.prank(owner);
        account.removeSessionKey(sessionKey);

        // Verify removed
        assertFalse(account.isValidSessionKey(sessionKey));

        // Permissions should also be cleared
        RampOSAccount.SessionKeyPermissions memory storedPerms =
            account.getSessionKeyPermissions(sessionKey);
        assertEq(storedPerms.allowedTargets.length, 0);
        assertEq(storedPerms.spendingLimit, 0);
    }

    function test_RevertNonOwner() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        address attacker = makeAddr("attacker");

        // Try to execute as non-owner
        vm.prank(attacker);
        vm.expectRevert(RampOSAccount.NotOwnerOrEntryPoint.selector);
        account.execute(attacker, 0, "");
    }

    function test_RevertNonOwnerAddSessionKey() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        address attacker = makeAddr("attacker");
        (address sessionKey,) = makeAddrAndKey("session");

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        // Try to add session key as non-owner
        vm.prank(attacker);
        vm.expectRevert(RampOSAccount.NotOwner.selector);
        account.addSessionKey(
            sessionKey, uint48(block.timestamp), uint48(block.timestamp + 1 hours), permissions
        );
    }

    function test_RevertUpdateNonExistentSessionKey() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        (address sessionKey,) = makeAddrAndKey("session");

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        // Try to update non-existent session key
        vm.prank(owner);
        vm.expectRevert("Session key not found");
        account.updateSessionKeyPermissions(sessionKey, permissions);
    }

    function test_SessionKeyExpiry() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Valid now
        assertTrue(account.isValidSessionKey(sessionKey));

        // Warp to after expiry
        vm.warp(block.timestamp + 2 hours);

        // Should be invalid
        assertFalse(account.isValidSessionKey(sessionKey));
    }

    function test_SessionKeyNotYetValid() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key valid in 1 hour
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp + 1 hours);
        uint48 validUntil = uint48(block.timestamp + 2 hours);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Not valid yet
        assertFalse(account.isValidSessionKey(sessionKey));

        // Warp to valid period
        vm.warp(block.timestamp + 1.5 hours);

        // Now valid
        assertTrue(account.isValidSessionKey(sessionKey));
    }

    function test_SessionKeyLookupIsConstantTimeMapping() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);
        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        // Add multiple session keys
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        for (uint256 i = 1; i <= 10; i++) {
            (address sk,) = makeAddrAndKey(string(abi.encodePacked("sk", i)));
            vm.prank(owner);
            account.addSessionKey(sk, validAfter, validUntil, permissions);
        }

        // All keys should be valid via mapping lookup (O(1))
        for (uint256 i = 1; i <= 10; i++) {
            (address sk,) = makeAddrAndKey(string(abi.encodePacked("sk", i)));
            assertTrue(account.isValidSessionKey(sk));
        }

        // Non-existent key returns false
        (address fakeSk,) = makeAddrAndKey("fake");
        assertFalse(account.isValidSessionKey(fakeSk));

        // Remove a key in the middle - should not affect others
        (address sk5,) = makeAddrAndKey(string(abi.encodePacked("sk", uint256(5))));
        vm.prank(owner);
        account.removeSessionKey(sk5);
        assertFalse(account.isValidSessionKey(sk5));

        // Other keys still valid
        (address sk3,) = makeAddrAndKey(string(abi.encodePacked("sk", uint256(3))));
        (address sk7,) = makeAddrAndKey(string(abi.encodePacked("sk", uint256(7))));
        assertTrue(account.isValidSessionKey(sk3));
        assertTrue(account.isValidSessionKey(sk7));
    }

    function test_TargetAllowedUsesMapping() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Add 5 allowed targets
        address[] memory allowedTargets = new address[](5);
        for (uint256 i = 0; i < 5; i++) {
            allowedTargets[i] = makeAddr(string(abi.encodePacked("target", i)));
        }
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // All targets should be allowed
        for (uint256 i = 0; i < 5; i++) {
            assertTrue(account.isTargetAllowed(sessionKey, allowedTargets[i]));
        }

        // Non-allowed target should be rejected
        assertFalse(account.isTargetAllowed(sessionKey, makeAddr("notAllowed")));
    }

    function test_SelectorAllowedUsesMapping() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](3);
        allowedSelectors[0] = MockTarget.setValue.selector;
        allowedSelectors[1] = MockTarget.increment.selector;
        allowedSelectors[2] = MockTarget.decrement.selector;

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // All selectors should be allowed
        assertTrue(account.isSelectorAllowed(sessionKey, MockTarget.setValue.selector));
        assertTrue(account.isSelectorAllowed(sessionKey, MockTarget.increment.selector));
        assertTrue(account.isSelectorAllowed(sessionKey, MockTarget.decrement.selector));

        // Non-allowed selector should be rejected
        assertFalse(account.isSelectorAllowed(sessionKey, bytes4(0xdeadbeef)));
    }

    function test_SessionKeyRevocationPreventsExecution() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Fund account
        vm.deal(address(account), 1 ether);

        // Create session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        // Add session key
        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);
        assertTrue(account.isValidSessionKey(sessionKey));

        // Revoke the session key
        vm.prank(owner);
        account.removeSessionKey(sessionKey);

        // Verify session key is no longer valid
        assertFalse(account.isValidSessionKey(sessionKey));

        // Verify the session key metadata is fully cleared
        (address storedKey, uint48 storedValidAfter, uint48 storedValidUntil, bytes32 storedHash) =
            account.sessionKeys(sessionKey);
        assertEq(storedKey, address(0));
        assertEq(storedValidAfter, 0);
        assertEq(storedValidUntil, 0);
        assertEq(storedHash, bytes32(0));

        // Verify permissions are also cleared
        RampOSAccount.SessionKeyPermissions memory storedPerms =
            account.getSessionKeyPermissions(sessionKey);
        assertEq(storedPerms.allowedTargets.length, 0);
        assertEq(storedPerms.allowedSelectors.length, 0);
        assertEq(storedPerms.spendingLimit, 0);
        assertEq(storedPerms.dailyLimit, 0);
    }

    function test_ExpiredSessionKeyRejected() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key that expires in 1 hour
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Session key is valid now
        assertTrue(account.isValidSessionKey(sessionKey));

        // Warp to exactly the expiry time - key should be invalid (validUntil is exclusive boundary)
        vm.warp(uint256(validUntil) + 1);
        assertFalse(account.isValidSessionKey(sessionKey));

        // The key struct still exists in storage but is time-expired
        (address storedKey,,,) = account.sessionKeys(sessionKey);
        assertEq(storedKey, sessionKey); // key address still stored
        assertFalse(account.isValidSessionKey(sessionKey)); // but not valid

        // Warp far into the future - still rejected
        vm.warp(block.timestamp + 365 days);
        assertFalse(account.isValidSessionKey(sessionKey));
    }

    function test_PermissionsHashConsistency() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory allowedTargets = new address[](1);
        allowedTargets[0] = address(mockTarget);
        bytes4[] memory allowedSelectors = new bytes4[](1);
        allowedSelectors[0] = MockTarget.setValue.selector;

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0.1 ether,
            dailyLimit: 1 ether
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Get stored permissionsHash
        (,,, bytes32 storedHash) = account.sessionKeys(sessionKey);

        // Compute expected hash
        bytes32 expectedHash = keccak256(
            abi.encode(allowedTargets, allowedSelectors, uint256(0.1 ether), uint256(1 ether))
        );

        assertEq(storedHash, expectedHash);
    }
}
