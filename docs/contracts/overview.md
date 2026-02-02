# RampOS Smart Contracts Overview

## Introduction to ERC-4337 Account Abstraction

RampOS implements **ERC-4337 Account Abstraction**, enabling smart contract wallets that provide enhanced user experience and security compared to traditional EOA (Externally Owned Accounts).

### What is Account Abstraction?

Account Abstraction allows users to interact with the blockchain using smart contract wallets instead of EOAs. Key benefits include:

- **Gasless Transactions**: Users can transact without holding ETH for gas
- **Batch Transactions**: Execute multiple operations in a single transaction
- **Session Keys**: Delegate limited signing authority to dApps
- **Social Recovery**: Recover accounts without seed phrases
- **Custom Validation**: Flexible signature schemes and multi-sig support

### ERC-4337 Architecture

```
+------------------+     +------------------+     +------------------+
|                  |     |                  |     |                  |
|   User/dApp      |---->|   Bundler        |---->|   EntryPoint     |
|                  |     |                  |     |   (singleton)    |
+------------------+     +------------------+     +--------+---------+
                                                          |
                         +--------------------------------+
                         |                                |
              +----------v---------+           +----------v---------+
              |                    |           |                    |
              |   Smart Account    |           |   Paymaster        |
              |   (RampOSAccount)  |           |   (RampOSPaymaster)|
              |                    |           |                    |
              +--------------------+           +--------------------+
```

## RampOS Contract Architecture

RampOS consists of three core smart contracts that work together to provide a complete account abstraction solution:

### 1. RampOSAccount

The main smart account contract implementing ERC-4337. Each user has their own instance.

```solidity
contract RampOSAccount is BaseAccount, Initializable, UUPSUpgradeable {
    address public owner;
    mapping(address => SessionKey) public sessionKeys;

    function execute(address dest, uint256 value, bytes calldata data) external;
    function executeBatch(address[] calldata dests, uint256[] calldata values, bytes[] calldata datas) external;
}
```

**Key Features:**
- Single owner ECDSA signatures
- Session key support with time-based validity
- Batch transaction execution
- UUPS upgradeable pattern

### 2. RampOSAccountFactory

Factory contract for deploying new smart accounts using the EIP-1167 minimal proxy pattern.

```solidity
contract RampOSAccountFactory {
    RampOSAccount public immutable accountImplementation;

    function createAccount(address owner, uint256 salt) external returns (RampOSAccount);
    function getAddress(address owner, uint256 salt) public view returns (address);
}
```

**Key Features:**
- Gas-efficient deployment via minimal proxies (clones)
- Deterministic address generation (CREATE2)
- Idempotent account creation

### 3. RampOSPaymaster

Verifying paymaster that sponsors gas fees for approved transactions.

```solidity
contract RampOSPaymaster is IPaymaster, Ownable {
    address public verifyingSigner;
    mapping(bytes32 => uint256) public tenantDailyLimit;

    function validatePaymasterUserOp(...) external returns (bytes memory context, uint256 validationData);
}
```

**Key Features:**
- Signature-based sponsorship verification
- Per-tenant spending limits
- User rate limiting
- Daily limit auto-reset

## Contract Relationships

```
                    +------------------------+
                    |     EntryPoint         |
                    |  (ERC-4337 Singleton)  |
                    +-----------+------------+
                                |
            +-------------------+-------------------+
            |                                       |
+-----------v------------+              +-----------v------------+
|  RampOSAccountFactory  |              |    RampOSPaymaster     |
|                        |              |                        |
| - Creates accounts     |              | - Sponsors gas fees    |
| - Deploys proxies      |              | - Verifies signatures  |
+-----------+------------+              | - Enforces limits      |
            |                           +------------------------+
            |
+-----------v------------+
|     RampOSAccount      |
|    (User's Wallet)     |
|                        |
| - Holds assets         |
| - Executes txs         |
| - Manages session keys |
+------------------------+
```

## Deployment Addresses

### Mainnet (Ethereum)

| Contract | Address |
|----------|---------|
| EntryPoint (v0.7) | `0x0000000071727De22E5E9d8BAf0edAc6f37da032` |
| RampOSAccountFactory | *To be deployed* |
| RampOSPaymaster | *To be deployed* |

### Testnet (Sepolia)

| Contract | Address |
|----------|---------|
| EntryPoint (v0.7) | `0x0000000071727De22E5E9d8BAf0edAc6f37da032` |
| RampOSAccountFactory | *To be deployed* |
| RampOSPaymaster | *To be deployed* |

### Supported Networks

RampOS contracts are designed to be deployed on any EVM-compatible chain with ERC-4337 EntryPoint support:

- Ethereum Mainnet
- Polygon
- Arbitrum
- Optimism
- Base
- Sepolia (testnet)

## Quick Start

### 1. Create a Smart Account

```solidity
// Get the factory
RampOSAccountFactory factory = RampOSAccountFactory(FACTORY_ADDRESS);

// Predict the account address
address accountAddress = factory.getAddress(ownerAddress, salt);

// Create the account (if not already deployed)
RampOSAccount account = factory.createAccount(ownerAddress, salt);
```

### 2. Execute a Transaction

```solidity
// Single transaction
account.execute(
    recipientAddress,
    1 ether,
    "" // empty data for simple ETH transfer
);

// Batch transactions
address[] memory dests = new address[](2);
uint256[] memory values = new uint256[](2);
bytes[] memory datas = new bytes[](2);

dests[0] = token1;
values[0] = 0;
datas[0] = abi.encodeCall(IERC20.transfer, (recipient, amount1));

dests[1] = token2;
values[1] = 0;
datas[1] = abi.encodeCall(IERC20.transfer, (recipient, amount2));

account.executeBatch(dests, values, datas);
```

### 3. Set Up a Session Key

```solidity
account.addSessionKey(
    sessionKeyAddress,
    uint48(block.timestamp),        // validAfter
    uint48(block.timestamp + 1 hours), // validUntil
    bytes32(0)                      // permissionsHash (reserved)
);
```

## Gas Optimization

RampOS contracts are optimized for gas efficiency:

| Operation | Estimated Gas |
|-----------|---------------|
| Account Creation (first time) | ~200,000 |
| Account Creation (existing) | ~2,600 |
| Single Execute | ~50,000 |
| Batch Execute (3 txs) | ~100,000 |
| Add Session Key | ~45,000 |

## Next Steps

- [Account Documentation](./account.md) - Detailed RampOSAccount features
- [Paymaster Documentation](./paymaster.md) - Gas sponsorship guide
- [Security Documentation](./security.md) - Security model and best practices
