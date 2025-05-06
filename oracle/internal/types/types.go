package types

import (
	"math/big"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/log"
)

type DepositTx struct {
	// SourceHash uniquely identifies the source of the deposit
	SourceHash common.Hash
	// From is exposed through the types.Signer, not through TxData
	From common.Address
	// nil means contract creation
	To *common.Address `rlp:"nil"`
	// Mint is minted on L2, locked on L1, nil if no minting.
	Mint *big.Int `rlp:"nil"`
	// Value is transferred from L2 balance, executed after Mint (if any)
	Value *big.Int
	// gas limit
	Gas uint64
	// Field indicating if this transaction is exempt from the L2 gas limit.
	IsSystemTransaction bool
	// Normal Tx data
	Data []byte
}

func (t *DepositTx) Print() {
	log.Info(
		"----Decoded Transaction----",
		"SourceHash", t.SourceHash,
		"From", t.From,
		"To", t.To,
		"Mint", t.Mint,
		"Value", t.Value,
		"Gas", t.Gas,
		"IsSystemTransaction", t.IsSystemTransaction,
		"Data", common.Bytes2Hex(t.Data),
	)
}
