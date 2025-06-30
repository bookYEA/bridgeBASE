// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Script} from "forge-std/Script.sol";

import {ERC1967Factory} from "solady/utils/ERC1967Factory.sol";
import {ERC1967FactoryConstants} from "solady/utils/ERC1967FactoryConstants.sol";

import {Pubkey} from "../src/libraries/SVMLib.sol";

contract HelperConfig is Script {
    struct NetworkConfig {
        address initialOwner;
        Pubkey remoteBridge;
        address trustedRelayer;
        address erc1967Factory;
    }

    NetworkConfig private _activeNetworkConfig;

    constructor() {
        if (block.chainid == 84532) {
            _activeNetworkConfig = getBaseSepoliaConfig();
        } else {
            _activeNetworkConfig = getLocalConfig();
        }
    }

    function getConfig() public returns (NetworkConfig memory) {
        HelperConfig.NetworkConfig memory cfg = _activeNetworkConfig;

        vm.label(cfg.initialOwner, "INITIAL_OWNER");
        vm.label(cfg.erc1967Factory, "ERC1967_FACTORY");

        return cfg;
    }

    function getBaseSepoliaConfig() public pure returns (NetworkConfig memory) {
        return NetworkConfig({
            initialOwner: 0x0fe884546476dDd290eC46318785046ef68a0BA9, // Base Sepolia Proxy Admin
            remoteBridge: Pubkey.wrap(0x5547ad75815ba369e7fd8f9a8c37c0c5e1c6f930a68564449d619d21755551b9), // 6ju3gpXy6BvWECqiG41wedXsaanb5TyYzCnNzAZpDvtg
            trustedRelayer: 0x0e9a877906EBc3b7098DA2404412BF0Ed1A5EFb4,
            erc1967Factory: ERC1967FactoryConstants.ADDRESS
        });
    }

    function getLocalConfig() public returns (NetworkConfig memory) {
        if (_activeNetworkConfig.initialOwner != address(0)) {
            return _activeNetworkConfig;
        }

        ERC1967Factory f = new ERC1967Factory();

        return NetworkConfig({
            initialOwner: makeAddr("initialOwner"),
            remoteBridge: Pubkey.wrap(0xc4c16980efe2a570c1a7599fd2ebb40ca7f85daf897482b9c85d4b8933a61608),
            trustedRelayer: makeAddr("trustedRelayer"),
            erc1967Factory: address(f)
        });
    }
}
