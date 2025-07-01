-include .env

BASE_RPC=https://base-sepolia.cbhq.net
# BASE_RPC=https://mainnet.base.org

BRIDGE = 0xDEf3d2717A1E11b5bd9F8B1b46D988a678282cC3

LOCAL_SOL = 0xFF1f35657E32bE8cA75682D72EB91FD2EbD4cBE0
REMOTE_SOL = 0x069be72ab836d4eacc02525b7350a78a395da2f1253a40ebafd6630000000000

LOCAL_SPL = 0x39CB8063F3a42cAacf9963678cA2AA363387484d
REMOTE_SPL = 0x6359542ed3ab0ce8b30eb0cb05a3258e7a40aaf09a9670fc84c09cb5e872f20f
USER_SPL_ATA = 0x166e5ff853d090dc9efdb35fcb085e109e91766f0f0cba4b54b9f4b52408bd30

LOCAL_ERC20 = 0x62C1332822983B8412A6Ffda0fd77cd7d5733Ee9
REMOTE_ERC20 = 0xcba7e05bf37dc99baccc0eea6af1f78e8639a601b6e84bf360ceab2486bb5f28
USER_REMOTE_ERC20_ATA = 0xea5ab330243767ac5f2c6036ade7dfaf58fc9ac86b38a2a7414c7a0e3d55f4c6

LOCAL_ETH = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
REMOTE_ETH = 0xa116c5dbe31944b35544dd2eb01185a7b4e4c01755c2c89b13c1147aa2ed4533
USER_REMOTE_ETH_ATA = 0x0072f3236243d650900abf49184d6cfa69c69fc615f378a15b503ddb1c5c5635

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
	TO=0x08bf17f6e4654896db9c5dfe39ce87574be08c0cfc3af8f664676b163f7f700e \
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
