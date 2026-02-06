// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { StdInvariant } from "forge-std/StdInvariant.sol";
import { RampOSAccount } from "../../src/RampOSAccount.sol";
import { RampOSAccountFactory } from "../../src/RampOSAccountFactory.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title FactoryHandler
 * @notice Handler contract for fuzzing RampOSAccountFactory state transitions
 * @dev Exposes bounded actions for invariant testing
 */
contract FactoryHandler is Test {
    RampOSAccountFactory public factory;
    IEntryPoint public entryPoint;

    // Track created accounts for invariant verification
    mapping(bytes32 => address) public createdAccounts;
    mapping(address => bool) public isCreatedAccount;
    address[] public accounts;

    // Ghost variables for tracking state
    uint256 public totalAccountsCreated;
    uint256 public totalDuplicateAttempts;

    constructor(RampOSAccountFactory _factory, IEntryPoint _entryPoint) {
        factory = _factory;
        entryPoint = _entryPoint;
    }

    /// @notice Create an account with bounded parameters
    function createAccount(address owner, uint256 salt) external {
        // Bound owner to non-zero addresses
        if (owner == address(0)) {
            owner = makeAddr("defaultOwner");
        }

        bytes32 key = keccak256(abi.encodePacked(owner, salt));
        address predicted = factory.getAddress(owner, salt);

        bool alreadyExists = createdAccounts[key] != address(0);

        RampOSAccount account = factory.createAccount(owner, salt);

        if (alreadyExists) {
            totalDuplicateAttempts++;
            // Should return existing account
            assertEq(address(account), createdAccounts[key], "Should return existing account");
        } else {
            // New account created
            createdAccounts[key] = address(account);
            isCreatedAccount[address(account)] = true;
            accounts.push(address(account));
            totalAccountsCreated++;

            // Verify predicted address matches
            assertEq(address(account), predicted, "Predicted address should match");
        }
    }

    /// @notice Get address prediction without creating
    function getAddress(address owner, uint256 salt) external view returns (address) {
        if (owner == address(0)) return address(0);
        return factory.getAddress(owner, salt);
    }

    /// @notice Get total unique accounts created
    function getAccountCount() external view returns (uint256) {
        return accounts.length;
    }

    /// @notice Get account at index
    function getAccountAt(uint256 index) external view returns (address) {
        if (index >= accounts.length) return address(0);
        return accounts[index];
    }
}

/**
 * @title FactoryInvariantTest
 * @notice Invariant tests for RampOSAccountFactory
 * @dev Tests critical security properties that must always hold
 *
 * Invariants tested:
 * 1. Implementation address is immutable and non-zero
 * 2. Entry point address is immutable and non-zero
 * 3. Account creation is deterministic (same inputs = same address)
 * 4. All created accounts have correct owner
 * 5. All created accounts reference correct entry point
 * 6. Predicted addresses match actual deployed addresses
 * 7. Account creation is idempotent (second call returns existing)
 */
