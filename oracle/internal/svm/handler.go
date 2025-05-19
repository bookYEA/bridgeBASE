package svm

import (
	"crypto/sha256"
	"encoding/base64"
	"fmt"
	"math/big"
	"strings"

	"github.com/base/alt-l1-bridge/oracle/internal/evm"
	"github.com/base/alt-l1-bridge/oracle/internal/types"
	"github.com/base/alt-l1-bridge/oracle/internal/utils"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/log"
	bin "github.com/gagliardetto/binary"
	"github.com/gagliardetto/solana-go"
	"github.com/gagliardetto/solana-go/rpc/ws"
	"github.com/urfave/cli/v2"
)

// TransactionDepositedEvent matches the Rust struct
type TransactionDepositedEvent struct {
	From       solana.PublicKey
	To         [20]byte
	Version    uint64
	OpaqueData []byte
}

// Anchor event discriminator prefix
const anchorEventPrefix = "Program data: "

var DepositEventVersion0 = uint64(0)

// Calculate the 8-byte discriminator for the TransactionDeposited event
// sha256("event:TransactionDeposited")[..8]
var transactionDepositedDiscriminator = func() []byte {
	h := sha256.New()
	h.Write([]byte("event:TransactionDeposited"))
	return h.Sum(nil)[:8]
}()

type SvmLogHandler struct {
	relayer        *evm.Relayer
	seenSignatures map[string]bool
}

func NewLogHandler(ctx *cli.Context) (*SvmLogHandler, error) {
	r, err := evm.NewRelayer(ctx)
	if err != nil {
		log.Error("Error creating relayer", "err", err)
		return nil, err
	}

	return &SvmLogHandler{
		relayer:        r,
		seenSignatures: map[string]bool{},
	}, nil
}

func (h *SvmLogHandler) HandleLogs(got *ws.LogResult) error {
	if got == nil {
		log.Warn("Received nil log result")
		return nil
	}

	for _, logMsg := range got.Value.Logs {
		if !strings.HasPrefix(logMsg, anchorEventPrefix) {
			continue // Not an Anchor event log
		}

		// Extract base64 encoded data
		encodedData := strings.TrimPrefix(logMsg, anchorEventPrefix)
		eventData, err := base64.StdEncoding.DecodeString(encodedData)
		if err != nil {
			log.Warn("Failed to base64 decode event data", "log", logMsg, "err", err)
			continue
		}

		// Check discriminator (first 8 bytes)
		if len(eventData) < 8 {
			log.Warn("Event data too short for discriminator", "len", len(eventData))
			continue
		}

		discriminator := eventData[:8]
		payload := eventData[8:]

		// Check if it matches TransactionDeposited event
		if string(discriminator) == string(transactionDepositedDiscriminator) {
			// Borsh decode the payload
			var event TransactionDepositedEvent
			decoder := bin.NewBorshDecoder(payload)
			err = decoder.Decode(&event)
			if err != nil {
				log.Error("Failed to Borsh decode TransactionDeposited event", "err", err, "payload_len", len(payload))
				continue
			}

			if h.seenSignatures[got.Value.Signature.String()] {
				continue
			}

			// Log the decoded event
			log.Info("<<< TransactionDeposited Event >>>",
				"slot", got.Context.Slot,
				"tx_signature", got.Value.Signature.String(),
				"from", event.From.String(),
				"to", fmt.Sprintf("0x%x", event.To), // Format EVM address
				"version", event.Version,
				"opaque_data_len", len(event.OpaqueData),
				"opaque_data", fmt.Sprintf("0x%x", event.OpaqueData),
			)

			h.seenSignatures[got.Value.Signature.String()] = true

			var dep types.DepositTx

			// source := UserDepositSource{
			// 	L1BlockHash: ev.BlockHash,
			// 	LogIndex:    uint64(ev.Index),
			// }
			// dep.SourceHash = source.SourceHash()
			dep.From = utils.SolanaPubkeyToEvmAddress(event.From)
			dep.IsSystemTransaction = false

			var err error
			switch event.Version {
			case DepositEventVersion0:
				err = parseOpaqueData(&dep, event.To, event.OpaqueData)
			default:
				return fmt.Errorf("invalid deposit version, got %v", int(event.Version))
			}
			if err != nil {
				return fmt.Errorf("failed to decode deposit (version %v): %w", int(event.Version), err)
			}

			dep.Print()
			err = h.relayer.SendTransactionToBase(dep)
			if err != nil {
				return err
			}
		}
	}

	return nil
}

func parseOpaqueData(dep *types.DepositTx, to common.Address, opaqueData []byte) error {
	if len(opaqueData) < 8+1 {
		return fmt.Errorf("unexpected opaqueData length: %d", len(opaqueData))
	}
	offset := uint64(0)

	// // uint256 mint
	// dep.Mint = new(big.Int).SetBytes(opaqueData[offset : offset+8])
	// // 0 mint is represented as nil to skip minting code
	// if dep.Mint.Cmp(new(big.Int)) == 0 {
	// 	dep.Mint = nil
	// }
	// offset += 8

	// uint256 value
	// dep.Value = new(big.Int).SetBytes(opaqueData[offset : offset+8])
	// offset += 8
	dep.Value = new(big.Int)

	// uint64 gas
	gas := new(big.Int).SetBytes(opaqueData[offset : offset+8])
	if !gas.IsUint64() {
		return fmt.Errorf("bad gas value: %x", opaqueData[offset:offset+8])
	}
	dep.Gas = gas.Uint64()
	offset += 8

	// uint8 isCreation
	// isCreation: If the boolean byte is 1 then dep.To will stay nil,
	// and it will create a contract using L2 account nonce to determine the created address.
	if opaqueData[offset] == 0 {
		dep.To = &to
	}
	offset += 1

	// The remainder of the opaqueData is the transaction data (without length prefix).
	// The data may be padded to a multiple of 32 bytes
	txDataLen := uint64(len(opaqueData)) - offset

	// remaining bytes fill the data
	dep.Data = opaqueData[offset : offset+txDataLen]

	return nil
}
