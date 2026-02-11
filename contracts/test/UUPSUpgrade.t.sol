// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { RampOSAccount } from "../src/RampOSAccount.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/// @title RampOSAccountV2 - Mock upgraded contract for testing UUPS upgrade
contract RampOSAccountV2 is RampOSAccount {
    uint256 public newStateVar;

    constructor(IEntryPoint anEntryPoint) RampOSAccount(anEntryPoint) {}

    function version() external pure override returns (string memory) {
        return "2.0.0";
    }

    function setNewStateVar(uint256 _val) external onlyOwner {
        newStateVar = _val;
    }
}

/**
 * @title UUPSUpgradeTest
 * @notice Tests for F14.05 - UUPS proxy pattern on RampOSAccount
 */
contract UUPSUpgradeTest is Test {
    RampOSAccountFactory factory;
    IEntryPoint entryPoint;
    address owner;
    uint256 ownerKey;

    function setUp() public {
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));
        (owner, ownerKey) = makeAddrAndKey("owner");
        factory = new RampOSAccountFactory(entryPoint);
    }

    // ============================================================
    // Test 1: Deploy upgradeable account via factory
    // ============================================================

    function test_DeployUpgradeableAccount() public {
        RampOSAccount account = factory.createUpgradeableAccount(owner, 100);

        // Account should be deployed
        assertTrue(address(account).code.length > 0, "Account should be deployed");

        // Owner should be set correctly
        assertEq(account.owner(), owner, "Owner should match");

        // Version should be 1.0.0
        assertEq(
            keccak256(bytes(account.version())),
            keccak256(bytes("1.0.0")),
            "Version should be 1.0.0"
        );
    }

    // ============================================================
    // Test 2: Owner can upgrade
    // ============================================================

    function test_OwnerCanUpgrade() public {
        RampOSAccount account = factory.createUpgradeableAccount(owner, 200);

        // Deploy new implementation
        RampOSAccountV2 newImpl = new RampOSAccountV2(entryPoint);

        // Owner upgrades
        vm.prank(owner);
        account.upgradeToAndCall(address(newImpl), "");

        // Verify upgraded version
        assertEq(
            keccak256(bytes(account.version())),
            keccak256(bytes("2.0.0")),
            "Version should be 2.0.0 after upgrade"
        );
    }

    // ============================================================
    // Test 3: Non-owner cannot upgrade
    // ============================================================

    function test_NonOwnerCannotUpgrade() public {
        RampOSAccount account = factory.createUpgradeableAccount(owner, 300);

        RampOSAccountV2 newImpl = new RampOSAccountV2(entryPoint);

        address attacker = makeAddr("attacker");
        vm.prank(attacker);
        vm.expectRevert(RampOSAccount.NotOwner.selector);
        account.upgradeToAndCall(address(newImpl), "");
    }

    // ============================================================
    // Test 4: Upgraded contract has new version
    // ============================================================

    function test_UpgradedContractHasNewVersion() public {
        RampOSAccount account = factory.createUpgradeableAccount(owner, 400);

        // Check initial version
        assertEq(
            keccak256(bytes(account.version())),
            keccak256(bytes("1.0.0")),
            "Initial version should be 1.0.0"
        );

        // Deploy V2 and upgrade
        RampOSAccountV2 newImpl = new RampOSAccountV2(entryPoint);
        vm.prank(owner);
        account.upgradeToAndCall(address(newImpl), "");

        // Check upgraded version
        assertEq(
            keccak256(bytes(account.version())),
            keccak256(bytes("2.0.0")),
            "Upgraded version should be 2.0.0"
        );
    }

    // ============================================================
    // Test 5: State is preserved after upgrade
    // ============================================================

    function test_StatePreservedAfterUpgrade() public {
        RampOSAccount account = factory.createUpgradeableAccount(owner, 500);

        // Set some state: add a session key
        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        address[] memory targets = new address[](0);
        bytes4[] memory selectors = new bytes4[](0);
        RampOSAccount.SessionKeyPermissions memory perms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets,
            allowedSelectors: selectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, perms);

        // Fund the account with ETH
        vm.deal(address(account), 1 ether);

        // Verify state before upgrade
        assertEq(account.owner(), owner, "Owner should be set before upgrade");
        assertTrue(account.isValidSessionKey(sessionKey), "Session key should be valid before upgrade");
        assertEq(address(account).balance, 1 ether, "ETH balance should be 1 ether before upgrade");

        // Upgrade to V2
        RampOSAccountV2 newImpl = new RampOSAccountV2(entryPoint);
        vm.prank(owner);
        account.upgradeToAndCall(address(newImpl), "");

        // Verify all state preserved after upgrade
        assertEq(account.owner(), owner, "Owner should be preserved after upgrade");
        assertTrue(
            account.isValidSessionKey(sessionKey), "Session key should be preserved after upgrade"
        );
        assertEq(
            address(account).balance, 1 ether, "ETH balance should be preserved after upgrade"
        );
    }

    // ============================================================
    // Test 6: Proxy delegates correctly (V2 new functionality works)
    // ============================================================

    function test_ProxyDelegatesCorrectly() public {
        RampOSAccount account = factory.createUpgradeableAccount(owner, 600);

        // Upgrade to V2
        RampOSAccountV2 newImpl = new RampOSAccountV2(entryPoint);
        vm.prank(owner);
        account.upgradeToAndCall(address(newImpl), "");

        // Cast to V2 to access new functions
        RampOSAccountV2 accountV2 = RampOSAccountV2(payable(address(account)));

        // Use new V2 functionality
        vm.prank(owner);
        accountV2.setNewStateVar(42);

        assertEq(accountV2.newStateVar(), 42, "New state variable should be set via proxy");
    }

    // ============================================================
    // Test 7: Counterfactual address prediction works
    // ============================================================

    function test_UpgradeableAddressPrediction() public {
        address predicted = factory.getUpgradeableAccountAddress(owner, 700);

        RampOSAccount account = factory.createUpgradeableAccount(owner, 700);

        assertEq(
            address(account),
            predicted,
            "Deployed address should match predicted address"
        );
    }

    // ============================================================
    // Test 8: Re-calling createUpgradeableAccount returns existing
    // ============================================================

    function test_CreateUpgradeableAccountIdempotent() public {
        RampOSAccount account1 = factory.createUpgradeableAccount(owner, 800);
        RampOSAccount account2 = factory.createUpgradeableAccount(owner, 800);

        assertEq(
            address(account1),
            address(account2),
            "Second call should return existing account"
        );
    }

    // ============================================================
    // Test 9: Legacy createAccount still works (backwards compatibility)
    // ============================================================

    function test_LegacyCreateAccountStillWorks() public {
        RampOSAccount account = factory.createAccount(owner, 900);

        assertTrue(address(account).code.length > 0, "Legacy account should deploy");
        assertEq(account.owner(), owner, "Legacy account owner should match");
    }

    // ============================================================
    // Test 10: Upgrade with initializer call via upgradeToAndCall
    // ============================================================

    function test_UpgradeToAndCallWithData() public {
        RampOSAccount account = factory.createUpgradeableAccount(owner, 1000);

        RampOSAccountV2 newImpl = new RampOSAccountV2(entryPoint);

        // Upgrade and call setNewStateVar in the same transaction
        bytes memory data =
            abi.encodeWithSelector(RampOSAccountV2.setNewStateVar.selector, 99);
        vm.prank(owner);
        account.upgradeToAndCall(address(newImpl), data);

        RampOSAccountV2 accountV2 = RampOSAccountV2(payable(address(account)));
        assertEq(accountV2.newStateVar(), 99, "State should be set during upgrade");
        assertEq(
            keccak256(bytes(accountV2.version())),
            keccak256(bytes("2.0.0")),
            "Version should be 2.0.0"
        );
    }
}
