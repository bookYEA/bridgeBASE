use alloy_sol_types::sol;

sol! {
    type Pubkey is bytes32;

    #[derive(Debug, PartialEq, Eq)]
    contract Bridge {
        /// @notice Finalizes a token bridge on this chain. Can only be triggered by the Bridge contract on the remote
        ///         chain.
        ///
        /// @param localToken Address of the ERC20 on this chain.
        /// @param remoteToken Address of the corresponding token on the remote chain.
        /// @param from Address of the sender.
        /// @param to Address of the receiver.
        /// @param amount Amount of the ERC20 being bridged.
        /// @param extraData Extra data to be sent with the transaction. Note that the recipient will not be triggered with
        ///                  this data, but it will be emitted and can be used to identify the transaction.
        function finalizeBridgeToken(
            address localToken,
            bytes32 remoteToken,
            bytes32 from,
            address to,
            uint256 amount,
            bytes calldata extraData
        ) public;

        /// @notice Registers a remote token with the bridge.
        ///
        /// @param localToken Address of the ERC20 token on this chain.
        /// @param remoteToken Pubkey of the remote token on Solana.
        /// @param scalerExponent Exponent to be used to convert local to remote amounts.
        function registerRemoteToken(address localToken, Pubkey remoteToken, uint8 scalerExponent)
            external;
    }
}
