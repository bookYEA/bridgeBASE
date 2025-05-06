// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {ICrossChainERC20} from "./interfaces/ICrossChainERC20.sol";
import {ERC20} from "solady/tokens/ERC20.sol";
import {Initializable} from "solady/utils/Initializable.sol";

/// @title CrossChainERC20
contract CrossChainERC20 is ERC20, Initializable, ICrossChainERC20 {
    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Storage slot that the OptimismSuperchainERC20Metadata struct is stored at.
    /// keccak256(abi.encode(uint256(keccak256("crossChainERC20.metadata")) - 1)) & ~bytes32(uint256(0xff));
    bytes32 internal constant CROSSCHAIN_ERC20_METADATA_SLOT =
        0xb06a38db87ccfa1ca3ebc512f371d7ee5852c1fc01165f9a67bc8465deb1ed00;

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
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Storage struct for the token metadata.
    /// @custom:storage-location erc7201:crossChainERC20.metadata
    struct MetadataStorage {
        address remoteToken;
        string name;
        string symbol;
        uint8 decimals;
    }

    //////////////////////////////////////////////////////////////
    ///                       Modifiers                        ///
    //////////////////////////////////////////////////////////////

    /// @notice A modifier that only allows the UniversalBridge to call.
    modifier onlyUniversalBridge() {
        address UNIVERSAL_BRIDGE = 0x4200000000000000000000000000000000000016;
        require(msg.sender == UNIVERSAL_BRIDGE, "CrossChainERC20: onlyUniversalBridge");
        _;
    }

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the OptimismSuperchainERC20 contract.
    constructor() {
        _disableInitializers();
    }

    /// @notice Semantic version.
    /// @custom:semver 1.0.1
    function version() external view virtual returns (string memory) {
        return "1.0.1";
    }

    /// @notice ERC165 interface check function.
    ///
    /// @param interfaceId Interface ID to check.
    ///
    /// @return Whether or not the interface is supported by this contract.
    function supportsInterface(bytes4 interfaceId) public view virtual returns (bool) {
        return interfaceId == type(ICrossChainERC20).interfaceId;
    }

    /// @notice Returns the address of the corresponding version of this token on the remote chain.
    function remoteToken() public view returns (address) {
        return _getStorage().remoteToken;
    }

    /// @notice Returns the name of the token.
    function name() public view virtual override returns (string memory) {
        return _getStorage().name;
    }

    /// @notice Returns the symbol of the token.
    function symbol() public view virtual override returns (string memory) {
        return _getStorage().symbol;
    }

    /// @notice Returns the number of decimals used to get its user representation.
    function decimals() public view override returns (uint8) {
        return _getStorage().decimals;
    }

    /// @notice Initializes the contract.
    ///
    /// @param remoteToken_ Address of the corresponding remote token.
    /// @param name_ ERC20 name.
    /// @param symbol_ ERC20 symbol.
    /// @param decimals_ ERC20 decimals.
    function initialize(address remoteToken_, string memory name_, string memory symbol_, uint8 decimals_)
        external
        initializer
    {
        MetadataStorage storage _storage = _getStorage();
        _storage.remoteToken = remoteToken_;
        _storage.name = name_;
        _storage.symbol = symbol_;
        _storage.decimals = decimals_;
    }

    /// @notice Allows the UniversalBridge to mint tokens.
    /// @param to Address to mint tokens to.
    /// @param amount Amount of tokens to mint.
    function mint(address to, uint256 amount) external virtual onlyUniversalBridge {
        require(to != address(0), "CrossChainERC20: mint to zero address");

        _mint(to, amount);
        emit Mint(to, amount);
    }

    /// @notice Allows the UniversalBridge to burn tokens.
    /// @param from Address to burn tokens from.
    /// @param amount Amount of tokens to burn.
    function burn(address from, uint256 amount) external virtual onlyUniversalBridge {
        require(from != address(0), "CrossChainERC20: burn from zero address");

        _burn(from, amount);
        emit Burn(from, amount);
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Sets Permit2 contract's allowance at infinity.
    function _givePermit2InfiniteAllowance() internal view virtual override returns (bool) {
        return true;
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @notice Returns the storage for the CrossChainERC20Metadata.
    function _getStorage() private pure returns (MetadataStorage storage storage_) {
        assembly {
            storage_.slot := CROSSCHAIN_ERC20_METADATA_SLOT
        }
    }
}
