// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../src/RampOSAccount.sol";
import "../src/RampOSAccountFactory.sol";
import "../src/RampOSPaymaster.sol";

contract DeployScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address entryPoint = vm.envAddress("ENTRY_POINT_ADDRESS");
        address paymasterSigner = vm.envAddress("PAYMASTER_SIGNER");

        vm.startBroadcast(deployerPrivateKey);

        // Deploy factory
        RampOSAccountFactory factory = new RampOSAccountFactory(
            IEntryPoint(entryPoint)
        );
        console.log("Factory deployed at:", address(factory));
        console.log("Account implementation:", address(factory.accountImplementation()));

        // Deploy paymaster
        RampOSPaymaster paymaster = new RampOSPaymaster(
            IEntryPoint(entryPoint),
            paymasterSigner
        );
        console.log("Paymaster deployed at:", address(paymaster));

        vm.stopBroadcast();
    }
}
