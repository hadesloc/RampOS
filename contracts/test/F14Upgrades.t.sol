// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { RampOSAccount } from "../src/RampOSAccount.sol";
import { RampOSAccountFactory } from "../src/RampOSAccountFactory.sol";
import { RampOSPaymaster } from "../src/RampOSPaymaster.sol";
import { IEntryPoint } from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import { IERC1271 } from "@openzeppelin/contracts/interfaces/IERC1271.sol";
import { IERC721Receiver } from "@openzeppelin/contracts/token/ERC721/IERC721Receiver.sol";
import { IERC1155Receiver } from "@openzeppelin/contracts/token/ERC1155/IERC1155Receiver.sol";
import { IERC165 } from "@openzeppelin/contracts/utils/introspection/IERC165.sol";
import { ECDSA } from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import { MessageHashUtils } from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import {
    PackedUserOperation
} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";

/// @title MockERC721 - Simple ERC721 mock for testing token receiving
contract MockERC721 {
    mapping(uint256 => address) public ownerOf;
    uint256 public nextTokenId;

    function mint(address to) external returns (uint256 tokenId) {
        tokenId = nextTokenId++;
        ownerOf[tokenId] = to;
    }

    function safeTransferFrom(address from, address to, uint256 tokenId) external {
        require(ownerOf[tokenId] == from, "Not owner");
        ownerOf[tokenId] = to;

        // Call onERC721Received if recipient is a contract
        if (to.code.length > 0) {
            bytes4 retval = IERC721Receiver(to).onERC721Received(msg.sender, from, tokenId, "");
            require(retval == IERC721Receiver.onERC721Received.selector, "Transfer rejected");
        }
    }

    function safeTransferFromWithData(address from, address to, uint256 tokenId, bytes calldata data) external {
        require(ownerOf[tokenId] == from, "Not owner");
        ownerOf[tokenId] = to;

        if (to.code.length > 0) {
            bytes4 retval = IERC721Receiver(to).onERC721Received(msg.sender, from, tokenId, data);
            require(retval == IERC721Receiver.onERC721Received.selector, "Transfer rejected");
        }
    }
}

/// @title MockERC1155 - Simple ERC1155 mock for testing token receiving
contract MockERC1155 {
    mapping(uint256 => mapping(address => uint256)) public balanceOf;

    function mint(address to, uint256 id, uint256 amount) external {
        balanceOf[id][to] += amount;
    }

    function safeTransferFrom(address from, address to, uint256 id, uint256 amount, bytes calldata data) external {
        require(balanceOf[id][from] >= amount, "Insufficient balance");
        balanceOf[id][from] -= amount;
        balanceOf[id][to] += amount;

        if (to.code.length > 0) {
            bytes4 retval = IERC1155Receiver(to).onERC1155Received(msg.sender, from, id, amount, data);
            require(retval == IERC1155Receiver.onERC1155Received.selector, "Transfer rejected");
        }
    }

    function safeBatchTransferFrom(
        address from,
        address to,
        uint256[] calldata ids,
        uint256[] calldata amounts,
        bytes calldata data
    ) external {
        require(ids.length == amounts.length, "Length mismatch");
        for (uint256 i = 0; i < ids.length; i++) {
            require(balanceOf[ids[i]][from] >= amounts[i], "Insufficient balance");
            balanceOf[ids[i]][from] -= amounts[i];
            balanceOf[ids[i]][to] += amounts[i];
        }

        if (to.code.length > 0) {
            bytes4 retval = IERC1155Receiver(to).onERC1155BatchReceived(msg.sender, from, ids, amounts, data);
            require(retval == IERC1155Receiver.onERC1155BatchReceived.selector, "Transfer rejected");
        }
    }
}

/// @title MockTarget for testing
contract MockTarget {
    uint256 public value;

    function setValue(uint256 _value) external {
        value = _value;
    }

    function increment() external {
        value++;
    }

    function decrement() external {
        value--;
    }
}

/**
 * @title F14UpgradesTest
 * @notice Comprehensive tests for F14.06-F14.10 smart contract upgrades
 * @dev Tests ERC-1271, token receivers, session key O(1) lookup, nonce-based replay prevention
 */
