// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { RampOSAccount } from "../../src/RampOSAccount.sol";
import { RampOSAccountFactory } from "../../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import { MockEntryPoint } from "../mocks/MockEntryPoint.sol";
import {
    AccountReentrancyAttacker,
    AccountBatchReentrancyAttacker,
    PaymasterReentrancyAttacker
} from "./Attacker.sol";

contract ReentrancyAttackTest is Test {
    RampOSAccountFactory factory;
    RampOSAccount account;
    RampOSPaymaster paymaster;
    IEntryPoint entryPoint;
    MockEntryPoint mockEntryPoint;
    address owner;
    uint256 ownerKey;
    address paymasterOwner;
    uint256 paymasterOwnerKey;
    address signer;
    uint256 signerKey;

    function setUp() public {
        // Deploy mock entry point
        mockEntryPoint = new MockEntryPoint();
        entryPoint = IEntryPoint(address(mockEntryPoint));

        // Create owner
        (owner, ownerKey) = makeAddrAndKey("owner");
        (paymasterOwner, paymasterOwnerKey) = makeAddrAndKey("paymasterOwner");
        vm.deal(paymasterOwner, 100 ether); // Give owner ETH for deposit
        (signer, signerKey) = makeAddrAndKey("signer");

        // Deploy factory
        factory = new RampOSAccountFactory(entryPoint);

        // Create account
        account = factory.createAccount(owner, 12345);
        vm.deal(address(account), 10 ether);

        // Deploy Paymaster
        vm.prank(paymasterOwner);
        paymaster = new RampOSPaymaster(entryPoint, signer);
        vm.deal(address(paymaster), 10 ether);

        // Fund paymaster's deposit in EntryPoint
        vm.prank(paymasterOwner);
        paymaster.deposit{value: 5 ether}();
    }

    function test_AccountExecuteReentrancy() public {
        // Deploy attacker
        AccountReentrancyAttacker attacker = new AccountReentrancyAttacker(account);

        // Setup attack: owner calls execute -> attacker -> execute (reenter)
        vm.prank(owner);
        try account.execute(address(attacker), 0, "") {
            // Success means the first call worked
        } catch {
            fail("First execute call failed");
        }

        // Check if reentrancy was successful
        assertFalse(attacker.success(), "Reentrancy attack should fail");
        // Verify only 1 attack attempt was made (it tries once)
        assertEq(attacker.attackCount(), 1, "Should have attempted attack");
    }

    function test_AccountExecuteBatchReentrancy() public {
        // Deploy attacker
        AccountBatchReentrancyAttacker attacker = new AccountBatchReentrancyAttacker(account);

        address[] memory dests = new address[](1);
        dests[0] = address(attacker);
        uint256[] memory values = new uint256[](1);
        values[0] = 0;
        bytes[] memory datas = new bytes[](1);
        datas[0] = "";

        // Setup attack: owner calls executeBatch -> attacker -> executeBatch (reenter)
        vm.prank(owner);
        try account.executeBatch(dests, values, datas) {
            // Success means the first call worked
        } catch {
            fail("First executeBatch call failed");
        }

        // Check if reentrancy was successful
        assertFalse(attacker.success(), "Batch reentrancy attack should fail");
        assertEq(attacker.attackCount(), 1, "Should have attempted attack");
    }

    function test_PaymasterExecuteWithdrawReentrancy() public {
        // Deploy attacker
        PaymasterReentrancyAttacker attacker = new PaymasterReentrancyAttacker(paymaster);

        // 1. Request withdraw to attacker
        vm.startPrank(paymasterOwner);
        paymaster.requestWithdraw(payable(address(attacker)), 1 ether);

        // 2. Warp time to pass delay
        vm.warp(block.timestamp + 24 hours + 1);

        // 3. Execute withdraw
        // The attacker will receive funds and try to call executeWithdraw again
        // We expect the second call (inside attacker) to fail, but the first one to succeed
        paymaster.executeWithdraw();
        vm.stopPrank();

        // Check if reentrancy was successful
        assertFalse(attacker.success(), "Paymaster reentrancy attack should fail");
        assertEq(attacker.attackCount(), 1, "Should have attempted attack");
        assertEq(address(attacker).balance, 1 ether, "Attacker should have received funds");

        // Verify state is cleared
        (address to, uint256 amount,,) = paymaster.getPendingWithdraw();
        assertEq(amount, 0, "Pending amount should be 0");
        assertEq(to, address(0), "Pending to should be 0");
    }

    function test_CrossFunctionReentrancy() public {
        // Test execute -> executeBatch reentrancy
        // Deploy a custom attacker for this
        CrossFunctionAttacker attacker = new CrossFunctionAttacker(account);

        vm.prank(owner);
        try account.execute(address(attacker), 0, "") {
        } catch {
            fail("First execute call failed");
        }

        assertFalse(attacker.success(), "Cross-function reentrancy should fail");
    }
}

contract CrossFunctionAttacker {
    RampOSAccount public target;
    bool public success;
    uint256 public attackCount;

    constructor(RampOSAccount _target) {
        target = _target;
    }

    receive() external payable {
        if (attackCount == 0) {
            attackCount++;

            address[] memory dests = new address[](1);
            dests[0] = address(this);
            uint256[] memory values = new uint256[](1);
            values[0] = 0;
            bytes[] memory datas = new bytes[](1);
            datas[0] = "";

            // Try to re-enter with executeBatch
            try target.executeBatch(dests, values, datas) {
                success = true;
            } catch {
                success = false;
            }
        }
    }
}
