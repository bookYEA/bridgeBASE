package svm

import (
	"context"
	"encoding/hex"
	"fmt"

	"github.com/base/alt-l1-bridge/oracle/internal/flags"
	"github.com/blocto/solana-go-sdk/client"
	"github.com/blocto/solana-go-sdk/rpc"
	"github.com/blocto/solana-go-sdk/types"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

type TransactionRequest struct {
	Instructions []types.Instruction
	Signers      []types.Account
}

type Relayer struct {
	FeePayer types.Account
	Client   *client.Client
}

func NewRelayer(ctx *cli.Context) (*Relayer, error) {
	secretKeyBytes, err := hex.DecodeString(ctx.String(flags.SolSecretKeyFlag.Name))
	if err != nil {
		log.Error("Error decoding secret key bytes", "err", err)
		return &Relayer{}, err
	}

	feePayer, err := types.AccountFromBytes(secretKeyBytes)
	if err != nil {
		log.Error("Error creating fee payer account", "err", err)
		return &Relayer{}, err
	}
	log.Info("Solana signer registered", "Pubkey", feePayer.PublicKey.String())

	rpcEndpoint := rpc.DevnetRPCEndpoint
	if ctx.Bool(flags.IsMainnetFlag.Name) {
		rpcEndpoint = rpc.MainnetRPCEndpoint
	}

	return &Relayer{FeePayer: feePayer, Client: client.NewClient(rpcEndpoint)}, nil
}

func (r *Relayer) SubmitTransaction(req *TransactionRequest) error {
	if len(req.Instructions) == 0 {
		log.Error("error: missing svm transaction data")
		return fmt.Errorf("missing transaction data")
	}

	resp, err := r.Client.GetLatestBlockhash(context.Background())
	if err != nil {
		log.Error("error getting latest svm blockhash", "error", err)
		return fmt.Errorf("error getting latest svm blockhash: %w", err)
	}

	tx, err := types.NewTransaction(types.NewTransactionParam{
		Message: types.NewMessage(types.NewMessageParam{
			FeePayer:        r.FeePayer.PublicKey,
			RecentBlockhash: resp.Blockhash,
			Instructions:    req.Instructions,
		}),
		Signers: append(req.Signers, r.FeePayer),
	})
	if err != nil {
		log.Error("error building svm transaction", "error", err)
		return fmt.Errorf("error building svm transaction: %w", err)
	}

	sig, err := r.Client.SendTransaction(context.Background(), tx)
	if err != nil {
		log.Error("error sending svm transaction", "error", err)
		return fmt.Errorf("error sending svm transaction: %w", err)
	}

	log.Info("SVM transaction successful", "tx hash", sig)

	return nil
}
