// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Ix, IxAccount, Pda, Pubkey, SVMLib} from "./SVMLib.sol";

library SVMTokenBridgeLib {
    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice The TokenProgram ID on Solana.
    Pubkey private constant _TOKEN_PROGRAM_ID =
        Pubkey.wrap(0x06ddf6e1d765a193d9cbe146ceeb79ac1cb485ed5f5b37913a8cf5857eff00a9);

    /// @notice The TokenProgram 2022 ID on Solana.
    Pubkey private constant _TOKEN_PROGRAM_2022_ID =
        Pubkey.wrap(0x06ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc);

    /// @notice The SystemProgram ID on Solana.
    Pubkey private constant _SYSTEM_PROGRAM_ID =
        Pubkey.wrap(0x0000000000000000000000000000000000000000000000000000000000000000);

    //////////////////////////////////////////////////////////////
    ///                     Internal Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Builds the TokenBridge's FinalizeBridgeToken instruction.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param to Pubkey of the recipient on Solana.
    /// @param amount Amount of tokens to bridge.
    /// @param decimals The number of decimals for the remote token on Solana.
    ///
    /// @return The instruction.
    function finalizeBridgeTokenIx(
        Pubkey portal,
        Pubkey remoteBridge,
        address localToken,
        Pubkey remoteToken,
        Pubkey to,
        uint64 amount,
        uint8 decimals
    ) internal view returns (Ix memory) {
        IxAccount[] memory accounts = new IxAccount[](4);
        accounts[0] = _portalAuthorityIxAccount(portal); // portal_authority
        accounts[1] = _wrappedMintIxAccount(remoteBridge, localToken, decimals); // mint
        accounts[2] = SVMLib.createPubkeyAccount({pubkey: to, isWritable: true, isSigner: false}); // to_token_account
        accounts[3] = SVMLib.createPubkeyAccount({pubkey: _TOKEN_PROGRAM_2022_ID, isWritable: false, isSigner: false}); // token_program

        return Ix({
            programId: remoteBridge,
            name: "finalize_bridge_token",
            accounts: accounts,
            data: abi.encodePacked(remoteToken, localToken, amount) // (expected_mint, remote_token, amount)
        });
    }

    /// @notice Builds the TokenBridge's FinalizeBridgeSol instruction.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param to Pubkey of the recipient on Solana.
    /// @param amount Amount of tokens to bridge.
    ///
    /// @return The instruction.
    function finalizeBridgeSolIx(Pubkey portal, Pubkey remoteBridge, address localToken, Pubkey to, uint64 amount)
        internal
        view
        returns (Ix memory)
    {
        IxAccount[] memory accounts = new IxAccount[](4);
        accounts[0] = _portalAuthorityIxAccount(portal); // portal_authority
        accounts[1] = _solVaultIxAccount(remoteBridge, localToken); // sol_vault
        accounts[2] = SVMLib.createPubkeyAccount({pubkey: to, isWritable: true, isSigner: false}); // to
        accounts[3] = SVMLib.createPubkeyAccount({pubkey: _SYSTEM_PROGRAM_ID, isWritable: false, isSigner: false}); // system_program

        return Ix({
            programId: remoteBridge,
            name: "finalize_bridge_sol",
            accounts: accounts,
            data: abi.encodePacked(localToken, amount)
        });
    }

    /// @notice Builds the TokenBridge's FinalizeBridgeSpl instruction.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param remoteToken Pubkey of the corresponding token on Solana.
    /// @param to Pubkey of the recipient on Solana.
    /// @param amount Amount of tokens to bridge.
    ///
    /// @return The instruction.
    function finalizeBridgeSplIx(
        Pubkey portal,
        Pubkey remoteBridge,
        address localToken,
        Pubkey remoteToken,
        Pubkey to,
        uint64 amount
    ) internal view returns (Ix memory) {
        IxAccount[] memory accounts = new IxAccount[](4);
        accounts[0] = _portalAuthorityIxAccount(portal); // portal_authority
        accounts[1] = SVMLib.createPubkeyAccount({pubkey: remoteToken, isWritable: true, isSigner: false}); // mint
        accounts[2] =
            _tokenVaultIxAccount({remoteBridge: remoteBridge, localToken: localToken, remoteToken: remoteToken}); // token_vault
        accounts[3] = SVMLib.createPubkeyAccount({pubkey: to, isWritable: true, isSigner: false}); // to_token_account
        accounts[4] = SVMLib.createPubkeyAccount({pubkey: _TOKEN_PROGRAM_2022_ID, isWritable: false, isSigner: false}); // token_program

        return Ix({
            programId: remoteBridge,
            name: "finalize_bridge_spl",
            accounts: accounts,
            data: abi.encodePacked(localToken, amount) // (remote_token, amount)
        });
    }

    //////////////////////////////////////////////////////////////
    ///                     Private Functions                  ///
    //////////////////////////////////////////////////////////////

    /// @notice Builds the TokenBridge's Portal authority PDA.
    ///
    /// @param portal Pubkey of the portal on Solana.
    ///
    /// @dev  #[account(
    ///         seeds = [PORTAL_AUTHORITY_SEED, REMOTE_BRIDGE.as_ref()],
    ///         bump,
    ///         seeds::program = portal::program::Portal::id()
    ///     )]
    ///
    /// @return The Portal authority PDA.
    function _portalAuthorityIxAccount(Pubkey portal) internal view returns (IxAccount memory) {
        bytes[] memory seeds = new bytes[](2);
        seeds[0] = "portal_authority";
        seeds[1] = abi.encodePacked(address(this)); // remote_bridge

        return
            SVMLib.createPdaAccount({pda: Pda({seeds: seeds, programId: portal}), isWritable: false, isSigner: false});
    }

    /// @notice Builds the TokenBridge's wrapped mint PDA.
    ///
    /// @param remoteBridge Pubkey of the remote bridge on Solana.
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param decimals The number of decimals for the remote token on Solana.
    ///
    /// @dev  #[account(
    ///         mut,
    ///         seeds = [
    ///             WRAPPED_TOKEN_SEED,
    ///             remote_token.as_ref(),
    ///             mint.decimals.to_le_bytes().as_ref()
    ///         ],
    ///         bump
    ///     )]
    ///
    /// @return The wrapped mint PDA.
    function _wrappedMintIxAccount(Pubkey remoteBridge, address localToken, uint8 decimals)
        private
        pure
        returns (IxAccount memory)
    {
        bytes[] memory seeds = new bytes[](3);
        seeds[0] = "wrapped_token";
        seeds[1] = abi.encodePacked(localToken); // remote_token
        seeds[2] = abi.encodePacked(decimals); // decimals

        return SVMLib.createPdaAccount({
            pda: Pda({seeds: seeds, programId: remoteBridge}),
            isWritable: true,
            isSigner: false
        });
    }

    /// @notice Builds the TokenBridge's sol vault PDA.
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
    function _solVaultIxAccount(Pubkey remoteBridge, address localToken) private pure returns (IxAccount memory) {
        bytes[] memory seeds = new bytes[](2);
        seeds[0] = "sol_vault";
        seeds[1] = abi.encodePacked(localToken); // remote_token

        return SVMLib.createPdaAccount({
            pda: Pda({seeds: seeds, programId: remoteBridge}),
            isWritable: true,
            isSigner: false
        });
    }

    /// @notice Builds the TokenBridge's token vault PDA.
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
        returns (IxAccount memory)
    {
        bytes[] memory seeds = new bytes[](3);
        seeds[0] = "token_vault";
        seeds[1] = abi.encodePacked(remoteToken); // mint
        seeds[2] = abi.encodePacked(localToken); // remote_token

        return SVMLib.createPdaAccount({
            pda: Pda({seeds: seeds, programId: remoteBridge}),
            isWritable: true,
            isSigner: false
        });
    }
}
