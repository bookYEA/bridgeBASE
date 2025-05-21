package evm

import (
	"context"
	"math/big"
	"regexp"
	"sync"
	"time"

	"github.com/base/alt-l1-bridge/oracle/bindings"
	"github.com/base/alt-l1-bridge/oracle/internal/flags"
	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/ethclient"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

type EvmIndexer struct {
	messagePasser *bindings.MessagePasser
	handler       *EvmLogHandler
	logs          chan *bindings.MessagePasserMessagePassed
	stop          chan struct{}
	wg            sync.WaitGroup
	pollRate      time.Duration
	pollReqCh     chan struct{}
	polling       bool
	startingBlock uint64
}

var httpRegex = regexp.MustCompile("^http(s)?://")

func NewIndexer(ctx *cli.Context) (*EvmIndexer, error) {
	client, err := ethclient.Dial(ctx.String(flags.BaseRpcUrlFlag.Name))
	if err != nil {
		return nil, err
	}

	contractAddress := common.HexToAddress(ctx.String(flags.BaseMessagePasserAddressFlag.Name))
	messagePasser, err := bindings.NewMessagePasser(contractAddress, client)
	if err != nil {
		log.Error("Failed to create MessagePasser binding", "error", err)
		return nil, err
	}

	handler, err := NewLogHandler(ctx)
	if err != nil {
		log.Error("Failed to initialize EVM log handler", "error", err)
		return nil, err
	}

	return &EvmIndexer{
		messagePasser: messagePasser,
		handler:       handler,
		logs:          make(chan *bindings.MessagePasserMessagePassed),
		stop:          make(chan struct{}),
		pollReqCh:     make(chan struct{}, 1),
		pollRate:      3 * time.Second,
		polling:       httpRegex.MatchString(ctx.String(flags.BaseRpcUrlFlag.Name)),
		startingBlock: ctx.Uint64(flags.MessagePasserDeploymentBlockNumber.Name),
	}, nil
}

func (i *EvmIndexer) Start(ctx context.Context) error {
	if i.polling {
		return i.pollListener(ctx)
	}

	return i.webSocketListener()
}

func (i *EvmIndexer) webSocketListener() error {
	sub, err := i.messagePasser.WatchMessagePassed(&bind.WatchOpts{Start: &i.startingBlock}, i.logs, []*big.Int{}, []common.Address{})
	if err != nil {
		log.Error("Failed to subscribe to logs", "error", err)
		return err
	}

	log.Info("Subscribed to logs")

	i.wg.Add(1)
	go i.loop(sub)

	return nil
}

func (i *EvmIndexer) pollListener(ctx context.Context) error {
	log.Info("Polling for logs")
	reqPollAfter := func() {
		if i.pollRate == 0 {
			return
		}
		time.AfterFunc(i.pollRate, i.reqPoll)
	}

	reqPollAfter()

	for {
		select {
		case <-i.pollReqCh:
			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			logIterator, err := i.messagePasser.FilterMessagePassed(&bind.FilterOpts{Context: ctx, Start: i.startingBlock}, []*big.Int{}, []common.Address{})
			if err != nil {
				log.Error("failed to filter logs", "error", err)
				cancel()
				logIterator.Close()
				reqPollAfter()
				continue
			}

			maxBlockNumber := uint64(0)

			for logIterator.Next() {
				err := logIterator.Error()
				if err != nil {
					log.Error("error iterating over logs", "error", err)
					continue
				}

				logRecv := logIterator.Event
				err = i.handler.HandleLog(logRecv)
				if err != nil {
					log.Error("failed to handle log", "error", err)
					continue
				}

				maxBlockNumber = max(maxBlockNumber, logRecv.Raw.BlockNumber)
			}

			if maxBlockNumber > 0 {
				i.startingBlock = maxBlockNumber + 1
			}

			cancel()
			logIterator.Close()
			reqPollAfter()
		case <-i.stop:
			log.Info("EVM indexer channel closed, stopping indexer.")
			return nil
		case <-ctx.Done():
			log.Info("Context passed to SvmIndexer.Start is done, stopping indexer.")
			return ctx.Err() // Propagate context error
		}
	}
}

func (i *EvmIndexer) loop(sub ethereum.Subscription) {
	defer i.wg.Done()
	for {
		select {
		case err := <-sub.Err():
			log.Info("Subscription error", "error", err)
		case recvLog := <-i.logs:
			log.Info("Log received!")
			log.Info("Log Block Number", "blockNumber", recvLog.Raw.BlockNumber)
			log.Info("Log Index", "index", recvLog.Raw.Index)

			err := i.handler.HandleLog(recvLog)
			if err != nil {
				log.Error("Error handling log", "error", err)
			}
		case <-i.stop:
			sub.Unsubscribe()
			return
		}
	}
}

func (i *EvmIndexer) Stop() {
	close(i.stop)
	i.wg.Wait()
}

func (i *EvmIndexer) reqPoll() {
	i.pollReqCh <- struct{}{}
}
