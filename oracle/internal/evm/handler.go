package evm

import (
	"bytes"
	"context"
	"encoding/binary"
	"fmt"

	"github.com/base/alt-l1-bridge/oracle/bindings"
	"github.com/base/alt-l1-bridge/oracle/internal/flags"
	"github.com/base/alt-l1-bridge/oracle/internal/mmr"
	"github.com/base/alt-l1-bridge/oracle/internal/signer"
	svmCommon "github.com/blocto/solana-go-sdk/common"
	"github.com/blocto/solana-go-sdk/types"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

const (
	OutputRootSeed   = "output_root"
	MessengerSeed    = "messenger_state"
	BridgeProgramID  = "Fb7KKBmjgKJh1N3aDUxLTj6TR3exH8Xi368bJ3AcDd5T"
	Version          = 1
	SubmitRootIxName = "submit_root"
)

type EvmLogHandler struct {
	svmSigner       *signer.SvmSigner
	mmr             *mmr.MMR
	bridgeProgramId svmCommon.PublicKey
}

func NewLogHandler(ctx *cli.Context) (*EvmLogHandler, error) {
	r, err := signer.NewSvmSigner(ctx)
	if err != nil {
		log.Error("Error initializing SVM relayer", "error", err)
		return nil, err
	}

	targetAddrStr := ctx.String(flags.TargetAddressFlag.Name)
	bridgeProgramId := svmCommon.PublicKeyFromString(targetAddrStr)

	return &EvmLogHandler{
		svmSigner:       r,
		mmr:             mmr.NewMMR(),
		bridgeProgramId: bridgeProgramId,
	}, nil
}

func (h *EvmLogHandler) GetStartingBlock() (uint64, error) {
	// 1. Calculate Messenger PDA
	versionBytes := make([]byte, 1)
	versionBytes[0] = byte(Version) // Version is a const in this package

	messengerPda, _, err := svmCommon.FindProgramAddress(
		[][]byte{
			[]byte(MessengerSeed), // MessengerSeed is a const in this package
			versionBytes,
		},
		h.bridgeProgramId,
	)
	if err != nil {
		log.Error("Error finding messenger PDA for starting block", "error", err)
		return 0, fmt.Errorf("failed to find messenger PDA: %w", err)
	}
	log.Info("Messenger PDA calculated for GetStartingBlock", "pda", messengerPda.ToBase58())

	// 2. Query Solana for account data
	accountInfo, err := h.svmSigner.Client.GetAccountInfo(context.Background(), messengerPda.ToBase58())
	if err != nil {
		log.Error("Error fetching messenger account info", "pda", messengerPda.ToBase58(), "error", err)
		return 0, fmt.Errorf("failed to fetch messenger account info for PDA %s: %w", messengerPda.ToBase58(), err)
	}

	if accountInfo.Data == nil {
		log.Error("Messenger account not found or has no data", "pda", messengerPda.ToBase58())
		// It might be acceptable for the account not to exist on a fresh deployment.
		// Depending on requirements, this could return nil or a specific error/status.
		// For now, treating as an error.
		return 0, fmt.Errorf("messenger account %s not found or empty", messengerPda.ToBase58())
	}

	// 3. Deserialize account data
	// Expected structure on Solana: 8-byte discriminator + 8-byte msg_nonce (u64) + 8-byte latest_block_number (u64)
	// Total = 24 bytes.
	discriminatorSize := 8
	msgNonceSize := 8
	latestBlockNumberSize := 8
	expectedMinSize := discriminatorSize + msgNonceSize + latestBlockNumberSize

	if len(accountInfo.Data) < expectedMinSize {
		log.Error("Account data too short", "pda", messengerPda.ToBase58(), "length", len(accountInfo.Data), "expected", expectedMinSize)
		return 0, fmt.Errorf("account data for %s too short: got %d bytes, expected at least %d", messengerPda.ToBase58(), len(accountInfo.Data), expectedMinSize)
	}

	// Skip 8-byte discriminator and 8-byte msg_nonce to get to latest_block_number
	offset := discriminatorSize + msgNonceSize
	latestBlockNumberBytes := accountInfo.Data[offset : offset+latestBlockNumberSize]
	latestBlockNumber := binary.LittleEndian.Uint64(latestBlockNumberBytes)

	log.Info("Successfully fetched latest_block_number from Messenger PDA",
		"pda", messengerPda.ToBase58(),
		"latest_block_number", latestBlockNumber,
	)

	return latestBlockNumber, nil
}

func (h *EvmLogHandler) HandleLog(logRecv *bindings.MessagePasserMessagePassed) error {
	log.Info("Received Base log",
		"withdrawalHash", "0x"+common.Bytes2Hex(logRecv.WithdrawalHash[:]),
		"sender", logRecv.Sender,
		"blockNumber", logRecv.Raw.BlockNumber,
		"txHash", logRecv.Raw.TxHash,
	)

	h.mmr.Append(logRecv.WithdrawalHash[:])

	root, err := h.mmr.Root()
	if err != nil {
		log.Error("Error getting ending MMR root", err)
	}

	log.Info("New MMR root", "root", "0x"+common.Bytes2Hex(root))

	blockNumberBytes := make([]byte, 8)
	binary.LittleEndian.PutUint64(blockNumberBytes, logRecv.Raw.BlockNumber)

	outputRootPda, _, err := svmCommon.FindProgramAddress(
		[][]byte{
			[]byte(OutputRootSeed),
			blockNumberBytes,
		},
		h.bridgeProgramId,
	)
	if err != nil {
		log.Error("Error finding output root PDA", "error", err)
		return fmt.Errorf("failed to find output root PDA: %w", err)
	}

	versionBytes := make([]byte, 1) // VERSION is u8 in Rust
	versionBytes[0] = byte(Version) // Directly assign the u8 value

	messengerPda, _, err := svmCommon.FindProgramAddress(
		[][]byte{
			[]byte(MessengerSeed),
			versionBytes, // This needs to be to_le_bytes() of VERSION from rust (u8)
		},
		h.bridgeProgramId,
	)
	if err != nil {
		log.Error("Error finding messenger PDA", "error", err)
		return fmt.Errorf("failed to find messenger PDA: %w", err)
	}

	// Instruction data:
	// 8 bytes for discriminator (sha256("global:submit_root")[..8])
	// 32 bytes for root
	// 8 bytes for block_number

	ixData := new(bytes.Buffer)
	ixDiscriminator := []byte{15, 86, 198, 221, 22, 34, 184, 178} // sighash("global:submit_root")

	_, err = ixData.Write(ixDiscriminator)
	if err != nil {
		return fmt.Errorf("failed to write ix discriminator: %w", err)
	}
	_, err = ixData.Write(root)
	if err != nil {
		return fmt.Errorf("failed to write root to ix data: %w", err)
	}
	_, err = ixData.Write(blockNumberBytes)
	if err != nil {
		return fmt.Errorf("failed to write block number to ix data: %w", err)
	}

	// root, messenger, payer, systemProgram
	submitRootIx := types.Instruction{
		ProgramID: h.bridgeProgramId,
		Accounts: []types.AccountMeta{
			{PubKey: outputRootPda, IsSigner: false, IsWritable: true},                 // root
			{PubKey: messengerPda, IsSigner: false, IsWritable: true},                  // messenger
			{PubKey: h.svmSigner.FeePayer.PublicKey, IsSigner: true, IsWritable: true}, // payer
			{PubKey: svmCommon.SystemProgramID, IsSigner: false, IsWritable: false},    // system_program
		},
		Data: ixData.Bytes(),
	}
	svmReq := signer.TransactionRequest{
		Instructions: []types.Instruction{submitRootIx},
		Signers:      []types.Account{}, // Payer (TrustedOracle) will be a signer, svmSigner handles this
	}
	err = h.svmSigner.SubmitTransaction(&svmReq)
	if err != nil {
		log.Error("Error submitting root to Solana")
		return err
	}

	return nil
}
