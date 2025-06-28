-include .env

BASE_RPC=https://base-sepolia.cbhq.net
# BASE_RPC=https://mainnet.base.org

LOCAL_SOL = 0x96A91f8C455390e220A4Efb0e2F9aC36dC536fC8
REMOTE_SOL = 0x0501550155015501550155015501550155015501550155015501550155015501

LOCAL_SPL = 0x184Dfb2276CeB179960acbB6210b5a27544D9700
REMOTE_SPL = 0xcd43b9e91032f9346b89b99c2155981f799ebf0d8cf4f56b03dbec65bdd6bbe5
USER_SPL_ATA = 0x1e1112994ab6232a643fd9d1cff130f5b829ac336b910b91c6304a96f776fd9c

LOCAL_ERC20 = 0x62C1332822983B8412A6Ffda0fd77cd7d5733Ee9
REMOTE_ERC20 = 0xcc3440262c62c93feec031ae37197e0e940947ab70c30709a3634316e17bad1d
USER_REMOTE_ERC20_ATA = 0xd0444439bbe07a2edbdf4c683cbf02ff75c870e649db23f5ec69f56ec25c0501

LOCAL_ETH = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
REMOTE_ETH = 0x51ed5f7171ba6789605bbaf67beee412038532f616679f21be97867d95113ab8
USER_REMOTE_ETH_ATA = 0x08933bf64da91e85dcaaac6e5a8593e7c8556d93b60162cf0e2e9d60e50c9314

.PHONY: deps
deps: clean-lib forge-deps

.PHONY: clean-lib
clean-lib:
	rm -rf lib

.PHONY: forge-deps
forge-deps:
	forge install --no-git github.com/foundry-rs/forge-std \
		github.com/Vectorized/solady@92e647bb2b387e952480a5a37800e9b4d54f7a06

.PHONY: dev-deploy
dev-deploy: deploy create-wrapped-sol create-token

.PHONY: deploy
deploy:
	forge script DeployScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

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
	TO=0x9827993b7b317eb6c74279d49074a13b33a3495a30637c86a7513922014ba424 \
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
	AMOUNT=1000000 \
	NEEDS_APPROVAL=true \
	forge script BridgeTokensToSolanaScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: bridge-eth-to-solana
bridge-eth-to-solana:
	LOCAL_TOKEN=$(LOCAL_ETH) \
	REMOTE_TOKEN=$(REMOTE_ETH) \
	TO=$(USER_REMOTE_ETH_ATA) \
	AMOUNT=1000000 \
	forge script BridgeTokensToSolanaScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv
