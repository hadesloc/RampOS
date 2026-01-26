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
        // address(20) + tenantId(32) + validUntil(6) + validAfter(6) + signature(65)
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
        (address sender, bytes32 tid, uint256 cost) = abi.decode(context, (address, bytes32, uint256));
        assertEq(sender, userOp.sender);
        assertEq(tid, tenantId);
        assertEq(cost, 1e18);
    }

    function test_TenantLimit() public {
        bytes32 tenantId = keccak256("tenant1");

        vm.prank(owner);
        paymaster.setTenantLimit(tenantId, 1 ether);

        // Test usage
        PackedUserOperation memory userOp;
        userOp.sender = makeAddr("sender");
        bytes32 userOpHash = keccak256("userOp");
         uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

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

        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster),
            tenantId,
            validUntil,
            validAfter,
            signature
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
