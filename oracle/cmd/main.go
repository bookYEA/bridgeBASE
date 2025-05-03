package main

import (
	"os"

	"github.com/base/alt-l1-bridge/oracle/internal/flags"
	"github.com/base/alt-l1-bridge/oracle/internal/oracle"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

func main() {
	app := &cli.App{
		Name:    "oracle",
		Version: "0.0.1",
		Usage:   "Base bridge oracle",
		Flags:   flags.Flags,
		Action:  oracle.Main,
	}

	if err := app.Run(os.Args); err != nil {
		log.Crit("Failed to run app", "error", err)
	}
}
