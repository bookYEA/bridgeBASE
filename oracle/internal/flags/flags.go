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
	BaseRpcUrlFlag = &cli.StringFlag{
		Name:     "base-rpc-url",
		Usage:    "RPC URL for Base",
		Required: true,
		EnvVars:  []string{"BASE_RPC_URL"},
	}
	PrivateKeyFlag = &cli.StringFlag{
		Name:     "private-key",
		Usage:    "Private key used to submit transactions to Base",
		Required: true,
		EnvVars:  []string{"PRIVATE_KEY"},
	}
	SolSecretKeyFlag = &cli.StringFlag{
		Name:     "sol-secret-key",
		Usage:    "Secret key used to submit transactions to Solana",
		Required: true,
		EnvVars:  []string{"SOL_SECRET_KEY"},
	}
)

var Flags = []cli.Flag{TargetAddressFlag, IsMainnetFlag, BaseRpcUrlFlag, PrivateKeyFlag, SolSecretKeyFlag}
