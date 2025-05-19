package svm

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/base/alt-l1-bridge/oracle/internal/flags"
	"github.com/ethereum/go-ethereum/log"
	"github.com/gagliardetto/solana-go"
	"github.com/gagliardetto/solana-go/rpc"
	"github.com/gagliardetto/solana-go/rpc/ws"
	"github.com/urfave/cli/v2"
)

type SvmIndexer struct {
	stop        chan struct{}
	wg          sync.WaitGroup
	wsClient    *ws.Client
	programAddr solana.PublicKey
	handler     *SvmLogHandler
}

func NewIndexer(ctx *cli.Context) (*SvmIndexer, error) {
	wsUrl := rpc.DevNet_WS
	if ctx.Bool(flags.IsMainnetFlag.Name) {
		wsUrl = rpc.MainNetBeta_WS
	}

	// The targetAddrStr is now interpreted as the Program address
	targetAddrStr := ctx.String(flags.TargetAddressFlag.Name)

	programAddr, err := solana.PublicKeyFromBase58(targetAddrStr)
	if err != nil {
		log.Error("Invalid program address", "address", targetAddrStr, "err", err)
		return nil, err
	}

	handler, err := NewLogHandler(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize SVM log handler: %w", err)
	}

	wsClient, err := ws.Connect(ctx.Context, wsUrl)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to WebSocket %s: %w", wsUrl, err)
	}

	log.Info("WebSocket client connected", "url", wsUrl)
	log.Info("Starting Solana event indexer", "url", wsUrl, "program", programAddr.String())

	return &SvmIndexer{
		stop:        make(chan struct{}),
		wsClient:    wsClient,
		programAddr: programAddr,
		handler:     handler,
	}, nil
}

// startSolanaIndexer connects to the Solana WebSocket endpoint and subscribes to program logs.
func (i *SvmIndexer) Start(ctx context.Context) error {
	defer i.wsClient.Close()

	i.wg.Add(1)

	// Subscribe to logs mentioning the program address
	sub, err := i.wsClient.LogsSubscribeMentions(i.programAddr, rpc.CommitmentFinalized)
	if err != nil {
		return fmt.Errorf("failed to subscribe to logs for program %s: %w", i.programAddr.String(), err)
	}
	defer sub.Unsubscribe()
	log.Info("Subscribed to program logs", "program", i.programAddr.String())

	return i.loop(ctx, sub)
}

func (i *SvmIndexer) Stop() {
	close(i.stop)
	i.wg.Wait()
}

func (i *SvmIndexer) loop(ctx context.Context, sub *ws.LogSubscription) error {
	defer i.wg.Done()
	for {
		// Select on stop channel, context cancellation, or proceed with receiving logs.
		select {
		case <-i.stop:
			log.Info("SvmIndexer.stop channel closed, stopping indexer.")
			return nil
		case <-ctx.Done():
			log.Info("Context passed to SvmIndexer.Start is done, stopping indexer.")
			return ctx.Err() // Propagate context error
		default:
			// Non-blocking check before potentially blocking Recv
		}

		// Use a timeout for Recv, and allow the loop to reiterate to check stop signals.
		recvCtx, cancelRecv := context.WithTimeout(ctx, 5*time.Second) // Shorter timeout for responsiveness
		got, err := sub.Recv(recvCtx)
		cancelRecv() // Release resources associated with recvCtx

		if err != nil {
			if err == context.DeadlineExceeded {
				// Timeout is expected, loop will check i.stop and ctx.Done() again.
				continue
			}

			// If the main context (ctx) for Start is already done, this is part of shutdown.
			if ctx.Err() != nil {
				log.Info("Context cancelled while receiving, stopping indexer.", "recv_err", err)
				return nil // Normal shutdown if context is cancelled
			}

			if err == ws.ErrSubscriptionClosed {
				log.Warn("WebSocket subscription closed unexpectedly.", "err", err)
				// Depending on desired behavior, could attempt reconnect or just exit.
				return fmt.Errorf("subscription closed: %w", err)
			}

			log.Error("Error receiving log update", "err", err)
			return fmt.Errorf("error receiving log update: %w", err) // Treat other errors as fatal
		}

		err = i.handler.HandleLogs(got)
		if err != nil {
			return err
		}
	}
}
