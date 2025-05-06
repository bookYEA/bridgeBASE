package relayer

import (
	"context"
	"crypto/ecdsa"
	"fmt"
	"math/big"

	"github.com/base/alt-l1-bridge/oracle/internal/flags"
	localTypes "github.com/base/alt-l1-bridge/oracle/internal/types"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

type Relayer struct {
	client     *ethclient.Client
	privateKey *ecdsa.PrivateKey
	address    common.Address
	chainId    *big.Int
}

func New(ctx *cli.Context) (*Relayer, error) {
	client, err := ethclient.Dial(ctx.String(flags.BaseRpcUrlFlag.Name))
	if err != nil {
		return nil, err
	}

	key, err := crypto.HexToECDSA(ctx.String(flags.PrivateKeyFlag.Name))
	if err != nil {
		return nil, err
	}

	publicKey := key.Public()
	publicKeyECDSA, ok := publicKey.(*ecdsa.PublicKey)
	if !ok {
		return nil, fmt.Errorf("error casting public key to ECDSA: %s", publicKey)
	}

	chainId, err := client.NetworkID(context.Background())
	if err != nil {
		return nil, err
	}

	return &Relayer{
		client:     client,
		privateKey: key,
		address:    crypto.PubkeyToAddress(*publicKeyECDSA),
		chainId:    chainId,
	}, nil
}

func (r *Relayer) SendTransactionToBase(dep localTypes.DepositTx) error {
	nonce, err := r.client.PendingNonceAt(context.Background(), r.address)
	if err != nil {
		return err
	}

	gasPrice, err := r.client.SuggestGasPrice(context.Background())
	if err != nil {
		return err
	}

	tx := types.NewTransaction(nonce, *dep.To, dep.Value, dep.Gas, gasPrice, dep.Data)

	signedTx, err := types.SignTx(tx, types.NewEIP155Signer(r.chainId), r.privateKey)
	if err != nil {
		return err
	}

	log.Info("Sending transaction")
	err = r.client.SendTransaction(context.Background(), signedTx)
	if err != nil {
		return err
	}

	log.Info("Transaction sent", "hash", tx.Hash().Hex())

	return nil
}
