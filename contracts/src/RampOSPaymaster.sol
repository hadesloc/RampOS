// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@account-abstraction/contracts/interfaces/IPaymaster.sol";
import "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title RampOSPaymaster
 * @notice Verifying paymaster for RampOS sponsored transactions
 * @dev Supports:
 *  - Signature-based sponsorship verification
 *  - Per-tenant spending limits
 *  - Rate limiting
 */
contract RampOSPaymaster is IPaymaster, Ownable {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    /// @notice Entry point
    IEntryPoint public immutable entryPoint;

    /// @notice Signer for paymaster data
    address public verifyingSigner;

    /// @notice Tenant spending limits
    mapping(bytes32 => uint256) public tenantDailySpent;
    mapping(bytes32 => uint256) public tenantDailyLimit;
    mapping(bytes32 => uint256) public tenantLastResetDay;

    /// @notice User rate limits
    mapping(address => uint256) public userDailyOps;
    mapping(address => uint256) public userLastResetDay;
    uint256 public maxOpsPerUserPerDay = 100;

    /// @notice Events
    event SignerUpdated(address indexed oldSigner, address indexed newSigner);
    event TenantLimitSet(bytes32 indexed tenantId, uint256 limit);
    event Sponsored(address indexed sender, bytes32 indexed tenantId, uint256 gasCost);

    /// @notice Errors
    error InvalidSignature();
    error TenantLimitExceeded();
    error UserRateLimitExceeded();
    error PaymasterDepositTooLow();

    constructor(IEntryPoint _entryPoint, address _signer) Ownable(msg.sender) {
        entryPoint = _entryPoint;
        verifyingSigner = _signer;
    }

    /// @notice Validate a user operation for sponsorship
    function validatePaymasterUserOp(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    ) external override returns (bytes memory context, uint256 validationData) {
        require(msg.sender == address(entryPoint), "Only entry point");

        // Decode paymaster data: tenantId (32) + validUntil (6) + validAfter (6) + signature (65)
        bytes calldata paymasterData = userOp.paymasterAndData[20:];
        require(paymasterData.length >= 109, "Invalid paymaster data length");

        bytes32 tenantId = bytes32(paymasterData[0:32]);
        uint48 validUntil = uint48(bytes6(paymasterData[32:38]));
        uint48 validAfter = uint48(bytes6(paymasterData[38:44]));
        bytes calldata signature = paymasterData[44:109];

        // Verify signature
        bytes32 hash = keccak256(
            abi.encodePacked(
                userOpHash,
                tenantId,
                validUntil,
                validAfter
            )
        ).toEthSignedMessageHash();

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

    /// @notice Post-operation handler
    function postOp(
        PostOpMode mode,
        bytes calldata context,
        uint256 actualGasCost,
        uint256 actualUserOpFeePerGas
    ) external override {
        require(msg.sender == address(entryPoint), "Only entry point");

        (address sender, bytes32 tenantId, uint256 maxCost) = abi.decode(
            context,
            (address, bytes32, uint256)
        );

        if (mode == PostOpMode.postOpReverted) {
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
    function _packValidationData(
        bool sigFailed,
        uint48 validUntil,
        uint48 validAfter
    ) internal pure returns (uint256) {
        return
            (sigFailed ? 1 : 0) |
            (uint256(validUntil) << 160) |
            (uint256(validAfter) << 208);
    }

    // Admin functions

    function setSigner(address _signer) external onlyOwner {
        emit SignerUpdated(verifyingSigner, _signer);
        verifyingSigner = _signer;
    }

    function setTenantLimit(bytes32 tenantId, uint256 limit) external onlyOwner {
        tenantDailyLimit[tenantId] = limit;
        emit TenantLimitSet(tenantId, limit);
    }

    function setMaxOpsPerUser(uint256 maxOps) external onlyOwner {
        maxOpsPerUserPerDay = maxOps;
    }

    function deposit() external payable {
        entryPoint.depositTo{value: msg.value}(address(this));
    }

    function withdrawTo(address payable to, uint256 amount) external onlyOwner {
        entryPoint.withdrawTo(to, amount);
    }

    function getDeposit() external view returns (uint256) {
        return entryPoint.balanceOf(address(this));
    }
}
