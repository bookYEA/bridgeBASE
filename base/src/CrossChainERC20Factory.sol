// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {CREATE3} from "solady/utils/CREATE3.sol";
import {Initializable} from "solady/utils/Initializable.sol";
import {LibClone} from "solady/utils/LibClone.sol";

import {ISemver} from "optimism/packages/contracts-bedrock/interfaces/universal/ISemver.sol";
import {Predeploys} from "optimism/packages/contracts-bedrock/src/libraries/Predeploys.sol";

import {CrossChainERC20} from "./CrossChainERC20.sol";

contract CrossChainERC20Factory is ISemver {
    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Semantic version.
    /// @custom:semver 1.0.1
    string public constant version = "1.0.1";

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a CrossChainERC20 is deployed.
    ///
    /// @param localToken Address of the CrossChainERC20 deployment.
    /// @param remoteToken Address of the corresponding token on the remote chain.
    /// @param deployer Address of the account that deployed the token.
    event CrossChainERC20Created(address indexed localToken, address indexed remoteToken, address deployer);

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Mapping of the deployed CrossChainERC20 to the remote token address.
    ///         This is used to keep track of the token deployments.
    mapping(address localToken => address remoteToken) public deployments;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Deploys a CrossChainERC20 Beacon Proxy using CREATE3.
    ///
    /// @param remoteToken Address of the remote token.
    /// @param name Name of the CrossChainERC20.
    /// @param symbol Symbol of the CrossChainERC20.
    /// @param decimals Decimals of the CrossChainERC20.
    ///
    /// @return crossChainERC20 Address of the CrossChainERC20 deployment.
    function deploy(address remoteToken, string memory name, string memory symbol, uint8 decimals)
        external
        returns (address crossChainERC20)
    {
        bytes32 salt = keccak256(abi.encode(remoteToken, name, symbol, decimals));

        bytes memory initCode = LibClone.initCodeERC1967BeaconProxy(
            Predeploys.OPTIMISM_SUPERCHAIN_ERC20_BEACON,
            abi.encodeCall(CrossChainERC20.initialize, (remoteToken, name, symbol, decimals))
        );

        crossChainERC20 = CREATE3.deployDeterministic({salt: salt, initCode: initCode});
        deployments[crossChainERC20] = remoteToken;

        emit CrossChainERC20Created({localToken: crossChainERC20, remoteToken: remoteToken, deployer: msg.sender});
    }
}
