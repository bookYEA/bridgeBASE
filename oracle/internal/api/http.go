package api

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"

	"github.com/base/alt-l1-bridge/oracle/internal/mmr"
	"github.com/ethereum/go-ethereum/log"
	"github.com/gorilla/mux"
)

type API struct {
	mmrInstance *mmr.MMR
	server      *http.Server
}

func NewAPI(mmrInstance *mmr.MMR) *API {
	return &API{mmrInstance: mmrInstance}
}

func (a *API) StartHTTPServer(listenAddr string) error {
	router := mux.NewRouter()
	router.HandleFunc("/proof/{leafIndex}", a.handleGenerateProof).Methods("GET")

	a.server = &http.Server{
		Addr:    listenAddr,
		Handler: router,
	}

	log.Info("Starting HTTP server", "listenAddr", listenAddr)
	if err := a.server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		return fmt.Errorf("HTTP server ListenAndServe: %w", err)
	}
	return nil
}

// Shutdown gracefully shuts down the HTTP server.
func (a *API) Shutdown(ctx context.Context) error {
	if a.server == nil {
		return fmt.Errorf("server not initialized")
	}
	log.Info("Attempting to shut down HTTP server gracefully...")
	if err := a.server.Shutdown(ctx); err != nil {
		return fmt.Errorf("HTTP server Shutdown: %w", err)
	}
	log.Info("HTTP server shut down gracefully.")
	return nil
}

func (a *API) handleGenerateProof(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	leafIndexStr, ok := vars["leafIndex"]
	if !ok {
		http.Error(w, "leafIndex not provided in path", http.StatusBadRequest)
		return
	}

	leafIndex, err := strconv.ParseUint(leafIndexStr, 10, 64)
	if err != nil {
		http.Error(w, fmt.Sprintf("Invalid leafIndex: %v", err), http.StatusBadRequest)
		return
	}

	log.Info("API generating proof", "leafIndex", leafIndex)

	if a.mmrInstance == nil {
		log.Error("MMR instance is not initialized in API handler")
		http.Error(w, "Internal server error: MMR not available", http.StatusInternalServerError)
		return
	}

	proof, err := a.mmrInstance.GenerateProof(leafIndex)
	if err != nil {
		log.Error("Error generating proof", "leafIndex", leafIndex, "error", err)
		http.Error(w, fmt.Sprintf("Error generating proof: %v", err), http.StatusInternalServerError)
		return
	}

	// Convert []mmr.Hash to [][]byte
	rawProof := make([][]byte, len(proof))
	for i, p := range proof {
		rawProof[i] = p // mmr.Hash is []byte, so this direct assignment works element-wise
	}

	log.Info("Proof generated", "rawProof", rawProof)

	response := struct {
		LeafIndex uint64   `json:"leafIndex"`
		Proof     [][]byte `json:"proof"`
	}{
		LeafIndex: leafIndex,
		Proof:     rawProof,
	}

	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(response); err != nil {
		log.Error("Error encoding response", "error", err)
		http.Error(w, "Error encoding response", http.StatusInternalServerError)
	}
}
