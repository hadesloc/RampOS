# RampOSAccountFactory
**Title:**
RampOSAccountFactory

**Author:**
RampOS Team

Factory for deploying RampOS smart accounts using minimal proxies

Uses EIP-1167 minimal proxy pattern for gas-efficient deployment.
Features:
- Deterministic address generation using CREATE2
- Gas-efficient deployment via minimal proxy clones
- Counterfactual address prediction before deployment
Security considerations:
- Account addresses are deterministic based on owner + salt
- Implementation is immutable and set at factory deployment


## State Variables
### ACCOUNT_IMPLEMENTATION
Account implementation contract (immutable for gas savings)


```solidity
RampOSAccount public immutable ACCOUNT_IMPLEMENTATION
```


### ENTRY_POINT
ERC-4337 Entry Point contract reference


```solidity
IEntryPoint public immutable ENTRY_POINT
```


## Functions
### constructor

Constructor - deploys the account implementation


```solidity
constructor(IEntryPoint _entryPoint) ;
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`_entryPoint`|`IEntryPoint`|The ERC-4337 EntryPoint contract address|


### createAccount

Create a new account or return existing one

Uses EIP-1167 minimal proxy for gas-efficient deployment


```solidity
function createAccount(address owner, uint256 salt) external returns (RampOSAccount account);
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`owner`|`address`|The owner of the account|
|`salt`|`uint256`|Salt for CREATE2 deterministic deployment|

**Returns**

|Name|Type|Description|
|----|----|-----------|
|`account`|`RampOSAccount`|The created or existing account instance|


### getAddress

Get the counterfactual address of an account before deployment

Useful for pre-computing addresses for gasless onboarding


```solidity
function getAddress(address owner, uint256 salt) public view returns (address);
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`owner`|`address`|The owner of the account|
|`salt`|`uint256`|Salt for CREATE2 deterministic deployment|

**Returns**

|Name|Type|Description|
|----|----|-----------|
|`<none>`|`address`|The predicted address of the account|


### _getSalt

Compute the combined salt for CREATE2


```solidity
function _getSalt(address owner, uint256 salt) internal pure returns (bytes32);
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`owner`|`address`|The owner address to include in salt|
|`salt`|`uint256`|The user-provided salt value|

**Returns**

|Name|Type|Description|
|----|----|-----------|
|`<none>`|`bytes32`|The combined salt hash|


## Events
### AccountCreated
Emitted when a new account is created


```solidity
event AccountCreated(address indexed account, address indexed owner, uint256 salt);
```

**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`account`|`address`|The deployed account address|
|`owner`|`address`|The owner of the account|
|`salt`|`uint256`|The salt used for deterministic deployment|

