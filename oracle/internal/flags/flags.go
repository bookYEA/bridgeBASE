package flags

import (
	"github.com/urfave/cli/v2"
)

var (
	MockFlag = &cli.StringFlag{
		Name:     "mock-flag",
		Usage:    "This is a dummy mock flag",
		EnvVars:  []string{"MOCK_FLAG"},
		Required: true,
	}
)

var Flags = []cli.Flag{MockFlag}
