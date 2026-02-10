// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import { BaseAccount } from "@account-abstraction/contracts/core/BaseAccount.sol";
import {
    PackedUserOperation
} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";
import {
    SIG_VALIDATION_FAILED,
    _packValidationData
} from "@account-abstraction/contracts/core/Helpers.sol";
import { ECDSA } from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import { MessageHashUtils } from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import { Initializable } from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import { IERC1271 } from "@openzeppelin/contracts/interfaces/IERC1271.sol";
import { IERC721Receiver } from "@openzeppelin/contracts/token/ERC721/IERC721Receiver.sol";
import { IERC1155Receiver } from "@openzeppelin/contracts/token/ERC1155/IERC1155Receiver.sol";
import { IERC165 } from "@openzeppelin/contracts/utils/introspection/IERC165.sol";
import { P256Verifier } from "./passkey/PasskeySigner.sol";

/**
 * @title RampOSAccount
 * @author RampOS Team
 * @notice ERC-4337 compatible smart account for RampOS on/off-ramp operations
 * @dev Implements Account Abstraction (ERC-4337) with extended session key support.
 *
 * Features:
 *  - Single owner ECDSA signature validation
 *  - Passkey (P256/secp256r1) signature validation for WebAuthn
 *  - Batch transaction execution for gas efficiency
 *  - Session keys with granular permissions (target, selector, spending limits)
 *  - Gasless transactions via paymaster integration
 *  - ERC-1271 signature validation
 *  - ERC-721 and ERC-1155 token receiving support
 *
 * Security considerations:
 *  - Only owner or EntryPoint can execute transactions
 *  - Session keys have time-bounded validity
 *  - Per-transaction and daily spending limits for session keys
 *  - Target and function selector restrictions for session keys
 *  - Passkey signatures are verified using P256 curve verification
 */
