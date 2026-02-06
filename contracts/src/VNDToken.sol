// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title VNDToken
 * @notice Vietnamese Dong stablecoin for RampOS platform
 * @dev ERC20 token with mint/burn capabilities for on-ramp/off-ramp operations
 *
 * IMPORTANT: This token uses 0 decimals (not the standard 18).
 * This is intentional as Vietnamese Dong has no fractional units.
 * Integrators MUST account for this when handling token amounts.
 * Example: 1000000 VND = 1,000,000 tokens (not 1,000,000 * 10^18).
 *
 * Flow:
 * 1. User deposits VND via bank transfer
 * 2. Backend receives bank webhook confirmation
 * 3. Backend calls mint() to credit user's wallet
 * 4. User can trade VND for other crypto
 * 5. When user withdraws to bank, burn() is called
 */
contract VNDToken is ERC20, ERC20Burnable, ERC20Permit, Ownable {
    /// @notice Decimals - VND typically has 0 decimals but we use 0 for simplicity
    uint8 private constant _decimals = 0;

    /// @notice Minter role - addresses that can mint new tokens
    mapping(address => bool) public minters;

    /// @notice Events
    event MinterAdded(address indexed minter);
    event MinterRemoved(address indexed minter);
    event Mint(address indexed to, uint256 amount, string referenceCode);
    event BurnWithReference(address indexed from, uint256 amount, string referenceCode);

    /// @notice Errors
    error NotMinter();
    error ZeroAddress();
    error ZeroAmount();

    modifier onlyMinter() {
        if (!minters[msg.sender] && msg.sender != owner()) {
            revert NotMinter();
        }
        _;
    }

    constructor(address initialOwner)
        ERC20("Vietnamese Dong", "VND")
        ERC20Permit("Vietnamese Dong")
        Ownable(initialOwner)
    {
        // Owner is automatically a minter
        minters[initialOwner] = true;
        emit MinterAdded(initialOwner);
    }

    /// @notice Returns the number of decimals (0 for VND)
    function decimals() public pure override returns (uint8) {
        return _decimals;
    }

    /// @notice Add a new minter (only owner)
    /// @param minter Address to add as minter
    function addMinter(address minter) external onlyOwner {
        if (minter == address(0)) revert ZeroAddress();
        minters[minter] = true;
        emit MinterAdded(minter);
    }

    /// @notice Remove a minter (only owner)
    /// @param minter Address to remove as minter
    function removeMinter(address minter) external onlyOwner {
        minters[minter] = false;
        emit MinterRemoved(minter);
    }

    /// @notice Mint new tokens (only minter)
    /// @dev Emits a Mint event with empty reference code for consistency with mintWithReference
    /// @param to Recipient address
    /// @param amount Amount to mint
    function mint(address to, uint256 amount) external onlyMinter {
        if (to == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
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
    ) external onlyMinter {
        if (to == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        _mint(to, amount);
        emit Mint(to, amount, referenceCode);
    }

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
}
