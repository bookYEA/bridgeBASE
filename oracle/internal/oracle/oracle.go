package oracle

import (
	"context"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/base/alt-l1-bridge/oracle/internal/api"
	"github.com/base/alt-l1-bridge/oracle/internal/evm"
	"github.com/base/alt-l1-bridge/oracle/internal/flags"
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

	// Setup and start HTTP API
	httpAPI := api.NewAPI(evmIndexer.GetHandler().GetMMR())
	httpListenAddr := ctx.String(flags.HTTPListenAddrFlag.Name)

	wg.Add(1)
	go func() {
		defer wg.Done()
		log.Info("Starting HTTP API server...")
		if err := httpAPI.StartHTTPServer(httpListenAddr); err != nil {
			log.Error("HTTP API server returned an error, initiating shutdown", "err", err)
			stop()
		}
		log.Info("HTTP API server goroutine finished.")
	}()

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
		log.Info("Shutdown signal received by cleanup goroutine, calling stops...")
		// Stop indexers
		if svmIndexer != nil {
			svmIndexer.Stop()
		}
		if evmIndexer != nil {
			evmIndexer.Stop()
		}

		// Gracefully shut down the HTTP server
		if httpAPI != nil {
			// Create a timeout context for the server shutdown
			shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second) // 10 seconds timeout
			defer cancel()

			if err := httpAPI.Shutdown(shutdownCtx); err != nil {
				log.Error("HTTP server graceful shutdown failed", "err", err)
			} else {
				log.Info("HTTP server shutdown initiated.")
			}
		}

		log.Info("Indexer Stop() methods and HTTP server shutdown called by cleanup goroutine.")
	}()

	c := make(chan os.Signal, 1)
	signal.Notify(c, os.Interrupt, syscall.SIGTERM)

	select {
	case sig := <-c:
		log.Info("Received OS signal, initiating shutdown...", "signal", sig.String())
	case <-stopped.Done():
		log.Info("Shutdown initiated internally (e.g., Indexer.Start or HTTP server failed or completed).")
	}

	log.Info("Shutting down main application components...")
	stop()

	log.Info("Waiting for services to gracefully stop...")
	wg.Wait()

	log.Info("All services stopped. Oracle exiting.")
	return nil
}
