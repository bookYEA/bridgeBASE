// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {IERC165} from "./IERC165.sol";

/// @title ICrossChainERC20
/// @notice This interface is available on the CrossChainERC20 contract. We declare it as a separate interface so that
///         it can be used in custom implementations of CrossChainERC20.
interface ICrossChainERC20 is IERC165 {
    function remoteToken() external view returns (address);
    function mint(address _to, uint256 _amount) external;
    function burn(address _from, uint256 _amount) external;
}
