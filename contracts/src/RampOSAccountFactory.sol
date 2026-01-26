// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/proxy/Clones.sol";
import "./RampOSAccount.sol";

/**
 * @title RampOSAccountFactory
 * @notice Factory for deploying RampOS smart accounts
 * @dev Uses EIP-1167 minimal proxy pattern for gas-efficient deployment
 */
contract RampOSAccountFactory {
    /// @notice Account implementation
    RampOSAccount public immutable accountImplementation;

    /// @notice Entry point
    IEntryPoint public immutable entryPoint;

    /// @notice Events
    event AccountCreated(address indexed account, address indexed owner, uint256 salt);

    constructor(IEntryPoint _entryPoint) {
        entryPoint = _entryPoint;
        accountImplementation = new RampOSAccount(_entryPoint);
    }

    /**
     * @notice Create a new account
     * @param owner The owner of the account
     * @param salt Salt for CREATE2
     * @return account The created account address
     */
    function createAccount(
        address owner,
        uint256 salt
    ) external returns (RampOSAccount account) {
        address addr = getAddress(owner, salt);

        // Check if already deployed
        if (addr.code.length > 0) {
            return RampOSAccount(payable(addr));
        }

        // Deploy using minimal proxy
        account = RampOSAccount(
            payable(
                Clones.cloneDeterministic(
                    address(accountImplementation),
                    _getSalt(owner, salt)
                )
            )
        );

        account.initialize(owner);

        emit AccountCreated(address(account), owner, salt);
    }

    /**
     * @notice Get the counterfactual address of an account
     * @param owner The owner of the account
     * @param salt Salt for CREATE2
     * @return The predicted address
     */
    function getAddress(
        address owner,
        uint256 salt
    ) public view returns (address) {
        return
            Clones.predictDeterministicAddress(
                address(accountImplementation),
                _getSalt(owner, salt)
            );
    }

    /**
     * @notice Compute the salt for CREATE2
     */
    function _getSalt(
        address owner,
        uint256 salt
    ) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(owner, salt));
    }
}
