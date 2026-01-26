// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/RampOSAccountFactory.sol";
import "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

contract RampOSAccountFactoryTest is Test {
    RampOSAccountFactory factory;
    IEntryPoint entryPoint;
    address owner;

    function setUp() public {
        // Use a mock entry point
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));

        factory = new RampOSAccountFactory(entryPoint);
        owner = makeAddr("owner");
    }

    function test_CreateAccount() public {
        uint256 salt = 123;

        address predicted = factory.getAddress(owner, salt);

        // Expect event
        vm.expectEmit(true, true, true, true);
        emit RampOSAccountFactory.AccountCreated(predicted, owner, salt);

        RampOSAccount account = factory.createAccount(owner, salt);

        assertEq(address(account), predicted);
        assertEq(account.owner(), owner);
        assertEq(address(account.entryPoint()), address(entryPoint));
    }

    function test_CreateAccountDeterministic() public {
        uint256 salt = 456;

        address addr1 = factory.getAddress(owner, salt);
        RampOSAccount account1 = factory.createAccount(owner, salt);
        assertEq(address(account1), addr1);

        // Calling create again should return existing address
        RampOSAccount account2 = factory.createAccount(owner, salt);
        assertEq(address(account2), addr1);
    }
}
