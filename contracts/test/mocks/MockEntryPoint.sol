// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract MockEntryPoint {
    mapping(address => uint256) public deposits;

    function depositTo(address account) external payable {
        deposits[account] += msg.value;
    }

    function withdrawTo(address payable withdrawAddress, uint256 withdrawAmount) external {
        // In real EP, it checks if msg.sender has enough deposit.
        // Here we just check deposit mapping.
        require(deposits[msg.sender] >= withdrawAmount, "MockEntryPoint: Insufficient deposit");
        deposits[msg.sender] -= withdrawAmount;

        (bool success, ) = withdrawAddress.call{value: withdrawAmount}("");
        require(success, "MockEntryPoint: Withdraw failed");
    }

    function balanceOf(address account) external view returns (uint256) {
        return deposits[account];
    }

    // Fallback to accept calls
    fallback() external payable {}
    receive() external payable {}
}
