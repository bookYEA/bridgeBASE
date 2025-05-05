package oracle

import (
	"context"
	"crypto/sha256"
	"encoding/base64"
	"fmt"
	"math/big"
	"os"
	"strings"
	"time"

	"github.com/base/alt-l1-bridge/oracle/internal/flags"
	"github.com/base/alt-l1-bridge/oracle/internal/utils"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/log"
	bin "github.com/gagliardetto/binary"
	"github.com/gagliardetto/solana-go"
	"github.com/gagliardetto/solana-go/rpc"
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

type DepositTx struct {
	// SourceHash uniquely identifies the source of the deposit
	SourceHash common.Hash
	// From is exposed through the types.Signer, not through TxData
	From common.Address
	// nil means contract creation
	To *common.Address `rlp:"nil"`
	// Mint is minted on L2, locked on L1, nil if no minting.
	Mint *big.Int `rlp:"nil"`
	// Value is transferred from L2 balance, executed after Mint (if any)
	Value *big.Int
	// gas limit
	Gas uint64
	// Field indicating if this transaction is exempt from the L2 gas limit.
	IsSystemTransaction bool
	// Normal Tx data
	Data []byte
}

func (t *DepositTx) Print() {
	log.Info(
		"----Decoded Transaction----",
		"SourceHash", t.SourceHash,
		"From", t.From,
		"To", t.To,
		"Mint", t.Mint,
		"Value", t.Value,
		"Gas", t.Gas,
		"IsSystemTransaction", t.IsSystemTransaction,
		"Data", common.Bytes2Hex(t.Data),
	)
}

// Anchor event discriminator prefix
const anchorEventPrefix = "Program data: "

// Calculate the 8-byte discriminator for the TransactionDeposited event
// sha256("event:TransactionDeposited")[..8]
var transactionDepositedDiscriminator = func() []byte {
	h := sha256.New()
	h.Write([]byte("event:TransactionDeposited"))
	return h.Sum(nil)[:8]
}()

var DepositEventVersion0 = uint64(0)

func Main(ctx *cli.Context) error {
	log.SetDefault(log.NewLogger(log.NewTerminalHandlerWithLevel(os.Stderr, log.LevelInfo, true)))

	wsUrl := rpc.DevNet_WS
	if ctx.Bool(flags.IsMainnetFlag.Name) {
		wsUrl = rpc.MainNetBeta_WS
	}
	// The targetAddrStr is now interpreted as the Program address
	targetAddrStr := ctx.String(flags.TargetAddressFlag.Name)

	programAddr, err := solana.PublicKeyFromBase58(targetAddrStr)
	if err != nil {
		log.Crit("Invalid program address", "address", targetAddrStr, "err", err)
		return err
	}

	log.Info("Starting Solana event indexer", "url", wsUrl, "program", programAddr.String())

	err = startIndexer(ctx.Context, wsUrl, programAddr)
	if err != nil {
		log.Crit("Indexer failed", "err", err)
		return err
	}

	log.Info("Indexer stopped")
	return nil
}

// startIndexer connects to the Solana WebSocket endpoint and subscribes to program logs.
func startIndexer(ctx context.Context, wsUrl string, programAddr solana.PublicKey) error {
	wsClient, err := ws.Connect(ctx, wsUrl)
	if err != nil {
		return fmt.Errorf("failed to connect to WebSocket %s: %w", wsUrl, err)
	}
	defer wsClient.Close()
	log.Info("WebSocket client connected", "url", wsUrl)

	// Subscribe to logs mentioning the program address
	sub, err := wsClient.LogsSubscribeMentions(programAddr, rpc.CommitmentFinalized)
	if err != nil {
		return fmt.Errorf("failed to subscribe to logs for program %s: %w", programAddr.String(), err)
	}
	defer sub.Unsubscribe()
	log.Info("Subscribed to program logs", "program", programAddr.String())

	for {
		// Use a timeout for Recv, but the main loop continues until context is cancelled
		recvCtx, cancel := context.WithTimeout(ctx, 10*time.Second) // Adjust timeout as needed
		got, err := sub.Recv(recvCtx)
		cancel()

		if err != nil {
			if err == context.DeadlineExceeded {
				// Timeout is expected, check main context and continue loop
				if ctx.Err() != nil {
					log.Info("Context cancelled, stopping indexer.")
					return nil // Normal shutdown
				}
				continue // Continue loop on timeout
			}
			if ctx.Err() != nil {
				log.Info("Context cancelled while receiving, stopping indexer.")
				return nil // Normal shutdown
			}
			if err == ws.ErrSubscriptionClosed {
				log.Warn("Subscription closed unexpectedly.")
				return err // Or attempt to resubscribe
			}
			log.Error("Error receiving log update", "err", err)
			return fmt.Errorf("error receiving log update: %w", err) // Treat other errors as fatal for now
		}

		if got == nil {
			log.Warn("Received nil log result")
			continue
		}

		// Process logs
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

				var dep DepositTx

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
			}
		}
	}
}

func parseOpaqueData(dep *DepositTx, to common.Address, opaqueData []byte) error {
	if len(opaqueData) < 8+8+8+1 {
		return fmt.Errorf("unexpected opaqueData length: %d", len(opaqueData))
	}
	offset := uint64(0)

	// uint256 mint
	dep.Mint = new(big.Int).SetBytes(opaqueData[offset : offset+8])
	// 0 mint is represented as nil to skip minting code
	if dep.Mint.Cmp(new(big.Int)) == 0 {
		dep.Mint = nil
	}
	offset += 8

	// uint256 value
	dep.Value = new(big.Int).SetBytes(opaqueData[offset : offset+8])
	offset += 8

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
