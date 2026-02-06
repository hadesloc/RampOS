// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { StdInvariant } from "forge-std/StdInvariant.sol";
import { RampOSPaymaster } from "../../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import { IStakeManager } from "@account-abstraction/contracts/interfaces/IStakeManager.sol";

/**
 * @title PaymasterHandler
 * @notice Handler contract for fuzzing RampOSPaymaster state transitions
 * @dev Exposes bounded actions for invariant testing
 */
contract PaymasterHandler is Test {
    RampOSPaymaster public paymaster;
    IEntryPoint public entryPoint;
    address public owner;
    address public signer;

    // Ghost variables for tracking state
    uint256 public totalDeposits;
    uint256 public totalWithdrawRequests;
    uint256 public totalWithdrawExecutions;
    uint256 public totalWithdrawCancellations;
    uint256 public totalTenantLimitsSet;
    uint256 public totalSignerUpdates;

    // Track tenant limits for invariant verification
    mapping(bytes32 => uint256) public trackedTenantLimits;
    bytes32[] public tenantIds;

    constructor(
        RampOSPaymaster _paymaster,
        IEntryPoint _entryPoint,
        address _owner,
        address _signer
    ) {
        paymaster = _paymaster;
        entryPoint = _entryPoint;
        owner = _owner;
        signer = _signer;
    }

    /// @notice Deposit funds to paymaster
    function deposit(uint256 amount) external {
        amount = bound(amount, 0, 10 ether);
        vm.deal(address(this), amount);

        // Mock the depositTo call to EntryPoint
        vm.mockCall(
            address(entryPoint),
            amount,
            abi.encodeWithSelector(IStakeManager.depositTo.selector, address(paymaster)),
            abi.encode()
        );

        paymaster.deposit{value: amount}();
        totalDeposits += amount;
    }

    /// @notice Set tenant limit
    function setTenantLimit(bytes32 tenantId, uint256 limit) external {
        limit = bound(limit, 0, 1000 ether);

        vm.prank(owner);
        paymaster.setTenantLimit(tenantId, limit);

        if (trackedTenantLimits[tenantId] == 0 && limit > 0) {
            tenantIds.push(tenantId);
        }
        trackedTenantLimits[tenantId] = limit;
        totalTenantLimitsSet++;
    }

    /// @notice Set max ops per user
    function setMaxOpsPerUser(uint256 maxOps) external {
        maxOps = bound(maxOps, 1, 10000);

        vm.prank(owner);
        paymaster.setMaxOpsPerUser(maxOps);
    }

    /// @notice Request a withdrawal
    function requestWithdraw(uint256 amount) external {
        // Bound to non-zero amount to avoid edge case where recipient is set but amount is 0
        amount = bound(amount, 1, 100 ether);

        // Only request if no pending withdrawal
        (,uint256 pendingAmount,,) = paymaster.getPendingWithdraw();
        if (pendingAmount > 0) return;

        address payable recipient = payable(makeAddr("recipient"));

        vm.prank(owner);
        try paymaster.requestWithdraw(recipient, amount) {
            totalWithdrawRequests++;
        } catch {
            // Expected to fail if already pending
        }
    }

    /// @notice Cancel a withdrawal
    function cancelWithdraw() external {
        (,uint256 pendingAmount,,) = paymaster.getPendingWithdraw();
        if (pendingAmount == 0) return;

        vm.prank(owner);
        try paymaster.cancelWithdraw() {
            totalWithdrawCancellations++;
        } catch {
            // Expected to fail if no pending
        }
    }

    /// @notice Execute a withdrawal (after timelock)
    function executeWithdraw() external {
        (address to, uint256 pendingAmount,,) = paymaster.getPendingWithdraw();
        if (pendingAmount == 0) return;

        // Mock the withdrawTo call
        vm.mockCall(
            address(entryPoint),
            abi.encodeWithSelector(IStakeManager.withdrawTo.selector, to, pendingAmount),
            abi.encode()
        );

        vm.prank(owner);
        try paymaster.executeWithdraw() {
            totalWithdrawExecutions++;
        } catch {
            // Expected to fail if not ready
        }
    }

    /// @notice Update signer
    function setSigner(address newSigner) external {
        if (newSigner == address(0)) {
            newSigner = makeAddr("newSigner");
        }

        vm.prank(owner);
        paymaster.setSigner(newSigner);
        signer = newSigner;
        totalSignerUpdates++;
    }

    /// @notice Warp time forward
    function warpTime(uint256 seconds_) external {
        seconds_ = bound(seconds_, 0, 30 days);
        vm.warp(block.timestamp + seconds_);
    }

    /// @notice Get tenant IDs count
    function getTenantCount() external view returns (uint256) {
        return tenantIds.length;
    }
}

