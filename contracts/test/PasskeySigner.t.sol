// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test, console } from "forge-std/Test.sol";
import { PasskeySigner, P256Verifier } from "../src/passkey/PasskeySigner.sol";
import { PasskeyAccountFactory } from "../src/passkey/PasskeyAccountFactory.sol";
import { RampOSAccount } from "../src/RampOSAccount.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";

/**
 * @title PasskeySignerTest
 * @notice Comprehensive tests for PasskeySigner, P256Verifier, PasskeyAccountFactory,
 *         and the passkey integration in RampOSAccount.
 */
contract PasskeySignerTest is Test {
    PasskeySigner signer;
    PasskeyAccountFactory passkeyFactory;
    RampOSAccountFactory standardFactory;
    IEntryPoint entryPoint;

    address owner;
    uint256 ownerKey;
    address user;

    // Well-known P256 test public key coordinates
    // (deterministic test values within field prime range)
    uint256 constant TEST_PUB_KEY_X =
        0x6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296;
    uint256 constant TEST_PUB_KEY_Y =
        0x4FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5;

    // Alternative test public key
    uint256 constant ALT_PUB_KEY_X =
        0x7CF27B188D034F7E8A52380304B51AC3C90F894AAAA12F6D6F7C2E0A9D4B1234;
    uint256 constant ALT_PUB_KEY_Y =
        0x07775510DB8ED040293D9AC69F7430DBBA7DADE63CE982299E04B79D2C875678;

    function setUp() public {
        // Use a mock entry point
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));

        // Create owner
        (owner, ownerKey) = makeAddrAndKey("owner");
        user = makeAddr("user");

        // Deploy PasskeySigner
        signer = new PasskeySigner(owner);

        // Deploy factories
        passkeyFactory = new PasskeyAccountFactory(entryPoint);
        standardFactory = new RampOSAccountFactory(entryPoint);
    }

    // ============ PasskeySigner Tests ============

    function test_PasskeySigner_Constructor() public view {
        assertEq(signer.owner(), owner);
        assertEq(signer.isPasskeySet(), false);
        assertEq(signer.pubKeyX(), 0);
        assertEq(signer.pubKeyY(), 0);
    }

    function test_PasskeySigner_RegisterPasskey() public {
        bytes memory credentialId = hex"aabbccdd";

        vm.prank(owner);
        signer.registerPasskey(TEST_PUB_KEY_X, TEST_PUB_KEY_Y, credentialId);

        assertEq(signer.pubKeyX(), TEST_PUB_KEY_X);
        assertEq(signer.pubKeyY(), TEST_PUB_KEY_Y);
        assertTrue(signer.isPasskeySet());

        (uint256 x, uint256 y) = signer.getPublicKey();
        assertEq(x, TEST_PUB_KEY_X);
        assertEq(y, TEST_PUB_KEY_Y);
    }

    function test_PasskeySigner_RegisterPasskey_RevertNotOwner() public {
        vm.prank(user);
        vm.expectRevert(PasskeySigner.NotOwner.selector);
        signer.registerPasskey(TEST_PUB_KEY_X, TEST_PUB_KEY_Y, hex"aabb");
    }

    function test_PasskeySigner_RegisterPasskey_RevertInvalidKey() public {
        vm.prank(owner);
        vm.expectRevert(PasskeySigner.InvalidPublicKey.selector);
        signer.registerPasskey(0, TEST_PUB_KEY_Y, hex"aabb");

        vm.prank(owner);
        vm.expectRevert(PasskeySigner.InvalidPublicKey.selector);
        signer.registerPasskey(TEST_PUB_KEY_X, 0, hex"aabb");
    }

    function test_PasskeySigner_UpdatePasskey() public {
        // First register
        vm.prank(owner);
        signer.registerPasskey(TEST_PUB_KEY_X, TEST_PUB_KEY_Y, hex"aabb");

        // Then update
        vm.prank(owner);
        signer.updatePasskey(ALT_PUB_KEY_X, ALT_PUB_KEY_Y);

        assertEq(signer.pubKeyX(), ALT_PUB_KEY_X);
        assertEq(signer.pubKeyY(), ALT_PUB_KEY_Y);
    }

    function test_PasskeySigner_UpdatePasskey_RevertNotOwner() public {
        vm.prank(owner);
        signer.registerPasskey(TEST_PUB_KEY_X, TEST_PUB_KEY_Y, hex"aabb");

        vm.prank(user);
        vm.expectRevert(PasskeySigner.NotOwner.selector);
        signer.updatePasskey(ALT_PUB_KEY_X, ALT_PUB_KEY_Y);
    }

    function test_PasskeySigner_ERC1271_RevertWithoutPasskey() public view {
        // Should return invalid magic value when no passkey is set
        bytes memory sig = new bytes(65);
        sig[0] = 0x00;
        bytes4 result = signer.isValidSignature(bytes32(0), sig);
        assertEq(result, bytes4(0xffffffff));
    }

    function test_PasskeySigner_TransferOwnership() public {
        address newOwner = makeAddr("newOwner");

        vm.prank(owner);
        signer.transferOwnership(newOwner);

        assertEq(signer.owner(), newOwner);
    }

    function test_PasskeySigner_TransferOwnership_RevertZeroAddress() public {
        vm.prank(owner);
        vm.expectRevert(PasskeySigner.ZeroAddress.selector);
        signer.transferOwnership(address(0));
    }

    function test_PasskeySigner_ConstructorRevertZeroAddress() public {
        vm.expectRevert(PasskeySigner.ZeroAddress.selector);
        new PasskeySigner(address(0));
    }

    // ============ PasskeyAccountFactory Tests ============

    function test_PasskeyFactory_CreateAccount() public {
        uint256 salt = 42;

        // Get predicted address
        address predicted = passkeyFactory.getAddress(TEST_PUB_KEY_X, TEST_PUB_KEY_Y, salt);

        // Create account
        RampOSAccount account = passkeyFactory.createAccount(
            owner, TEST_PUB_KEY_X, TEST_PUB_KEY_Y, salt
        );

        // Verify
        assertEq(address(account), predicted);
        assertEq(account.owner(), owner);
    }

    function test_PasskeyFactory_CreateAccountIdempotent() public {
        uint256 salt = 42;

        // Create twice
        RampOSAccount account1 = passkeyFactory.createAccount(
            owner, TEST_PUB_KEY_X, TEST_PUB_KEY_Y, salt
        );
        RampOSAccount account2 = passkeyFactory.createAccount(
            owner, TEST_PUB_KEY_X, TEST_PUB_KEY_Y, salt
        );

        // Should return same address
        assertEq(address(account1), address(account2));
    }

    function test_PasskeyFactory_DifferentKeysGetDifferentAddresses() public {
        uint256 salt = 42;

        address addr1 = passkeyFactory.getAddress(TEST_PUB_KEY_X, TEST_PUB_KEY_Y, salt);
        address addr2 = passkeyFactory.getAddress(ALT_PUB_KEY_X, ALT_PUB_KEY_Y, salt);

        assertTrue(addr1 != addr2, "Different keys should produce different addresses");
    }

    function test_PasskeyFactory_DifferentSaltsGetDifferentAddresses() public {
        address addr1 = passkeyFactory.getAddress(TEST_PUB_KEY_X, TEST_PUB_KEY_Y, 1);
        address addr2 = passkeyFactory.getAddress(TEST_PUB_KEY_X, TEST_PUB_KEY_Y, 2);

        assertTrue(addr1 != addr2, "Different salts should produce different addresses");
    }

    function test_PasskeyFactory_RevertInvalidPublicKey() public {
        vm.expectRevert(PasskeyAccountFactory.InvalidPublicKey.selector);
        passkeyFactory.createAccount(owner, 0, TEST_PUB_KEY_Y, 42);

        vm.expectRevert(PasskeyAccountFactory.InvalidPublicKey.selector);
        passkeyFactory.createAccount(owner, TEST_PUB_KEY_X, 0, 42);
    }

    function test_PasskeyFactory_RevertInvalidOwner() public {
        vm.expectRevert(PasskeyAccountFactory.InvalidOwner.selector);
        passkeyFactory.createAccount(address(0), TEST_PUB_KEY_X, TEST_PUB_KEY_Y, 42);
    }

    // ============ RampOSAccount Passkey Integration Tests ============

    function test_Account_SetPasskeySigner() public {
        // Create account through standard factory
        RampOSAccount account = standardFactory.createAccount(owner, 123);

        // Set passkey signer (as owner)
        vm.prank(owner);
        account.setPasskeySigner(TEST_PUB_KEY_X, TEST_PUB_KEY_Y);

        // Verify
        (uint256 x, uint256 y, bool isSet) = account.getPasskeySigner();
        assertEq(x, TEST_PUB_KEY_X);
        assertEq(y, TEST_PUB_KEY_Y);
        assertTrue(isSet);
    }

    function test_Account_SetPasskeySigner_RevertNotOwner() public {
        RampOSAccount account = standardFactory.createAccount(owner, 123);

        vm.prank(user);
        vm.expectRevert();
        account.setPasskeySigner(TEST_PUB_KEY_X, TEST_PUB_KEY_Y);
    }

    function test_Account_SetPasskeySigner_RevertInvalidKey() public {
        RampOSAccount account = standardFactory.createAccount(owner, 123);

        vm.prank(owner);
        vm.expectRevert();
        account.setPasskeySigner(0, TEST_PUB_KEY_Y);
    }

    function test_Account_RemovePasskeySigner() public {
        RampOSAccount account = standardFactory.createAccount(owner, 123);

        // Set passkey
        vm.prank(owner);
        account.setPasskeySigner(TEST_PUB_KEY_X, TEST_PUB_KEY_Y);

        // Remove passkey
        vm.prank(owner);
        account.removePasskeySigner();

        (uint256 x, uint256 y, bool isSet) = account.getPasskeySigner();
        assertEq(x, 0);
        assertEq(y, 0);
        assertFalse(isSet);
    }

    function test_Account_PasskeySignerState() public {
        RampOSAccount account = standardFactory.createAccount(owner, 123);

        // Initially no passkey
        (, , bool isSet) = account.getPasskeySigner();
        assertFalse(isSet);

        // Set passkey
        vm.prank(owner);
        account.setPasskeySigner(TEST_PUB_KEY_X, TEST_PUB_KEY_Y);
        assertTrue(account.isPasskeySignerSet());

        // Update passkey
        vm.prank(owner);
        account.setPasskeySigner(ALT_PUB_KEY_X, ALT_PUB_KEY_Y);
        (uint256 x, uint256 y, ) = account.getPasskeySigner();
        assertEq(x, ALT_PUB_KEY_X);
        assertEq(y, ALT_PUB_KEY_Y);
    }

    // ============ P256Verifier Library Tests ============

    function test_P256Verifier_RejectZeroR() public view {
        bool result = P256Verifier.verify(
            bytes32(uint256(1)),
            0, // r = 0 should be rejected
            1,
            TEST_PUB_KEY_X,
            TEST_PUB_KEY_Y
        );
        assertFalse(result);
    }

    function test_P256Verifier_RejectZeroS() public view {
        bool result = P256Verifier.verify(
            bytes32(uint256(1)),
            1,
            0, // s = 0 should be rejected
            TEST_PUB_KEY_X,
            TEST_PUB_KEY_Y
        );
        assertFalse(result);
    }

    function test_P256Verifier_RejectZeroPubKey() public view {
        bool result = P256Verifier.verify(
            bytes32(uint256(1)),
            1,
            1,
            0, // pubKeyX = 0 should be rejected
            TEST_PUB_KEY_Y
        );
        assertFalse(result);
    }

    function test_P256Verifier_RejectOutOfRangeR() public view {
        // r >= n should be rejected
        bool result = P256Verifier.verify(
            bytes32(uint256(1)),
            P256Verifier.P256_N, // r = n (out of range)
            1,
            TEST_PUB_KEY_X,
            TEST_PUB_KEY_Y
        );
        assertFalse(result);
    }

    function test_P256Verifier_RejectHighS() public view {
        // s > n/2 should be rejected (malleability protection)
        uint256 highS = P256Verifier.P256_N / 2 + 1;
        bool result = P256Verifier.verify(
            bytes32(uint256(1)),
            1,
            highS,
            TEST_PUB_KEY_X,
            TEST_PUB_KEY_Y
        );
        assertFalse(result);
    }

    function test_P256Verifier_RejectPubKeyOutOfRange() public view {
        // pubKeyX >= p should be rejected
        bool result = P256Verifier.verify(
            bytes32(uint256(1)),
            1,
            1,
            P256Verifier.P256_P, // pubKeyX = p (out of range)
            TEST_PUB_KEY_Y
        );
        assertFalse(result);
    }
}
