// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Call, CallLib} from "./libraries/CallLib.sol";

contract Twin {
    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the caller is not the portal.
    error NotPortal();

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @dev The address of the Portal contract.
    address public immutable PORTAL;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    constructor(address portal) {
        PORTAL = portal;
    }

    receive() external payable {}

    /// @notice Executes a batch of calls.
    ///
    /// @param data The encoded calls to execute.
    function executeBatch(bytes calldata data) external payable {
        Call[] memory calls = abi.decode(data, (Call[]));
        if (msg.sender != PORTAL) revert NotPortal();

        for (uint256 i; i < calls.length; i++) {
            CallLib.execute(calls[i]);
        }
    }
}
