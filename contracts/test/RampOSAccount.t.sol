// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/RampOSAccount.sol";
import "../src/RampOSAccountFactory.sol";
import "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

contract RampOSAccountTest is Test {
    RampOSAccountFactory factory;
    IEntryPoint entryPoint;
    address owner;
    uint256 ownerKey;

    function setUp() public {
        // Use a mock entry point for testing
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));

        // Create owner
        (owner, ownerKey) = makeAddrAndKey("owner");

        // Deploy factory
        factory = new RampOSAccountFactory(entryPoint);
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

    function test_SessionKey() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey, ) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Add session key
        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, bytes32(0));

        // Verify
        assertTrue(account.isValidSessionKey(sessionKey));

        // Remove session key
        vm.prank(owner);
        account.removeSessionKey(sessionKey);

        // Verify removed
        assertFalse(account.isValidSessionKey(sessionKey));
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
}
