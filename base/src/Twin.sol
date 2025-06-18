// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Initializable} from "solady/utils/Initializable.sol";

import {Call, CallLib} from "./libraries/CallLib.sol";

contract Twin is Initializable {
    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the caller is not the deployer.
    error NotDeployer();

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @dev The address that deployed the Twin contract.
    address public deployer;

    /// @dev The Solana owner's pubkey.
    bytes32 public remoteOwner;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    constructor() {
        _disableInitializers();
    }

    receive() external payable {}

    /// @notice Initializes the Twin contract.
    ///
    /// @param remoteOwner_ The Solana owner's pubkey.
    function initialize(bytes32 remoteOwner_) external reinitializer(1) {
        deployer = msg.sender;
        remoteOwner = remoteOwner_;
    }

    /// @notice Executes a batch of calls.
    ///
    /// @param data The encoded calls to execute.
    function executeBatch(bytes calldata data) external payable {
        Call[] memory calls = abi.decode(data, (Call[]));
        if (msg.sender != deployer) revert NotDeployer();

        for (uint256 i; i < calls.length; i++) {
            CallLib.execute(calls[i]);
        }
    }
}
