package flags

import (
	"github.com/urfave/cli/v2"
)

var (
	TargetAddressFlag = &cli.StringFlag{
		Name:     "target-address",
		Usage:    "Solana address to monitor",
		Required: true,
		EnvVars:  []string{"TARGET_ADDRESS"},
	}
	IsMainnetFlag = &cli.BoolFlag{
		Name:     "is-mainnet",
		Usage:    "Subscribes to mainnet if prod environment",
		Required: false,
		EnvVars:  []string{"IS_MAINNET"},
	}
)

var Flags = []cli.Flag{TargetAddressFlag, IsMainnetFlag}
