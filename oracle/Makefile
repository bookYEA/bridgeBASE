-include .env
export

SOLANA_PROGRAM_ID = Fb7KKBmjgKJh1N3aDUxLTj6TR3exH8Xi368bJ3AcDd5T
BASE_SEPOLIA_RPC_URL = https://base-sepolia.cbhq.net
BASE_MAINNET_RPC_URL = https://mainnet.base.org
BASE_MESSAGE_PASSER_ADDRESS = 0x2586a2AfeEdae8697cA732b271D23C594d990FF1
MESSAGE_PASSER_DEPLOYMENT_BLOCK_NUMBER = 26093417

.PHONY: bindings
bindings:
	go install github.com/ethereum/go-ethereum/cmd/abigen@v1.14.11
	cd ../base && forge build
	abigen --abi ../base/out/MessagePasser.sol/MessagePasser.abi.json --pkg bindings --type MessagePasser --out bindings/message_passer.go

.PHONY: run-dev
run-dev:
	go run ./cmd --target-address $(SOLANA_PROGRAM_ID) \
	--base-rpc-url $(BASE_SEPOLIA_RPC_URL) \
	--base-message-passer-address $(BASE_MESSAGE_PASSER_ADDRESS) \
	--message-passer-deployment-block-number $(MESSAGE_PASSER_DEPLOYMENT_BLOCK_NUMBER)

.PHONY: run-prod
run-prod:
	go run ./cmd --target-address $(SOLANA_PROGRAM_ID) --is-mainnet --base-rpc-url $(BASE_MAINNET_RPC_URL) --base-message-passer-address $(BASE_MESSAGE_PASSER_ADDRESS)
