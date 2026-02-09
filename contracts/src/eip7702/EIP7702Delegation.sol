// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "@account-abstraction/contracts/accounts/Simple7702Account.sol";
import "@account-abstraction/contracts/core/Helpers.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import "./EIP7702Auth.sol";

/**
 * @title EIP7702Delegation
 * @notice An account contract designed for EIP-7702 delegation.
 * @dev Inherits from Simple7702Account for basic AA features and EIP7702Auth for delegation logic.
 */
contract EIP7702Delegation is Simple7702Account, EIP7702Auth {

    // Mapping from delegate address to authorization status
    mapping(address => bool) public isDelegate;

    constructor(IEntryPoint anEntryPoint)
        Simple7702Account(anEntryPoint)
        EIP7702Auth("EIP7702Delegation", "1.0")
    {}

    /**
     * @notice Authorizes a new delegate via signature or direct call.
     * @param delegate The address to authorize.
     * @param deadline The signature deadline.
     * @param signature The EIP-712 signature from the owner (address(this)).
     */
    function authorizeDelegate(address delegate, uint256 deadline, bytes calldata signature) external {
        if (msg.sender == address(this)) {
            // If called by the account itself (e.g. signed by owner key), just set it.
            isDelegate[delegate] = true;
            emit Delegated(address(this), delegate, 0);
        } else {
            // Relayed call or someone else submitting the signature
            _consumeNonce(address(this), delegate, deadline, signature);
            isDelegate[delegate] = true;
        }
    }

    /**
     * @notice Revokes a delegate.
     */
    function revokeDelegate(address delegate) external {
        require(
            msg.sender == address(this) || msg.sender == delegate,
            "Only owner or self-revoke"
        );
        isDelegate[delegate] = false;
    }

    /**
     * @dev Override to allow delegates to call execute/executeBatch.
     */
    function _requireForExecute() internal view virtual override {
        require(
            msg.sender == address(this) ||
            msg.sender == address(entryPoint()) ||
            isDelegate[msg.sender],
            NotFromEntryPoint(
                msg.sender,
                address(this),
                address(entryPoint())
            )
        );
    }

    /**
     * @dev Internal check for signature validity (owner or delegate).
     */
    function _checkSignatureDelegated(bytes32 hash, bytes memory signature) internal view returns (bool) {
        bytes32 ethHash = MessageHashUtils.toEthSignedMessageHash(hash);
        address signer = ECDSA.recover(ethHash, signature);
        return signer == address(this) || isDelegate[signer];
    }

    function _validateSignature(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash
    ) internal virtual override returns (uint256 validationData) {
        return _checkSignatureDelegated(userOpHash, userOp.signature) ? SIG_VALIDATION_SUCCESS : SIG_VALIDATION_FAILED;
    }

    function isValidSignature(bytes32 hash, bytes memory signature) public virtual override view returns (bytes4 magicValue) {
        return _checkSignatureDelegated(hash, signature) ? this.isValidSignature.selector : bytes4(0xffffffff);
    }
}
