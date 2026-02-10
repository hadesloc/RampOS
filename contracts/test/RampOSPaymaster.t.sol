// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import { IStakeManager } from "@account-abstraction/contracts/interfaces/IStakeManager.sol";
import {
    PackedUserOperation
} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";
import { MessageHashUtils } from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title RampOSPaymasterTest
 * @notice Unit tests for RampOSPaymaster
 */
contract RampOSPaymasterTest is Test {
    using MessageHashUtils for bytes32;

    RampOSPaymaster paymaster;
    IEntryPoint entryPoint;
    address signer;
    uint256 signerKey;
    address owner;
    address payable recipient;

    function setUp() public {
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));
        (signer, signerKey) = makeAddrAndKey("signer");
        owner = makeAddr("owner");
        recipient = payable(makeAddr("recipient"));

        vm.prank(owner);
        paymaster = new RampOSPaymaster(entryPoint, signer);
    }

    function test_ValidateUserOp() public {
        PackedUserOperation memory userOp;
        userOp.sender = makeAddr("sender");
        userOp.nonce = 0;

        bytes32 userOpHash = keccak256("userOp");

        bytes32 tenantId = keccak256("tenant1");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        uint256 nonce = 0;

        // Construct signature (now includes nonce, chainid, and paymaster address)
        bytes32 hash = keccak256(
            abi.encodePacked(
                userOpHash, tenantId, validUntil, validAfter,
                nonce,
                block.chainid, address(paymaster)
            )
        ).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        // Construct paymasterAndData
        // address(20) + tenantId(32) + validUntil(6) + validAfter(6) + nonce(32) + signature(65)
        bytes memory paymasterAndData =
            abi.encodePacked(address(paymaster), tenantId, validUntil, validAfter, nonce, signature);
        userOp.paymasterAndData = paymasterAndData;

        // Mock entry point call
        vm.prank(address(entryPoint));
        (bytes memory context, uint256 validationData) =
            paymaster.validatePaymasterUserOp(
                userOp,
                userOpHash,
                1e18 // maxCost
            );

        assertEq(validationData & 1, 0); // Success (sigFailed bit is 0)

        // Decode context
        (address sender, bytes32 tid, uint256 cost) =
            abi.decode(context, (address, bytes32, uint256));
        assertEq(sender, userOp.sender);
        assertEq(tid, tenantId);
        assertEq(cost, 1e18);

        // Verify nonce was incremented
        assertEq(paymaster.senderNonces(userOp.sender), 1);
    }

    function test_TenantLimit() public {
        bytes32 tenantId = keccak256("tenant1");

        vm.prank(owner);
        paymaster.setTenantLimit(tenantId, 1 ether);

        address sender = makeAddr("sender");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        // First op ok (0.5 eth) - nonce 0
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 0, 0.5 ether);

        // Second op ok (0.5 eth) - nonce 1
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 1, 0.5 ether);

        // Third op fails (> 1 eth total) - nonce 2
        vm.expectRevert(RampOSPaymaster.TenantLimitExceeded.selector);
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 2, 0.1 ether);
    }

    // ============ Timelock Tests ============

    function test_RequestWithdraw() public {
        uint256 amount = 1 ether;

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, amount);

        (address to, uint256 pendingAmount, uint256 requestTime, uint256 executeAfter) =
            paymaster.getPendingWithdraw();

        assertEq(to, recipient);
        assertEq(pendingAmount, amount);
        assertEq(requestTime, block.timestamp);
        assertEq(executeAfter, block.timestamp + 24 hours);
    }

    function test_RequestWithdraw_EmitsEvent() public {
        uint256 amount = 1 ether;

        vm.expectEmit(true, false, false, true);
        emit RampOSPaymaster.WithdrawRequested(recipient, amount, block.timestamp + 24 hours);

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, amount);
    }

    function test_RequestWithdraw_RevertIfAlreadyPending() public {
        vm.startPrank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        vm.expectRevert(RampOSPaymaster.WithdrawAlreadyPending.selector);
        paymaster.requestWithdraw(recipient, 2 ether);
        vm.stopPrank();
    }

    function test_RequestWithdraw_OnlyOwner() public {
        address notOwner = makeAddr("notOwner");

        vm.prank(notOwner);
        vm.expectRevert(abi.encodeWithSignature("OwnableUnauthorizedAccount(address)", notOwner));
        paymaster.requestWithdraw(recipient, 1 ether);
    }

    function test_ExecuteWithdraw_RevertIfNotReady() public {
        vm.startPrank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        // Try to execute immediately
        vm.expectRevert(RampOSPaymaster.WithdrawNotReady.selector);
        paymaster.executeWithdraw();
        vm.stopPrank();
    }

    function test_ExecuteWithdraw_RevertIfNoPending() public {
        vm.prank(owner);
        vm.expectRevert(RampOSPaymaster.NoWithdrawPending.selector);
        paymaster.executeWithdraw();
    }

    function test_ExecuteWithdraw_AfterTimelock() public {
        uint256 amount = 1 ether;

        // Setup: Mock the entryPoint to handle the withdrawal
        vm.mockCall(
            address(entryPoint),
            abi.encodeWithSelector(IStakeManager.withdrawTo.selector, recipient, amount),
            abi.encode()
        );

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, amount);

        // Warp time past the timelock
        vm.warp(block.timestamp + 24 hours + 1);

        vm.expectEmit(true, false, false, true);
        emit RampOSPaymaster.WithdrawExecuted(recipient, amount);

        vm.prank(owner);
        paymaster.executeWithdraw();

        // Verify state is cleared
        (address to, uint256 pendingAmount,,) = paymaster.getPendingWithdraw();
        assertEq(to, address(0));
        assertEq(pendingAmount, 0);
    }

    function test_ExecuteWithdraw_RevertIfExpired() public {
        uint256 amount = 1 ether;

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, amount);

        // Warp time past the expiry window (24 hours + 7 days + 1 second)
        vm.warp(block.timestamp + 24 hours + 7 days + 1);

        vm.prank(owner);
        vm.expectRevert(RampOSPaymaster.WithdrawExpired.selector);
        paymaster.executeWithdraw();
    }

    function test_ExecuteWithdraw_OnlyOwner() public {
        vm.prank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        vm.warp(block.timestamp + 24 hours + 1);

        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(abi.encodeWithSignature("OwnableUnauthorizedAccount(address)", notOwner));
        paymaster.executeWithdraw();
    }

    function test_CancelWithdraw() public {
        uint256 amount = 1 ether;

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, amount);

        vm.expectEmit(true, false, false, true);
        emit RampOSPaymaster.WithdrawCancelled(recipient, amount);

        vm.prank(owner);
        paymaster.cancelWithdraw();

        // Verify state is cleared
        (address to, uint256 pendingAmount,,) = paymaster.getPendingWithdraw();
        assertEq(to, address(0));
        assertEq(pendingAmount, 0);
    }

    function test_CancelWithdraw_RevertIfNoPending() public {
        vm.prank(owner);
        vm.expectRevert(RampOSPaymaster.NoWithdrawPending.selector);
        paymaster.cancelWithdraw();
    }

    function test_CancelWithdraw_OnlyOwner() public {
        vm.prank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(abi.encodeWithSignature("OwnableUnauthorizedAccount(address)", notOwner));
        paymaster.cancelWithdraw();
    }

    function test_GetWithdrawTimeRemaining() public {
        // No pending withdrawal
        assertEq(paymaster.getWithdrawTimeRemaining(), 0);

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        // Should be ~24 hours remaining
        assertEq(paymaster.getWithdrawTimeRemaining(), 24 hours);

        // Warp 12 hours
        vm.warp(block.timestamp + 12 hours);
        assertEq(paymaster.getWithdrawTimeRemaining(), 12 hours);

        // Warp past timelock
        vm.warp(block.timestamp + 13 hours);
        assertEq(paymaster.getWithdrawTimeRemaining(), 0);
    }

    function test_IsWithdrawReady() public {
        // No pending withdrawal
        assertFalse(paymaster.isWithdrawReady());

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        // Not ready yet
        assertFalse(paymaster.isWithdrawReady());

        // Warp to exactly 24 hours
        vm.warp(block.timestamp + 24 hours);
        assertTrue(paymaster.isWithdrawReady());

        // Still ready at 24 hours + 7 days
        vm.warp(block.timestamp + 7 days);
        assertTrue(paymaster.isWithdrawReady());

        // Expired after 24 hours + 7 days + 1 second
        vm.warp(block.timestamp + 1);
        assertFalse(paymaster.isWithdrawReady());
    }

    function test_WithdrawToLegacy_Reverts() public {
        vm.prank(owner);
        vm.expectRevert("Use requestWithdraw + executeWithdraw");
        paymaster.withdrawTo(recipient, 1 ether);
    }

    function test_RequestWithdraw_ThenCancel_ThenNewRequest() public {
        vm.startPrank(owner);

        // First request
        paymaster.requestWithdraw(recipient, 1 ether);

        // Cancel it
        paymaster.cancelWithdraw();

        // Should be able to make a new request
        address payable newRecipient = payable(makeAddr("newRecipient"));
        paymaster.requestWithdraw(newRecipient, 2 ether);

        (address to, uint256 amount,,) = paymaster.getPendingWithdraw();
        assertEq(to, newRecipient);
        assertEq(amount, 2 ether);

        vm.stopPrank();
    }

    function test_WITHDRAW_DELAY_Is24Hours() public view {
        assertEq(paymaster.WITHDRAW_DELAY(), 24 hours);
    }

    // ============ Nonce-Based Replay Prevention Tests ============

    function test_PaymasterNonceReplayPrevention() public {
        bytes32 tenantId = keccak256("tenant1");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        address sender = makeAddr("sender");

        // Nonce 0 should work
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 0, 0.1 ether);

        // Check nonce incremented
        assertEq(paymaster.senderNonces(sender), 1);

        // Nonce 1 should work
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 1, 0.1 ether);

        // Check nonce incremented again
        assertEq(paymaster.senderNonces(sender), 2);
    }

    function test_PaymasterNonceRejectsReplay() public {
        bytes32 tenantId = keccak256("tenant1");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        address sender = makeAddr("sender");

        // Nonce 0 should work
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 0, 0.1 ether);

        // Trying nonce 0 again should fail (replay)
        vm.expectRevert(RampOSPaymaster.InvalidNonce.selector);
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 0, 0.1 ether);
    }

    function test_PaymasterNonceRejectsOutOfOrder() public {
        bytes32 tenantId = keccak256("tenant1");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        address sender = makeAddr("sender");

        // Trying nonce 1 before nonce 0 should fail
        vm.expectRevert(RampOSPaymaster.InvalidNonce.selector);
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 1, 0.1 ether);
    }

    function test_PaymasterNoncesPerSenderIsolation() public {
        bytes32 tenantId = keccak256("tenant1");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        address sender1 = makeAddr("sender1");
        address sender2 = makeAddr("sender2");

        // Sender1 nonce 0
        _validateWithNonce(sender1, tenantId, validUntil, validAfter, 0, 0.1 ether);

        // Sender2 nonce 0 should also work (independent nonce tracking)
        _validateWithNonce(sender2, tenantId, validUntil, validAfter, 0, 0.1 ether);

        assertEq(paymaster.senderNonces(sender1), 1);
        assertEq(paymaster.senderNonces(sender2), 1);
    }

    function test_PaymasterNonceConcurrentReplay() public {
        bytes32 tenantId = keccak256("tenant1");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        address sender = makeAddr("sender");

        // First call with nonce 0 succeeds
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 0, 0.1 ether);
        assertEq(paymaster.senderNonces(sender), 1);

        // Second call with same nonce 0 MUST revert (replay attack)
        vm.expectRevert(RampOSPaymaster.InvalidNonce.selector);
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 0, 0.1 ether);

        // Nonce should still be 1 (unchanged by failed tx)
        assertEq(paymaster.senderNonces(sender), 1);
    }

    function test_PaymasterNonceProgressesSequentially() public {
        bytes32 tenantId = keccak256("tenant1");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        address sender = makeAddr("sender");

        // Nonce 0 works
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 0, 0.1 ether);
        assertEq(paymaster.senderNonces(sender), 1);

        // Nonce 1 works
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 1, 0.1 ether);
        assertEq(paymaster.senderNonces(sender), 2);

        // Nonce 2 works
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 2, 0.1 ether);
        assertEq(paymaster.senderNonces(sender), 3);

        // Skipping nonce 3 and trying nonce 4 should fail
        vm.expectRevert(RampOSPaymaster.InvalidNonce.selector);
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 4, 0.1 ether);

        // Nonce should still be 3
        assertEq(paymaster.senderNonces(sender), 3);

        // Nonce 3 should work (sequential continues)
        _validateWithNonce(sender, tenantId, validUntil, validAfter, 3, 0.1 ether);
        assertEq(paymaster.senderNonces(sender), 4);
    }

    /// @dev Helper to construct and validate a paymaster op with a nonce
    function _validateWithNonce(
        address sender,
        bytes32 tenantId,
        uint48 validUntil,
        uint48 validAfter,
        uint256 nonce,
        uint256 maxCost
    ) internal {
        PackedUserOperation memory userOp;
        userOp.sender = sender;
        userOp.nonce = 0;

        bytes32 userOpHash = keccak256(abi.encodePacked("userOp", nonce, sender));

        // Construct signature including nonce
        bytes32 hash = keccak256(
            abi.encodePacked(
                userOpHash, tenantId, validUntil, validAfter,
                nonce,
                block.chainid, address(paymaster)
            )
        ).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        // paymasterAndData: address(20) + tenantId(32) + validUntil(6) + validAfter(6) + nonce(32) + signature(65)
        bytes memory paymasterAndData = abi.encodePacked(
            address(paymaster), tenantId, validUntil, validAfter, nonce, signature
        );
        userOp.paymasterAndData = paymasterAndData;

        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash, maxCost);
    }
}
