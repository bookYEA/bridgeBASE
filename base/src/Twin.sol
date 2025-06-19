// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Call, CallLib} from "./libraries/CallLib.sol";

contract Twin {
    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the caller is neither the portal nor the twin itself.
    error Unauthorized();

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

    /// @notice Executes a call.
    ///
    /// @param call The encoded call to execute.
    function execute(Call calldata call) external payable {
        require(msg.sender == PORTAL || msg.sender == address(this), Unauthorized());
        CallLib.execute(call);
    }
}
