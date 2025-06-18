// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

contract Twin {
    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Enum representing the type of call.
    enum CallType {
        Call,
        DelegateCall,
        Create,
        Create2
    }

    /// @notice Struct representing a call to execute.
    ///
    /// @custom:field ty The type of call.
    /// @custom:field to The target address to call.
    /// @custom:field gasLimit The gas limit for the call.
    /// @custom:field value The value to send with the call.
    /// @custom:field data The data to pass to the call.
    struct Call {
        CallType ty;
        address to;
        uint256 gasLimit;
        uint256 value;
        bytes data;
    }

    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the caller is not the deployer.
    error NotDeployer();

    /// @notice Thrown when the delegate call has a value.
    error DelegateCallCannotHaveValue();

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////
    address public immutable DEPLOYER;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    constructor() {
        DEPLOYER = msg.sender;
    }

    receive() external payable {}

    function execute(Call[] calldata calls) external payable {
        if (msg.sender != DEPLOYER) revert NotDeployer();

        for (uint256 i; i < calls.length; i++) {
            if (calls[i].ty == CallType.Call) {
                (bool success, bytes memory result) =
                    calls[i].to.call{gas: calls[i].gasLimit, value: calls[i].value}(calls[i].data);

                if (!success) {
                    revert(string(result));
                }
            } else if (calls[i].ty == CallType.DelegateCall) {
                if (calls[i].value != 0) revert DelegateCallCannotHaveValue();

                (bool success, bytes memory result) = calls[i].to.delegatecall{gas: calls[i].gasLimit}(calls[i].data);
                if (!success) {
                    revert(string(result));
                }
            } else if (calls[i].ty == CallType.Create) {
                uint256 value = calls[i].value;
                bytes memory data = calls[i].data;
                assembly {
                    let result := create(value, add(data, 0x20), mload(data))
                    if iszero(result) { revert(0, 0) }
                }
            } else if (calls[i].ty == CallType.Create2) {
                uint256 value = calls[i].value;
                (bytes32 salt, bytes memory data) = abi.decode(calls[i].data, (bytes32, bytes));
                assembly {
                    let result := create2(value, add(data, 0x20), mload(data), salt)
                    if iszero(result) { revert(0, 0) }
                }
            }
        }
    }
}
