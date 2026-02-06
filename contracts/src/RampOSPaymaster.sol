// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { IPaymaster } from "@account-abstraction/contracts/interfaces/IPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import {
    PackedUserOperation
} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { ECDSA } from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import { MessageHashUtils } from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title RampOSPaymaster
 * @author RampOS Team
 * @notice Verifying paymaster for RampOS sponsored transactions
 * @dev Implements ERC-4337 paymaster interface with signature-based sponsorship.
 *
 * Features:
 *  - Signature-based sponsorship verification using ECDSA
 *  - Per-tenant daily spending limits
 *  - Per-user daily rate limiting
 *  - Timelocked withdrawals for security (24h delay)
 *
 * Security considerations:
 *  - Only the verifying signer can authorize sponsorships
 *  - Withdrawals require 24h timelock to prevent instant drains
 *  - Rate limits prevent abuse
 */
contract RampOSPaymaster is IPaymaster, Ownable {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    /// @notice ERC-4337 Entry Point contract reference
    IEntryPoint public immutable ENTRY_POINT;

    /// @notice Authorized signer for validating paymaster sponsorship data
    address public verifyingSigner;

    /// @notice Tenant spending limits
    mapping(bytes32 => uint256) public tenantDailySpent;
    mapping(bytes32 => uint256) public tenantDailyLimit;
    mapping(bytes32 => uint256) public tenantLastResetDay;

    /// @notice User rate limits
    mapping(address => uint256) public userDailyOps;
    mapping(address => uint256) public userLastResetDay;
    uint256 public maxOpsPerUserPerDay = 100;

    /// @notice Used signatures to prevent replay attacks
    mapping(bytes32 => bool) public usedSignatures;

    /// @notice Timelock configuration
    uint256 public constant WITHDRAW_DELAY = 24 hours;

    /// @notice Pending withdrawal state
    uint256 public pendingWithdrawAmount;
    uint256 public withdrawRequestTime;
    address public pendingWithdrawTo;

    /// @notice Events
    event SignerUpdated(address indexed oldSigner, address indexed newSigner);
    event TenantLimitSet(bytes32 indexed tenantId, uint256 limit);
    event Sponsored(address indexed sender, bytes32 indexed tenantId, uint256 gasCost);
    event WithdrawRequested(address indexed to, uint256 amount, uint256 executeAfter);
    event WithdrawExecuted(address indexed to, uint256 amount);
    event WithdrawCancelled(address indexed to, uint256 amount);

    /// @notice Errors
    error InvalidSignature();
    error TenantLimitExceeded();
    error UserRateLimitExceeded();
    error PaymasterDepositTooLow();
    error WithdrawAlreadyPending();
    error NoWithdrawPending();
    error WithdrawNotReady();
    error WithdrawExpired();

    constructor(IEntryPoint _entryPoint, address _signer) Ownable(msg.sender) {
        require(_signer != address(0), "Invalid signer");
        ENTRY_POINT = _entryPoint;
        verifyingSigner = _signer;
    }

    /// @notice Validate a user operation for sponsorship
    /// @param userOp The packed user operation to validate
    /// @param userOpHash Hash of the user operation
    /// @param maxCost Maximum gas cost to be paid
    /// @return context Encoded context for postOp
    /// @return validationData Packed validation data with time range
    function validatePaymasterUserOp(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    ) external override returns (bytes memory context, uint256 validationData) {
        require(msg.sender == address(ENTRY_POINT), "Only entry point");

        // Decode paymaster data: tenantId (32) + validUntil (6) + validAfter (6) + signature (65)
        bytes calldata paymasterData = userOp.paymasterAndData[20:];
        require(paymasterData.length >= 109, "Invalid paymaster data length");

        bytes32 tenantId = bytes32(paymasterData[0:32]);
        uint48 validUntil = uint48(bytes6(paymasterData[32:38]));
        uint48 validAfter = uint48(bytes6(paymasterData[38:44]));
        bytes calldata signature = paymasterData[44:109];

        // Verify signature (includes chainid and contract address to prevent cross-chain replay)
        bytes32 hash = keccak256(abi.encodePacked(
            userOpHash, tenantId, validUntil, validAfter,
            block.chainid, address(this)
        )).toEthSignedMessageHash();

        // Prevent signature replay attacks
        require(!usedSignatures[hash], "Signature already used");
        usedSignatures[hash] = true;

        if (hash.recover(signature) != verifyingSigner) {
            revert InvalidSignature();
        }

        // Check tenant daily limit
        _checkAndUpdateTenantLimit(tenantId, maxCost);

        // Check user rate limit
        _checkAndUpdateUserRateLimit(userOp.sender);

        // Return context for postOp
        context = abi.encode(userOp.sender, tenantId, maxCost);

        // Return validation data with time range
        validationData = _packValidationData(false, validUntil, validAfter);
    }

    /// @notice Post-operation handler called after user operation execution
    /// @param mode The post-op mode (success, reverted, or postOpReverted)
    /// @param context Encoded context from validatePaymasterUserOp
    /// @param actualGasCost Actual gas cost of the operation
    /// @param - Actual fee per gas (unused, reserved for future use)
    function postOp(
        IPaymaster.PostOpMode mode,
        bytes calldata context,
        uint256 actualGasCost,
        uint256 /* actualUserOpFeePerGas */
    ) external override {
        require(msg.sender == address(ENTRY_POINT), "Only entry point");

        (address sender, bytes32 tenantId, uint256 maxCost) =
            abi.decode(context, (address, bytes32, uint256));

        if (mode == IPaymaster.PostOpMode.postOpReverted) {
            // In reverted mode, the userOp is reverted, but we still pay for gas.
            // We can optionally refund some amount if we want, but usually we just track the cost.
            emit Sponsored(sender, tenantId, actualGasCost);

            // Refund the difference between maxCost and actualGasCost
            if (maxCost > actualGasCost) {
                uint256 refund = maxCost - actualGasCost;
                if (tenantDailySpent[tenantId] >= refund) {
                    tenantDailySpent[tenantId] -= refund;
                } else {
                    tenantDailySpent[tenantId] = 0; // Should not happen ideally
                }
            }
            return;
        }

        emit Sponsored(sender, tenantId, actualGasCost);

        // Refund the difference between maxCost and actualGasCost
        if (maxCost > actualGasCost) {
            uint256 refund = maxCost - actualGasCost;
            // Prevent underflow
            if (tenantDailySpent[tenantId] >= refund) {
                tenantDailySpent[tenantId] -= refund;
            } else {
                tenantDailySpent[tenantId] = 0;
            }
        }
    }

    /// @notice Check and update tenant daily limit
    function _checkAndUpdateTenantLimit(bytes32 tenantId, uint256 cost) internal {
        uint256 today = block.timestamp / 1 days;

        if (tenantLastResetDay[tenantId] < today) {
            tenantDailySpent[tenantId] = 0;
            tenantLastResetDay[tenantId] = today;
        }

        uint256 limit = tenantDailyLimit[tenantId];
        if (limit > 0 && tenantDailySpent[tenantId] + cost > limit) {
            revert TenantLimitExceeded();
        }

        tenantDailySpent[tenantId] += cost;
    }

    /// @notice Check and update user rate limit
    function _checkAndUpdateUserRateLimit(address user) internal {
        uint256 today = block.timestamp / 1 days;

        if (userLastResetDay[user] < today) {
            userDailyOps[user] = 0;
            userLastResetDay[user] = today;
        }

        if (userDailyOps[user] >= maxOpsPerUserPerDay) {
            revert UserRateLimitExceeded();
        }

        userDailyOps[user]++;
    }

    /// @notice Pack validation data
    function _packValidationData(bool sigFailed, uint48 validUntil, uint48 validAfter)
        internal
        pure
        returns (uint256)
    {
        return (sigFailed ? 1 : 0) | (uint256(validUntil) << 160) | (uint256(validAfter) << 208);
    }

    // ============ Admin Functions ============

    /// @notice Update the verifying signer address
    /// @dev Only callable by owner. Emits SignerUpdated event.
    /// @param _signer The new signer address
    function setSigner(address _signer) external onlyOwner {
        require(_signer != address(0), "Invalid signer");
        emit SignerUpdated(verifyingSigner, _signer);
        verifyingSigner = _signer;
    }

    /// @notice Set the daily spending limit for a tenant
    /// @dev Only callable by owner. Set to 0 for unlimited.
    /// @param tenantId The tenant identifier
    /// @param limit The daily limit in wei (0 = unlimited)
    function setTenantLimit(bytes32 tenantId, uint256 limit) external onlyOwner {
        tenantDailyLimit[tenantId] = limit;
        emit TenantLimitSet(tenantId, limit);
    }

    /// @notice Set the maximum operations per user per day
    /// @dev Only callable by owner. Used for rate limiting.
    /// @param maxOps The maximum number of operations per user per day
    function setMaxOpsPerUser(uint256 maxOps) external onlyOwner {
        maxOpsPerUserPerDay = maxOps;
    }

    /// @notice Deposit ETH to EntryPoint for gas sponsorship
    function deposit() external payable {
        ENTRY_POINT.depositTo{ value: msg.value }(address(this));
    }

    /// @notice Request a withdrawal with timelock
    /// @param to The address to withdraw to
    /// @param amount The amount to withdraw
    function requestWithdraw(address payable to, uint256 amount) external onlyOwner {
        if (pendingWithdrawAmount != 0) {
            revert WithdrawAlreadyPending();
        }

        pendingWithdrawAmount = amount;
        pendingWithdrawTo = to;
        withdrawRequestTime = block.timestamp;

        emit WithdrawRequested(to, amount, block.timestamp + WITHDRAW_DELAY);
    }

    /// @notice Execute a pending withdrawal after timelock expires
    function executeWithdraw() external onlyOwner {
        if (pendingWithdrawAmount == 0) {
            revert NoWithdrawPending();
        }

        if (block.timestamp < withdrawRequestTime + WITHDRAW_DELAY) {
            revert WithdrawNotReady();
        }

        // Optional: Add expiry window (e.g., 7 days after ready)
        // This prevents very old requests from being executed
        if (block.timestamp > withdrawRequestTime + WITHDRAW_DELAY + 7 days) {
            revert WithdrawExpired();
        }

        uint256 amount = pendingWithdrawAmount;
        address to = pendingWithdrawTo;

        // Clear pending state before external call (CEI pattern)
        pendingWithdrawAmount = 0;
        pendingWithdrawTo = address(0);
        withdrawRequestTime = 0;

        ENTRY_POINT.withdrawTo(payable(to), amount);

        emit WithdrawExecuted(to, amount);
    }

    /// @notice Cancel a pending withdrawal request
    function cancelWithdraw() external onlyOwner {
        if (pendingWithdrawAmount == 0) {
            revert NoWithdrawPending();
        }

        uint256 amount = pendingWithdrawAmount;
        address to = pendingWithdrawTo;

        pendingWithdrawAmount = 0;
        pendingWithdrawTo = address(0);
        withdrawRequestTime = 0;

        emit WithdrawCancelled(to, amount);
    }

    /// @notice Get the time remaining until withdrawal can be executed
    /// @return timeRemaining Seconds remaining, or 0 if ready/no pending
    function getWithdrawTimeRemaining() external view returns (uint256 timeRemaining) {
        if (pendingWithdrawAmount == 0) {
            return 0;
        }

        uint256 readyTime = withdrawRequestTime + WITHDRAW_DELAY;
        if (block.timestamp >= readyTime) {
            return 0;
        }

        return readyTime - block.timestamp;
    }

    /// @notice Check if a withdrawal is ready to execute
    /// @return ready True if withdrawal can be executed
    function isWithdrawReady() external view returns (bool ready) {
        if (pendingWithdrawAmount == 0) {
            return false;
        }

        uint256 readyTime = withdrawRequestTime + WITHDRAW_DELAY;
        uint256 expiryTime = readyTime + 7 days;

        return block.timestamp >= readyTime && block.timestamp <= expiryTime;
    }

    /// @notice Get pending withdrawal details
    /// @return to Recipient address
    /// @return amount Pending amount
    /// @return requestTime Time of request
    /// @return executeAfter Time when execution becomes possible
    function getPendingWithdraw()
        external
        view
        returns (address to, uint256 amount, uint256 requestTime, uint256 executeAfter)
    {
        return (
            pendingWithdrawTo,
            pendingWithdrawAmount,
            withdrawRequestTime,
            withdrawRequestTime + WITHDRAW_DELAY
        );
    }

    /// @dev Legacy function - deprecated, use requestWithdraw + executeWithdraw
    /// @notice This function is kept for interface compatibility but reverts
    function withdrawTo(address payable, uint256) external view onlyOwner {
        revert("Use requestWithdraw + executeWithdraw");
    }

    /// @notice Get current deposit balance in EntryPoint
    /// @return The balance of this paymaster in the EntryPoint
    function getDeposit() external view returns (uint256) {
        return ENTRY_POINT.balanceOf(address(this));
    }
}
