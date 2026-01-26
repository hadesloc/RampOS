// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import "@account-abstraction/contracts/core/BaseAccount.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts/proxy/utils/UUPSUpgradeable.sol";

/**
 * @title RampOSAccount
 * @notice ERC-4337 compatible smart account for RampOS
 * @dev Supports:
 *  - Single owner ECDSA signatures
 *  - Batch execution
 *  - Session keys
 *  - Gasless transactions via paymaster
 */
contract RampOSAccount is BaseAccount, Initializable, UUPSUpgradeable {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    /// @notice Account owner
    address public owner;

    /// @notice Entry point contract
    IEntryPoint private immutable _entryPoint;

    /// @notice Session key data
    struct SessionKey {
        address key;
        uint48 validAfter;
        uint48 validUntil;
        bytes32 permissionsHash;
    }

    /// @notice Active session keys
    mapping(address => SessionKey) public sessionKeys;

    /// @notice Events
    event AccountInitialized(address indexed owner);
    event SessionKeyAdded(address indexed key, uint48 validUntil);
    event SessionKeyRemoved(address indexed key);

    /// @notice Errors
    error NotOwner();
    error NotOwnerOrEntryPoint();
    error InvalidSessionKey();
    error SessionKeyExpired();

    /// @notice Modifier for owner-only functions
    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    /// @notice Modifier for owner or entry point
    modifier onlyOwnerOrEntryPoint() {
        if (msg.sender != owner && msg.sender != address(_entryPoint)) {
            revert NotOwnerOrEntryPoint();
        }
        _;
    }

    constructor(IEntryPoint anEntryPoint) {
        _entryPoint = anEntryPoint;
        _disableInitializers();
    }

    /// @notice Initialize the account with an owner
    function initialize(address anOwner) public virtual initializer {
        owner = anOwner;
        emit AccountInitialized(anOwner);
    }

    /// @notice Get the entry point
    function entryPoint() public view virtual override returns (IEntryPoint) {
        return _entryPoint;
    }

    /// @notice Execute a single transaction
    function execute(
        address dest,
        uint256 value,
        bytes calldata data
    ) external onlyOwnerOrEntryPoint {
        _call(dest, value, data);
    }

    /// @notice Execute a batch of transactions
    function executeBatch(
        address[] calldata dests,
        uint256[] calldata values,
        bytes[] calldata datas
    ) external onlyOwnerOrEntryPoint {
        require(
            dests.length == values.length && values.length == datas.length,
            "Array length mismatch"
        );

        for (uint256 i = 0; i < dests.length; i++) {
            _call(dests[i], values[i], datas[i]);
        }
    }

    /// @notice Add a session key
    function addSessionKey(
        address key,
        uint48 validAfter,
        uint48 validUntil,
        bytes32 permissionsHash
    ) external onlyOwner {
        sessionKeys[key] = SessionKey({
            key: key,
            validAfter: validAfter,
            validUntil: validUntil,
            permissionsHash: permissionsHash
        });

        emit SessionKeyAdded(key, validUntil);
    }

    /// @notice Remove a session key
    function removeSessionKey(address key) external onlyOwner {
        delete sessionKeys[key];
        emit SessionKeyRemoved(key);
    }

    /// @notice Check if a session key is valid
    function isValidSessionKey(address key) public view returns (bool) {
        SessionKey memory session = sessionKeys[key];
        if (session.key == address(0)) return false;
        if (block.timestamp < session.validAfter) return false;
        if (block.timestamp > session.validUntil) return false;
        return true;
    }

    /// @notice Validate user operation signature
    function _validateSignature(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash
    ) internal virtual override returns (uint256 validationData) {
        bytes32 hash = userOpHash.toEthSignedMessageHash();
        address signer = hash.recover(userOp.signature);

        // Check if signer is owner
        if (signer == owner) {
            return 0; // Valid
        }

        // Check if signer is a valid session key
        SessionKey memory session = sessionKeys[signer];
        if (session.key != address(0)) {
            // NOTE: permissionsHash is currently unused/reserved for future scope-based permissions.
            // Current session keys have full account access within the time validity window.

            if (block.timestamp < session.validAfter) {
                return SIG_VALIDATION_FAILED;
            }
            if (block.timestamp > session.validUntil) {
                return SIG_VALIDATION_FAILED;
            }
            // Return validation data with time range
            return _packValidationData(false, session.validUntil, session.validAfter);
        }

        return SIG_VALIDATION_FAILED;
    }

    /// @notice Internal call function
    function _call(address target, uint256 value, bytes memory data) internal {
        (bool success, bytes memory result) = target.call{value: value}(data);
        if (!success) {
            assembly {
                revert(add(result, 32), mload(result))
            }
        }
    }

    /// @notice Authorize upgrade (only owner)
    function _authorizeUpgrade(
        address newImplementation
    ) internal override onlyOwner {}

    /// @notice Receive ETH
    receive() external payable {}
}
