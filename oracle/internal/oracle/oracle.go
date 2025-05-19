package oracle

import (
	"context"
	"os"
	"os/signal"
	"sync"
	"syscall"

	"github.com/base/alt-l1-bridge/oracle/internal/svm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

func Main(ctx *cli.Context) error {
	log.SetDefault(log.NewLogger(log.NewTerminalHandlerWithLevel(os.Stderr, log.LevelInfo, true)))

	var wg sync.WaitGroup
	stopped, stop := context.WithCancel(context.Background())

	_, err := svm.NewRelayer(ctx)
	if err != nil {
		log.Crit("Error creating solana signer", "err", err)
	}

	svmIndexer, err := svm.NewIndexer(ctx)
	if err != nil {
		log.Crit("Error creating SVM indexer", "err", err)
	}

	wg.Add(1)
	go func() {
		defer wg.Done()
		log.Info("svmIndexer.Start goroutine starting...")
		startErr := svmIndexer.Start(ctx.Context)
		if startErr != nil {
			log.Error("svmIndexer.Start returned an error, initiating shutdown", "err", startErr)
			stop()
		}
		log.Info("svmIndexer.Start goroutine finished.")
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()
		<-stopped.Done()
		log.Info("Shutdown signal received by cleanup goroutine, calling svmIndexer.Stop()...")
		svmIndexer.Stop()
		log.Info("svmIndexer.Stop() called by cleanup goroutine.")
	}()

	c := make(chan os.Signal, 1)
	signal.Notify(c, os.Interrupt, syscall.SIGTERM)

	select {
	case sig := <-c:
		log.Info("Received OS signal, initiating shutdown...", "signal", sig.String())
	case <-stopped.Done():
		log.Info("Shutdown initiated internally (e.g., Indexer.Start failed or completed).")
	}

	log.Info("Shutting down...")
	stop()

	log.Info("Waiting for services to gracefully stop...")
	wg.Wait()

	log.Info("All services stopped. Oracle exiting.")
	return nil
}
