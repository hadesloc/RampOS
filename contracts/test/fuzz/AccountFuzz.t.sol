// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { RampOSAccount } from "../../src/RampOSAccount.sol";
import { RampOSAccountFactory } from "../../src/RampOSAccountFactory.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title MockFuzzTarget
 * @notice Mock contract for fuzz testing account execution
 */
contract MockFuzzTarget {
    uint256 public value;
    mapping(address => uint256) public balances;

    event ValueSet(uint256 newValue);
    event Deposited(address indexed from, uint256 amount);

    function setValue(uint256 _value) external {
        value = _value;
        emit ValueSet(_value);
    }

    function deposit() external payable {
        balances[msg.sender] += msg.value;
        emit Deposited(msg.sender, msg.value);
    }

    function withdraw(uint256 amount) external {
        require(balances[msg.sender] >= amount, "Insufficient balance");
        balances[msg.sender] -= amount;
        payable(msg.sender).transfer(amount);
    }

    function complexOperation(uint256 a, uint256 b, bytes calldata data) external pure returns (uint256) {
        if (data.length > 0) {
            return a + b + uint256(uint8(data[0]));
        }
        return a * b;
    }

    receive() external payable {}
}

/**
 * @title AccountFuzz
 * @notice Comprehensive fuzz tests for RampOSAccount
 * @dev Tests edge cases through randomized inputs for security audit compliance
 */