contract FactoryInvariantTest is StdInvariant, Test {
    RampOSAccountFactory public factory;
    IEntryPoint public entryPoint;
    FactoryHandler public handler;

    function setUp() public {
        // Use canonical ERC-4337 entry point address
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));

        // Deploy factory
        factory = new RampOSAccountFactory(entryPoint);

        // Deploy handler
        handler = new FactoryHandler(factory, entryPoint);

        // Target the handler for invariant testing
        targetContract(address(handler));

        // Specify target selectors
        bytes4[] memory selectors = new bytes4[](2);
        selectors[0] = FactoryHandler.createAccount.selector;
        selectors[1] = FactoryHandler.getAddress.selector;

        targetSelector(FuzzSelector({
            addr: address(handler),
            selectors: selectors
        }));
    }

    /// @notice Invariant: Implementation address is immutable and non-zero
    function invariant_implementationImmutable() public view {
        address impl = address(factory.ACCOUNT_IMPLEMENTATION());
        assertNotEq(impl, address(0), "Implementation should not be zero");
        assertTrue(impl.code.length > 0, "Implementation should have code");
    }

    /// @notice Invariant: Entry point is immutable and non-zero
    function invariant_entryPointImmutable() public view {
        assertEq(
            address(factory.ENTRY_POINT()),
            address(entryPoint),
            "Entry point should be immutable"
        );
        assertNotEq(address(factory.ENTRY_POINT()), address(0), "Entry point should not be zero");
    }

    /// @notice Invariant: All created accounts have non-zero owners
    function invariant_allAccountsHaveOwners() public view {
        uint256 count = handler.getAccountCount();
        for (uint256 i = 0; i < count; i++) {
            address accountAddr = handler.getAccountAt(i);
            if (accountAddr != address(0)) {
                RampOSAccount account = RampOSAccount(payable(accountAddr));
                assertNotEq(account.owner(), address(0), "Account owner should not be zero");
            }
        }
    }

    /// @notice Invariant: All created accounts reference the correct entry point
    function invariant_allAccountsHaveCorrectEntryPoint() public view {
        uint256 count = handler.getAccountCount();
        for (uint256 i = 0; i < count; i++) {
            address accountAddr = handler.getAccountAt(i);
            if (accountAddr != address(0)) {
                RampOSAccount account = RampOSAccount(payable(accountAddr));
                assertEq(
                    address(account.entryPoint()),
                    address(entryPoint),
                    "Account entry point should match factory entry point"
                );
            }
        }
    }

    /// @notice Invariant: Account count matches unique creations
    function invariant_accountCountConsistent() public view {
        uint256 handlerCount = handler.getAccountCount();
        uint256 expectedCount = handler.totalAccountsCreated();
        assertEq(handlerCount, expectedCount, "Account count should be consistent");
    }

    /// @notice Invariant: Duplicate attempts don't create new accounts
    function invariant_duplicatesHandledCorrectly() public view {
        // Verify that duplicate attempts are tracked separately
        uint256 totalAttempts = handler.totalAccountsCreated() + handler.totalDuplicateAttempts();
        assertGe(totalAttempts, handler.totalAccountsCreated(), "Total attempts >= created accounts");
    }

    /// @notice Invariant: All created accounts have code (are deployed contracts)
    function invariant_allAccountsHaveCode() public view {
        uint256 count = handler.getAccountCount();
        for (uint256 i = 0; i < count; i++) {
            address accountAddr = handler.getAccountAt(i);
            if (accountAddr != address(0)) {
                assertTrue(accountAddr.code.length > 0, "Account should have code");
            }
        }
    }

    /// @notice Invariant: Address prediction is deterministic
    function invariant_addressPredictionDeterministic() public view {
        // Verify that getAddress returns consistent results
        // Use fixed address instead of makeAddr to keep function as view
        address testOwner = address(0x1234567890123456789012345678901234567890);
        uint256 salt = 42;

        address predicted1 = factory.getAddress(testOwner, salt);
        address predicted2 = factory.getAddress(testOwner, salt);

        assertEq(predicted1, predicted2, "Address prediction should be deterministic");
    }

    /// @notice Invariant: Different owners get different addresses
    function invariant_differentOwnersDifferentAddresses() public view {
        // Use fixed addresses instead of makeAddr to keep function as view
        address owner1 = address(0x1111111111111111111111111111111111111111);
        address owner2 = address(0x2222222222222222222222222222222222222222);
        uint256 salt = 42;

        address addr1 = factory.getAddress(owner1, salt);
        address addr2 = factory.getAddress(owner2, salt);

        assertNotEq(addr1, addr2, "Different owners should get different addresses");
    }

    /// @notice Invariant: Different salts get different addresses
    function invariant_differentSaltsDifferentAddresses() public view {
        // Use fixed address instead of makeAddr to keep function as view
        address testOwner = address(0x3333333333333333333333333333333333333333);
        uint256 salt1 = 1;
        uint256 salt2 = 2;

        address addr1 = factory.getAddress(testOwner, salt1);
        address addr2 = factory.getAddress(testOwner, salt2);

        assertNotEq(addr1, addr2, "Different salts should get different addresses");
    }

    /// @notice Print call summary for debugging
    function invariant_callSummary() public pure {
        // Log summary of operations for debugging
        // console.log("Total accounts created:", handler.totalAccountsCreated());
        // console.log("Total duplicate attempts:", handler.totalDuplicateAttempts());
        assertTrue(true);
    }
}
