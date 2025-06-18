// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Initializable} from "solady/utils/Initializable.sol";
import {LibClone} from "solady/utils/LibClone.sol";

import {CrossChainERC20} from "./CrossChainERC20.sol";

/// @title CrossChainERC20Factory
///
/// @notice Factory contract for deploying ERC-1967 beacon proxies of CrossChainERC20 tokens.
contract CrossChainERC20Factory is Initializable {
    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a CrossChainERC20 token is successfully deployed
    ///
    /// @dev The salt used for CREATE2 is derived from keccak256(abi.encode(remoteToken, name, symbol, decimals))
    ///
    /// @param localToken  Address of the newly deployed CrossChainERC20 contract on this chain
    /// @param remoteToken The 32-byte identifier of the corresponding token on the remote chain
    /// @param deployer    Address of the account that initiated the token deployment
    event CrossChainERC20Created(address indexed localToken, bytes32 indexed remoteToken, address deployer);

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Address of the CrossChainERC20 beacon proxy.
    address public immutable BEACON;

    /// @notice Address of the TokenBridge contract that will manage cross-chain token transfers
    address public immutable TOKEN_BRIDGE;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Mapping to track deployed CrossChainERC20 tokens and their remote counterparts
    ///
    /// @dev Maps local token address to the remote token identifier for verification and lookup.
    ///      Key: localToken - The address of the CrossChainERC20 deployed on this chain
    ///      Value: remoteToken - The 32-byte identifier of the token on the remote chain
    mapping(address localToken => bytes32 remoteToken) public deployments;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the CrossChainERC20Factory contract
    ///
    /// @dev Disables initializers to prevent the implementation contract from being initialized
    constructor(address beacon_, address tokenBridge_) {
        BEACON = beacon_;
        TOKEN_BRIDGE = tokenBridge_;

        _disableInitializers();
    }

    /// @notice Deploys a new CrossChainERC20 token with deterministic address using CREATE2
    ///
    /// @dev Uses CREATE2 with a salt derived from token parameters to ensure deterministic addresses.
    ///      The same parameters will always result in the same deployment address.
    ///      Emits CrossChainERC20Created event upon successful deployment.
    ///
    /// @param remoteToken The 32-byte identifier of the corresponding token on the remote chain
    /// @param name The human-readable name of the token (e.g., "My Token")
    /// @param symbol The symbol of the token (e.g., "MTK")
    /// @param decimals The number of decimal places the token uses
    ///
    /// @return crossChainERC20 The address of the newly deployed CrossChainERC20 contract
    function deploy(bytes32 remoteToken, string memory name, string memory symbol, uint8 decimals)
        external
        returns (address crossChainERC20)
    {
        bytes32 salt = keccak256(abi.encode(remoteToken, name, symbol, decimals));
        address localToken = LibClone.deployDeterministicERC1967BeaconProxy({beacon: BEACON, salt: salt});

        // Store the deployment mapping for future reference
        deployments[localToken] = remoteToken;

        emit CrossChainERC20Created({localToken: localToken, remoteToken: remoteToken, deployer: msg.sender});

        return localToken;
    }
}
