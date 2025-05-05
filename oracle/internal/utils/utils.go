package utils

import (
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/gagliardetto/solana-go"
)

func SolanaPubkeyToEvmAddress(key solana.PublicKey) common.Address {
	return common.Address(crypto.Keccak256(key.Bytes()))
}
