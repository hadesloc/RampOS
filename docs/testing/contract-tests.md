# Smart Contract Testing Guide

This document covers testing Solidity smart contracts in RampOS using Foundry.

## Overview

RampOS uses [Foundry](https://book.getfoundry.sh/) for smart contract development and testing. The contracts implement ERC-4337 Account Abstraction for the AA wallet system.

## Contract Structure

```
contracts/
├── src/
│   ├── RampOSAccount.sol        # Smart account implementation
│   ├── RampOSAccountFactory.sol # Account factory (CREATE2)
│   └── RampOSPaymaster.sol      # Gas sponsorship paymaster
├── test/
│   ├── RampOSAccount.t.sol
│   ├── RampOSAccountFactory.t.sol
│   └── RampOSPaymaster.t.sol
├── lib/                          # Dependencies
├── script/                       # Deployment scripts
└── foundry.toml                  # Configuration
```

## Installation

### Install Foundry

```bash
# Install foundryup
curl -L https://foundry.paradigm.xyz | bash

# Install Foundry tools
foundryup

# Verify installation
forge --version
```

### Install Dependencies

```bash
cd contracts

# Install dependencies
forge install

# Update dependencies
forge update
```

## Running Tests

### Basic Commands

```bash
# Navigate to contracts directory
cd contracts

# Run all tests
forge test

# Run tests with verbosity
forge test -vvv

# Run tests with gas report
forge test --gas-report

# Run specific test file
forge test --match-path test/RampOSAccount.t.sol

# Run specific test function
forge test --match-test test_CreateAccount

# Run tests matching pattern
forge test --match-test "test_.*Session"

# Run tests excluding pattern
forge test --no-match-test "test_Fuzz"
```

### Verbosity Levels

```bash
# Level 1: Show test names only
forge test -v

# Level 2: Show logs
forge test -vv

# Level 3: Show stack traces
forge test -vvv

# Level 4: Show all traces
forge test -vvvv

# Level 5: Show all traces + setup
forge test -vvvvv
```

## Test Examples

### RampOSAccount Tests

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/RampOSAccount.sol";
import "../src/RampOSAccountFactory.sol";
import "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

contract RampOSAccountTest is Test {
    RampOSAccountFactory factory;
    IEntryPoint entryPoint;
    address owner;
    uint256 ownerKey;

    function setUp() public {
        // Use EntryPoint address (same across all chains)
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));

        // Create owner with known private key
        (owner, ownerKey) = makeAddrAndKey("owner");

        // Deploy factory
        factory = new RampOSAccountFactory(entryPoint);
    }

    function test_CreateAccount() public {
        uint256 salt = 12345;

        // Get predicted address
        address predicted = factory.getAddress(owner, salt);

        // Create account
        RampOSAccount account = factory.createAccount(owner, salt);

        // Verify
        assertEq(address(account), predicted);
        assertEq(account.owner(), owner);
    }

    function test_CreateAccountIdempotent() public {
        uint256 salt = 12345;

        // Create twice
        RampOSAccount account1 = factory.createAccount(owner, salt);
        RampOSAccount account2 = factory.createAccount(owner, salt);

        // Should return same address
        assertEq(address(account1), address(account2));
    }

    function test_Execute() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Fund the account
        vm.deal(address(account), 1 ether);

        // Create a recipient
        address recipient = makeAddr("recipient");

        // Execute transfer as owner
        vm.prank(owner);
        account.execute(recipient, 0.1 ether, "");

        // Verify
        assertEq(recipient.balance, 0.1 ether);
    }

    function test_ExecuteBatch() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Fund the account
        vm.deal(address(account), 1 ether);

        // Create recipients
        address[] memory dests = new address[](3);
        uint256[] memory values = new uint256[](3);
        bytes[] memory datas = new bytes[](3);

        for (uint256 i = 0; i < 3; i++) {
            dests[i] = makeAddr(string(abi.encodePacked("recipient", i)));
            values[i] = 0.1 ether;
            datas[i] = "";
        }

        // Execute batch as owner
        vm.prank(owner);
        account.executeBatch(dests, values, datas);

        // Verify
        for (uint256 i = 0; i < 3; i++) {
            assertEq(dests[i].balance, 0.1 ether);
        }
    }

    function test_SessionKey() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        // Create session key
        (address sessionKey, ) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Add session key
        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, bytes32(0));

        // Verify
        assertTrue(account.isValidSessionKey(sessionKey));

        // Remove session key
        vm.prank(owner);
        account.removeSessionKey(sessionKey);

        // Verify removed
        assertFalse(account.isValidSessionKey(sessionKey));
    }

    function test_RevertNonOwner() public {
        uint256 salt = 12345;
        RampOSAccount account = factory.createAccount(owner, salt);

        address attacker = makeAddr("attacker");

        // Try to execute as non-owner
        vm.prank(attacker);
        vm.expectRevert(RampOSAccount.NotOwnerOrEntryPoint.selector);
        account.execute(attacker, 0, "");
    }
}
```

### RampOSAccountFactory Tests

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/RampOSAccountFactory.sol";
import "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

contract RampOSAccountFactoryTest is Test {
    RampOSAccountFactory factory;
    IEntryPoint entryPoint;
    address owner;

    function setUp() public {
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));
        factory = new RampOSAccountFactory(entryPoint);
        owner = makeAddr("owner");
    }

    function test_CreateAccount() public {
        uint256 salt = 123;

        address predicted = factory.getAddress(owner, salt);

        // Expect event
        vm.expectEmit(true, true, true, true);
        emit RampOSAccountFactory.AccountCreated(predicted, owner, salt);

        RampOSAccount account = factory.createAccount(owner, salt);

        assertEq(address(account), predicted);
        assertEq(account.owner(), owner);
        assertEq(address(account.entryPoint()), address(entryPoint));
    }

    function test_CreateAccountDeterministic() public {
        uint256 salt = 456;

        address addr1 = factory.getAddress(owner, salt);
        RampOSAccount account1 = factory.createAccount(owner, salt);
        assertEq(address(account1), addr1);

        // Calling create again should return existing address
        RampOSAccount account2 = factory.createAccount(owner, salt);
        assertEq(address(account2), addr1);
    }
}
```

