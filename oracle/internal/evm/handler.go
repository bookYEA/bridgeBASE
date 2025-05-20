package evm

import (
	"bytes"
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
