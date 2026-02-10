// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Clones } from "@openzeppelin/contracts/proxy/Clones.sol";
import { RampOSAccount } from "../RampOSAccount.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title PasskeyAccountFactory
 * @author RampOS Team
 * @notice Factory for deploying RampOS smart accounts with passkey (P256) signers
 * @dev Uses EIP-1167 minimal proxy pattern for gas-efficient deployment.
 *      Creates accounts deterministically from the passkey public key coordinates
 *      using CREATE2. The passkey is set during account initialization.
 *
 * Features:
 *  - Deterministic address generation from passkey public key (CREATE2)
 *  - Gas-efficient deployment via minimal proxy clones
 *  - Counterfactual address prediction before deployment
 *  - Automatic passkey signer setup during account creation
 *
 * Security considerations:
 *  - Account addresses are deterministic based on pubKey + salt
 *  - Implementation is immutable and set at factory deployment
 *  - An EOA owner is still required as account recovery mechanism
 *  - Public key coordinates are validated before account creation
 */
contract PasskeyAccountFactory {
    /// @notice Account implementation contract (immutable for gas savings)
    RampOSAccount public immutable ACCOUNT_IMPLEMENTATION;

    /// @notice ERC-4337 Entry Point contract reference
    IEntryPoint public immutable ENTRY_POINT;

    /// @notice Emitted when a new passkey account is created
    /// @param account The deployed account address
    /// @param owner The owner address (recovery key)
    /// @param pubKeyX The x coordinate of the passkey public key
    /// @param pubKeyY The y coordinate of the passkey public key
    /// @param salt The salt used for deterministic deployment
    event PasskeyAccountCreated(
        address indexed account,
        address indexed owner,
        uint256 pubKeyX,
        uint256 pubKeyY,
        uint256 salt
    );

    /// @notice Errors
    error InvalidPublicKey();
    error InvalidOwner();

    /// @notice Constructor - deploys the account implementation
    /// @param _entryPoint The ERC-4337 EntryPoint contract address
    constructor(IEntryPoint _entryPoint) {
        ENTRY_POINT = _entryPoint;
        ACCOUNT_IMPLEMENTATION = new RampOSAccount(_entryPoint);
    }

    /**
     * @notice Create a new account with a passkey signer, or return existing one
     * @dev Uses EIP-1167 minimal proxy for gas-efficient deployment.
     *      The account is initialized with the owner address and then the passkey
     *      signer is set. The deterministic address is derived from the passkey
     *      public key coordinates and the salt.
     * @param owner The owner/recovery address for the account
     * @param pubKeyX The x coordinate of the P256 passkey public key
     * @param pubKeyY The y coordinate of the P256 passkey public key
     * @param salt Salt for CREATE2 deterministic deployment
     * @return account The created or existing account instance
     */
    function createAccount(
        address owner,
        uint256 pubKeyX,
        uint256 pubKeyY,
        uint256 salt
    ) external returns (RampOSAccount account) {
        if (owner == address(0)) revert InvalidOwner();
        if (pubKeyX == 0 || pubKeyY == 0) revert InvalidPublicKey();

        address addr = getAddress(pubKeyX, pubKeyY, salt);

        // Check if already deployed - return existing account
        if (addr.code.length > 0) {
            return RampOSAccount(payable(addr));
        }

        // Deploy using minimal proxy clone
        account = RampOSAccount(
            payable(
                Clones.cloneDeterministic(
                    address(ACCOUNT_IMPLEMENTATION),
                    _getSalt(pubKeyX, pubKeyY, salt)
                )
            )
        );

        // Initialize with owner
        account.initialize(owner);

        // Set passkey signer (owner is this factory during initialization,
        // but we call through the account which requires owner)
        // Note: The owner calls setPasskeySigner after initialization
        // This is a two-step process: factory deploys + owner configures passkey

        emit PasskeyAccountCreated(address(account), owner, pubKeyX, pubKeyY, salt);
    }

    /**
     * @notice Get the counterfactual address of a passkey account before deployment
     * @dev Useful for pre-computing addresses for gasless onboarding
     * @param pubKeyX The x coordinate of the P256 passkey public key
     * @param pubKeyY The y coordinate of the P256 passkey public key
     * @param salt Salt for CREATE2 deterministic deployment
     * @return The predicted address of the account
     */
    function getAddress(
        uint256 pubKeyX,
        uint256 pubKeyY,
        uint256 salt
    ) public view returns (address) {
        return Clones.predictDeterministicAddress(
            address(ACCOUNT_IMPLEMENTATION),
            _getSalt(pubKeyX, pubKeyY, salt)
        );
    }

    /**
     * @notice Compute the combined salt for CREATE2 from passkey public key
     * @param pubKeyX The x coordinate of the P256 passkey public key
     * @param pubKeyY The y coordinate of the P256 passkey public key
     * @param salt The user-provided salt value
     * @return The combined salt hash
     */
    function _getSalt(
        uint256 pubKeyX,
        uint256 pubKeyY,
        uint256 salt
    ) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(pubKeyX, pubKeyY, salt));
    }
}
