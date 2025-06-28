// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Ix, Pda, Pubkey, SVMLib} from "./SVMLib.sol";

library SVMBridgeLib {
    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice The TokenProgram ID on Solana (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA).
    Pubkey private constant _TOKEN_PROGRAM_ID =
        Pubkey.wrap(0x06ddf6e1d765a193d9cbe146ceeb79ac1cb485ed5f5b37913a8cf5857eff00a9);

    /// @notice The TokenProgram 2022 ID on Solana (TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb).
    Pubkey private constant _TOKEN_PROGRAM_2022_ID =
        Pubkey.wrap(0x06ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc);

    /// @notice The SystemProgram ID on Solana.
    Pubkey private constant _SYSTEM_PROGRAM_ID =
        Pubkey.wrap(0x0000000000000000000000000000000000000000000000000000000000000000);

    //////////////////////////////////////////////////////////////
    ///                     Internal Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Serializes a Message::Call variant to Borsh-compatible bytes.
    ///
    /// @param ixs The Solana instructions.
    ///
    /// @return Serialized Message::Call bytes ready for Solana deserialization
    function serializeCall(Ix[] memory ixs) internal pure returns (bytes memory) {
        // Variant discriminator for Call (0)
        bytes memory result = abi.encodePacked(uint8(0));

        // Serialize the Anchor instructions bytes.
        result = abi.encodePacked(result, SVMLib.serializeIxs(ixs));

        return result;
    }

    /// @notice Serializes a Message::Transfer variant to Borsh-compatible bytes.
    ///
    /// @param transfer The transfer instruction
    /// @param ixs The Solana instructions.
    ///
    /// @return Serialized Message::Transfer bytes ready for Solana deserialization
    function serializeTransfer(Ix memory transfer, Ix[] memory ixs) internal pure returns (bytes memory) {
        // Variant discriminator for Transfer (1)
        bytes memory result = abi.encodePacked(uint8(1));

        // Serialize the transfer instruction
        result = abi.encodePacked(result, SVMLib.serializeIx(transfer));

        // Serialize the instructions array
        result = abi.encodePacked(result, SVMLib.serializeIxs(ixs));

        return result;
    }

    /// @notice Builds the Bridge's FinalizeBridgeToken instruction.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param remoteToken Pubkey of the corresponding token on Solana.
    /// @param to Pubkey of the recipient on Solana.
    /// @param remoteAmount Amount of tokens to bridge.
    ///
    /// @return The instruction.
    function finalizeBridgeTokenIx(
        Pubkey remoteBridge,
        address localToken,
        Pubkey remoteToken,
        Pubkey to,
        uint64 remoteAmount
    ) internal pure returns (Ix memory) {
        bytes[] memory serializedAccounts = new bytes[](3);
        serializedAccounts[0] = SVMLib.serializePubkeyAccount({pubkey: remoteToken, isWritable: true, isSigner: false}); // mint
        serializedAccounts[1] = SVMLib.serializePubkeyAccount({pubkey: to, isWritable: true, isSigner: false}); // to_token_account
        serializedAccounts[2] =
            SVMLib.serializePubkeyAccount({pubkey: _TOKEN_PROGRAM_2022_ID, isWritable: false, isSigner: false}); // token_program

        // (remote_token, amount)
        bytes memory ixData = abi.encodePacked(localToken, SVMLib.toU64LittleEndian(remoteAmount));

        bytes32 ixDiscriminator = sha256("global:finalize_bridge_token");
        ixData = abi.encodePacked(bytes8(ixDiscriminator), ixData);

        return Ix({programId: remoteBridge, serializedAccounts: serializedAccounts, data: ixData});
    }

    /// @notice Builds the Bridge's FinalizeBridgeSol instruction.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param to Pubkey of the recipient on Solana.
    /// @param remoteAmount Amount of tokens to bridge.
    ///
    /// @return The instruction.
    function finalizeBridgeSolIx(Pubkey remoteBridge, address localToken, Pubkey to, uint64 remoteAmount)
        internal
        pure
        returns (Ix memory)
    {
        bytes[] memory serializedAccounts = new bytes[](3);
        serializedAccounts[0] = _solVaultIxAccount(remoteBridge, localToken); // sol_vault
        serializedAccounts[1] = SVMLib.serializePubkeyAccount({pubkey: to, isWritable: true, isSigner: false}); // to
        serializedAccounts[2] =
            SVMLib.serializePubkeyAccount({pubkey: _SYSTEM_PROGRAM_ID, isWritable: false, isSigner: false}); // system_program

        // (remote_token, amount)
        bytes memory ixData = abi.encodePacked(localToken, SVMLib.toU64LittleEndian(remoteAmount));
        bytes32 ixDiscriminator = sha256("global:finalize_bridge_sol");
        ixData = abi.encodePacked(bytes8(ixDiscriminator), ixData);

        return Ix({programId: remoteBridge, serializedAccounts: serializedAccounts, data: ixData});
    }

    /// @notice Builds the Bridge's FinalizeBridgeSpl instruction.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param remoteToken Pubkey of the corresponding token on Solana.
    /// @param to Pubkey of the recipient on Solana.
    /// @param remoteAmount Amount of tokens to bridge.
    ///
    /// @return The instruction.
    function finalizeBridgeSplIx(
        Pubkey remoteBridge,
        address localToken,
        Pubkey remoteToken,
        Pubkey to,
        uint64 remoteAmount
    ) internal pure returns (Ix memory) {
        bytes[] memory serializedAccounts = new bytes[](5);
        serializedAccounts[0] = SVMLib.serializePubkeyAccount({pubkey: remoteToken, isWritable: true, isSigner: false}); // mint
        serializedAccounts[1] =
            _tokenVaultIxAccount({remoteBridge: remoteBridge, localToken: localToken, remoteToken: remoteToken}); // token_vault
        serializedAccounts[2] = SVMLib.serializePubkeyAccount({pubkey: to, isWritable: true, isSigner: false}); // to_token_account
        serializedAccounts[3] =
            SVMLib.serializePubkeyAccount({pubkey: _TOKEN_PROGRAM_ID, isWritable: false, isSigner: false}); // token_program
        serializedAccounts[4] =
            SVMLib.serializePubkeyAccount({pubkey: _TOKEN_PROGRAM_2022_ID, isWritable: false, isSigner: false}); // token_program_2022

        // (remote_token, amount)
        bytes memory ixData = abi.encodePacked(localToken, SVMLib.toU64LittleEndian(remoteAmount));
        bytes32 ixDiscriminator = sha256("global:finalize_bridge_spl");
        ixData = abi.encodePacked(bytes8(ixDiscriminator), ixData);

        return Ix({programId: remoteBridge, serializedAccounts: serializedAccounts, data: ixData});
    }

    //////////////////////////////////////////////////////////////
    ///                     Private Functions                  ///
    //////////////////////////////////////////////////////////////

    /// @notice Builds the Bridge's sol vault PDA.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    ///
    /// @dev  #[account(
    ///         mut,
    ///         seeds = [SOL_VAULT_SEED, remote_token.as_ref()],
    ///         bump
    ///     )]
    ///
    /// @return The sol vault PDA.
    function _solVaultIxAccount(Pubkey remoteBridge, address localToken) private pure returns (bytes memory) {
        bytes[] memory seeds = new bytes[](2);
        seeds[0] = "sol_vault";
        seeds[1] = abi.encodePacked(localToken); // remote_token

        return SVMLib.serializePdaAccount({
            pda: Pda({seeds: seeds, programId: remoteBridge}),
            isWritable: true,
            isSigner: false
        });
    }

    /// @notice Builds the Bridge's token vault PDA.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param remoteToken Pubkey of the corresponding token on Solana.
    ///
    /// @dev  #[account(
    ///         mut,
    ///         seeds = [TOKEN_VAULT_SEED, mint.key().as_ref(), remote_token.as_ref()],
    ///         bump
    ///     )]
    ///
    /// @return The token vault PDA.
    function _tokenVaultIxAccount(Pubkey remoteBridge, address localToken, Pubkey remoteToken)
        private
        pure
        returns (bytes memory)
    {
        bytes[] memory seeds = new bytes[](3);
        seeds[0] = "token_vault";
        seeds[1] = abi.encodePacked(remoteToken); // mint
        seeds[2] = abi.encodePacked(localToken); // remote_token

        return SVMLib.serializePdaAccount({
            pda: Pda({seeds: seeds, programId: remoteBridge}),
            isWritable: true,
            isSigner: false
        });
    }
}
