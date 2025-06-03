use alloy_sol_types::sol;

sol! {
    #[derive(Debug, PartialEq, Eq)]
    contract CrossChainMessenger {
        /// @notice Relays a message that was sent by the remote CrossChainMessenger contract. Can only be executed via
        ///         cross-chain call from the remote messenger OR if the message was already received once and is currently
        ///         being replayed.
        ///
        /// @param nonce Nonce of the message being relayed.
        /// @param sender Address of the user who sent the message.
        /// @param target Address that the message is targeted at.
        /// @param minGasLimit Minimum amount of gas that the message can be executed with.
        /// @param message Message to send to the target.
        function relayMessage(
            uint256 nonce,
            bytes32 sender,
            address target,
            uint256 minGasLimit,
            bytes calldata message
        ) external payable;
    }
}