### RampOSPaymaster Tests

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/RampOSPaymaster.sol";
import "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract RampOSPaymasterTest is Test {
    using MessageHashUtils for bytes32;

    RampOSPaymaster paymaster;
    IEntryPoint entryPoint;
    address signer;
    uint256 signerKey;
    address owner;

    function setUp() public {
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));
        (signer, signerKey) = makeAddrAndKey("signer");
        owner = makeAddr("owner");

        vm.prank(owner);
        paymaster = new RampOSPaymaster(entryPoint, signer);
    }

    function test_ValidateUserOp() public {
        PackedUserOperation memory userOp;
        userOp.sender = makeAddr("sender");
        userOp.nonce = 0;

        bytes32 userOpHash = keccak256("userOp");

        bytes32 tenantId = keccak256("tenant1");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        // Construct signature
        bytes32 hash = keccak256(
            abi.encodePacked(
                userOpHash,
                tenantId,
                validUntil,
                validAfter
            )
        ).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        // Construct paymasterAndData
        bytes memory paymasterAndData = abi.encodePacked(
            address(paymaster),
            tenantId,
            validUntil,
            validAfter,
            signature
        );
        userOp.paymasterAndData = paymasterAndData;

        // Mock entry point call
        vm.prank(address(entryPoint));
        (bytes memory context, uint256 validationData) = paymaster.validatePaymasterUserOp(
            userOp,
            userOpHash,
            1e18 // maxCost
        );

        assertEq(validationData & 1, 0); // Success (sigFailed bit is 0)

        // Decode context
        (address sender, bytes32 tid, uint256 cost) = abi.decode(
            context, (address, bytes32, uint256)
        );
        assertEq(sender, userOp.sender);
        assertEq(tid, tenantId);
        assertEq(cost, 1e18);
    }

    function test_TenantLimit() public {
        bytes32 tenantId = keccak256("tenant1");

        vm.prank(owner);
        paymaster.setTenantLimit(tenantId, 1 ether);

        PackedUserOperation memory userOp;
        userOp.sender = makeAddr("sender");
        bytes32 userOpHash = keccak256("userOp");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        bytes32 hash = keccak256(
            abi.encodePacked(userOpHash, tenantId, validUntil, validAfter)
        ).toEthSignedMessageHash();
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster), tenantId, validUntil, validAfter, signature
        );

        // First op ok (0.5 eth)
        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.5 ether);

        // Second op ok (0.5 eth)
        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.5 ether);

        // Third op fails (> 1 eth total)
        vm.prank(address(entryPoint));
        vm.expectRevert(RampOSPaymaster.TenantLimitExceeded.selector);
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.1 ether);
    }
}
```

## Fuzz Testing

### Basic Fuzz Test

```solidity
function testFuzz_CreateAccount(uint256 salt) public {
    // Salt is randomly generated
    address predicted = factory.getAddress(owner, salt);
    RampOSAccount account = factory.createAccount(owner, salt);

    assertEq(address(account), predicted);
    assertEq(account.owner(), owner);
}
```

### Bounded Fuzz Test

```solidity
function testFuzz_ExecuteTransfer(uint256 amount) public {
    // Bound amount to valid range
    amount = bound(amount, 0, 1 ether);

    RampOSAccount account = factory.createAccount(owner, 12345);
    vm.deal(address(account), 1 ether);

    address recipient = makeAddr("recipient");

    vm.prank(owner);
    account.execute(recipient, amount, "");

    assertEq(recipient.balance, amount);
}
```

### Invariant Testing

```solidity
contract AccountInvariantTest is Test {
    RampOSAccountFactory factory;
    RampOSAccount account;
    address owner;

    function setUp() public {
        IEntryPoint entryPoint = IEntryPoint(
            address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)
        );
        factory = new RampOSAccountFactory(entryPoint);
        owner = makeAddr("owner");
        account = factory.createAccount(owner, 12345);
    }

    // Invariant: Owner should never change
    function invariant_ownerNeverChanges() public {
        assertEq(account.owner(), owner);
    }

    // Invariant: EntryPoint should never change
    function invariant_entryPointNeverChanges() public {
        assertEq(
            address(account.entryPoint()),
            address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)
        );
    }
}
```

## Test Coverage

### Generate Coverage Report

```bash
# Generate coverage report
forge coverage

