// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Test } from "forge-std/Test.sol";
import { ZkKycRegistry } from "../src/zk/ZkKycRegistry.sol";
import { ZkKycVerifier } from "../src/zk/ZkKycVerifier.sol";

/**
 * @title ZkKycTest
 * @notice Tests for ZkKycRegistry and ZkKycVerifier
 */
contract ZkKycTest is Test {
    ZkKycVerifier verifier;
    ZkKycRegistry registry;

    address admin;
    address verifierAddr;
    address user1;
    address user2;
    address unauthorized;

    bytes32 commitment1;
    bytes32 commitment2;
    bytes validProof;
    bytes invalidProof;
    bytes shortProof;

    function setUp() public {
        admin = makeAddr("admin");
        verifierAddr = makeAddr("verifier");
        user1 = makeAddr("user1");
        user2 = makeAddr("user2");
        unauthorized = makeAddr("unauthorized");

        commitment1 = keccak256(abi.encodePacked("user1-kyc-data", "salt1"));
        commitment2 = keccak256(abi.encodePacked("user2-kyc-data", "salt2"));

        // Valid proof: 32 bytes, non-zero
        validProof = abi.encodePacked(keccak256("valid-proof-data"));
        // Invalid proof: 32 bytes, all zeros
        invalidProof = new bytes(32);
        // Short proof: less than MIN_PROOF_LENGTH
        shortProof = hex"aabbccdd";

        // Deploy contracts
        verifier = new ZkKycVerifier();

        vm.prank(admin);
        registry = new ZkKycRegistry(admin, verifier);

        // Grant verifier role
        vm.prank(admin);
        registry.addVerifier(verifierAddr);
    }

    // ========================================
    // ZkKycVerifier Tests
    // ========================================

    function test_VerifierAcceptsValidProof() public {
        bool result = verifier.verifyProof(commitment1, validProof);
        assertTrue(result, "Valid proof should be accepted");
    }

    function test_VerifierRejectsAllZeroProof() public {
        bool result = verifier.verifyProof(commitment1, invalidProof);
        assertFalse(result, "All-zero proof should be rejected");
    }

    function test_VerifierRejectsEmptyCommitment() public {
        vm.expectRevert(ZkKycVerifier.EmptyCommitment.selector);
        verifier.verifyProof(bytes32(0), validProof);
    }

    function test_VerifierRejectsShortProof() public {
        vm.expectRevert(ZkKycVerifier.ProofTooShort.selector);
        verifier.verifyProof(commitment1, shortProof);
    }

    function test_VerifierViewFunction() public view {
        assertTrue(verifier.verifyProofView(commitment1, validProof));
        assertFalse(verifier.verifyProofView(commitment1, invalidProof));
        assertFalse(verifier.verifyProofView(bytes32(0), validProof));
        assertFalse(verifier.verifyProofView(commitment1, shortProof));
    }

    // ========================================
    // ZkKycRegistry: Registration Tests
    // ========================================

    function test_RegisterVerification() public {
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);

        assertTrue(registry.isVerified(user1, commitment1));
    }

    function test_RegisterVerificationEmitsEvent() public {
        vm.expectEmit(true, true, true, true);
        emit ZkKycRegistry.VerificationRegistered(user1, commitment1, verifierAddr);

        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);
    }

    function test_RegisterVerificationRevertsForUnauthorized() public {
        vm.prank(unauthorized);
        vm.expectRevert(ZkKycRegistry.NotVerifier.selector);
        registry.registerVerification(user1, commitment1);
    }

    function test_RegisterVerificationRevertsForZeroAddress() public {
        vm.prank(verifierAddr);
        vm.expectRevert(ZkKycRegistry.ZeroAddress.selector);
        registry.registerVerification(address(0), commitment1);
    }

    function test_RegisterVerificationRevertsForEmptyCommitment() public {
        vm.prank(verifierAddr);
        vm.expectRevert(ZkKycRegistry.EmptyCommitment.selector);
        registry.registerVerification(user1, bytes32(0));
    }

    function test_RegisterVerificationRevertsIfAlreadyVerified() public {
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);

        vm.prank(verifierAddr);
        vm.expectRevert(ZkKycRegistry.AlreadyVerified.selector);
        registry.registerVerification(user1, commitment1);
    }

    // ========================================
    // ZkKycRegistry: Verification Check Tests
    // ========================================

    function test_IsVerifiedReturnsFalseForUnknown() public view {
        assertFalse(registry.isVerified(user1, commitment1));
    }

    function test_HasAnyVerificationReturnsFalse() public view {
        assertFalse(registry.hasAnyVerification(user1));
    }

    function test_HasAnyVerificationReturnsTrue() public {
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);

        assertTrue(registry.hasAnyVerification(user1));
    }

    function test_GetVerificationCount() public {
        assertEq(registry.getVerificationCount(user1), 0);

        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);
        assertEq(registry.getVerificationCount(user1), 1);

        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment2);
        assertEq(registry.getVerificationCount(user1), 2);
    }

    // ========================================
    // ZkKycRegistry: Revocation Tests
    // ========================================

    function test_RevokeVerification() public {
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);
        assertTrue(registry.isVerified(user1, commitment1));

        vm.prank(admin);
        registry.revokeVerification(user1, commitment1);
        assertFalse(registry.isVerified(user1, commitment1));
    }

    function test_RevokeVerificationEmitsEvent() public {
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);

        vm.expectEmit(true, true, true, true);
        emit ZkKycRegistry.VerificationRevoked(user1, commitment1, admin);

        vm.prank(admin);
        registry.revokeVerification(user1, commitment1);
    }

    function test_RevokeVerificationOnlyAdmin() public {
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);

        vm.prank(unauthorized);
        vm.expectRevert(ZkKycRegistry.NotAdmin.selector);
        registry.revokeVerification(user1, commitment1);
    }

    function test_RevokeNonexistentVerification() public {
        vm.prank(admin);
        vm.expectRevert(ZkKycRegistry.NotVerifiedCommitment.selector);
        registry.revokeVerification(user1, commitment1);
    }

    function test_RevokeDecreasesCount() public {
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);

        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment2);
        assertEq(registry.getVerificationCount(user1), 2);

        vm.prank(admin);
        registry.revokeVerification(user1, commitment1);
        assertEq(registry.getVerificationCount(user1), 1);

        assertFalse(registry.hasAnyVerification(user1) && registry.isVerified(user1, commitment1));
        assertTrue(registry.isVerified(user1, commitment2));
    }

    // ========================================
    // ZkKycRegistry: Admin / Access Control Tests
    // ========================================

    function test_AddVerifier() public {
        address newVerifier = makeAddr("newVerifier");

        vm.prank(admin);
        registry.addVerifier(newVerifier);
        assertTrue(registry.verifiers(newVerifier));
    }

    function test_AddVerifierOnlyAdmin() public {
        vm.prank(unauthorized);
        vm.expectRevert(ZkKycRegistry.NotAdmin.selector);
        registry.addVerifier(makeAddr("x"));
    }

    function test_AddVerifierZeroAddress() public {
        vm.prank(admin);
        vm.expectRevert(ZkKycRegistry.ZeroAddress.selector);
        registry.addVerifier(address(0));
    }

    function test_AddVerifierAlreadyVerifier() public {
        vm.prank(admin);
        vm.expectRevert(ZkKycRegistry.AlreadyVerifier.selector);
        registry.addVerifier(verifierAddr); // Already added in setUp
    }

    function test_RemoveVerifier() public {
        vm.prank(admin);
        registry.removeVerifier(verifierAddr);
        assertFalse(registry.verifiers(verifierAddr));
    }

    function test_RemoveVerifierNotCurrent() public {
        vm.prank(admin);
        vm.expectRevert(ZkKycRegistry.NotCurrentVerifier.selector);
        registry.removeVerifier(makeAddr("notverifier"));
    }

    function test_TransferAdmin() public {
        address newAdmin = makeAddr("newAdmin");

        vm.prank(admin);
        registry.transferAdmin(newAdmin);

        assertEq(registry.admin(), newAdmin);

        // Old admin can no longer act
        vm.prank(admin);
        vm.expectRevert(ZkKycRegistry.NotAdmin.selector);
        registry.addVerifier(makeAddr("y"));
    }

    function test_TransferAdminZeroAddress() public {
        vm.prank(admin);
        vm.expectRevert(ZkKycRegistry.ZeroAddress.selector);
        registry.transferAdmin(address(0));
    }

    function test_ConstructorZeroAdmin() public {
        vm.expectRevert(ZkKycRegistry.ZeroAddress.selector);
        new ZkKycRegistry(address(0), verifier);
    }

    // ========================================
    // ZkKycRegistry: Register with Proof Tests
    // ========================================

    function test_RegisterWithProof() public {
        vm.prank(verifierAddr);
        registry.registerVerificationWithProof(user1, commitment1, validProof);

        assertTrue(registry.isVerified(user1, commitment1));
    }

    function test_RegisterWithInvalidProofFails() public {
        vm.prank(verifierAddr);
        vm.expectRevert("ZK proof verification failed");
        registry.registerVerificationWithProof(user1, commitment1, invalidProof);
    }

    // ========================================
    // ZkKycRegistry: Multi-user Isolation
    // ========================================

    function test_DifferentUsersIndependent() public {
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);

        assertTrue(registry.isVerified(user1, commitment1));
        assertFalse(registry.isVerified(user2, commitment1));
        assertFalse(registry.isVerified(user1, commitment2));
    }

    // ========================================
    // Full Flow Test
    // ========================================

    function test_FullFlowRegisterCheckRevoke() public {
        // 1. Register verification
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment1);
        assertTrue(registry.isVerified(user1, commitment1));
        assertTrue(registry.hasAnyVerification(user1));
        assertEq(registry.getVerificationCount(user1), 1);

        // 2. Register second commitment
        vm.prank(verifierAddr);
        registry.registerVerification(user1, commitment2);
        assertEq(registry.getVerificationCount(user1), 2);

        // 3. Revoke first
        vm.prank(admin);
        registry.revokeVerification(user1, commitment1);
        assertFalse(registry.isVerified(user1, commitment1));
        assertTrue(registry.isVerified(user1, commitment2));
        assertEq(registry.getVerificationCount(user1), 1);

        // 4. Revoke second
        vm.prank(admin);
        registry.revokeVerification(user1, commitment2);
        assertFalse(registry.hasAnyVerification(user1));
        assertEq(registry.getVerificationCount(user1), 0);
    }
}
