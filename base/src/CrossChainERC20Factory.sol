// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Initializable} from "solady/utils/Initializable.sol";

import {CrossChainERC20} from "./CrossChainERC20.sol";

/// @title CrossChainERC20Factory
///
/// @notice Factory contract for deploying CrossChainERC20 tokens with deterministic addresses
///
/// @dev This factory creates CrossChainERC20 tokens that are bridgeable between chains.
///      Uses CREATE2 with salt for deterministic deployment addresses based on token parameters.
///      The factory must be initialized with a bridge address before tokens can be deployed.
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
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Address of the Bridge contract that will manage cross-chain token transfers
    ///
    /// @dev Set during initialization and used by all deployed CrossChainERC20 tokens
    address public bridge;

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
    constructor() {
        _disableInitializers();
    }

    /// @notice Returns the semantic version of this contract
    function version() external pure returns (string memory) {
        return "1.0.1";
    }

    /// @notice Initializes the factory with the bridge contract address
    ///
    /// @dev Can only be called once due to the initializer modifier. Sets the bridge address
    ///      that will be used by all deployed CrossChainERC20 tokens.
    ///
    /// @param bridge_ The address of the Bridge contract that will manage cross-chain transfers
    function initialize(address bridge_) external initializer {
        bridge = bridge_;
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
        address localToken = address(
            new CrossChainERC20{salt: salt}({
                bridge_: bridge,
                remoteToken_: remoteToken,
                name_: name,
                symbol_: symbol,
                decimals_: decimals
            })
        );

        // Store the deployment mapping for future reference
        deployments[localToken] = remoteToken;

        emit CrossChainERC20Created({localToken: localToken, remoteToken: remoteToken, deployer: msg.sender});

        return localToken;
    }
}
