package evm

import (
	"github.com/base/alt-l1-bridge/oracle/bindings"
	"github.com/base/alt-l1-bridge/oracle/internal/signer"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

type EvmLogHandler struct {
	svmSigner *signer.SvmSigner
}

func NewLogHandler(ctx *cli.Context) (*EvmLogHandler, error) {
	r, err := signer.NewSvmSigner(ctx)
	if err != nil {
		log.Error("Error initializing SVM relayer", "error", err)
		return nil, err
	}

	return &EvmLogHandler{
		svmSigner: r,
	}, nil
}

func (h *EvmLogHandler) HandleLog(logRecv *bindings.MessagePasserMessagePassed) error {
	log.Info("Received Base log", "log", logRecv)
	return nil
}
