// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {ERC20} from "solady/tokens/ERC20.sol";
import {Initializable} from "solady/utils/Initializable.sol";

/// @title CrossChainERC20
contract CrossChainERC20 is ERC20 {
    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    address private immutable _bridge;
    bytes32 private immutable _remoteToken;
    uint8 private immutable _decimals;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    string private _name;
    string private _symbol;

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted whenever tokens are minted for an account.
    /// @param to Address of the account tokens are being minted for.
    /// @param amount  Amount of tokens minted.
    event Mint(address indexed to, uint256 amount);

    /// @notice Emitted whenever tokens are burned from an account.
    /// @param from Address of the account tokens are being burned from.
    /// @param amount  Amount of tokens burned.
    event Burn(address indexed from, uint256 amount);

    //////////////////////////////////////////////////////////////
    ///                       Modifiers                        ///
    //////////////////////////////////////////////////////////////

    /// @notice A modifier that only allows the Bridge to call.
    modifier onlyBridge() {
        require(msg.sender == _bridge, "CrossChainERC20: onlyBridge");
        _;
    }

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the OptimismSuperchainERC20 contract.
    ///
    /// @param bridge_ Address of the bridge contract.
    /// @param remoteToken_ Address of the corresponding remote token.
    /// @param name_ ERC20 name.
    /// @param symbol_ ERC20 symbol.
    /// @param decimals_ ERC20 decimals.
    constructor(address bridge_, bytes32 remoteToken_, string memory name_, string memory symbol_, uint8 decimals_) {
        _bridge = bridge_;
        _remoteToken = remoteToken_;
        _name = name_;
        _symbol = symbol_;
        _decimals = decimals_;
    }

    /// @notice Semantic version.
    /// @custom:semver 1.0.1
    function version() external pure returns (string memory) {
        return "1.0.1";
    }

    /// @dev Returns the bridge address.
    function bridge() public view returns (address) {
        return _bridge;
    }

    /// @dev Returns the remote token address.
    function remoteToken() public view returns (bytes32) {
        return _remoteToken;
    }

    /// @dev Returns the name of the token.
    function name() public view override returns (string memory) {
        return _name;
    }

    /// @dev Returns the symbol of the token.
    function symbol() public view override returns (string memory) {
        return _symbol;
    }

    /// @dev Returns the decimals places of the token.
    function decimals() public view override returns (uint8) {
        return _decimals;
    }

    /// @notice Allows the Bridge to mint tokens.
    /// @param to Address to mint tokens to.
    /// @param amount Amount of tokens to mint.
    function mint(address to, uint256 amount) external onlyBridge {
        require(to != address(0), "CrossChainERC20: mint to zero address");

        _mint(to, amount);
        emit Mint(to, amount);
    }

    /// @notice Allows the Bridge to burn tokens.
    /// @param from Address to burn tokens from.
    /// @param amount Amount of tokens to burn.
    function burn(address from, uint256 amount) external onlyBridge {
        require(from != address(0), "CrossChainERC20: burn from zero address");

        _burn(from, amount);
        emit Burn(from, amount);
    }
}
