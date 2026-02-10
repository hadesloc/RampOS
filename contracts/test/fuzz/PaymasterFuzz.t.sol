// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { RampOSPaymaster } from "../../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import { IPaymaster } from "@account-abstraction/contracts/interfaces/IPaymaster.sol";
import { IStakeManager } from "@account-abstraction/contracts/interfaces/IStakeManager.sol";
import {
    PackedUserOperation
} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";
import { MessageHashUtils } from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title PaymasterFuzz
 * @notice Comprehensive fuzz tests for RampOSPaymaster
 * @dev Tests signature validation, rate limiting, and timelock functionality
 */
contract PaymasterFuzz is Test {
    using MessageHashUtils for bytes32;

    RampOSPaymaster paymaster;
    IEntryPoint entryPoint;
    address signer;
    uint256 signerKey;
    address owner;
    address payable recipient;

    // Constants
    uint256 constant MAX_COST = 100 ether;
    uint256 constant MAX_LIMIT = 1000 ether;
    uint256 constant WITHDRAW_DELAY = 24 hours;

    function setUp() public {
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));
        (signer, signerKey) = makeAddrAndKey("signer");
        owner = makeAddr("owner");
        recipient = payable(makeAddr("recipient"));

        vm.prank(owner);
        paymaster = new RampOSPaymaster(entryPoint, signer);
    }

    // ============ Helper Functions ============

    function _createUserOp(address sender) internal pure returns (PackedUserOperation memory) {
        PackedUserOperation memory userOp;
        userOp.sender = sender;
        userOp.nonce = 0;
        return userOp;
    }

    /// @notice Track nonces per sender for creating paymaster data in tests
    mapping(address => uint256) internal _testNonces;

    function _createPaymasterData(
        bytes32 userOpHash,
        bytes32 tenantId,
        uint48 validUntil,
        uint48 validAfter
    ) internal returns (bytes memory) {
        return _createPaymasterDataForSender(userOpHash, tenantId, validUntil, validAfter, makeAddr("sender"));
    }

    function _createPaymasterDataForSender(
        bytes32 userOpHash,
        bytes32 tenantId,
        uint48 validUntil,
        uint48 validAfter,
        address sender
    ) internal returns (bytes memory) {
        uint256 nonce = _testNonces[sender];
        _testNonces[sender]++;

        bytes32 hash = keccak256(
            abi.encodePacked(
                userOpHash,
                tenantId,
                validUntil,
                validAfter,
                nonce,
                block.chainid,
                address(paymaster)
            )
        ).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        return abi.encodePacked(address(paymaster), tenantId, validUntil, validAfter, nonce, signature);
    }

    // ============ Tenant Limit Fuzz Tests ============

    /**
     * @notice Fuzz test: Set tenant limit with random values
     * @dev Tests tenant limit storage and retrieval
     */
    function testFuzz_SetTenantLimit(bytes32 tenantId, uint256 limit) public {
        vm.assume(limit <= MAX_LIMIT);

        vm.prank(owner);
        paymaster.setTenantLimit(tenantId, limit);

        assertEq(paymaster.tenantDailyLimit(tenantId), limit, "Limit should be stored");
    }

    /**
     * @notice Fuzz test: Tenant limit enforcement
     * @dev Tests that spending respects limits
     */
    function testFuzz_TenantLimitEnforcement(bytes32 tenantId, uint256 limit, uint256 cost1, uint256 cost2) public {
        // Bound values to reasonable ranges
        limit = bound(limit, 1 ether, MAX_LIMIT);
        cost1 = bound(cost1, 0.1 ether, limit - 0.01 ether);
        cost2 = bound(cost2, limit - cost1 + 0.01 ether, limit + 1 ether);
        // Ensure cost1 + cost2 > limit
        vm.assume(cost1 + cost2 > limit);

        vm.prank(owner);
        paymaster.setTenantLimit(tenantId, limit);

        // First operation
        address sender1 = makeAddr("sender1");
        PackedUserOperation memory userOp = _createUserOp(sender1);
        bytes32 userOpHash1 = keccak256(abi.encode("userOp1", block.timestamp));
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        userOp.paymasterAndData = _createPaymasterDataForSender(userOpHash1, tenantId, validUntil, validAfter, sender1);

        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash1, cost1);

        // Second operation should fail if it exceeds limit
        bytes32 userOpHash2 = keccak256(abi.encode("userOp2", block.timestamp));
        userOp.paymasterAndData = _createPaymasterDataForSender(userOpHash2, tenantId, validUntil, validAfter, sender1);

        vm.prank(address(entryPoint));
        vm.expectRevert(RampOSPaymaster.TenantLimitExceeded.selector);
        paymaster.validatePaymasterUserOp(userOp, userOpHash2, cost2);
    }

    /**
     * @notice Fuzz test: Tenant limit resets daily
     * @dev Tests daily reset mechanism
     */
    function testFuzz_TenantLimitDailyReset(bytes32 tenantId, uint256 limit, uint256 cost) public {
        vm.assume(limit > 0 && limit <= MAX_LIMIT);
        vm.assume(cost > 0 && cost <= limit);

        vm.prank(owner);
        paymaster.setTenantLimit(tenantId, limit);

        // First operation
        PackedUserOperation memory userOp = _createUserOp(makeAddr("sender"));
        bytes32 userOpHash1 = keccak256(abi.encode("userOp1", block.timestamp));
        uint48 validUntil = uint48(block.timestamp + 2 days);
        uint48 validAfter = uint48(block.timestamp);

        userOp.paymasterAndData = _createPaymasterData(userOpHash1, tenantId, validUntil, validAfter);

        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash1, cost);

        // Warp to next day
        vm.warp(block.timestamp + 1 days);

        // Should be able to spend again after reset
        bytes32 userOpHash2 = keccak256(abi.encode("userOp2", block.timestamp));
        userOp.paymasterAndData = _createPaymasterData(userOpHash2, tenantId, validUntil, validAfter);

        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash2, cost);

        // Verify spent is reset to just this cost
        assertEq(paymaster.tenantDailySpent(tenantId), cost, "Should only reflect today's spending");
    }

    /**
     * @notice Fuzz test: Unlimited tenant (limit = 0)
     * @dev Tests that zero limit means unlimited
     */
    function testFuzz_TenantUnlimited(bytes32 tenantId, uint256 cost) public {
        vm.assume(cost > 0 && cost <= MAX_LIMIT);

        // Don't set a limit (defaults to 0 = unlimited)
        assertEq(paymaster.tenantDailyLimit(tenantId), 0);

        PackedUserOperation memory userOp = _createUserOp(makeAddr("sender"));
        bytes32 userOpHash = keccak256(abi.encode("userOp", block.timestamp));
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        userOp.paymasterAndData = _createPaymasterData(userOpHash, tenantId, validUntil, validAfter);

        // Should succeed even with high cost
        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash, cost);
    }

    // ============ User Rate Limit Fuzz Tests ============

    /**
     * @notice Fuzz test: Set max ops per user
     * @dev Tests user rate limit configuration
     */
    function testFuzz_SetMaxOpsPerUser(uint256 maxOps) public {
        vm.assume(maxOps > 0 && maxOps <= 10000);

        vm.prank(owner);
        paymaster.setMaxOpsPerUser(maxOps);

        assertEq(paymaster.maxOpsPerUserPerDay(), maxOps, "Max ops should be stored");
    }

    /**
     * @notice Fuzz test: User rate limit enforcement
     * @dev Tests that users can't exceed daily op limit
     */
    function testFuzz_UserRateLimitEnforcement(uint8 maxOps) public {
        vm.assume(maxOps > 0 && maxOps <= 20);

        vm.prank(owner);
        paymaster.setMaxOpsPerUser(maxOps);

        address sender = makeAddr("sender");
        bytes32 tenantId = keccak256("tenant");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        // Use all allowed ops
        for (uint256 i = 0; i < maxOps; i++) {
            PackedUserOperation memory userOp = _createUserOp(sender);
            bytes32 userOpHash = keccak256(abi.encode("userOp", i, block.timestamp));
            userOp.paymasterAndData = _createPaymasterData(userOpHash, tenantId, validUntil, validAfter);

            vm.prank(address(entryPoint));
            paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
        }

        // Next op should fail
        PackedUserOperation memory userOp = _createUserOp(sender);
        bytes32 userOpHash = keccak256(abi.encode("userOp", maxOps, block.timestamp));
        userOp.paymasterAndData = _createPaymasterData(userOpHash, tenantId, validUntil, validAfter);

        vm.prank(address(entryPoint));
        vm.expectRevert(RampOSPaymaster.UserRateLimitExceeded.selector);
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
    }

    /**
     * @notice Fuzz test: User rate limit resets daily
     * @dev Tests user rate limit daily reset
     */
    function testFuzz_UserRateLimitDailyReset(uint8 maxOps) public {
        vm.assume(maxOps > 0 && maxOps <= 10);

        vm.prank(owner);
        paymaster.setMaxOpsPerUser(maxOps);

        address sender = makeAddr("sender");
        bytes32 tenantId = keccak256("tenant");
        uint48 validUntil = uint48(block.timestamp + 2 days);
        uint48 validAfter = uint48(block.timestamp);

        // Use all allowed ops
        for (uint256 i = 0; i < maxOps; i++) {
            PackedUserOperation memory userOp = _createUserOp(sender);
            bytes32 userOpHash = keccak256(abi.encode("userOp", i, block.timestamp));
            userOp.paymasterAndData = _createPaymasterData(userOpHash, tenantId, validUntil, validAfter);

            vm.prank(address(entryPoint));
            paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
        }

        // Warp to next day
        vm.warp(block.timestamp + 1 days);

        // Should be able to use ops again
        for (uint256 i = 0; i < maxOps; i++) {
            PackedUserOperation memory userOp = _createUserOp(sender);
            bytes32 userOpHash = keccak256(abi.encode("userOp_day2", i, block.timestamp));
            userOp.paymasterAndData = _createPaymasterData(userOpHash, tenantId, validUntil, validAfter);

            vm.prank(address(entryPoint));
            paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
        }
    }

    // ============ Signature Validation Fuzz Tests ============

    /**
     * @notice Fuzz test: Valid time bounds
     * @dev Tests signature validation respects time bounds
     */
    function testFuzz_ValidTimeBounds(uint48 validAfter, uint48 duration) public {
        vm.assume(duration > 0 && duration <= 365 days);
        vm.assume(validAfter >= block.timestamp && validAfter < type(uint48).max - duration);

        uint48 validUntil = validAfter + duration;

        PackedUserOperation memory userOp = _createUserOp(makeAddr("sender"));
        bytes32 userOpHash = keccak256(abi.encode("userOp", block.timestamp));
        bytes32 tenantId = keccak256("tenant");

        userOp.paymasterAndData = _createPaymasterData(userOpHash, tenantId, validUntil, validAfter);

        vm.prank(address(entryPoint));
        (bytes memory context, uint256 validationData) = paymaster.validatePaymasterUserOp(
            userOp,
            userOpHash,
            0.01 ether
        );

        // Check sig is valid (bit 0 = 0)
        assertEq(validationData & 1, 0, "Signature should be valid");

        // Verify context
        (address sender,,) = abi.decode(context, (address, bytes32, uint256));
        assertEq(sender, userOp.sender, "Context sender should match");
    }

    /**
     * @notice Fuzz test: Invalid signer should fail
     * @dev Tests wrong signature is rejected
     */
    function testFuzz_InvalidSignerRejected(uint256 wrongSignerKey) public {
        // Bound to valid secp256k1 private key range
        wrongSignerKey = bound(wrongSignerKey, 1, 115792089237316195423570985008687907852837564279074904382605163141518161494336);
        vm.assume(wrongSignerKey != signerKey);

        address sender = makeAddr("sender");
        PackedUserOperation memory userOp = _createUserOp(sender);
        bytes32 userOpHash = keccak256(abi.encode("userOp", block.timestamp));
        bytes32 tenantId = keccak256("tenant");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        uint256 nonce = 0;

        // Sign with wrong key (including nonce)
        bytes32 hash = keccak256(
            abi.encodePacked(
                userOpHash,
                tenantId,
                validUntil,
                validAfter,
                nonce,
                block.chainid,
                address(paymaster)
            )
        ).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(wrongSignerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster),
            tenantId,
            validUntil,
            validAfter,
            nonce,
            signature
        );

        vm.prank(address(entryPoint));
        vm.expectRevert(RampOSPaymaster.InvalidSignature.selector);
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
    }

    /**
     * @notice Fuzz test: Signature replay prevention
     * @dev Tests that same signature can't be used twice
     */
    function testFuzz_SignatureReplayPrevention(bytes32 tenantId) public {
        address sender = makeAddr("sender");
        PackedUserOperation memory userOp = _createUserOp(sender);
        bytes32 userOpHash = keccak256(abi.encode("userOp", tenantId, block.timestamp));
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        // Create paymaster data with nonce 0
        uint256 nonce = 0;
        bytes32 hash = keccak256(
            abi.encodePacked(
                userOpHash, tenantId, validUntil, validAfter,
                nonce, block.chainid, address(paymaster)
            )
        ).toEthSignedMessageHash();
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);
        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster), tenantId, validUntil, validAfter, nonce, signature
        );

        // First use should succeed
        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);

        // Second use with same nonce 0 should fail (nonce already consumed)
        vm.prank(address(entryPoint));
        vm.expectRevert(RampOSPaymaster.InvalidNonce.selector);
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
    }

    // ============ Timelock Fuzz Tests ============

    /**
     * @notice Fuzz test: Request withdraw with random amounts
     * @dev Tests withdraw request with various amounts
     */
    function testFuzz_RequestWithdraw(uint256 amount) public {
        vm.assume(amount > 0 && amount <= MAX_LIMIT);

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, amount);

        (address to, uint256 pendingAmount, uint256 requestTime, uint256 executeAfter) =
            paymaster.getPendingWithdraw();

        assertEq(to, recipient, "Recipient should match");
        assertEq(pendingAmount, amount, "Amount should match");
        assertEq(requestTime, block.timestamp, "Request time should be now");
        assertEq(executeAfter, block.timestamp + WITHDRAW_DELAY, "Execute after should be correct");
    }

    /**
     * @notice Fuzz test: Withdraw time remaining calculation
     * @dev Tests time remaining with random elapsed times
     */
    function testFuzz_WithdrawTimeRemaining(uint256 elapsedTime) public {
        vm.assume(elapsedTime <= WITHDRAW_DELAY + 7 days);

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        vm.warp(block.timestamp + elapsedTime);

        uint256 remaining = paymaster.getWithdrawTimeRemaining();

        if (elapsedTime >= WITHDRAW_DELAY) {
            assertEq(remaining, 0, "Should be 0 when ready");
        } else {
            assertEq(remaining, WITHDRAW_DELAY - elapsedTime, "Should reflect remaining time");
        }
    }

    /**
     * @notice Fuzz test: Withdraw ready window
     * @dev Tests isWithdrawReady at various times
     */
    function testFuzz_WithdrawReadyWindow(uint256 elapsedTime) public {
        vm.assume(elapsedTime <= WITHDRAW_DELAY + 8 days);

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        vm.warp(block.timestamp + elapsedTime);

        bool ready = paymaster.isWithdrawReady();

        if (elapsedTime < WITHDRAW_DELAY) {
            assertFalse(ready, "Should not be ready before delay");
        } else if (elapsedTime <= WITHDRAW_DELAY + 7 days) {
            assertTrue(ready, "Should be ready within window");
        } else {
            assertFalse(ready, "Should not be ready after expiry");
        }
    }

    /**
     * @notice Fuzz test: Execute withdraw at various valid times
     * @dev Tests withdraw execution within valid window
     */
    function testFuzz_ExecuteWithdrawValidWindow(uint256 offsetWithinWindow) public {
        vm.assume(offsetWithinWindow < 7 days);

        uint256 amount = 1 ether;
        uint256 executeTime = WITHDRAW_DELAY + offsetWithinWindow;

        // Mock the entryPoint withdrawal
        vm.mockCall(
            address(entryPoint),
            abi.encodeWithSelector(IStakeManager.withdrawTo.selector, recipient, amount),
            abi.encode()
        );

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, amount);

        vm.warp(block.timestamp + executeTime);

        vm.prank(owner);
        paymaster.executeWithdraw();

        // Verify state cleared
        (address to, uint256 pendingAmount,,) = paymaster.getPendingWithdraw();
        assertEq(to, address(0), "Recipient should be cleared");
        assertEq(pendingAmount, 0, "Amount should be cleared");
    }

    /**
     * @notice Fuzz test: Execute withdraw before ready should fail
     * @dev Tests early execution rejection
     */
    function testFuzz_ExecuteWithdrawTooEarly(uint256 offsetBeforeReady) public {
        vm.assume(offsetBeforeReady < WITHDRAW_DELAY);

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        vm.warp(block.timestamp + offsetBeforeReady);

        vm.prank(owner);
        vm.expectRevert(RampOSPaymaster.WithdrawNotReady.selector);
        paymaster.executeWithdraw();
    }

    /**
     * @notice Fuzz test: Execute withdraw after expiry should fail
     * @dev Tests expired execution rejection
     */
    function testFuzz_ExecuteWithdrawExpired(uint256 offsetAfterExpiry) public {
        vm.assume(offsetAfterExpiry > 0 && offsetAfterExpiry < 365 days);

        vm.prank(owner);
        paymaster.requestWithdraw(recipient, 1 ether);

        vm.warp(block.timestamp + WITHDRAW_DELAY + 7 days + offsetAfterExpiry);

        vm.prank(owner);
        vm.expectRevert(RampOSPaymaster.WithdrawExpired.selector);
        paymaster.executeWithdraw();
    }

    /**
     * @notice Fuzz test: Cancel and new request
     * @dev Tests cancel then new request flow
     */
    function testFuzz_CancelAndNewRequest(uint256 amount1, uint256 amount2) public {
        vm.assume(amount1 > 0 && amount1 <= MAX_LIMIT);
        vm.assume(amount2 > 0 && amount2 <= MAX_LIMIT);

        vm.startPrank(owner);

        paymaster.requestWithdraw(recipient, amount1);
        paymaster.cancelWithdraw();

        // Should be able to make new request
        address payable newRecipient = payable(makeAddr("newRecipient"));
        paymaster.requestWithdraw(newRecipient, amount2);

        (address to, uint256 amount,,) = paymaster.getPendingWithdraw();
        assertEq(to, newRecipient, "New recipient should be set");
        assertEq(amount, amount2, "New amount should be set");

        vm.stopPrank();
    }

    // ============ Admin Function Fuzz Tests ============

    /**
     * @notice Fuzz test: Update signer
     * @dev Tests signer update with random addresses
     */
    function testFuzz_UpdateSigner(address newSigner) public {
        vm.assume(newSigner != address(0));

        vm.prank(owner);
        paymaster.setSigner(newSigner);

        assertEq(paymaster.verifyingSigner(), newSigner, "Signer should be updated");
    }

    /**
     * @notice Fuzz test: Update signer to zero address should fail
     * @dev Tests zero address validation
     */
    function testFuzz_UpdateSignerZeroAddressFails() public {
        vm.prank(owner);
        vm.expectRevert("Invalid signer");
        paymaster.setSigner(address(0));
    }

    /**
     * @notice Fuzz test: Non-owner cannot update signer
     * @dev Tests access control on setSigner
     */
    function testFuzz_NonOwnerCannotUpdateSigner(address attacker, address newSigner) public {
        vm.assume(attacker != owner);
        vm.assume(newSigner != address(0));

        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSignature("OwnableUnauthorizedAccount(address)", attacker));
        paymaster.setSigner(newSigner);
    }

    /**
     * @notice Fuzz test: Non-owner cannot set tenant limit
     * @dev Tests access control on setTenantLimit
     */
    function testFuzz_NonOwnerCannotSetTenantLimit(address attacker, bytes32 tenantId, uint256 limit) public {
        vm.assume(attacker != owner);

        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSignature("OwnableUnauthorizedAccount(address)", attacker));
        paymaster.setTenantLimit(tenantId, limit);
    }

    /**
     * @notice Fuzz test: Non-owner cannot request withdraw
     * @dev Tests access control on requestWithdraw
     */
    function testFuzz_NonOwnerCannotRequestWithdraw(address attacker, uint256 amount) public {
        vm.assume(attacker != owner);
        vm.assume(amount > 0);

        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSignature("OwnableUnauthorizedAccount(address)", attacker));
        paymaster.requestWithdraw(recipient, amount);
    }

    // ============ PostOp Fuzz Tests ============

    /**
     * @notice Fuzz test: PostOp refund calculation
     * @dev Tests refund when actualGasCost < maxCost
     */
    function testFuzz_PostOpRefund(uint256 maxCost, uint256 actualGasCost) public {
        vm.assume(maxCost > 0 && maxCost <= MAX_LIMIT);
        vm.assume(actualGasCost <= maxCost);

        bytes32 tenantId = keccak256("tenant");
        address sender = makeAddr("sender");

        // Setup: perform a validate first to set daily spent
        PackedUserOperation memory userOp = _createUserOp(sender);
        bytes32 userOpHash = keccak256(abi.encode("userOp", block.timestamp));
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        userOp.paymasterAndData = _createPaymasterData(userOpHash, tenantId, validUntil, validAfter);

        vm.prank(address(entryPoint));
        (bytes memory context,) = paymaster.validatePaymasterUserOp(userOp, userOpHash, maxCost);

        // Initial spent should be maxCost
        assertEq(paymaster.tenantDailySpent(tenantId), maxCost, "Initial spent should be maxCost");

        // Call postOp
        vm.prank(address(entryPoint));
        paymaster.postOp(IPaymaster.PostOpMode.opSucceeded, context, actualGasCost, 0);

        // Verify refund
        uint256 expectedSpent = actualGasCost;
        assertEq(paymaster.tenantDailySpent(tenantId), expectedSpent, "Spent should equal actual cost after refund");
    }

    /**
     * @notice Fuzz test: Only entry point can call validatePaymasterUserOp
     * @dev Tests access control
     */
    function testFuzz_OnlyEntryPointCanValidate(address caller) public {
        vm.assume(caller != address(entryPoint));

        PackedUserOperation memory userOp = _createUserOp(makeAddr("sender"));
        bytes32 userOpHash = keccak256("userOp");

        vm.prank(caller);
        vm.expectRevert("Only entry point");
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
    }

    /**
     * @notice Fuzz test: Only entry point can call postOp
     * @dev Tests access control
     */
    function testFuzz_OnlyEntryPointCanPostOp(address caller) public {
        vm.assume(caller != address(entryPoint));

        bytes memory context = abi.encode(makeAddr("sender"), keccak256("tenant"), uint256(0.01 ether));

        vm.prank(caller);
        vm.expectRevert("Only entry point");
        paymaster.postOp(IPaymaster.PostOpMode.opSucceeded, context, 0.005 ether, 0);
    }
}