contract AccountFuzz is Test {
    RampOSAccountFactory factory;
    IEntryPoint entryPoint;
    address owner;
    uint256 ownerKey;
    MockFuzzTarget target;

    // Constants for bounds
    uint256 constant MAX_ETH = 1000 ether;
    uint256 constant MAX_BATCH = 32; // MAX_BATCH_SIZE in contract

    function setUp() public {
        // Use canonical ERC-4337 entry point address
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));

        (owner, ownerKey) = makeAddrAndKey("owner");
        factory = new RampOSAccountFactory(entryPoint);
        target = new MockFuzzTarget();
    }

    // ============ Account Creation Fuzz Tests ============

    /**
     * @notice Fuzz test: Account creation with random salts
     * @dev Ensures deterministic address generation for any salt value
     */
    function testFuzz_CreateAccountWithSalt(uint256 salt) public {
        address predicted = factory.getAddress(owner, salt);
        RampOSAccount account = factory.createAccount(owner, salt);

        assertEq(address(account), predicted, "Address should match prediction");
        assertEq(account.owner(), owner, "Owner should be set correctly");
    }

    /**
     * @notice Fuzz test: Multiple account creation with different owners
     * @dev Ensures unique addresses for different owner/salt combinations
     */
    function testFuzz_CreateAccountDifferentOwners(address owner1, address owner2, uint256 salt) public {
        vm.assume(owner1 != address(0) && owner2 != address(0));
        vm.assume(owner1 != owner2);

        RampOSAccount account1 = factory.createAccount(owner1, salt);
        RampOSAccount account2 = factory.createAccount(owner2, salt);

        assertTrue(address(account1) != address(account2), "Different owners should have different addresses");
    }

    /**
     * @notice Fuzz test: Account creation is idempotent
     * @dev Same owner/salt should always return same account
     */
    function testFuzz_CreateAccountIdempotent(uint256 salt) public {
        RampOSAccount account1 = factory.createAccount(owner, salt);
        RampOSAccount account2 = factory.createAccount(owner, salt);

        assertEq(address(account1), address(account2), "Same params should return same account");
    }

    // ============ Execute Fuzz Tests ============

    /**
     * @notice Fuzz test: Execute ETH transfer with random amounts
     * @dev Tests boundary conditions for ETH transfers
     */
    function testFuzz_ExecuteEthTransfer(uint256 amount, uint256 salt) public {
        vm.assume(amount > 0 && amount <= MAX_ETH);

        RampOSAccount account = factory.createAccount(owner, salt);
        vm.deal(address(account), amount);

        address recipient = makeAddr("recipient");

        vm.prank(owner);
        account.execute(recipient, amount, "");

        assertEq(recipient.balance, amount, "Recipient should receive exact amount");
        assertEq(address(account).balance, 0, "Account should be empty");
    }

    /**
     * @notice Fuzz test: Execute with zero value
     * @dev Ensures zero-value calls work correctly
     */
    function testFuzz_ExecuteZeroValue(uint256 salt, bytes calldata data) public {
        vm.assume(data.length >= 4 || data.length == 0);

        RampOSAccount account = factory.createAccount(owner, salt);

        // Zero value call to target
        vm.prank(owner);
        if (data.length >= 4) {
            // Skip if trying to call a function that doesn't exist
            vm.expectRevert();
            account.execute(address(target), 0, data);
        } else {
            account.execute(address(target), 0, "");
        }
    }

    /**
     * @notice Fuzz test: Execute contract call with random value
     * @dev Tests setValue with random parameters
     */
    function testFuzz_ExecuteContractCall(uint256 newValue, uint256 salt) public {
        RampOSAccount account = factory.createAccount(owner, salt);

        bytes memory data = abi.encodeWithSelector(MockFuzzTarget.setValue.selector, newValue);

        vm.prank(owner);
        account.execute(address(target), 0, data);

        assertEq(target.value(), newValue, "Target value should be updated");
    }

    /**
     * @notice Fuzz test: Execute with value to contract
     * @dev Tests payable function calls with random ETH amounts
     */
    function testFuzz_ExecutePayableCall(uint256 amount, uint256 salt) public {
        vm.assume(amount > 0 && amount <= MAX_ETH);

        RampOSAccount account = factory.createAccount(owner, salt);
        vm.deal(address(account), amount);

        bytes memory data = abi.encodeWithSelector(MockFuzzTarget.deposit.selector);

        vm.prank(owner);
        account.execute(address(target), amount, data);

        assertEq(target.balances(address(account)), amount, "Deposit should be recorded");
    }

    // ============ ExecuteBatch Fuzz Tests ============

    /**
     * @notice Fuzz test: Batch execution with random batch sizes
     * @dev Tests batch execution within valid size limits
     */
    function testFuzz_ExecuteBatch(uint8 batchSize, uint256 salt) public {
        vm.assume(batchSize > 0 && batchSize <= MAX_BATCH);

        RampOSAccount account = factory.createAccount(owner, salt);
        uint256 perRecipient = 0.1 ether;
        uint256 totalNeeded = uint256(batchSize) * perRecipient;
        vm.deal(address(account), totalNeeded);

        address[] memory dests = new address[](batchSize);
        uint256[] memory values = new uint256[](batchSize);
        bytes[] memory datas = new bytes[](batchSize);

        for (uint256 i = 0; i < batchSize; i++) {
            dests[i] = address(uint160(i + 1000)); // Simple deterministic addresses
            values[i] = perRecipient;
            datas[i] = "";
        }

        vm.prank(owner);
        account.executeBatch(dests, values, datas);

        for (uint256 i = 0; i < batchSize; i++) {
            assertEq(dests[i].balance, perRecipient, "Each recipient should receive funds");
        }
    }

    /**
     * @notice Fuzz test: Batch with random value distribution
     * @dev Tests batch execution with varying amounts per recipient
     */
    function testFuzz_ExecuteBatchRandomValues(uint256 seed, uint256 salt) public {
        // Bound seed to prevent overflow
        seed = bound(seed, 1, type(uint128).max);
        uint256 batchSize = (seed % 10) + 1; // 1-10 recipients

        RampOSAccount account = factory.createAccount(owner, salt);

        address[] memory dests = new address[](batchSize);
        uint256[] memory values = new uint256[](batchSize);
        bytes[] memory datas = new bytes[](batchSize);

        uint256 total = 0;
        for (uint256 i = 0; i < batchSize; i++) {
            // Use modular arithmetic to prevent overflow
            dests[i] = address(uint160(uint256(keccak256(abi.encode(seed, i)))));
            values[i] = ((seed + i) % 1 ether) + 1; // 1 wei to 1 ether
            datas[i] = "";
            total += values[i];
        }

        vm.deal(address(account), total);

        vm.prank(owner);
        account.executeBatch(dests, values, datas);

        for (uint256 i = 0; i < batchSize; i++) {
            assertEq(dests[i].balance, values[i], "Each recipient should receive their amount");
        }
    }

    /**
     * @notice Fuzz test: Batch size exceeds limit should revert
     * @dev Ensures batch size limit is enforced
     */
    function testFuzz_ExecuteBatchExceedsLimit(uint8 extraSize, uint256 salt) public {
        vm.assume(extraSize > 0 && extraSize < 100);
        uint256 batchSize = MAX_BATCH + uint256(extraSize);

        RampOSAccount account = factory.createAccount(owner, salt);

        address[] memory dests = new address[](batchSize);
        uint256[] memory values = new uint256[](batchSize);
        bytes[] memory datas = new bytes[](batchSize);

        for (uint256 i = 0; i < batchSize; i++) {
            dests[i] = address(uint160(i + 1000));
            values[i] = 0;
            datas[i] = "";
        }

        vm.prank(owner);
        vm.expectRevert("Batch size exceeds limit");
        account.executeBatch(dests, values, datas);
    }

    /**
     * @notice Fuzz test: Batch with mismatched array lengths should revert
     * @dev Ensures array length validation
     */
    function testFuzz_ExecuteBatchMismatchedArrays(uint8 destLen, uint8 valueLen, uint256 salt) public {
        vm.assume(destLen != valueLen);
        vm.assume(destLen > 0 && destLen <= MAX_BATCH);
        vm.assume(valueLen > 0 && valueLen <= MAX_BATCH);

        RampOSAccount account = factory.createAccount(owner, salt);

        address[] memory dests = new address[](destLen);
        uint256[] memory values = new uint256[](valueLen);
        bytes[] memory datas = new bytes[](destLen);

        vm.prank(owner);
        vm.expectRevert("Array length mismatch");
        account.executeBatch(dests, values, datas);
    }

    // ============ Session Key Fuzz Tests ============

    /**
     * @notice Fuzz test: Session key with random time bounds
     * @dev Tests session key validity across time ranges
     */
    function testFuzz_SessionKeyTimeBounds(
        uint48 validAfter,
        uint48 duration,
        uint256 salt
    ) public {
        vm.assume(duration > 0 && duration <= 365 days);
        vm.assume(validAfter < type(uint48).max - duration);
        uint48 validUntil = validAfter + duration;

        // Ensure we're testing within valid time range
        vm.assume(validUntil > block.timestamp);

        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, permissions);

        // Check validity at different times
        if (block.timestamp < validAfter) {
            assertFalse(account.isValidSessionKey(sessionKey), "Should be invalid before validAfter");
        } else if (block.timestamp <= validUntil) {
            assertTrue(account.isValidSessionKey(sessionKey), "Should be valid within range");
        }

        // Test after expiry
        vm.warp(validUntil + 1);
        assertFalse(account.isValidSessionKey(sessionKey), "Should be invalid after validUntil");
    }

    /**
     * @notice Fuzz test: Session key spending limits
     * @dev Tests spending limit enforcement with random limits
     */
    function testFuzz_SessionKeySpendingLimit(
        uint256 spendingLimit,
        uint256 dailyLimit,
        uint256 salt
    ) public {
        vm.assume(spendingLimit > 0 && spendingLimit <= MAX_ETH);
        vm.assume(dailyLimit >= spendingLimit && dailyLimit <= MAX_ETH * 10);

        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: spendingLimit,
            dailyLimit: dailyLimit
        });

        vm.prank(owner);
        account.addSessionKey(
            sessionKey,
            uint48(block.timestamp),
            uint48(block.timestamp + 1 hours),
            permissions
        );

        // Verify spending info
        (uint256 dailySpent, uint256 dailyRemaining, uint256 storedLimit) =
            account.getSessionKeySpendingInfo(sessionKey);

        assertEq(dailySpent, 0, "Daily spent should start at 0");
        assertEq(dailyRemaining, dailyLimit, "Daily remaining should equal limit");
        assertEq(storedLimit, spendingLimit, "Spending limit should be stored");
    }

    /**
     * @notice Fuzz test: Session key with random targets
     * @dev Tests target restriction with multiple random addresses
     */
    function testFuzz_SessionKeyTargetRestriction(
        address target1,
        address target2,
        address queryTarget,
        uint256 salt
    ) public {
        vm.assume(target1 != address(0) && target2 != address(0));

        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        address[] memory allowedTargets = new address[](2);
        allowedTargets[0] = target1;
        allowedTargets[1] = target2;
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(
            sessionKey,
            uint48(block.timestamp),
            uint48(block.timestamp + 1 hours),
            permissions
        );

        // Verify target restrictions
        assertTrue(account.isTargetAllowed(sessionKey, target1), "Target1 should be allowed");
        assertTrue(account.isTargetAllowed(sessionKey, target2), "Target2 should be allowed");

        if (queryTarget != target1 && queryTarget != target2) {
            assertFalse(account.isTargetAllowed(sessionKey, queryTarget), "Other targets should not be allowed");
        }
    }

    /**
     * @notice Fuzz test: Session key with random selectors
     * @dev Tests selector restriction with random function selectors
     */
    function testFuzz_SessionKeySelectorRestriction(
        bytes4 selector1,
        bytes4 selector2,
        bytes4 querySelector,
        uint256 salt
    ) public {
        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](2);
        allowedSelectors[0] = selector1;
        allowedSelectors[1] = selector2;

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(
            sessionKey,
            uint48(block.timestamp),
            uint48(block.timestamp + 1 hours),
            permissions
        );

        // Verify selector restrictions
        assertTrue(account.isSelectorAllowed(sessionKey, selector1), "Selector1 should be allowed");
        assertTrue(account.isSelectorAllowed(sessionKey, selector2), "Selector2 should be allowed");

        if (querySelector != selector1 && querySelector != selector2) {
            assertFalse(account.isSelectorAllowed(sessionKey, querySelector), "Other selectors should not be allowed");
        }
    }

    /**
     * @notice Fuzz test: Legacy session key with 7-day limit
     * @dev Ensures legacy keys are limited to 7 days max
     */
    function testFuzz_LegacySessionKeyDurationLimit(uint48 duration, uint256 salt) public {
        // Ensure duration exceeds 7 days but doesn't overflow
        duration = uint48(bound(duration, 7 days + 1, 30 days));

        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = validAfter + duration;

        vm.prank(owner);
        vm.expectRevert("Legacy key max 7 days");
        account.addSessionKeyLegacy(sessionKey, validAfter, validUntil, bytes32(0));
    }

    /**
     * @notice Fuzz test: Session key update permissions
     * @dev Tests permission updates with random values
     */
    function testFuzz_UpdateSessionKeyPermissions(
        uint256 newSpendingLimit,
        uint256 newDailyLimit,
        uint256 salt
    ) public {
        vm.assume(newSpendingLimit <= MAX_ETH);
        vm.assume(newDailyLimit <= MAX_ETH * 10);

        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        // Initial permissions
        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory initialPerms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0.1 ether,
            dailyLimit: 1 ether
        });

        vm.prank(owner);
        account.addSessionKey(
            sessionKey,
            uint48(block.timestamp),
            uint48(block.timestamp + 1 hours),
            initialPerms
        );

        // Update permissions
        RampOSAccount.SessionKeyPermissions memory newPerms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: newSpendingLimit,
            dailyLimit: newDailyLimit
        });

        vm.prank(owner);
        account.updateSessionKeyPermissions(sessionKey, newPerms);

        // Verify update
        RampOSAccount.SessionKeyPermissions memory storedPerms = account.getSessionKeyPermissions(sessionKey);
        assertEq(storedPerms.spendingLimit, newSpendingLimit, "Spending limit should be updated");
        assertEq(storedPerms.dailyLimit, newDailyLimit, "Daily limit should be updated");
    }

    // ============ Access Control Fuzz Tests ============

    /**
     * @notice Fuzz test: Non-owner cannot execute
     * @dev Ensures execute is protected
     */
    function testFuzz_RevertNonOwnerExecute(address attacker, uint256 salt) public {
        vm.assume(attacker != owner && attacker != address(entryPoint));

        RampOSAccount account = factory.createAccount(owner, salt);

        vm.prank(attacker);
        vm.expectRevert(RampOSAccount.NotOwnerOrEntryPoint.selector);
        account.execute(attacker, 0, "");
    }

    /**
     * @notice Fuzz test: Non-owner cannot add session key
     * @dev Ensures addSessionKey is protected
     */
    function testFuzz_RevertNonOwnerAddSessionKey(address attacker, uint256 salt) public {
        vm.assume(attacker != owner);

        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(attacker);
        vm.expectRevert(RampOSAccount.NotOwner.selector);
        account.addSessionKey(
            sessionKey,
            uint48(block.timestamp),
            uint48(block.timestamp + 1 hours),
            permissions
        );
    }

    /**
     * @notice Fuzz test: Non-owner cannot remove session key
     * @dev Ensures removeSessionKey is protected
     */
    function testFuzz_RevertNonOwnerRemoveSessionKey(address attacker, uint256 salt) public {
        vm.assume(attacker != owner);

        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        // Owner adds session key
        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(
            sessionKey,
            uint48(block.timestamp),
            uint48(block.timestamp + 1 hours),
            permissions
        );

        // Attacker tries to remove
        vm.prank(attacker);
        vm.expectRevert(RampOSAccount.NotOwner.selector);
        account.removeSessionKey(sessionKey);
    }

    // ============ Edge Case Tests ============

    /**
     * @notice Fuzz test: Receive ETH with random amounts
     * @dev Tests account can receive any amount of ETH
     */
    function testFuzz_ReceiveEth(uint256 amount, uint256 salt) public {
        vm.assume(amount > 0 && amount <= MAX_ETH);

        RampOSAccount account = factory.createAccount(owner, salt);
        address sender = makeAddr("sender");
        vm.deal(sender, amount);

        vm.prank(sender);
        (bool success,) = address(account).call{value: amount}("");

        assertTrue(success, "Account should receive ETH");
        assertEq(address(account).balance, amount, "Account balance should match");
    }

    /**
     * @notice Fuzz test: Invalid session key address (zero)
     * @dev Ensures zero address session key is rejected
     */
    function testFuzz_RevertZeroAddressSessionKey(uint256 salt) public {
        RampOSAccount account = factory.createAccount(owner, salt);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        vm.expectRevert("Invalid session key");
        account.addSessionKey(
            address(0),
            uint48(block.timestamp),
            uint48(block.timestamp + 1 hours),
            permissions
        );
    }

    /**
     * @notice Fuzz test: Session key already expired at creation
     * @dev Ensures session keys can't be created with past expiry
     */
    function testFuzz_RevertExpiredSessionKey(uint256 salt) public {
        RampOSAccount account = factory.createAccount(owner, salt);
        (address sessionKey,) = makeAddrAndKey("session");

        // Warp forward to ensure we have room for past timestamps
        vm.warp(block.timestamp + 1000);

        address[] memory allowedTargets = new address[](0);
        bytes4[] memory allowedSelectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory permissions = RampOSAccount.SessionKeyPermissions({
            allowedTargets: allowedTargets,
            allowedSelectors: allowedSelectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        vm.expectRevert("Session already expired");
        account.addSessionKey(
            sessionKey,
            uint48(block.timestamp - 100), // past
            uint48(block.timestamp - 50), // already expired
            permissions
        );
    }

    /**
     * @notice Fuzz test: Call to non-contract with data should revert
     * @dev Tests TargetNotContract error
     */
    function testFuzz_RevertCallToNonContract(address nonContract, uint256 salt) public {
        vm.assume(nonContract.code.length == 0);
        vm.assume(nonContract != address(0));

        RampOSAccount account = factory.createAccount(owner, salt);

        bytes memory data = abi.encodeWithSelector(bytes4(0x12345678));

        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(RampOSAccount.TargetNotContract.selector, nonContract));
        account.execute(nonContract, 0, data);
    }
}
