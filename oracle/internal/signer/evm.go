package signer

import (
	"context"
	"crypto/ecdsa"
	"fmt"
	"math/big"

	"github.com/base/alt-l1-bridge/oracle/internal/flags"
	localTypes "github.com/base/alt-l1-bridge/oracle/internal/types"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

type EvmSigner struct {
	client     *ethclient.Client
	privateKey *ecdsa.PrivateKey
	address    common.Address
	chainId    *big.Int
}

func NewEvmSigner(ctx *cli.Context) (*EvmSigner, error) {
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

	log.Info("EVM signer registered", "address", crypto.PubkeyToAddress(*publicKeyECDSA))

	return &EvmSigner{
		client:     client,
		privateKey: key,
		address:    crypto.PubkeyToAddress(*publicKeyECDSA),
		chainId:    chainId,
	}, nil
}

func (r *EvmSigner) SendTransactionToBase(dep localTypes.DepositTx) error {
	nonce, err := r.client.PendingNonceAt(context.Background(), r.address)
	if err != nil {
		return err
	}

	suggestedGasPrice, err := r.client.SuggestGasPrice(context.Background())
	if err != nil {
		return err
	}
	// Increase suggested gas price by 20%
	gasPrice := new(big.Int).Mul(suggestedGasPrice, big.NewInt(120))
	gasPrice = new(big.Int).Div(gasPrice, big.NewInt(100))
	log.Info("Gas price details", "suggested", suggestedGasPrice, "used", gasPrice)

	balance, err := r.client.BalanceAt(context.Background(), r.address, nil)
	if err != nil {
		log.Warn("Failed to get account balance", "address", r.address.Hex(), "err", err)
		// Continue even if balance check fails, but log it.
	} else {
		log.Info("Account balance", "address", r.address.Hex(), "balance", balance)
		// Basic check if balance can cover gas, assuming value is zero or handled elsewhere.
		// This is a very rough check: (gasLimit * gasPrice)
		cost := new(big.Int).Mul(big.NewInt(int64(dep.Gas)), gasPrice)
		if balance.Cmp(cost) < 0 {
			log.Error("Insufficient balance for gas", "balance", balance, "estimatedCost", cost)
			return fmt.Errorf("insufficient balance for gas: got %s, need at least %s", balance.String(), cost.String())
		}
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

	log.Info("Transaction submitted, waiting for receipt...", "hash", signedTx.Hash().Hex())

	receipt, err := bind.WaitMined(context.Background(), r.client, signedTx)
	if err != nil {
		log.Error("Error waiting for transaction to be mined", "hash", signedTx.Hash().Hex(), "err", err)
		return fmt.Errorf("error waiting for transaction to be mined: %w", err)
	}

	if receipt.Status == types.ReceiptStatusSuccessful {
		log.Info("Transaction successfully mined", "hash", signedTx.Hash().Hex(), "blockHash", receipt.BlockHash.Hex(), "blockNumber", receipt.BlockNumber)
	} else {
		log.Error("Transaction failed to mine", "hash", signedTx.Hash().Hex(), "status", receipt.Status, "blockHash", receipt.BlockHash.Hex(), "blockNumber", receipt.BlockNumber)
		return fmt.Errorf("transaction %s failed to mine with status %d", signedTx.Hash().Hex(), receipt.Status)
	}

	log.Info("Transaction sent", "hash", signedTx.Hash().Hex())

	return nil
}