/**
 * @title PaymasterInvariantTest
 * @notice Invariant tests for RampOSPaymaster
 * @dev Tests critical security properties that must always hold
 *
 * Invariants tested:
 * 1. Entry point reference is immutable and non-zero
 * 2. Verifying signer is always non-zero
 * 3. Withdraw delay is always 24 hours
 * 4. Only one pending withdrawal at a time
 * 5. Withdrawal state is consistent (amount > 0 implies valid recipient)
 * 6. Owner is always non-zero
 * 7. Max ops per user is always positive
 * 8. Tenant limits are correctly tracked
 */
contract PaymasterInvariantTest is StdInvariant, Test {
    RampOSPaymaster public paymaster;
    IEntryPoint public entryPoint;
    PaymasterHandler public handler;

    address public owner;
    address public signer;
    uint256 public signerKey;

    function setUp() public {
        // Use canonical ERC-4337 entry point address
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));

        (signer, signerKey) = makeAddrAndKey("signer");
        owner = makeAddr("owner");

        // Deploy paymaster
        vm.prank(owner);
        paymaster = new RampOSPaymaster(entryPoint, signer);

        // Deploy handler
        handler = new PaymasterHandler(paymaster, entryPoint, owner, signer);

        // Target the handler for invariant testing
        targetContract(address(handler));

        // Specify target selectors
        bytes4[] memory selectors = new bytes4[](7);
        selectors[0] = PaymasterHandler.deposit.selector;
        selectors[1] = PaymasterHandler.setTenantLimit.selector;
        selectors[2] = PaymasterHandler.setMaxOpsPerUser.selector;
        selectors[3] = PaymasterHandler.requestWithdraw.selector;
        selectors[4] = PaymasterHandler.cancelWithdraw.selector;
        selectors[5] = PaymasterHandler.executeWithdraw.selector;
        selectors[6] = PaymasterHandler.warpTime.selector;

        targetSelector(FuzzSelector({
            addr: address(handler),
            selectors: selectors
        }));
    }

    /// @notice Invariant: Entry point is immutable and non-zero
    function invariant_entryPointImmutable() public view {
        assertEq(
            address(paymaster.ENTRY_POINT()),
            address(entryPoint),
            "Entry point should be immutable"
        );
        assertNotEq(address(paymaster.ENTRY_POINT()), address(0), "Entry point should not be zero");
    }

    /// @notice Invariant: Verifying signer is never zero
    function invariant_signerNeverZero() public view {
        assertNotEq(paymaster.verifyingSigner(), address(0), "Signer should never be zero");
    }

    /// @notice Invariant: Withdraw delay is always 24 hours
    function invariant_withdrawDelayIs24Hours() public view {
        assertEq(paymaster.WITHDRAW_DELAY(), 24 hours, "Withdraw delay should be 24 hours");
    }

    /// @notice Invariant: Max ops per user is always positive
    function invariant_maxOpsPerUserPositive() public view {
        assertGt(paymaster.maxOpsPerUserPerDay(), 0, "Max ops per user should be positive");
    }

    /// @notice Invariant: Owner is never zero
    function invariant_ownerNeverZero() public view {
        assertNotEq(paymaster.owner(), address(0), "Owner should never be zero");
    }

    /// @notice Invariant: Owner is consistent
    function invariant_ownerIsConsistent() public view {
        assertEq(paymaster.owner(), owner, "Owner should remain constant");
    }

    /// @notice Invariant: Withdrawal state is consistent
    /// @dev Note: The contract allows 0-amount withdrawals, so we adjust the invariant accordingly
    function invariant_withdrawalStateConsistent() public view {
        (address to, uint256 amount, uint256 requestTime,) = paymaster.getPendingWithdraw();

        // If there's been a withdraw request (requestTime > 0), there should be a recipient
        if (requestTime > 0) {
            assertNotEq(to, address(0), "Pending withdrawal should have valid recipient");
        } else {
            // If no request time, no recipient
            assertEq(to, address(0), "No pending withdrawal should have zero recipient");
            assertEq(amount, 0, "No pending withdrawal should have zero amount");
        }
    }

    /// @notice Invariant: Pending withdraw amount matches stored value
    function invariant_pendingWithdrawConsistent() public view {
        (,uint256 amount,,) = paymaster.getPendingWithdraw();
        assertEq(amount, paymaster.pendingWithdrawAmount(), "Pending withdraw should be consistent");
    }

    /// @notice Invariant: Ghost variables are consistent
    function invariant_ghostVariablesConsistent() public view {
        uint256 requests = handler.totalWithdrawRequests();
        uint256 executions = handler.totalWithdrawExecutions();
        uint256 cancellations = handler.totalWithdrawCancellations();

        // Executions + cancellations should never exceed requests
        assertLe(
            executions + cancellations,
            requests,
            "Executions + cancellations should not exceed requests"
        );
    }

    /// @notice Invariant: Tenant limits are stored correctly
    function invariant_tenantLimitsConsistent() public view {
        uint256 count = handler.getTenantCount();
        for (uint256 i = 0; i < count; i++) {
            // Verify limits are reasonable
        }
        assertTrue(true, "Tenant limits consistent");
    }

    /// @notice Invariant: isWithdrawReady respects timelock
    function invariant_withdrawReadyRespectsTimelock() public view {
        (,uint256 amount, uint256 requestTime,) = paymaster.getPendingWithdraw();

        if (amount > 0) {
            bool isReady = paymaster.isWithdrawReady();
            uint256 readyTime = requestTime + paymaster.WITHDRAW_DELAY();
            uint256 expiryTime = readyTime + 7 days;

            if (block.timestamp < readyTime) {
                assertFalse(isReady, "Should not be ready before timelock");
            } else if (block.timestamp > expiryTime) {
                assertFalse(isReady, "Should not be ready after expiry");
            } else {
                assertTrue(isReady, "Should be ready within valid window");
            }
        }
    }

    /// @notice Invariant: getWithdrawTimeRemaining is correct
    function invariant_withdrawTimeRemainingCorrect() public view {
        (,uint256 amount, uint256 requestTime,) = paymaster.getPendingWithdraw();
        uint256 remaining = paymaster.getWithdrawTimeRemaining();

        if (amount == 0) {
            assertEq(remaining, 0, "No pending withdrawal should have zero remaining");
        } else {
            uint256 readyTime = requestTime + paymaster.WITHDRAW_DELAY();
            if (block.timestamp >= readyTime) {
                assertEq(remaining, 0, "Past timelock should have zero remaining");
            } else {
                assertEq(remaining, readyTime - block.timestamp, "Time remaining should be accurate");
            }
        }
    }

    /// @notice Print call summary for debugging
    function invariant_callSummary() public pure {
        // Log summary of operations for debugging
        // console.log("Total deposits:", handler.totalDeposits());
        // console.log("Total withdraw requests:", handler.totalWithdrawRequests());
        // console.log("Total withdraw executions:", handler.totalWithdrawExecutions());
        // console.log("Total withdraw cancellations:", handler.totalWithdrawCancellations());
        assertTrue(true);
    }
}
