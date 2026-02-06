// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

/**
 * @title EIP7702Auth
 * @notice Logic to verify EIP-712 signatures for delegation.
 * @dev Intended to be inherited by the main delegation contract.
 */
abstract contract EIP7702Auth is EIP712 {
    using ECDSA for bytes32;

    bytes32 private constant _DELEGATION_TYPEHASH =
        keccak256("Delegation(address delegate,uint256 nonce,uint256 deadline)");

    mapping(address => uint256) private _nonces;

    /**
     * @dev Emitted when a user delegates execution rights to a new delegate.
     */
    event Delegated(address indexed user, address indexed delegate, uint256 nonce);

    constructor(string memory name, string memory version) EIP712(name, version) {}

    /**
     * @notice Returns the nonce for a given user.
     */
    function nonces(address owner) public view returns (uint256) {
        return _nonces[owner];
    }

    /**
     * @notice Verifies a delegation signature and returns the signer.
     * @param delegate The address being authorized.
     * @param nonce The user's current nonce.
     * @param deadline The timestamp at which the signature expires.
     * @param signature The EIP-712 signature.
     */
    function _verifyDelegation(
        address delegate,
        uint256 nonce,
        uint256 deadline,
        bytes calldata signature
    ) internal view returns (address) {
        if (block.timestamp > deadline) {
            revert("EIP7702Auth: expired deadline");
        }

        bytes32 structHash = keccak256(
            abi.encode(_DELEGATION_TYPEHASH, delegate, nonce, deadline)
        );

        bytes32 hash = _hashTypedDataV4(structHash);
        address signer = ECDSA.recover(hash, signature);

        return signer;
    }

    /**
     * @notice Consumes a nonce for a user, verifying the signature.
     * @dev Should be called by the function that accepts the delegation.
     */
    function _consumeNonce(
        address owner,
        address delegate,
        uint256 deadline,
        bytes calldata signature
    ) internal {
        uint256 currentNonce = _nonces[owner];
        address signer = _verifyDelegation(delegate, currentNonce, deadline, signature);

        require(signer == owner, "EIP7702Auth: invalid signature");

        _nonces[owner]++;

        emit Delegated(owner, delegate, currentNonce);
    }
}
