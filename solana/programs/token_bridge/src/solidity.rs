use alloy_sol_types::sol;

sol! {
    type Pubkey is bytes32;

    #[derive(Debug, PartialEq, Eq)]
    contract Bridge {
        /// @notice Finalizes a token bridge transaction initiated from Solana.
        ///
        /// @dev This function can only be called by the remote bridge through the messenger system. For CrossChainERC20
        ///      tokens, it mints new tokens. For standard tokens, it withdraws from the deposit pool. Supports both
        ///      ERC20 tokens and native ETH.
        ///
        /// @param localToken Address of the ERC20 token on this chain (use ETH_ADDRESS for native ETH).
        /// @param remoteToken Pubkey of the remote token on Solana.
        /// @param from Pubkey of the original sender on Solana.
        /// @param to Address of the recipient on this chain.
        /// @param remoteAmount Amount of tokens being bridged from Solana (expressed in Solana units).
        /// @param extraData Additional data associated with the original bridge transaction.
        function finalizeBridgeToken(
            address localToken,
            Pubkey remoteToken,
            Pubkey from,
            address to,
            uint64 remoteAmount,
            bytes calldata extraData
        ) external payable;

        /// @notice Registers a remote token with the bridge.
        ///
        /// @param localToken Address of the ERC20 token on this chain.
        /// @param remoteToken Pubkey of the remote token on Solana.
        /// @param scalerExponent Exponent to be used to convert local to remote amounts.
        function registerRemoteToken(address localToken, Pubkey remoteToken, uint8 scalerExponent)
            external;
    }
}
