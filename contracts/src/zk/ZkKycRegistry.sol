// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { ZkKycVerifier } from "./ZkKycVerifier.sol";

/**
 * @title ZkKycRegistry
 * @notice On-chain registry of verified ZK-KYC commitments
 * @dev Stores commitment hashes for users who have proven KYC status
 *      via zero-knowledge proofs. No personal data is stored on-chain.
 *
 * Access control:
 * - ADMIN: Can grant/revoke verifier roles, revoke verifications
 * - VERIFIER: Can register new verifications (typically the backend service)
 *
 * Flow:
 * 1. User submits ZK proof to off-chain verifier service
 * 2. Verifier validates proof and calls registerVerification()
 * 3. dApps check isVerified() to gate access
 * 4. Admin can revokeVerification() if needed
 */
contract ZkKycRegistry {
    // ========================
    // State
    // ========================

    /// @notice Admin address (can manage verifiers and revoke)
    address public admin;

    /// @notice The ZK proof verifier contract
    ZkKycVerifier public verifier;

    /// @notice Authorized verifiers: address => bool
    mapping(address => bool) public verifiers;

    /// @notice Verified commitments: user => commitment => verified
    mapping(address => mapping(bytes32 => bool)) public verifiedCommitments;

    /// @notice Track all commitments per user for enumeration
    mapping(address => bytes32[]) private userCommitments;

    // ========================
    // Events
    // ========================

    /// @notice Emitted when a ZK-KYC verification is registered
    event VerificationRegistered(
        address indexed user,
        bytes32 indexed commitment,
        address indexed verifierAddress
    );

    /// @notice Emitted when a verification is revoked
    event VerificationRevoked(
        address indexed user,
        bytes32 indexed commitment,
        address indexed revokedBy
    );

    /// @notice Emitted when a verifier role is granted
    event VerifierAdded(address indexed verifierAddress);

    /// @notice Emitted when a verifier role is revoked
    event VerifierRemoved(address indexed verifierAddress);

    /// @notice Emitted when admin is transferred
    event AdminTransferred(address indexed previousAdmin, address indexed newAdmin);

    // ========================
    // Errors
    // ========================

    error NotAdmin();
    error NotVerifier();
    error ZeroAddress();
    error EmptyCommitment();
    error AlreadyVerified();
    error NotVerifiedCommitment();
    error AlreadyVerifier();
    error NotCurrentVerifier();

    // ========================
    // Modifiers
    // ========================

    modifier onlyAdmin() {
        if (msg.sender != admin) revert NotAdmin();
        _;
    }

    modifier onlyVerifier() {
        if (!verifiers[msg.sender]) revert NotVerifier();
        _;
    }

    // ========================
    // Constructor
    // ========================

    /**
     * @notice Initialize the registry with admin and verifier contract
     * @param _admin Admin address
     * @param _verifier ZkKycVerifier contract address
     */
    constructor(address _admin, ZkKycVerifier _verifier) {
        if (_admin == address(0)) revert ZeroAddress();
        admin = _admin;
        verifier = _verifier;
    }

    // ========================
    // Admin Functions
    // ========================

    /**
     * @notice Transfer admin role
     * @param newAdmin New admin address
     */
    function transferAdmin(address newAdmin) external onlyAdmin {
        if (newAdmin == address(0)) revert ZeroAddress();
        emit AdminTransferred(admin, newAdmin);
        admin = newAdmin;
    }

    /**
     * @notice Grant verifier role to an address
     * @param verifierAddress Address to grant verifier role
     */
    function addVerifier(address verifierAddress) external onlyAdmin {
        if (verifierAddress == address(0)) revert ZeroAddress();
        if (verifiers[verifierAddress]) revert AlreadyVerifier();
        verifiers[verifierAddress] = true;
        emit VerifierAdded(verifierAddress);
    }

    /**
     * @notice Revoke verifier role from an address
     * @param verifierAddress Address to revoke verifier role
     */
    function removeVerifier(address verifierAddress) external onlyAdmin {
        if (!verifiers[verifierAddress]) revert NotCurrentVerifier();
        verifiers[verifierAddress] = false;
        emit VerifierRemoved(verifierAddress);
    }

    /**
     * @notice Revoke a user's verification (admin only)
     * @param user User address
     * @param commitment Commitment hash to revoke
     */
    function revokeVerification(
        address user,
        bytes32 commitment
    ) external onlyAdmin {
        if (!verifiedCommitments[user][commitment]) revert NotVerifiedCommitment();
        verifiedCommitments[user][commitment] = false;
        emit VerificationRevoked(user, commitment, msg.sender);
    }

    // ========================
    // Verifier Functions
    // ========================

    /**
     * @notice Register a verified ZK-KYC commitment for a user
     * @param user User address whose KYC was verified
     * @param commitment The commitment hash from the ZK proof
     * @dev Only callable by authorized verifiers
     */
    function registerVerification(
        address user,
        bytes32 commitment
    ) external onlyVerifier {
        if (user == address(0)) revert ZeroAddress();
        if (commitment == bytes32(0)) revert EmptyCommitment();
        if (verifiedCommitments[user][commitment]) revert AlreadyVerified();

        verifiedCommitments[user][commitment] = true;
        userCommitments[user].push(commitment);

        emit VerificationRegistered(user, commitment, msg.sender);
    }

    /**
     * @notice Register verification with on-chain proof check
     * @param user User address
     * @param commitment Commitment hash
     * @param proof ZK proof data to verify on-chain
     * @dev Calls the ZkKycVerifier before registering
     */
    function registerVerificationWithProof(
        address user,
        bytes32 commitment,
        bytes calldata proof
    ) external onlyVerifier {
        if (user == address(0)) revert ZeroAddress();
        if (commitment == bytes32(0)) revert EmptyCommitment();
        if (verifiedCommitments[user][commitment]) revert AlreadyVerified();

        // Verify proof on-chain
        bool valid = verifier.verifyProof(commitment, proof);
        require(valid, "ZK proof verification failed");

        verifiedCommitments[user][commitment] = true;
        userCommitments[user].push(commitment);

        emit VerificationRegistered(user, commitment, msg.sender);
    }

    // ========================
    // View Functions
    // ========================

    /**
     * @notice Check if a user has a verified commitment
     * @param user User address
     * @param commitment Commitment hash to check
     * @return Whether the commitment is verified
     */
    function isVerified(
        address user,
        bytes32 commitment
    ) external view returns (bool) {
        return verifiedCommitments[user][commitment];
    }

    /**
     * @notice Check if a user has any verified commitment
     * @param user User address
     * @return Whether the user has at least one verified commitment
     */
    function hasAnyVerification(address user) external view returns (bool) {
        bytes32[] storage commitments = userCommitments[user];
        for (uint256 i = 0; i < commitments.length; i++) {
            if (verifiedCommitments[user][commitments[i]]) {
                return true;
            }
        }
        return false;
    }

    /**
     * @notice Get the count of verified commitments for a user
     * @param user User address
     * @return count Number of currently verified commitments
     */
    function getVerificationCount(address user) external view returns (uint256 count) {
        bytes32[] storage commitments = userCommitments[user];
        for (uint256 i = 0; i < commitments.length; i++) {
            if (verifiedCommitments[user][commitments[i]]) {
                count++;
            }
        }
    }
}
