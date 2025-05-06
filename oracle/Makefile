-include .env
export

SOLANA_PROGRAM_ID = Fb7KKBmjgKJh1N3aDUxLTj6TR3exH8Xi368bJ3AcDd5T
BASE_SEPOLIA_RPC_URL = https://base-sepolia.cbhq.net
BASE_MAINNET_RPC_URL = https://mainnet.base.org

.PHONY: run-dev
run-dev:
	go run ./cmd --target-address $(SOLANA_PROGRAM_ID) --base-rpc-url $(BASE_SEPOLIA_RPC_URL)

.PHONY: run-prod
run-prod:
	go run ./cmd --target-address $(SOLANA_PROGRAM_ID) --is-mainnet --base-rpc-url $(BASE_MAINNET_RPC_URL)
