// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Clones } from "@openzeppelin/contracts/proxy/Clones.sol";
import { RampOSAccount } from "./RampOSAccount.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title RampOSAccountFactory
 * @author RampOS Team
 * @notice Factory for deploying RampOS smart accounts using minimal proxies
 * @dev Uses EIP-1167 minimal proxy pattern for gas-efficient deployment.
 *
 * Features:
 *  - Deterministic address generation using CREATE2
 *  - Gas-efficient deployment via minimal proxy clones
 *  - Counterfactual address prediction before deployment
 *
 * Security considerations:
 *  - Account addresses are deterministic based on owner + salt
 *  - Implementation is immutable and set at factory deployment
 */
contract RampOSAccountFactory {
    /// @notice Account implementation contract (immutable for gas savings)
    RampOSAccount public immutable ACCOUNT_IMPLEMENTATION;

    /// @notice ERC-4337 Entry Point contract reference
    IEntryPoint public immutable ENTRY_POINT;

    /// @notice Emitted when a new account is created
    /// @param account The deployed account address
    /// @param owner The owner of the account
    /// @param salt The salt used for deterministic deployment
    event AccountCreated(address indexed account, address indexed owner, uint256 salt);

    /// @notice Constructor - deploys the account implementation
    /// @param _entryPoint The ERC-4337 EntryPoint contract address
    constructor(IEntryPoint _entryPoint) {
        ENTRY_POINT = _entryPoint;
        ACCOUNT_IMPLEMENTATION = new RampOSAccount(_entryPoint);
    }

    /**
     * @notice Create a new account or return existing one
     * @dev Uses EIP-1167 minimal proxy for gas-efficient deployment
     * @param owner The owner of the account
     * @param salt Salt for CREATE2 deterministic deployment
     * @return account The created or existing account instance
     */
    function createAccount(address owner, uint256 salt) external returns (RampOSAccount account) {
        require(owner != address(0), "Invalid owner");
        address addr = getAddress(owner, salt);

        // Check if already deployed - return existing account
        if (addr.code.length > 0) {
            return RampOSAccount(payable(addr));
        }

        // Deploy using minimal proxy clone
        account = RampOSAccount(
            payable(Clones.cloneDeterministic(
                    address(ACCOUNT_IMPLEMENTATION), _getSalt(owner, salt)
                ))
        );

        account.initialize(owner);

        emit AccountCreated(address(account), owner, salt);
    }

    /**
     * @notice Get the counterfactual address of an account before deployment
     * @dev Useful for pre-computing addresses for gasless onboarding
     * @param owner The owner of the account
     * @param salt Salt for CREATE2 deterministic deployment
     * @return The predicted address of the account
     */
    function getAddress(address owner, uint256 salt) public view returns (address) {
        return Clones.predictDeterministicAddress(
            address(ACCOUNT_IMPLEMENTATION), _getSalt(owner, salt)
        );
    }

    /**
     * @notice Compute the combined salt for CREATE2
     * @param owner The owner address to include in salt
     * @param salt The user-provided salt value
     * @return The combined salt hash
     */
    function _getSalt(address owner, uint256 salt) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(owner, salt));
    }
}
