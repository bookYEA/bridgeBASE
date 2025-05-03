package oracle

import (
	"os"

	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

func Main(ctx *cli.Context) error {
	log.SetDefault(log.NewLogger(log.NewTerminalHandlerWithLevel(os.Stderr, log.LevelInfo, true)))

	log.Info("Successfully started")

	return nil
}
