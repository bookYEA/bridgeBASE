package oracle

import (
	"context"
	"os"
	"os/signal"
	"sync"
	"syscall"

	"github.com/base/alt-l1-bridge/oracle/internal/evm"
	"github.com/base/alt-l1-bridge/oracle/internal/svm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/urfave/cli/v2"
)

func Main(ctx *cli.Context) error {
	log.SetDefault(log.NewLogger(log.NewTerminalHandlerWithLevel(os.Stderr, log.LevelInfo, true)))

	var wg sync.WaitGroup
	stopped, stop := context.WithCancel(context.Background())

	svmIndexer, err := svm.NewIndexer(ctx)
	if err != nil {
		log.Crit("Error creating SVM indexer", "error", err)
	}

	evmIndexer, err := evm.NewIndexer(ctx)
	if err != nil {
		log.Crit("Error creating EVM indexer", "error", err)
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
		log.Info("Starting EVM Indexer goroutine")
		startErr := evmIndexer.Start(ctx.Context)
		if startErr != nil {
			log.Error("EVM indexer returned an error, initiating shutdown", "err", startErr)
			stop()
		}
		log.Info("EVM Indexer goroutine finished.")
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()
		<-stopped.Done()
		log.Info("Shutdown signal received by cleanup goroutine, calling svmIndexer.Stop()...")
		svmIndexer.Stop()
		evmIndexer.Stop()
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
