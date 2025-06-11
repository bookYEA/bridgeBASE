use alloy_sol_types::sol;

sol! {
    #[derive(Debug, PartialEq, Eq)]
    contract CrossChainMessenger {
        /// @notice Relays a message that was sent by the remote Messenger contract. Can only be executed via
        ///         cross-chain call from the remote messenger OR if the message previously failed and is being replayed.
        ///
        /// @dev Gas estimation: If the transaction origin is ESTIMATION_ADDRESS, failures will cause reverts to help
        ///      compute accurate gas limits during estimation.
        ///
        /// @param nonce Unique nonce of the message being relayed.
        /// @param sender Address of the user who sent the message on the remote chain.
        /// @param target Address that the message is targeted at on this chain.
        /// @param minGasLimit Minimum amount of gas that the message must be executed with.
        /// @param message Encoded message data to send to the target address.
        function relayMessage(
            uint256 nonce,
            bytes32 sender,
            address target,
            uint256 minGasLimit,
            bytes calldata message
        ) external;
    }
}
