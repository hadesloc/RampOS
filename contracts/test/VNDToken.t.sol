// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "../src/VNDToken.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract VNDTokenTest is Test {
    VNDToken public implementation;
    VNDToken public token;
    ERC1967Proxy public proxy;

    address public admin = makeAddr("admin");
    address public minter = makeAddr("minter");
    address public upgrader = makeAddr("upgrader");
    address public user1 = makeAddr("user1");
    address public user2 = makeAddr("user2");
    address public attacker = makeAddr("attacker");

    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant UPGRADER_ROLE = keccak256("UPGRADER_ROLE");

    function setUp() public {
        // Deploy implementation
        implementation = new VNDToken();

        // Deploy proxy with initialize
        bytes memory initData = abi.encodeCall(VNDToken.initialize, (admin));
        proxy = new ERC1967Proxy(address(implementation), initData);
        token = VNDToken(address(proxy));

        // Grant minter role to dedicated minter
        vm.startPrank(admin);
        token.grantRole(MINTER_ROLE, minter);
        token.grantRole(UPGRADER_ROLE, upgrader);
        vm.stopPrank();
    }

    // ─── Basic ERC20 Tests ──────────────────────────────────────────────

    function test_Name() public view {
        assertEq(token.name(), "Vietnamese Dong");
    }

    function test_Symbol() public view {
        assertEq(token.symbol(), "VND");
    }

    function test_Decimals() public view {
        assertEq(token.decimals(), 0);
    }

    function test_MaxSupply() public view {
        assertEq(token.MAX_SUPPLY(), 100_000_000_000_000);
    }

    // ─── Initialize Tests ───────────────────────────────────────────────

    function test_InitializeGrantsRoles() public view {
        assertTrue(token.hasRole(token.DEFAULT_ADMIN_ROLE(), admin));
        assertTrue(token.hasRole(ADMIN_ROLE, admin));
        assertTrue(token.hasRole(MINTER_ROLE, admin));
        assertTrue(token.hasRole(UPGRADER_ROLE, admin));
    }

    function test_CannotInitializeTwice() public {
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        token.initialize(admin);
    }

    function test_CannotInitializeImplementation() public {
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        implementation.initialize(admin);
    }

    function test_InitializeRevertsZeroAddress() public {
        VNDToken newImpl = new VNDToken();
        bytes memory initData = abi.encodeCall(VNDToken.initialize, (address(0)));
        vm.expectRevert(VNDToken.ZeroAddress.selector);
        new ERC1967Proxy(address(newImpl), initData);
    }

    // ─── Minting Tests ──────────────────────────────────────────────────

    function test_MintByMinter() public {
        vm.prank(minter);
        token.mint(user1, 1000);
        assertEq(token.balanceOf(user1), 1000);
    }

    function test_MintByAdmin() public {
        vm.prank(admin);
        token.mint(user1, 500);
        assertEq(token.balanceOf(user1), 500);
    }

    function test_MintRevertsNonMinter() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.mint(user1, 1000);
    }

    function test_MintRevertsZeroAddress() public {
        vm.prank(minter);
        vm.expectRevert(VNDToken.ZeroAddress.selector);
        token.mint(address(0), 1000);
    }

    function test_MintRevertsZeroAmount() public {
        vm.prank(minter);
        vm.expectRevert(VNDToken.ZeroAmount.selector);
        token.mint(user1, 0);
    }

    function test_MintRevertsSupplyCapExceeded() public {
        vm.prank(minter);
        vm.expectRevert(VNDToken.SupplyCapExceeded.selector);
        token.mint(user1, 100_000_000_000_001);
    }

    function test_MintWithReference() public {
        vm.prank(minter);
        token.mintWithReference(user1, 1000, "REF-001");
        assertEq(token.balanceOf(user1), 1000);
    }

    function test_MintEmitsEvent() public {
        vm.prank(minter);
        vm.expectEmit(true, false, false, true);
        emit VNDToken.Mint(user1, 1000, "REF-001");
        token.mintWithReference(user1, 1000, "REF-001");
    }

    // ─── Burning Tests ──────────────────────────────────────────────────

    function test_BurnWithReference() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(user1);
        token.burnWithReference(500, "WITHDRAW-001");
        assertEq(token.balanceOf(user1), 500);
    }

    function test_BurnWithReferenceRevertsZeroAmount() public {
        vm.prank(user1);
        vm.expectRevert(VNDToken.ZeroAmount.selector);
        token.burnWithReference(0, "REF");
    }

    function test_BurnFromWithReference() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(user1);
        token.approve(user2, 500);

        vm.prank(user2);
        token.burnFromWithReference(user1, 500, "WITHDRAW-002");
        assertEq(token.balanceOf(user1), 500);
    }

    // ─── Pausable Tests (F14.01) ────────────────────────────────────────

    function test_PauseByAdmin() public {
        vm.prank(admin);
        token.pause();
        assertTrue(token.paused());
    }

    function test_UnpauseByAdmin() public {
        vm.prank(admin);
        token.pause();
        vm.prank(admin);
        token.unpause();
        assertFalse(token.paused());
    }

    function test_PauseRevertsNonAdmin() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.pause();
    }

    function test_TransferBlockedWhenPaused() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(admin);
        token.pause();

        vm.prank(user1);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.transfer(user2, 100);
    }

    function test_MintBlockedWhenPaused() public {
        vm.prank(admin);
        token.pause();

        vm.prank(minter);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.mint(user1, 1000);
    }

    function test_BurnBlockedWhenPaused() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(admin);
        token.pause();

        vm.prank(user1);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.burnWithReference(500, "REF");
    }

    function test_TransferWorksAfterUnpause() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(admin);
        token.pause();
        vm.prank(admin);
        token.unpause();

        vm.prank(user1);
        token.transfer(user2, 100);
        assertEq(token.balanceOf(user2), 100);
    }

    // ─── Blacklist Tests (F14.02) ───────────────────────────────────────

    function test_BlacklistByAdmin() public {
        vm.prank(admin);
        token.blacklist(user1);
        assertTrue(token.isBlacklisted(user1));
    }

    function test_UnBlacklistByAdmin() public {
        vm.prank(admin);
        token.blacklist(user1);
        vm.prank(admin);
        token.unBlacklist(user1);
        assertFalse(token.isBlacklisted(user1));
    }

    function test_BlacklistRevertsNonAdmin() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.blacklist(user1);
    }

    function test_BlacklistRevertsZeroAddress() public {
        vm.prank(admin);
        vm.expectRevert(VNDToken.ZeroAddress.selector);
        token.blacklist(address(0));
    }

    function test_BlacklistedCannotSend() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(admin);
        token.blacklist(user1);

        vm.prank(user1);
        vm.expectRevert(abi.encodeWithSelector(VNDToken.AccountBlacklisted.selector, user1));
        token.transfer(user2, 100);
    }

    function test_CannotSendToBlacklisted() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(admin);
        token.blacklist(user2);

        vm.prank(user1);
        vm.expectRevert(abi.encodeWithSelector(VNDToken.AccountBlacklisted.selector, user2));
        token.transfer(user2, 100);
    }

    function test_CannotMintToBlacklisted() public {
        vm.prank(admin);
        token.blacklist(user1);

        vm.prank(minter);
        vm.expectRevert(abi.encodeWithSelector(VNDToken.AccountBlacklisted.selector, user1));
        token.mint(user1, 1000);
    }

    function test_BlacklistedCannotBurn() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(admin);
        token.blacklist(user1);

        vm.prank(user1);
        vm.expectRevert(abi.encodeWithSelector(VNDToken.AccountBlacklisted.selector, user1));
        token.burnWithReference(500, "REF");
    }

    function test_TransferWorksAfterUnBlacklist() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        vm.prank(admin);
        token.blacklist(user1);
        vm.prank(admin);
        token.unBlacklist(user1);

        vm.prank(user1);
        token.transfer(user2, 100);
        assertEq(token.balanceOf(user2), 100);
    }

    function test_BlacklistEmitsEvent() public {
        vm.prank(admin);
        vm.expectEmit(true, false, false, false);
        emit VNDToken.Blacklisted(user1);
        token.blacklist(user1);
    }

    function test_UnBlacklistEmitsEvent() public {
        vm.prank(admin);
        token.blacklist(user1);

        vm.prank(admin);
        vm.expectEmit(true, false, false, false);
        emit VNDToken.UnBlacklisted(user1);
        token.unBlacklist(user1);
    }

    // ─── AccessControl Tests (F14.04) ───────────────────────────────────

    function test_AdminCanGrantMinterRole() public {
        address newMinter = makeAddr("newMinter");
        vm.prank(admin);
        token.grantRole(MINTER_ROLE, newMinter);
        assertTrue(token.hasRole(MINTER_ROLE, newMinter));

        vm.prank(newMinter);
        token.mint(user1, 100);
        assertEq(token.balanceOf(user1), 100);
    }

    function test_AdminCanRevokeMinterRole() public {
        vm.prank(admin);
        token.revokeRole(MINTER_ROLE, minter);

        vm.prank(minter);
        vm.expectRevert();
        token.mint(user1, 100);
    }

    function test_NonAdminCannotGrantRoles() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.grantRole(MINTER_ROLE, attacker);
    }

    function test_SupportsInterface() public view {
        // IAccessControl interface
        assertTrue(token.supportsInterface(type(IAccessControl).interfaceId));
    }

    // ─── UUPS Upgrade Tests (F14.05) ────────────────────────────────────

    function test_UpgradeInterfaceVersion() public view {
        assertEq(token.UPGRADE_INTERFACE_VERSION(), "5.0.0");
    }

    function test_UpgradeRevertsNonUpgrader() public {
        VNDToken newImpl = new VNDToken();
        vm.prank(attacker);
        vm.expectRevert();
        token.upgradeToAndCall(address(newImpl), "");
    }

    function test_UpgradeByUpgrader() public {
        VNDToken newImpl = new VNDToken();

        // Mint before upgrade
        vm.prank(minter);
        token.mint(user1, 500);

        // Upgrade
        vm.prank(upgrader);
        token.upgradeToAndCall(address(newImpl), "");

        // State preserved after upgrade
        assertEq(token.balanceOf(user1), 500);
        assertTrue(token.hasRole(ADMIN_ROLE, admin));
        assertTrue(token.hasRole(MINTER_ROLE, minter));
    }

    // ─── Combined Scenario Tests ────────────────────────────────────────

    function test_PauseAndBlacklistCombined() public {
        vm.prank(minter);
        token.mint(user1, 1000);

        // Blacklist user1 and pause
        vm.startPrank(admin);
        token.blacklist(user1);
        token.pause();
        vm.stopPrank();

        // Both should block (pause checked first via modifier)
        vm.prank(user1);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.transfer(user2, 100);

        // Unpause but still blacklisted
        vm.prank(admin);
        token.unpause();

        vm.prank(user1);
        vm.expectRevert(abi.encodeWithSelector(VNDToken.AccountBlacklisted.selector, user1));
        token.transfer(user2, 100);

        // Unblacklist - now should work
        vm.prank(admin);
        token.unBlacklist(user1);

        vm.prank(user1);
        token.transfer(user2, 100);
        assertEq(token.balanceOf(user2), 100);
    }

    function test_FullLifecycle() public {
        // Mint with reference
        vm.prank(minter);
        token.mintWithReference(user1, 10000, "BANK-REF-001");
        assertEq(token.balanceOf(user1), 10000);

        // Transfer
        vm.prank(user1);
        token.transfer(user2, 3000);
        assertEq(token.balanceOf(user1), 7000);
        assertEq(token.balanceOf(user2), 3000);

        // Burn with reference (off-ramp)
        vm.prank(user1);
        token.burnWithReference(2000, "WITHDRAW-001");
        assertEq(token.balanceOf(user1), 5000);

        // Admin pauses
        vm.prank(admin);
        token.pause();

        // Transfers blocked
        vm.prank(user1);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.transfer(user2, 100);

        // Unpause
        vm.prank(admin);
        token.unpause();

        // Transfers resume
        vm.prank(user1);
        token.transfer(user2, 100);
        assertEq(token.balanceOf(user2), 3100);
    }
}
