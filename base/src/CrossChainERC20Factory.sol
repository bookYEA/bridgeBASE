// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Initializable} from "solady/utils/Initializable.sol";

import {CrossChainERC20} from "./CrossChainERC20.sol";

contract CrossChainERC20Factory is Initializable {
    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a CrossChainERC20 is deployed.
    ///
    /// @param localToken Address of the CrossChainERC20 deployment.
    /// @param remoteToken Address of the corresponding token on the remote chain.
    /// @param deployer Address of the account that deployed the token.
    event CrossChainERC20Created(address indexed localToken, bytes32 indexed remoteToken, address deployer);

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Address of the Bridge contract.
    address public bridge;

    /// @notice Mapping of the deployed CrossChainERC20 to the remote token address.
    ///         This is used to keep track of the token deployments.
    mapping(address localToken => bytes32 remoteToken) public deployments;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Bridge contract.
    constructor() {
        _disableInitializers();
    }

    /// @notice Semantic version.
    ///
    /// @custom:semver 1.0.1
    function version() external pure returns (string memory) {
        return "1.0.1";
    }

    /// @notice Initializer.
    ///
    /// @param bridge_ Address of the Bridge contract.
    function initialize(address bridge_) external initializer {
        bridge = bridge_;
    }

    /// @notice Deploys a CrossChainERC20 Beacon Proxy using CREATE3.
    ///
    /// @param remoteToken Address of the remote token.
    /// @param name Name of the CrossChainERC20.
    /// @param symbol Symbol of the CrossChainERC20.
    /// @param decimals Decimals of the CrossChainERC20.
    ///
    /// @return crossChainERC20 Address of the CrossChainERC20 deployment.
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

        emit CrossChainERC20Created({localToken: localToken, remoteToken: remoteToken, deployer: msg.sender});

        return localToken;
    }
}
