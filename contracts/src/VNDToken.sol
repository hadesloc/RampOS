// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts/proxy/utils/UUPSUpgradeable.sol";

/**
 * @title VNDToken
 * @notice Vietnamese Dong stablecoin for RampOS platform
 * @dev ERC20 token with mint/burn capabilities for on-ramp/off-ramp operations.
 *      Features: Pausable, Blacklist, AccessControl (RBAC), UUPS upgradeable.
 *
 * IMPORTANT: This token uses 0 decimals (not the standard 18).
 * This is intentional as Vietnamese Dong has no fractional units.
 * Integrators MUST account for this when handling token amounts.
 * Example: 1000000 VND = 1,000,000 tokens (not 1,000,000 * 10^18).
 *
 * Roles:
 *   - DEFAULT_ADMIN_ROLE: Can grant/revoke all roles
 *   - ADMIN_ROLE: Can pause/unpause and blacklist/unblacklist addresses
 *   - MINTER_ROLE: Can mint new tokens
 *   - UPGRADER_ROLE: Can authorize UUPS upgrades
 *
 * Deployment:
 *   1. Deploy implementation: `new VNDToken()`
 *   2. Deploy proxy: `new ERC1967Proxy(impl, abi.encodeCall(VNDToken.initialize, (admin)))`
 *   3. Interact via proxy address
 */
contract VNDToken is
    Initializable,
    ERC20,
    ERC20Burnable,
    ERC20Permit,
    AccessControl,
    Pausable,
    UUPSUpgradeable
{
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant UPGRADER_ROLE = keccak256("UPGRADER_ROLE");

    uint8 private constant _decimals = 0;

    /// @notice Maximum supply cap: 100 trillion VND tokens (0 decimals)
    uint256 public constant MAX_SUPPLY = 100_000_000_000_000;

    /// @notice Blacklisted addresses cannot send or receive tokens
    mapping(address => bool) private _blacklisted;

    /// @notice Events
    event Mint(address indexed to, uint256 amount, string referenceCode);
    event BurnWithReference(address indexed from, uint256 amount, string referenceCode);
    event Blacklisted(address indexed account);
    event UnBlacklisted(address indexed account);

    /// @notice Errors
    error ZeroAddress();
    error ZeroAmount();
    error SupplyCapExceeded();
    error AccountBlacklisted(address account);

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor()
        ERC20("Vietnamese Dong", "VND")
        ERC20Permit("Vietnamese Dong")
    {
        _disableInitializers();
    }

    /// @notice Initialize the contract (called once via proxy)
    /// @param admin Address that receives DEFAULT_ADMIN_ROLE, ADMIN_ROLE, MINTER_ROLE, UPGRADER_ROLE
    function initialize(address admin) external initializer {
        if (admin == address(0)) revert ZeroAddress();

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(ADMIN_ROLE, admin);
        _grantRole(MINTER_ROLE, admin);
        _grantRole(UPGRADER_ROLE, admin);
    }

    /// @notice Returns the number of decimals (0 for VND)
    function decimals() public pure override returns (uint8) {
        return _decimals;
    }

    /// @notice Returns the token name (hardcoded for proxy compatibility)
    function name() public pure override returns (string memory) {
        return "Vietnamese Dong";
    }

    /// @notice Returns the token symbol (hardcoded for proxy compatibility)
    function symbol() public pure override returns (string memory) {
        return "VND";
    }

    // ─── Pausable (F14.01) ──────────────────────────────────────────────

    /// @notice Pause all token transfers
    function pause() external onlyRole(ADMIN_ROLE) {
        _pause();
    }

    /// @notice Unpause all token transfers
    function unpause() external onlyRole(ADMIN_ROLE) {
        _unpause();
    }

    // ─── Blacklist (F14.02) ─────────────────────────────────────────────

    /// @notice Check if an address is blacklisted
    function isBlacklisted(address account) public view returns (bool) {
        return _blacklisted[account];
    }

    /// @notice Blacklist an address (prevents sending and receiving)
    function blacklist(address account) external onlyRole(ADMIN_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        _blacklisted[account] = true;
        emit Blacklisted(account);
    }

    /// @notice Remove an address from blacklist
    function unBlacklist(address account) external onlyRole(ADMIN_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        _blacklisted[account] = false;
        emit UnBlacklisted(account);
    }

    // ─── Transfer hook: Pausable + Blacklist ────────────────────────────

    /// @dev Override _update to enforce pause and blacklist checks on all transfers
    function _update(
        address from,
        address to,
        uint256 value
    ) internal override whenNotPaused {
        if (from != address(0) && _blacklisted[from]) {
            revert AccountBlacklisted(from);
        }
        if (to != address(0) && _blacklisted[to]) {
            revert AccountBlacklisted(to);
        }
        super._update(from, to, value);
    }

    // ─── Minting ────────────────────────────────────────────────────────

    /// @notice Mint new tokens
    /// @param to Recipient address
    /// @param amount Amount to mint
    function mint(address to, uint256 amount) external onlyRole(MINTER_ROLE) {
        if (to == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        if (totalSupply() + amount > MAX_SUPPLY) revert SupplyCapExceeded();
        _mint(to, amount);
        emit Mint(to, amount, "");
    }

    /// @notice Mint with reference code for tracking
    /// @param to Recipient address
    /// @param amount Amount to mint
    /// @param referenceCode Bank transfer reference code
    function mintWithReference(
        address to,
        uint256 amount,
        string calldata referenceCode
    ) external onlyRole(MINTER_ROLE) {
        if (to == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        if (totalSupply() + amount > MAX_SUPPLY) revert SupplyCapExceeded();
        _mint(to, amount);
        emit Mint(to, amount, referenceCode);
    }

    // ─── Burning ────────────────────────────────────────────────────────

    /// @notice Burn tokens with reference code (for off-ramp tracking)
    /// @param amount Amount to burn
    /// @param referenceCode Withdrawal reference code
    function burnWithReference(
        uint256 amount,
        string calldata referenceCode
    ) external {
        if (amount == 0) revert ZeroAmount();
        _burn(msg.sender, amount);
        emit BurnWithReference(msg.sender, amount, referenceCode);
    }

    /// @notice Burn tokens from a specific address (requires allowance)
    /// @param from Address to burn from
    /// @param amount Amount to burn
    /// @param referenceCode Withdrawal reference code
    function burnFromWithReference(
        address from,
        uint256 amount,
        string calldata referenceCode
    ) external {
        if (amount == 0) revert ZeroAmount();
        _spendAllowance(from, msg.sender, amount);
        _burn(from, amount);
        emit BurnWithReference(from, amount, referenceCode);
    }

    // ─── UUPS Upgrade (F14.05) ──────────────────────────────────────────

    /// @dev Authorize upgrade - only UPGRADER_ROLE
    function _authorizeUpgrade(address newImplementation)
        internal
        override
        onlyRole(UPGRADER_ROLE)
    {}

    // ─── Required overrides ─────────────────────────────────────────────

    /// @dev Required override for AccessControl + ERC165
    function supportsInterface(bytes4 interfaceId)
        public
        view
        override(AccessControl)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}
