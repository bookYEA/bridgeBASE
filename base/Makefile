-include .env

BASE_RPC=https://base-sepolia.cbhq.net
# BASE_RPC=https://mainnet.base.org

BRIDGE = 0x7422A202FBDaef2Dd48E20B7727b56Eb337Ab323

LOCAL_SOL = 0x5143A6928F5Ea8792515FDfC61DbD3196CAb6783
REMOTE_SOL = 0x069be72ab836d4eacc02525b7350a78a395da2f1253a40ebafd6630000000000

LOCAL_SPL = 0xa05Edbf9b5b42729c931e73257F07F8EF9eCC250
REMOTE_SPL = 0x6359542ed3ab0ce8b30eb0cb05a3258e7a40aaf09a9670fc84c09cb5e872f20f
USER_SPL_ATA = 0x166e5ff853d090dc9efdb35fcb085e109e91766f0f0cba4b54b9f4b52408bd30

LOCAL_ERC20 = 0x62C1332822983B8412A6Ffda0fd77cd7d5733Ee9
REMOTE_ERC20 = 0xd3a7769953f7fb046eb4798298fcc95464633ac2431115abd9466b2e5e595637
USER_REMOTE_ERC20_ATA = 0x9012c89076034a0fa661a446930bbbbea61967cc0aecf53f7ef1f979b6981d22

LOCAL_ETH = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
REMOTE_ETH = 0xfd77b603f3afb749a8f12944db51cdef33ed6c23b5c7b96a6f5b4c0ac5cd37a6
USER_REMOTE_ETH_ATA = 0xbdcf27b8fab01afd8e6eae1784baeaf99cb6f3059b42c33e83ec5409ac5fc307

SOLANA_SOL_RECEIVER = 0x82c9f09a109bce580bb82c13c310689fd00e2225f8dd22015271620ecc035221

.PHONY: deps
deps: clean-lib forge-deps

.PHONY: clean-lib
clean-lib:
	rm -rf lib

.PHONY: forge-deps
forge-deps:
	forge install --no-git github.com/foundry-rs/forge-std \
		github.com/Vectorized/solady@92e647bb2b387e952480a5a37800e9b4d54f7a06

.PHONY: coverage
coverage:
	@ forge coverage --no-match-coverage "(script|test)"

.PHONY: dev-deploy
dev-deploy: deploy create-wrapped-sol create-token

.PHONY: deploy
deploy:
	forge script DeployScript --account testnet-admin --rpc-url $(BASE_RPC) --sender $(shell cast wallet address --account testnet-admin) --broadcast -vvvv

.PHONY: create-wrapped-sol
create-wrapped-sol:
	TOKEN_NAME=WrappedSOL REMOTE_TOKEN=$(REMOTE_SOL) forge script CreateTokenScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: create-token
create-token:
	TOKEN_NAME=WrappedSPL REMOTE_TOKEN=$(REMOTE_SPL) forge script CreateTokenScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: create-mock-token
create-mock-token:
	ADMIN=0x8C1a617BdB47342F9C17Ac8750E0b070c372C721 forge script DeployERC20 --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: bridge-sol-to-solana
bridge-sol-to-solana:
	LOCAL_TOKEN=$(LOCAL_SOL) \
	REMOTE_TOKEN=$(REMOTE_SOL) \
	TO=$(SOLANA_SOL_RECEIVER) \
	AMOUNT=1000000 \
	forge script BridgeTokensToSolanaScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: bridge-tokens-to-solana
bridge-tokens-to-solana:
	LOCAL_TOKEN=$(LOCAL_SPL) \
	REMOTE_TOKEN=$(REMOTE_SPL) \
	TO=$(USER_SPL_ATA) \
	AMOUNT=1000000 \
	forge script BridgeTokensToSolanaScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: bridge-erc20-to-solana
bridge-erc20-to-solana:
	LOCAL_TOKEN=$(LOCAL_ERC20) \
	REMOTE_TOKEN=$(REMOTE_ERC20) \
	TO=$(USER_REMOTE_ERC20_ATA) \
	AMOUNT=1000000000000 \
	NEEDS_APPROVAL=true \
	forge script BridgeTokensToSolanaScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: bridge-eth-to-solana
bridge-eth-to-solana:
	LOCAL_TOKEN=$(LOCAL_ETH) \
	REMOTE_TOKEN=$(REMOTE_ETH) \
	TO=$(USER_REMOTE_ETH_ATA) \
	AMOUNT=1000000000000 \
	forge script BridgeTokensToSolanaScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: check-root
check-root:
	@ cast call $(BRIDGE) "getRoot()" --rpc-url https://base-sepolia-dev.cbhq.net:8545

.PHONY: check-count
check-count:
	@ cast call 0xCdfe10f911eD5039E031D6a7be3a0F97fA061C38 "count()" --rpc-url https://base-sepolia-dev.cbhq.net:8545

.PHONY: log-args
log-args:
	forge script LogArgsScript --rpc-url $(BASE_RPC) --broadcast -vvvv