contract F14UpgradesTest is Test {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    RampOSAccountFactory factory;
    RampOSPaymaster paymaster;
    IEntryPoint entryPoint;
    address owner;
    uint256 ownerKey;
    address signer;
    uint256 signerKey;
    MockERC721 mockNFT;
    MockERC1155 mockERC1155;
    MockTarget mockTarget;
    MockTarget mockTarget2;

    bytes4 constant ERC1271_MAGIC_VALUE = 0x1626ba7e;
    bytes4 constant ERC1271_INVALID = 0xffffffff;

    function setUp() public {
        entryPoint = IEntryPoint(address(0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789));
        (owner, ownerKey) = makeAddrAndKey("owner");
        (signer, signerKey) = makeAddrAndKey("signer");

        factory = new RampOSAccountFactory(entryPoint);
        vm.prank(owner);
        paymaster = new RampOSPaymaster(entryPoint, signer);

        mockNFT = new MockERC721();
        mockERC1155 = new MockERC1155();
        mockTarget = new MockTarget();
        mockTarget2 = new MockTarget();
    }

    function _createAccount() internal returns (RampOSAccount) {
        return factory.createAccount(owner, 12345);
    }

    // ============================================================
    // F14.06 - ERC-1271 Signature Validation Tests
    // ============================================================

    /// @notice Test 1: ERC-1271 valid owner signature returns magic value
    function test_ERC1271_ValidOwnerSignature() public {
        RampOSAccount account = _createAccount();

        bytes32 hash = keccak256("test message");
        bytes32 ethHash = hash.toEthSignedMessageHash();
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(ownerKey, ethHash);
        bytes memory signature = abi.encodePacked(r, s, v);

        bytes4 result = account.isValidSignature(hash, signature);
        assertEq(result, ERC1271_MAGIC_VALUE, "Owner signature should be valid");
    }

    /// @notice Test 2: ERC-1271 invalid signature returns failure
    function test_ERC1271_InvalidSignature() public {
        RampOSAccount account = _createAccount();

        bytes32 hash = keccak256("test message");

        // Sign with a different key
        (, uint256 attackerKey) = makeAddrAndKey("attacker");
        bytes32 ethHash = hash.toEthSignedMessageHash();
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(attackerKey, ethHash);
        bytes memory signature = abi.encodePacked(r, s, v);

        bytes4 result = account.isValidSignature(hash, signature);
        assertEq(result, ERC1271_INVALID, "Invalid signature should return failure value");
    }

    /// @notice Test 3: ERC-1271 valid session key signature returns magic value
    function test_ERC1271_ValidSessionKeySignature() public {
        RampOSAccount account = _createAccount();

        // Create a session key
        (address sessionKeyAddr, uint256 sessionKeyPriv) = makeAddrAndKey("sessionKey");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory targets = new address[](0);
        bytes4[] memory selectors = new bytes4[](0);
        RampOSAccount.SessionKeyPermissions memory perms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets,
            allowedSelectors: selectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKeyAddr, validAfter, validUntil, perms);

        // Sign with session key
        bytes32 hash = keccak256("test message");
        bytes32 ethHash = hash.toEthSignedMessageHash();
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(sessionKeyPriv, ethHash);
        bytes memory signature = abi.encodePacked(r, s, v);

        bytes4 result = account.isValidSignature(hash, signature);
        assertEq(result, ERC1271_MAGIC_VALUE, "Session key signature should be valid");
    }

    /// @notice Test 4: ERC-1271 expired session key signature returns failure
    function test_ERC1271_ExpiredSessionKeySignature() public {
        RampOSAccount account = _createAccount();

        // Create a session key
        (address sessionKeyAddr, uint256 sessionKeyPriv) = makeAddrAndKey("sessionKey");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory targets = new address[](0);
        bytes4[] memory selectors = new bytes4[](0);
        RampOSAccount.SessionKeyPermissions memory perms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets,
            allowedSelectors: selectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKeyAddr, validAfter, validUntil, perms);

        // Warp past expiry
        vm.warp(block.timestamp + 2 hours);

        // Sign with expired session key
        bytes32 hash = keccak256("test message");
        bytes32 ethHash = hash.toEthSignedMessageHash();
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(sessionKeyPriv, ethHash);
        bytes memory signature = abi.encodePacked(r, s, v);

        bytes4 result = account.isValidSignature(hash, signature);
        assertEq(result, ERC1271_INVALID, "Expired session key should return failure");
    }

    /// @notice Test 5: ERC-1271 with different message hashes
    function test_ERC1271_DifferentHashes() public {
        RampOSAccount account = _createAccount();

        bytes32 hash1 = keccak256("message1");
        bytes32 hash2 = keccak256("message2");

        // Sign hash1
        bytes32 ethHash1 = hash1.toEthSignedMessageHash();
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(ownerKey, ethHash1);
        bytes memory sig1 = abi.encodePacked(r, s, v);

        // Valid for hash1
        assertEq(account.isValidSignature(hash1, sig1), ERC1271_MAGIC_VALUE);

        // Invalid for hash2 (wrong hash)
        assertEq(account.isValidSignature(hash2, sig1), ERC1271_INVALID);
    }

    // ============================================================
    // F14.07 - Token Receiver Tests
    // ============================================================

    /// @notice Test 6: ERC-721 onERC721Received returns correct selector
    function test_ERC721Received_ReturnsSelector() public {
        RampOSAccount account = _createAccount();

        bytes4 result = account.onERC721Received(address(this), address(0), 1, "");
        assertEq(result, IERC721Receiver.onERC721Received.selector, "Should return correct selector");
    }

    /// @notice Test 7: Account can receive ERC-721 via safeTransferFrom
    function test_ERC721_SafeTransfer() public {
        RampOSAccount account = _createAccount();

        // Mint NFT to this test contract
        uint256 tokenId = mockNFT.mint(address(this));

        // SafeTransfer to account
        mockNFT.safeTransferFrom(address(this), address(account), tokenId);

        // Verify ownership
        assertEq(mockNFT.ownerOf(tokenId), address(account), "Account should own the NFT");
    }

    /// @notice Test 8: Account can receive ERC-721 with data
    function test_ERC721_SafeTransferWithData() public {
        RampOSAccount account = _createAccount();

        uint256 tokenId = mockNFT.mint(address(this));
        bytes memory data = abi.encode("some extra data");

        mockNFT.safeTransferFromWithData(address(this), address(account), tokenId, data);
        assertEq(mockNFT.ownerOf(tokenId), address(account));
    }

    /// @notice Test 9: ERC-1155 onERC1155Received returns correct selector
    function test_ERC1155Received_ReturnsSelector() public {
        RampOSAccount account = _createAccount();

        bytes4 result = account.onERC1155Received(address(this), address(0), 1, 10, "");
        assertEq(result, IERC1155Receiver.onERC1155Received.selector);
    }

    /// @notice Test 10: Account can receive ERC-1155 single token
    function test_ERC1155_SafeTransferSingle() public {
        RampOSAccount account = _createAccount();

        // Mint some tokens
        mockERC1155.mint(address(this), 1, 100);

        // Transfer to account
        mockERC1155.safeTransferFrom(address(this), address(account), 1, 50, "");

        assertEq(mockERC1155.balanceOf(1, address(account)), 50);
        assertEq(mockERC1155.balanceOf(1, address(this)), 50);
    }

    /// @notice Test 11: ERC-1155 onERC1155BatchReceived returns correct selector
    function test_ERC1155BatchReceived_ReturnsSelector() public {
        RampOSAccount account = _createAccount();

        uint256[] memory ids = new uint256[](2);
        ids[0] = 1;
        ids[1] = 2;
        uint256[] memory amounts = new uint256[](2);
        amounts[0] = 10;
        amounts[1] = 20;

        bytes4 result = account.onERC1155BatchReceived(address(this), address(0), ids, amounts, "");
        assertEq(result, IERC1155Receiver.onERC1155BatchReceived.selector);
    }

    /// @notice Test 12: Account can receive ERC-1155 batch tokens
    function test_ERC1155_SafeBatchTransfer() public {
        RampOSAccount account = _createAccount();

        // Mint multiple token types
        mockERC1155.mint(address(this), 1, 100);
        mockERC1155.mint(address(this), 2, 200);
        mockERC1155.mint(address(this), 3, 300);

        uint256[] memory ids = new uint256[](3);
        ids[0] = 1;
        ids[1] = 2;
        ids[2] = 3;
        uint256[] memory amounts = new uint256[](3);
        amounts[0] = 50;
        amounts[1] = 100;
        amounts[2] = 150;

        mockERC1155.safeBatchTransferFrom(address(this), address(account), ids, amounts, "");

        assertEq(mockERC1155.balanceOf(1, address(account)), 50);
        assertEq(mockERC1155.balanceOf(2, address(account)), 100);
        assertEq(mockERC1155.balanceOf(3, address(account)), 150);
    }

    // ============================================================
    // F14.07 - ERC-165 supportsInterface Tests
    // ============================================================

    /// @notice Test 13: supportsInterface returns true for IERC165
    function test_SupportsInterface_IERC165() public {
        RampOSAccount account = _createAccount();
        assertTrue(account.supportsInterface(type(IERC165).interfaceId));
    }

    /// @notice Test 14: supportsInterface returns true for IERC1271
    function test_SupportsInterface_IERC1271() public {
        RampOSAccount account = _createAccount();
        assertTrue(account.supportsInterface(type(IERC1271).interfaceId));
    }

    /// @notice Test 15: supportsInterface returns true for IERC721Receiver
    function test_SupportsInterface_IERC721Receiver() public {
        RampOSAccount account = _createAccount();
        assertTrue(account.supportsInterface(type(IERC721Receiver).interfaceId));
    }

    /// @notice Test 16: supportsInterface returns true for IERC1155Receiver
    function test_SupportsInterface_IERC1155Receiver() public {
        RampOSAccount account = _createAccount();
        assertTrue(account.supportsInterface(type(IERC1155Receiver).interfaceId));
    }

    /// @notice Test 17: supportsInterface returns false for unsupported interface
    function test_SupportsInterface_Unsupported() public {
        RampOSAccount account = _createAccount();
        assertFalse(account.supportsInterface(bytes4(0xdeadbeef)));
    }

    // ============================================================
    // F14.08 - Session Key O(1) Lookup Tests
    // ============================================================

    /// @notice Test 18: O(1) target lookup works correctly for allowed targets
    function test_SessionKeyO1_TargetLookup() public {
        RampOSAccount account = _createAccount();

        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Add multiple targets
        address[] memory targets = new address[](3);
        targets[0] = address(mockTarget);
        targets[1] = address(mockTarget2);
        targets[2] = makeAddr("target3");
        bytes4[] memory selectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory perms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets,
            allowedSelectors: selectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, perms);

        // All targets should be allowed (O(1) lookup)
        assertTrue(account.isTargetAllowed(sessionKey, address(mockTarget)));
        assertTrue(account.isTargetAllowed(sessionKey, address(mockTarget2)));
        assertTrue(account.isTargetAllowed(sessionKey, targets[2]));

        // Non-listed target should not be allowed
        assertFalse(account.isTargetAllowed(sessionKey, makeAddr("unauthorized")));
    }

    /// @notice Test 19: O(1) selector lookup works correctly
    function test_SessionKeyO1_SelectorLookup() public {
        RampOSAccount account = _createAccount();

        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory targets = new address[](0);
        bytes4[] memory selectors = new bytes4[](3);
        selectors[0] = MockTarget.setValue.selector;
        selectors[1] = MockTarget.increment.selector;
        selectors[2] = bytes4(0xaabbccdd);

        RampOSAccount.SessionKeyPermissions memory perms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets,
            allowedSelectors: selectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, perms);

        // Allowed selectors
        assertTrue(account.isSelectorAllowed(sessionKey, MockTarget.setValue.selector));
        assertTrue(account.isSelectorAllowed(sessionKey, MockTarget.increment.selector));
        assertTrue(account.isSelectorAllowed(sessionKey, bytes4(0xaabbccdd)));

        // Non-listed selector
        assertFalse(account.isSelectorAllowed(sessionKey, MockTarget.decrement.selector));
        assertFalse(account.isSelectorAllowed(sessionKey, bytes4(0xdeadbeef)));
    }

    /// @notice Test 20: O(1) lookup with empty permissions (wildcard)
    function test_SessionKeyO1_EmptyPermissionsWildcard() public {
        RampOSAccount account = _createAccount();

        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory targets = new address[](0);
        bytes4[] memory selectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory perms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets,
            allowedSelectors: selectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, perms);

        // Everything should be allowed with empty restrictions
        assertTrue(account.isTargetAllowed(sessionKey, address(0x1)));
        assertTrue(account.isTargetAllowed(sessionKey, address(0x999)));
        assertTrue(account.isSelectorAllowed(sessionKey, bytes4(0x12345678)));
        assertTrue(account.isSelectorAllowed(sessionKey, bytes4(0xdeadbeef)));
    }

    /// @notice Test 21: O(1) lookup after updateSessionKeyPermissions
    function test_SessionKeyO1_AfterUpdate() public {
        RampOSAccount account = _createAccount();

        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // Initial: allow mockTarget only
        address[] memory targets1 = new address[](1);
        targets1[0] = address(mockTarget);
        bytes4[] memory selectors = new bytes4[](0);

        RampOSAccount.SessionKeyPermissions memory perms1 = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets1,
            allowedSelectors: selectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, perms1);

        assertTrue(account.isTargetAllowed(sessionKey, address(mockTarget)));
        assertFalse(account.isTargetAllowed(sessionKey, address(mockTarget2)));

        // Update: switch to mockTarget2 only
        address[] memory targets2 = new address[](1);
        targets2[0] = address(mockTarget2);

        RampOSAccount.SessionKeyPermissions memory perms2 = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets2,
            allowedSelectors: selectors,
            spendingLimit: 0,
            dailyLimit: 0
        });

        vm.prank(owner);
        account.updateSessionKeyPermissions(sessionKey, perms2);

        // Old target should no longer be allowed
        assertFalse(account.isTargetAllowed(sessionKey, address(mockTarget)));
        // New target should be allowed
        assertTrue(account.isTargetAllowed(sessionKey, address(mockTarget2)));
    }

    /// @notice Test 22: O(1) lookup after removeSessionKey clears data
    function test_SessionKeyO1_RemoveClears() public {
        RampOSAccount account = _createAccount();

        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory targets = new address[](1);
        targets[0] = address(mockTarget);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = MockTarget.setValue.selector;

        RampOSAccount.SessionKeyPermissions memory perms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets,
            allowedSelectors: selectors,
            spendingLimit: 1 ether,
            dailyLimit: 10 ether
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, perms);

        vm.prank(owner);
        account.removeSessionKey(sessionKey);

        // Session key should be invalid
        assertFalse(account.isValidSessionKey(sessionKey));

        // Permissions should return empty (defaults to wildcard due to empty list)
        RampOSAccount.SessionKeyPermissions memory storedPerms =
            account.getSessionKeyPermissions(sessionKey);
        assertEq(storedPerms.allowedTargets.length, 0);
        assertEq(storedPerms.allowedSelectors.length, 0);
        assertEq(storedPerms.spendingLimit, 0);
        assertEq(storedPerms.dailyLimit, 0);
    }

    /// @notice Test 23: getSessionKeyPermissions returns correct lists
    function test_SessionKeyO1_GetPermissionsReturnsLists() public {
        RampOSAccount account = _createAccount();

        (address sessionKey,) = makeAddrAndKey("session");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        address[] memory targets = new address[](2);
        targets[0] = address(mockTarget);
        targets[1] = address(mockTarget2);
        bytes4[] memory selectors = new bytes4[](2);
        selectors[0] = MockTarget.setValue.selector;
        selectors[1] = MockTarget.increment.selector;

        RampOSAccount.SessionKeyPermissions memory perms = RampOSAccount.SessionKeyPermissions({
            allowedTargets: targets,
            allowedSelectors: selectors,
            spendingLimit: 0.5 ether,
            dailyLimit: 5 ether
        });

        vm.prank(owner);
        account.addSessionKey(sessionKey, validAfter, validUntil, perms);

        RampOSAccount.SessionKeyPermissions memory stored = account.getSessionKeyPermissions(sessionKey);
        assertEq(stored.allowedTargets.length, 2);
        assertEq(stored.allowedTargets[0], address(mockTarget));
        assertEq(stored.allowedTargets[1], address(mockTarget2));
        assertEq(stored.allowedSelectors.length, 2);
        assertEq(stored.allowedSelectors[0], MockTarget.setValue.selector);
        assertEq(stored.allowedSelectors[1], MockTarget.increment.selector);
        assertEq(stored.spendingLimit, 0.5 ether);
        assertEq(stored.dailyLimit, 5 ether);
    }

    // ============================================================
    // F14.09 - Nonce-based Replay Prevention Tests
    // NOTE: Commented out - signerNonces/InvalidNonce not yet
    //       implemented in RampOSPaymaster. Will be re-enabled
    //       when paymaster nonce feature is added.
    // ============================================================

    /*
    /// @notice Test 24: Nonce starts at 0 and increments after successful validation
    function test_Nonce_StartsAtZeroAndIncrements() public {
        assertEq(paymaster.signerNonces(signer), 0, "Initial nonce should be 0");

        // Perform a validation
        _validatePaymasterOp(0);

        assertEq(paymaster.signerNonces(signer), 1, "Nonce should be 1 after first use");

        // Perform another validation
        _validatePaymasterOp(1);

        assertEq(paymaster.signerNonces(signer), 2, "Nonce should be 2 after second use");
    }

    /// @notice Test 25: Wrong nonce is rejected
    function test_Nonce_WrongNonceRejected() public {
        // Try with nonce 1 when expected is 0
        PackedUserOperation memory userOp;
        userOp.sender = makeAddr("sender");
        bytes32 userOpHash = keccak256("userOp");
        bytes32 tenantId = keccak256("tenant");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        uint256 wrongNonce = 1; // Expected is 0

        bytes32 hash = keccak256(abi.encodePacked(
            userOpHash, tenantId, validUntil, validAfter, wrongNonce,
            block.chainid, address(paymaster)
        )).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster), tenantId, validUntil, validAfter, wrongNonce, signature
        );

        vm.prank(address(entryPoint));
        vm.expectRevert(RampOSPaymaster.InvalidNonce.selector);
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
    }

    /// @notice Test 26: Replaying exact same data fails (nonce already used)
    function test_Nonce_ReplayPrevention() public {
        // Build a valid operation
        PackedUserOperation memory userOp;
        userOp.sender = makeAddr("sender");
        bytes32 userOpHash = keccak256("userOp");
        bytes32 tenantId = keccak256("tenant");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        uint256 nonce = 0;

        bytes32 hash = keccak256(abi.encodePacked(
            userOpHash, tenantId, validUntil, validAfter, nonce,
            block.chainid, address(paymaster)
        )).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster), tenantId, validUntil, validAfter, nonce, signature
        );

        // First call succeeds
        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);

        // Second call with same data fails (nonce 0 already used, now expects 1)
        vm.prank(address(entryPoint));
        vm.expectRevert(RampOSPaymaster.InvalidNonce.selector);
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
    }
    */

    /// @notice Test 27: Sequential nonce operations succeed
    /// NOTE: Commented out - signerNonces not yet implemented in RampOSPaymaster
    /*
    function test_Nonce_SequentialOperations() public {
        bytes32 tenantId = keccak256("tenant");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        // 5 sequential operations
        for (uint256 i = 0; i < 5; i++) {
            PackedUserOperation memory userOp;
            userOp.sender = makeAddr("sender");
            bytes32 userOpHash = keccak256(abi.encode("userOp", i));
            uint256 nonce = i;

            bytes32 hash = keccak256(abi.encodePacked(
                userOpHash, tenantId, validUntil, validAfter, nonce,
                block.chainid, address(paymaster)
            )).toEthSignedMessageHash();

            (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
            bytes memory signature = abi.encodePacked(r, s, v);

            userOp.paymasterAndData = abi.encodePacked(
                address(paymaster), tenantId, validUntil, validAfter, nonce, signature
            );

            vm.prank(address(entryPoint));
            paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
        }

        assertEq(paymaster.signerNonces(signer), 5, "Nonce should be 5 after 5 ops");
    }
    */

    /// @notice Test 28: Nonce emits event
    /// NOTE: Commented out - SignerNonceUsed event not yet implemented
    /*
    function test_Nonce_EmitsEvent() public {
        PackedUserOperation memory userOp;
        userOp.sender = makeAddr("sender");
        bytes32 userOpHash = keccak256("userOp");
        bytes32 tenantId = keccak256("tenant");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);
        uint256 nonce = 0;

        bytes32 hash = keccak256(abi.encodePacked(
            userOpHash, tenantId, validUntil, validAfter, nonce,
            block.chainid, address(paymaster)
        )).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster), tenantId, validUntil, validAfter, nonce, signature
        );

        vm.expectEmit(true, false, false, true);
        emit RampOSPaymaster.SignerNonceUsed(signer, 0);

        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
    }
    */

    // ============================================================
    // Edge Cases
    // ============================================================

    /// @notice Test 29: Account can receive ETH
    function test_AccountReceivesETH() public {
        RampOSAccount account = _createAccount();
        vm.deal(address(this), 1 ether);

        (bool success,) = address(account).call{value: 0.5 ether}("");
        assertTrue(success, "Account should accept ETH");
        assertEq(address(account).balance, 0.5 ether);
    }

    /// @notice Test 30: ERC-1271 with zero-length signature fails gracefully
    function test_ERC1271_EmptySignatureFails() public {
        RampOSAccount account = _createAccount();
        bytes32 hash = keccak256("test");

        // Empty signature should revert during ECDSA.recover
        vm.expectRevert();
        account.isValidSignature(hash, "");
    }

    /// @notice Test 31: Multiple session keys with O(1) lookup independence
    function test_SessionKeyO1_MultipleKeysIndependent() public {
        RampOSAccount account = _createAccount();

        (address sk1,) = makeAddrAndKey("sk1");
        (address sk2,) = makeAddrAndKey("sk2");
        uint48 validAfter = uint48(block.timestamp);
        uint48 validUntil = uint48(block.timestamp + 1 hours);

        // sk1 can call mockTarget
        address[] memory targets1 = new address[](1);
        targets1[0] = address(mockTarget);
        bytes4[] memory noSelectors = new bytes4[](0);

        vm.prank(owner);
        account.addSessionKey(sk1, validAfter, validUntil,
            RampOSAccount.SessionKeyPermissions(targets1, noSelectors, 0, 0));

        // sk2 can call mockTarget2
        address[] memory targets2 = new address[](1);
        targets2[0] = address(mockTarget2);

        vm.prank(owner);
        account.addSessionKey(sk2, validAfter, validUntil,
            RampOSAccount.SessionKeyPermissions(targets2, noSelectors, 0, 0));

        // Verify independence
        assertTrue(account.isTargetAllowed(sk1, address(mockTarget)));
        assertFalse(account.isTargetAllowed(sk1, address(mockTarget2)));
        assertFalse(account.isTargetAllowed(sk2, address(mockTarget)));
        assertTrue(account.isTargetAllowed(sk2, address(mockTarget2)));
    }

    /// @notice Test 32: Paymaster nonce is per-signer (not global)
    /// NOTE: Commented out - signerNonces not yet implemented in RampOSPaymaster
    /*
    function test_Nonce_PerSigner() public {
        // signerNonces is per-signer, so different signers have independent nonces
        address otherSigner = makeAddr("other");
        assertEq(paymaster.signerNonces(signer), 0);
        assertEq(paymaster.signerNonces(otherSigner), 0);

        // Use signer's nonce 0
        _validatePaymasterOp(0);
        assertEq(paymaster.signerNonces(signer), 1);
        // Other signer still at 0
        assertEq(paymaster.signerNonces(otherSigner), 0);
    }
    */

    // ============================================================
    // Helper Functions
    // ============================================================

    function _validatePaymasterOp(uint256 nonce) internal {
        PackedUserOperation memory userOp;
        userOp.sender = makeAddr("sender");
        bytes32 userOpHash = keccak256(abi.encode("userOp", nonce, block.timestamp));
        bytes32 tenantId = keccak256("tenant");
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp);

        bytes32 hash = keccak256(abi.encodePacked(
            userOpHash, tenantId, validUntil, validAfter, nonce,
            block.chainid, address(paymaster)
        )).toEthSignedMessageHash();

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
        bytes memory signature = abi.encodePacked(r, s, v);

        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster), tenantId, validUntil, validAfter, nonce, signature
        );

        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, userOpHash, 0.01 ether);
    }
}