contract RampOSAccount is BaseAccount, Initializable, IERC1271, IERC721Receiver, IERC1155Receiver {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    /// @notice Account owner address
    address public owner;

    /// @notice ERC-4337 Entry Point contract reference (immutable for gas savings)
    IEntryPoint private immutable _ENTRY_POINT;

    /// @notice Session key permissions structure
    struct SessionKeyPermissions {
        address[] allowedTargets; // Contracts session key can call
        bytes4[] allowedSelectors; // Function selectors session key can call
        uint256 spendingLimit; // Max ETH per transaction (0 = unlimited)
        uint256 dailyLimit; // Max ETH per day (0 = unlimited)
    }

    /// @notice Session key data
    struct SessionKey {
        address key;
        uint48 validAfter;
        uint48 validUntil;
        bytes32 permissionsHash;
    }

    /// @notice Storage for session key permissions
    struct SessionKeyStorage {
        SessionKey metadata;
        address[] allowedTargets;
        bytes4[] allowedSelectors;
        mapping(address => bool) allowedTargetMap;
        mapping(bytes4 => bool) allowedSelectorMap;
        uint256 spendingLimit;
        uint256 dailyLimit;
        uint256 dailySpent;
        uint256 lastResetDay;
    }

    /// @notice Active session keys (legacy mapping for compatibility)
    mapping(address => SessionKey) public sessionKeys;

    /// @notice Full session key storage with permissions
    mapping(address => SessionKeyStorage) internal _sessionKeyStorage;

    /// @notice Track pending session key for validation
    address internal _pendingSessionKey;

    /// @notice Events
    event AccountInitialized(address indexed owner);
    event SessionKeyAdded(address indexed key, uint48 validUntil);
    event SessionKeyRemoved(address indexed key);
    event SessionKeyPermissionsUpdated(address indexed key, bytes32 permissionsHash);
    event DailyLimitReset(address indexed key, uint256 day);
    event LegacySessionKeyDeprecated(address indexed key);

    /// @notice Errors
    error NotOwner();
    error NotOwnerOrEntryPoint();
    error InvalidSessionKey();
    error SessionKeyExpired();
    error TargetNotAllowed(address target);
    error SelectorNotAllowed(bytes4 selector);
    error SpendingLimitExceeded(uint256 requested, uint256 limit);
    error DailyLimitExceeded(uint256 requested, uint256 remaining);
    error TargetNotContract(address target);
    error InvalidPasskeyPublicKey();
    error PasskeySignatureInvalid();

    /// @notice Passkey signer state - P256 public key coordinates
    uint256 public passkeyPubKeyX;
    uint256 public passkeyPubKeyY;
    bool public isPasskeySignerSet;

    /// @notice Passkey signature type byte prefixes
    uint8 internal constant SIG_TYPE_ECDSA = 0x00;
    uint8 internal constant SIG_TYPE_PASSKEY = 0x01;

    /// @notice Events for passkey signer
    event PasskeySignerSet(uint256 indexed pubKeyX, uint256 indexed pubKeyY);
    event PasskeySignerRemoved();

    /// @notice Modifier for owner-only functions
    modifier onlyOwner() {
        _checkOwner();
        _;
    }

    /// @notice Modifier for owner or entry point access control
    modifier onlyOwnerOrEntryPoint() {
        _checkOwnerOrEntryPoint();
        _;
    }

    /// @notice Modifier to check session key permissions on execute
    modifier checkSessionKeyPermissions(address dest, uint256 value, bytes calldata data) {
        _checkSessionKeyPermissionsInternal(dest, value, data);
        _;
    }

    /// @dev Internal function to check owner access
    function _checkOwner() internal view {
        if (msg.sender != owner) revert NotOwner();
    }

    /// @dev Internal function to check owner or entry point access
    function _checkOwnerOrEntryPoint() internal view {
        if (msg.sender != owner && msg.sender != address(_ENTRY_POINT)) {
            revert NotOwnerOrEntryPoint();
        }
    }

    /// @dev Internal function to check session key permissions
    function _checkSessionKeyPermissionsInternal(address dest, uint256 value, bytes calldata data) internal {
        if (_pendingSessionKey != address(0) && msg.sender == address(_ENTRY_POINT)) {
            _validateSessionKeyPermissions(_pendingSessionKey, dest, value, data);
        }
    }

    /// @notice Constructor - sets immutable entry point and disables initializers
    /// @param anEntryPoint The ERC-4337 EntryPoint contract address
    constructor(IEntryPoint anEntryPoint) {
        require(address(anEntryPoint) != address(0), "Invalid entry point");
        _ENTRY_POINT = anEntryPoint;
        _disableInitializers();
    }

    /// @notice Initialize the account with an owner
    function initialize(address anOwner) public virtual initializer {
        require(anOwner != address(0), "Invalid owner");
        owner = anOwner;
        emit AccountInitialized(anOwner);
    }

    /// @notice Get the ERC-4337 entry point contract
    /// @return The IEntryPoint interface of the entry point
    function entryPoint() public view virtual override returns (IEntryPoint) {
        return _ENTRY_POINT;
    }

    /// @notice Execute a single transaction
    function execute(address dest, uint256 value, bytes calldata data)
        external
        override
        onlyOwnerOrEntryPoint
        checkSessionKeyPermissions(dest, value, data)
    {
        // Clear pending session key BEFORE external call to prevent reentrancy
        _pendingSessionKey = address(0);
        _call(dest, value, data);
    }

    /// @notice Maximum batch size to prevent out-of-gas issues
    uint256 public constant MAX_BATCH_SIZE = 32;

    /// @notice Execute a batch of transactions
    function executeBatch(
        address[] calldata dests,
        uint256[] calldata values,
        bytes[] calldata datas
    ) external onlyOwnerOrEntryPoint {
        require(
            dests.length == values.length && values.length == datas.length, "Array length mismatch"
        );
        require(dests.length <= MAX_BATCH_SIZE, "Batch size exceeds limit");

        // Check permissions for each call if session key is pending
        if (_pendingSessionKey != address(0) && msg.sender == address(_ENTRY_POINT)) {
            for (uint256 i = 0; i < dests.length; i++) {
                _validateSessionKeyPermissions(_pendingSessionKey, dests[i], values[i], datas[i]);
            }
        }

        // Clear pending session key BEFORE external calls to prevent reentrancy
        _pendingSessionKey = address(0);

        for (uint256 i = 0; i < dests.length; i++) {
            _call(dests[i], values[i], datas[i]);
        }
    }

    /// @notice Add a session key with permissions
    /// @param key The session key address
    /// @param validAfter Timestamp after which key is valid
    /// @param validUntil Timestamp until which key is valid
    /// @param permissions The permissions for this session key
    function addSessionKey(
        address key,
        uint48 validAfter,
        uint48 validUntil,
        SessionKeyPermissions calldata permissions
    ) external onlyOwner {
        require(key != address(0), "Invalid session key");
        require(validUntil > validAfter, "Invalid time bounds");
        require(validUntil > block.timestamp, "Session already expired");
        bytes32 permissionsHash = _computePermissionsHash(permissions);

        // Store metadata
        sessionKeys[key] = SessionKey({
            key: key,
            validAfter: validAfter,
            validUntil: validUntil,
            permissionsHash: permissionsHash
        });

        // Store full permissions
        SessionKeyStorage storage storage_ = _sessionKeyStorage[key];
        storage_.metadata = sessionKeys[key];

        // Clear old allowed targets (array + mapping)
        for (uint256 i = 0; i < storage_.allowedTargets.length; i++) {
            storage_.allowedTargetMap[storage_.allowedTargets[i]] = false;
        }
        delete storage_.allowedTargets;
        for (uint256 i = 0; i < permissions.allowedTargets.length; i++) {
            storage_.allowedTargets.push(permissions.allowedTargets[i]);
            storage_.allowedTargetMap[permissions.allowedTargets[i]] = true;
        }

        // Clear old allowed selectors (array + mapping)
        for (uint256 i = 0; i < storage_.allowedSelectors.length; i++) {
            storage_.allowedSelectorMap[storage_.allowedSelectors[i]] = false;
        }
        delete storage_.allowedSelectors;
        for (uint256 i = 0; i < permissions.allowedSelectors.length; i++) {
            storage_.allowedSelectors.push(permissions.allowedSelectors[i]);
            storage_.allowedSelectorMap[permissions.allowedSelectors[i]] = true;
        }

        storage_.spendingLimit = permissions.spendingLimit;
        storage_.dailyLimit = permissions.dailyLimit;
        storage_.dailySpent = 0;
        storage_.lastResetDay = block.timestamp / 1 days;

        emit SessionKeyAdded(key, validUntil);
    }

    /// @notice Add a session key with raw permissionsHash (legacy compatibility)
    /// @dev DEPRECATED: This creates a session key with unlimited permissions.
    ///      Use addSessionKey() with explicit permissions instead.
    ///      Limited to 7 days max duration to mitigate risk.
    function addSessionKeyLegacy(
        address key,
        uint48 validAfter,
        uint48 validUntil,
        bytes32 permissionsHash
    ) external onlyOwner {
        require(key != address(0), "Invalid session key");
        // Require valid time bounds to prevent unlimited session keys
        require(validUntil > validAfter, "Invalid time bounds");
        require(validUntil > block.timestamp, "Session already expired");
        // Limit session duration to 7 days max for legacy keys (security hardening)
        require(validUntil - validAfter <= 7 days, "Legacy key max 7 days");

        sessionKeys[key] = SessionKey({
            key: key,
            validAfter: validAfter,
            validUntil: validUntil,
            permissionsHash: permissionsHash
        });

        // Store with empty/unlimited permissions
        SessionKeyStorage storage storage_ = _sessionKeyStorage[key];
        storage_.metadata = sessionKeys[key];
        // Clear old targets/selectors mappings
        for (uint256 i = 0; i < storage_.allowedTargets.length; i++) {
            storage_.allowedTargetMap[storage_.allowedTargets[i]] = false;
        }
        delete storage_.allowedTargets;
        for (uint256 i = 0; i < storage_.allowedSelectors.length; i++) {
            storage_.allowedSelectorMap[storage_.allowedSelectors[i]] = false;
        }
        delete storage_.allowedSelectors;
        storage_.spendingLimit = 0;
        storage_.dailyLimit = 0;
        storage_.dailySpent = 0;
        storage_.lastResetDay = block.timestamp / 1 days;

        emit SessionKeyAdded(key, validUntil);
        emit LegacySessionKeyDeprecated(key);
    }

    /// @notice Remove a session key
    function removeSessionKey(address key) external onlyOwner {
        delete sessionKeys[key];
        delete _sessionKeyStorage[key];
        emit SessionKeyRemoved(key);
    }

    /// @notice Update session key permissions
    /// @param key The session key address
    /// @param permissions The new permissions
    function updateSessionKeyPermissions(address key, SessionKeyPermissions calldata permissions)
        external
        onlyOwner
    {
        require(sessionKeys[key].key != address(0), "Session key not found");

        bytes32 permissionsHash = _computePermissionsHash(permissions);

        // Update metadata
        sessionKeys[key].permissionsHash = permissionsHash;

        // Update storage
        SessionKeyStorage storage storage_ = _sessionKeyStorage[key];
        storage_.metadata.permissionsHash = permissionsHash;

        // Clear old allowed targets (array + mapping)
        for (uint256 i = 0; i < storage_.allowedTargets.length; i++) {
            storage_.allowedTargetMap[storage_.allowedTargets[i]] = false;
        }
        delete storage_.allowedTargets;
        for (uint256 i = 0; i < permissions.allowedTargets.length; i++) {
            storage_.allowedTargets.push(permissions.allowedTargets[i]);
            storage_.allowedTargetMap[permissions.allowedTargets[i]] = true;
        }

        // Clear old allowed selectors (array + mapping)
        for (uint256 i = 0; i < storage_.allowedSelectors.length; i++) {
            storage_.allowedSelectorMap[storage_.allowedSelectors[i]] = false;
        }
        delete storage_.allowedSelectors;
        for (uint256 i = 0; i < permissions.allowedSelectors.length; i++) {
            storage_.allowedSelectors.push(permissions.allowedSelectors[i]);
            storage_.allowedSelectorMap[permissions.allowedSelectors[i]] = true;
        }

        storage_.spendingLimit = permissions.spendingLimit;
        storage_.dailyLimit = permissions.dailyLimit;
        // Note: dailySpent is preserved, not reset

        emit SessionKeyPermissionsUpdated(key, permissionsHash);
    }

    /// @notice Get session key permissions
    /// @param key The session key address
    /// @return permissions The permissions struct
    function getSessionKeyPermissions(address key)
        external
        view
        returns (SessionKeyPermissions memory permissions)
    {
        SessionKeyStorage storage storage_ = _sessionKeyStorage[key];
        permissions.allowedTargets = storage_.allowedTargets;
        permissions.allowedSelectors = storage_.allowedSelectors;
        permissions.spendingLimit = storage_.spendingLimit;
        permissions.dailyLimit = storage_.dailyLimit;
    }

    /// @notice Get session key spending info
    /// @param key The session key address
    /// @return dailySpent Amount spent today
    /// @return dailyRemaining Amount remaining for today (0 if unlimited)
    /// @return spendingLimit Per-transaction limit (0 if unlimited)
    function getSessionKeySpendingInfo(address key)
        external
        view
        returns (uint256 dailySpent, uint256 dailyRemaining, uint256 spendingLimit)
    {
        SessionKeyStorage storage storage_ = _sessionKeyStorage[key];
        uint256 currentDay = block.timestamp / 1 days;

        if (storage_.lastResetDay < currentDay) {
            dailySpent = 0;
        } else {
            dailySpent = storage_.dailySpent;
        }

        if (storage_.dailyLimit == 0) {
            dailyRemaining = type(uint256).max; // Unlimited
        } else if (dailySpent >= storage_.dailyLimit) {
            dailyRemaining = 0;
        } else {
            dailyRemaining = storage_.dailyLimit - dailySpent;
        }

        spendingLimit = storage_.spendingLimit;
    }

    /// @notice Check if a session key is valid
    function isValidSessionKey(address key) public view returns (bool) {
        SessionKey memory session = sessionKeys[key];
        if (session.key == address(0)) return false;
        if (block.timestamp < session.validAfter) return false;
        if (block.timestamp > session.validUntil) return false;
        return true;
    }

    /// @notice Check if a target is allowed for a session key
    function isTargetAllowed(address key, address target) public view returns (bool) {
        SessionKeyStorage storage storage_ = _sessionKeyStorage[key];

        // If no targets specified, all targets are allowed
        if (storage_.allowedTargets.length == 0) return true;

        return storage_.allowedTargetMap[target];
    }

    /// @notice Check if a selector is allowed for a session key
    function isSelectorAllowed(address key, bytes4 selector) public view returns (bool) {
        SessionKeyStorage storage storage_ = _sessionKeyStorage[key];

        // If no selectors specified, all selectors are allowed
        if (storage_.allowedSelectors.length == 0) return true;

        return storage_.allowedSelectorMap[selector];
    }

    /// @notice Validate user operation signature
    /// @dev Supports two signature formats based on type byte prefix:
    ///      - Type 0x00 (ECDSA): Standard secp256k1 ECDSA signature (65 bytes after type byte)
    ///      - Type 0x01 (Passkey): P256 signature [type(1) || r(32) || s(32)]
    ///      If no type byte prefix is present, falls back to ECDSA for backward compatibility.
    function _validateSignature(PackedUserOperation calldata userOp, bytes32 userOpHash)
        internal
        virtual
        override
        returns (uint256 validationData)
    {
        // Clear pending session key at start to prevent state pollution
        _pendingSessionKey = address(0);

        bytes calldata sig = userOp.signature;

        // Check if signature has a type prefix
        if (sig.length > 0) {
            uint8 sigType = uint8(sig[0]);

            // Type 0x01: Passkey P256 signature
            if (sigType == SIG_TYPE_PASSKEY && sig.length == 65) {
                return _validatePasskeySignature(userOpHash, sig);
            }

            // Type 0x00: Explicit ECDSA (strip type byte)
            if (sigType == SIG_TYPE_ECDSA && sig.length == 66) {
                return _validateEcdsaSignature(userOpHash, sig[1:]);
            }
        }

        // Default: treat entire signature as ECDSA (backward compatible)
        return _validateEcdsaSignature(userOpHash, sig);
    }

    /// @dev Validate an ECDSA signature against owner or session keys
    function _validateEcdsaSignature(bytes32 userOpHash, bytes calldata sig)
        internal
        returns (uint256 validationData)
    {
        bytes32 hash = userOpHash.toEthSignedMessageHash();
        address signer = hash.recover(sig);

        // Check if signer is owner
        if (signer == owner) {
            return 0; // Valid with no restrictions
        }

        // Check if signer is a valid session key
        SessionKey memory session = sessionKeys[signer];
        if (session.key != address(0)) {
            if (block.timestamp < session.validAfter) {
                return SIG_VALIDATION_FAILED;
            }
            if (block.timestamp > session.validUntil) {
                return SIG_VALIDATION_FAILED;
            }

            // Store session key for permission checking during execution
            _pendingSessionKey = signer;

            // Return validation data with time range
            return _packValidationData(false, session.validUntil, session.validAfter);
        }

        return SIG_VALIDATION_FAILED;
    }

    /// @dev Validate a P256 passkey signature
    function _validatePasskeySignature(bytes32 userOpHash, bytes calldata sig)
        internal
        view
        returns (uint256 validationData)
    {
        if (!isPasskeySignerSet) return SIG_VALIDATION_FAILED;

        // Parse signature: [type(1) || r(32) || s(32)]
        uint256 r = uint256(bytes32(sig[1:33]));
        uint256 s = uint256(bytes32(sig[33:65]));

        // Verify P256 signature over the userOpHash
        if (P256Verifier.verify(userOpHash, r, s, passkeyPubKeyX, passkeyPubKeyY)) {
            return 0; // Valid with no restrictions (passkey == owner-level)
        }

        return SIG_VALIDATION_FAILED;
    }

    /// @notice Validate session key permissions for a call
    function _validateSessionKeyPermissions(
        address key,
        address target,
        uint256 value,
        bytes calldata data
    ) internal {
        SessionKeyStorage storage storage_ = _sessionKeyStorage[key];

        // Check target is allowed (O(1) mapping lookup)
        if (storage_.allowedTargets.length > 0) {
            if (!storage_.allowedTargetMap[target]) revert TargetNotAllowed(target);
        }

        // Check selector is allowed (only if data has a selector)
        /// @notice Empty calldata (ETH transfers) bypass selector restrictions
        /// @dev This is intentional - use spendingLimit to control ETH transfers
        if (storage_.allowedSelectors.length > 0 && data.length < 4) {
            revert SelectorNotAllowed(bytes4(0));
        }
        if (data.length >= 4 && storage_.allowedSelectors.length > 0) {
            bytes4 selector = bytes4(data[:4]);
            if (!storage_.allowedSelectorMap[selector]) revert SelectorNotAllowed(selector);
        }

        // Check spending limit
        if (storage_.spendingLimit > 0 && value > storage_.spendingLimit) {
            revert SpendingLimitExceeded(value, storage_.spendingLimit);
        }

        // Check and update daily limit
        if (storage_.dailyLimit > 0) {
            uint256 currentDay = block.timestamp / 1 days;

            // Reset daily spent if it's a new day
            if (storage_.lastResetDay < currentDay) {
                storage_.dailySpent = 0;
                storage_.lastResetDay = currentDay;
                emit DailyLimitReset(key, currentDay);
            }

            uint256 remaining = storage_.dailyLimit - storage_.dailySpent;
            if (value > remaining) {
                revert DailyLimitExceeded(value, remaining);
            }

            // Update daily spent
            storage_.dailySpent += value;
        }
    }

    /// @notice Compute permissions hash
    function _computePermissionsHash(SessionKeyPermissions calldata permissions)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encode(
                permissions.allowedTargets,
                permissions.allowedSelectors,
                permissions.spendingLimit,
                permissions.dailyLimit
            )
        );
    }

    /// @notice Internal call function
    /// @dev Checks contract existence for calls with data to prevent silent failures
    function _call(address target, uint256 value, bytes memory data) internal {
        // Check contract existence for calls with data
        if (data.length > 0 && target.code.length == 0) {
            revert TargetNotContract(target);
        }
        (bool success, bytes memory result) = target.call{ value: value }(data);
        if (!success) {
            assembly {
                revert(add(result, 32), mload(result))
            }
        }
    }

    // ============ Passkey Signer Management ============

    /// @notice Set a passkey signer for this account
    /// @dev Only the owner can set a passkey signer. The passkey acts as an
    ///      alternative signer alongside the ECDSA owner key.
    /// @param _pubKeyX The x coordinate of the P256 public key
    /// @param _pubKeyY The y coordinate of the P256 public key
    function setPasskeySigner(uint256 _pubKeyX, uint256 _pubKeyY) external onlyOwner {
        if (_pubKeyX == 0 || _pubKeyY == 0) revert InvalidPasskeyPublicKey();
        if (_pubKeyX >= P256Verifier.P256_P || _pubKeyY >= P256Verifier.P256_P) {
            revert InvalidPasskeyPublicKey();
        }

        passkeyPubKeyX = _pubKeyX;
        passkeyPubKeyY = _pubKeyY;
        isPasskeySignerSet = true;

        emit PasskeySignerSet(_pubKeyX, _pubKeyY);
    }

    /// @notice Remove the passkey signer from this account
    function removePasskeySigner() external onlyOwner {
        passkeyPubKeyX = 0;
        passkeyPubKeyY = 0;
        isPasskeySignerSet = false;

        emit PasskeySignerRemoved();
    }

    /// @notice Get passkey signer public key
    /// @return x The x coordinate
    /// @return y The y coordinate
    /// @return isSet Whether a passkey signer is configured
    function getPasskeySigner() external view returns (uint256 x, uint256 y, bool isSet) {
        return (passkeyPubKeyX, passkeyPubKeyY, isPasskeySignerSet);
    }

    // ============ ERC-1271 Signature Validation ============

    /// @notice ERC-1271 magic value for valid signatures
    bytes4 internal constant ERC1271_MAGIC_VALUE = 0x1626ba7e;

    /// @notice Validate a signature for ERC-1271 compliance
    /// @dev Supports owner signatures and valid session key signatures
    /// @param hash The hash of the data to validate
    /// @param signature The signature to validate
    /// @return magicValue ERC1271_MAGIC_VALUE if valid, 0xffffffff if invalid
    function isValidSignature(bytes32 hash, bytes calldata signature)
        external
        view
        override
        returns (bytes4 magicValue)
    {
        bytes32 ethSignedHash = hash.toEthSignedMessageHash();
        address signer = ethSignedHash.recover(signature);

        // Check if signer is the owner
        if (signer == owner) {
            return ERC1271_MAGIC_VALUE;
        }

        // Check if signer is a valid session key
        if (isValidSessionKey(signer)) {
            return ERC1271_MAGIC_VALUE;
        }

        return bytes4(0xffffffff);
    }

    // ============ Token Receivers (ERC-721, ERC-1155) ============

    /// @notice Handle receiving an ERC-721 token
    function onERC721Received(
        address /* operator */,
        address /* from */,
        uint256 /* tokenId */,
        bytes calldata /* data */
    ) external pure override returns (bytes4) {
        return IERC721Receiver.onERC721Received.selector;
    }

    /// @notice Handle receiving a single ERC-1155 token
    function onERC1155Received(
        address /* operator */,
        address /* from */,
        uint256 /* id */,
        uint256 /* value */,
        bytes calldata /* data */
    ) external pure override returns (bytes4) {
        return IERC1155Receiver.onERC1155Received.selector;
    }

    /// @notice Handle receiving a batch of ERC-1155 tokens
    function onERC1155BatchReceived(
        address /* operator */,
        address /* from */,
        uint256[] calldata /* ids */,
        uint256[] calldata /* values */,
        bytes calldata /* data */
    ) external pure override returns (bytes4) {
        return IERC1155Receiver.onERC1155BatchReceived.selector;
    }

    // ============ ERC-165 Introspection ============

    /// @notice Check if this contract supports a given interface
    function supportsInterface(bytes4 interfaceId) external pure override returns (bool) {
        return
            interfaceId == type(IERC165).interfaceId ||
            interfaceId == type(IERC1271).interfaceId ||
            interfaceId == type(IERC721Receiver).interfaceId ||
            interfaceId == type(IERC1155Receiver).interfaceId;
    }

    /// @notice Receive ETH
    receive() external payable { }
}
