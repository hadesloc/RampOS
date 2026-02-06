// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "../../src/eip7702/EIP7702Delegation.sol";
import "@account-abstraction/contracts/core/EntryPoint.sol";

contract EIP7702Test is Test {
    EIP7702Delegation implementation;
    EntryPoint entryPoint;
    address owner;
    address delegate;
    uint256 ownerKey;
    uint256 delegateKey;

    function setUp() public {
        (owner, ownerKey) = makeAddrAndKey("owner");
        (delegate, delegateKey) = makeAddrAndKey("delegate");
        entryPoint = new EntryPoint();
        // Deploy implementation normally
        implementation = new EIP7702Delegation(entryPoint);

        // Simulate EIP-7702: The owner EOA has the code of implementation
        // This effectively turns 'owner' into a contract with EIP7702Delegation logic
        vm.etch(owner, address(implementation).code);
    }

    function test_OwnerIsAddressThis() public {
        vm.prank(owner);
        // Owner calls itself to authorize a delegate
        EIP7702Delegation(payable(owner)).authorizeDelegate(delegate, 0, "");

        assertTrue(EIP7702Delegation(payable(owner)).isDelegate(delegate));
    }

    function test_DelegateExecution() public {
        vm.prank(owner);
        EIP7702Delegation(payable(owner)).authorizeDelegate(delegate, 0, "");

        address target = makeAddr("target");
        uint256 value = 1 ether;
        vm.deal(owner, 10 ether);

        bytes memory data = ""; // empty call

        vm.prank(delegate);
        EIP7702Delegation(payable(owner)).execute(target, value, data);

        assertEq(target.balance, 1 ether);
    }

    function test_RevertIfNotAuthorized() public {
        address target = makeAddr("target");
        vm.deal(owner, 10 ether);

        vm.prank(makeAddr("stranger"));
        vm.expectRevert("Not authorized");
        EIP7702Delegation(payable(owner)).execute(target, 1 ether, "");
    }

    function test_AuthorizeWithSignature() public {
        // Construct EIP-712 signature
        uint256 nonce = EIP7702Delegation(payable(owner)).nonces(owner);
        uint256 deadline = block.timestamp + 100;

        bytes32 typeHash = keccak256("Delegation(address delegate,uint256 nonce,uint256 deadline)");
        bytes32 structHash = keccak256(abi.encode(typeHash, delegate, nonce, deadline));

        // The implementation was deployed with name "EIP7702Delegation" and version "1.0"
        bytes32 domainSeparator = keccak256(
            abi.encode(
                keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
                keccak256(bytes("EIP7702Delegation")),
                keccak256(bytes("1.0")),
                block.chainid,
                owner // The verifying contract is the owner address because code is running there
            )
        );

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(ownerKey, digest);
        bytes memory signature = abi.encodePacked(r, s, v);

        // Execute authorization from a third party
        vm.prank(makeAddr("relayer"));
        EIP7702Delegation(payable(owner)).authorizeDelegate(delegate, deadline, signature);

        assertTrue(EIP7702Delegation(payable(owner)).isDelegate(delegate));
    }
}
