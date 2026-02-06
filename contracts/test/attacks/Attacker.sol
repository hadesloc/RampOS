// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { RampOSAccount } from "../../src/RampOSAccount.sol";
import { RampOSPaymaster } from "../../src/RampOSPaymaster.sol";

/**
 * @title AccountReentrancyAttacker
 * @notice Tries to re-enter RampOSAccount.execute during a call
 */
contract AccountReentrancyAttacker {
    RampOSAccount public target;
    uint256 public attackCount;
    uint256 public maxAttacks = 1;
    bool public success;

    constructor(RampOSAccount _target) {
        target = _target;
    }

    receive() external payable {
        if (attackCount < maxAttacks) {
            attackCount++;
            // Try to re-enter execute
            try target.execute(address(this), 0, "") {
                success = true;
            } catch {
                success = false;
            }
        }
    }

    fallback() external payable {
        if (attackCount < maxAttacks) {
            attackCount++;
            // Try to re-enter execute
            try target.execute(address(this), 0, "") {
                success = true;
            } catch {
                success = false;
            }
        }
    }
}

/**
 * @title AccountBatchReentrancyAttacker
 * @notice Tries to re-enter RampOSAccount.executeBatch during a call
 */
contract AccountBatchReentrancyAttacker {
    RampOSAccount public target;
    uint256 public attackCount;
    uint256 public maxAttacks = 1;
    bool public success;

    constructor(RampOSAccount _target) {
        target = _target;
    }

    receive() external payable {
        _attack();
    }

    fallback() external payable {
        _attack();
    }

    function _attack() internal {
        if (attackCount < maxAttacks) {
            attackCount++;

            address[] memory dests = new address[](1);
            dests[0] = address(this);
            uint256[] memory values = new uint256[](1);
            values[0] = 0;
            bytes[] memory datas = new bytes[](1);
            datas[0] = "";

            // Try to re-enter executeBatch
            try target.executeBatch(dests, values, datas) {
                success = true;
            } catch {
                success = false;
            }
        }
    }
}

/**
 * @title PaymasterReentrancyAttacker
 * @notice Tries to re-enter RampOSPaymaster.executeWithdraw during withdrawal
 */
contract PaymasterReentrancyAttacker {
    RampOSPaymaster public target;
    uint256 public attackCount;
    uint256 public maxAttacks = 1;
    bool public success;

    constructor(RampOSPaymaster _target) {
        target = _target;
    }

    receive() external payable {
        if (attackCount < maxAttacks) {
            attackCount++;
            // Try to re-enter executeWithdraw
            try target.executeWithdraw() {
                success = true;
            } catch {
                success = false;
            }
        }
    }
}
