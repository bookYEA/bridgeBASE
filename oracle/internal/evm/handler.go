package evm

import (
	"github.com/base/alt-l1-bridge/oracle/bindings"
	"github.com/base/alt-l1-bridge/oracle/internal/mmr"
	"github.com/base/alt-l1-bridge/oracle/internal/signer"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

type EvmLogHandler struct {
	svmSigner *signer.SvmSigner
	mmr       *mmr.MMR
}

func NewLogHandler(ctx *cli.Context) (*EvmLogHandler, error) {
	r, err := signer.NewSvmSigner(ctx)
	if err != nil {
		log.Error("Error initializing SVM relayer", "error", err)
		return nil, err
	}

	return &EvmLogHandler{
		svmSigner: r,
		mmr:       mmr.NewMMR(),
	}, nil
}

func (h *EvmLogHandler) HandleLog(logRecv *bindings.MessagePasserMessagePassed) error {
	log.Info("Received Base log",
		"withdrawalHash", "0x"+common.Bytes2Hex(logRecv.WithdrawalHash[:]),
		"sender", logRecv.Sender,
		"blockNumber", logRecv.Raw.BlockNumber,
		"txHash", logRecv.Raw.TxHash,
	)
	root, err := h.mmr.Root()
	if err != nil {
		log.Error("Error getting starting MMR root", err)
	}

	log.Info("Starting MMR root", "root", "0x"+common.Bytes2Hex(root))

	h.mmr.Append(logRecv.WithdrawalHash[:])

	root, err = h.mmr.Root()
	if err != nil {
		log.Error("Error getting ending MMR root", err)
	}

	log.Info("Ending MMR root", "root", "0x"+common.Bytes2Hex(root))

	return nil
}