# Generate coverage with detailed output
forge coverage --report lcov

# Generate HTML report (requires lcov)
forge coverage --report lcov
genhtml lcov.info -o coverage

# Ignore specific files in coverage
forge coverage --report lcov --skip test --skip script
```

### Coverage Output Example

```
| File                        | % Lines       | % Statements  | % Branches    | % Funcs       |
|-----------------------------|---------------|---------------|---------------|---------------|
| src/RampOSAccount.sol       | 95.00%        | 94.12%        | 87.50%        | 100.00%       |
| src/RampOSAccountFactory.sol| 100.00%       | 100.00%       | 100.00%       | 100.00%       |
| src/RampOSPaymaster.sol     | 88.24%        | 86.96%        | 75.00%        | 100.00%       |
```

## Gas Optimization

### Gas Report

```bash
# Run tests with gas report
forge test --gas-report

# Snapshot gas usage
forge snapshot

# Compare with previous snapshot
forge snapshot --check
```

### Gas Snapshot Example

```bash
# Create baseline
forge snapshot --snap .gas-snapshot

# After changes, compare
forge snapshot --diff .gas-snapshot
```

## Debugging

### Trace Execution

```bash
# Show call traces for failing tests
forge test -vvvv

# Debug specific test
forge test --match-test test_Execute -vvvvv
```

### Using console.log

```solidity
import "forge-std/console.sol";

function test_DebugExample() public {
    console.log("Owner address:", owner);
    console.log("Salt value:", 12345);

    RampOSAccount account = factory.createAccount(owner, 12345);
    console.log("Account address:", address(account));
}
```

## Fork Testing

### Test Against Mainnet Fork

```bash
# Fork mainnet
forge test --fork-url $MAINNET_RPC_URL

# Fork at specific block
forge test --fork-url $MAINNET_RPC_URL --fork-block-number 18000000
```

### Fork Test Example

```solidity
contract MainnetForkTest is Test {
    function setUp() public {
        // Fork is set via command line
    }

    function test_InteractWithDeployedContract() public {
        // Interact with real deployed contracts
        IEntryPoint entryPoint = IEntryPoint(
            0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789
        );

        // Test against real EntryPoint
        assertTrue(address(entryPoint).code.length > 0);
    }
}
```

## Configuration

### foundry.toml

```toml
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
optimizer = true
optimizer_runs = 200
solc = "0.8.24"

[profile.default.fmt]
line_length = 100
tab_width = 4
bracket_spacing = true

[rpc_endpoints]
mainnet = "${MAINNET_RPC_URL}"
polygon = "${POLYGON_RPC_URL}"
bnb = "${BNB_RPC_URL}"
sepolia = "${SEPOLIA_RPC_URL}"

[etherscan]
mainnet = { key = "${ETHERSCAN_API_KEY}" }
polygon = { key = "${POLYGONSCAN_API_KEY}" }
bnb = { key = "${BSCSCAN_API_KEY}" }
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Solidity Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive

      - name: Install Foundry
        uses: foundry-rs/foundry-toolchain@v1

      - name: Run Tests
        working-directory: contracts
        run: forge test -vvv

      - name: Check Coverage
        working-directory: contracts
        run: forge coverage

      - name: Check Gas Snapshot
        working-directory: contracts
        run: forge snapshot --check
```

## Best Practices

1. **Use setUp() for common setup**: Initialize contracts once per test file
2. **Use vm.prank() for caller impersonation**: Test access control
3. **Use vm.expectRevert() for error testing**: Verify reverts with specific errors
4. **Use vm.deal() to fund accounts**: Set ETH balances for testing
5. **Use makeAddr() and makeAddrAndKey()**: Create test addresses deterministically
6. **Bound fuzz inputs**: Use bound() to limit random values
7. **Test events**: Use vm.expectEmit() to verify events
8. **Write invariant tests**: Ensure core properties always hold
9. **Use snapshots for gas tracking**: Catch gas regressions
10. **Fork test against mainnet**: Verify integration with deployed contracts
