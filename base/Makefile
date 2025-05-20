-include .env

BASE_RPC=https://base-sepolia.cbhq.net
# BASE_RPC=https://mainnet.base.org

LOCAL_SOL = 0x6B8C683eC15bf08adf9EF921942c10cae89De475
REMOTE_SOL = 0x0501550155015501550155015501550155015501550155015501550155015501
REMOTE_SPL = 0xcd43b9e91032f9346b89b99c2155981f799ebf0d8cf4f56b03dbec65bdd6bbe5

.PHONY: deps
deps: clean-lib forge-deps checkout-op-commit

.PHONY: clean-lib
clean-lib:
	rm -rf lib

.PHONY: forge-deps
forge-deps:
	forge install --no-git github.com/foundry-rs/forge-std \
		github.com/Vectorized/solady@92e647bb2b387e952480a5a37800e9b4d54f7a06 \
		github.com/OpenZeppelin/openzeppelin-contracts@v4.9.3 \
		github.com/OpenZeppelin/openzeppelin-contracts-upgradeable@v4.7.3

.PHONY: checkout-op-commit
checkout-op-commit:
	[ -n "$(OP_COMMIT)" ] || (echo "OP_COMMIT must be set in .env" && exit 1)
	rm -rf lib/optimism
	mkdir -p lib/optimism
	cd lib/optimism; \
	git init; \
	git remote add origin https://github.com/ethereum-optimism/optimism.git; \
	git fetch --depth=1 origin $(OP_COMMIT); \
	git reset --hard FETCH_HEAD

.PHONY: deploy
deploy:
	forge script DeployScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: create-wrapped-sol
create-wrapped-sol:
	REMOTE_TOKEN=$(REMOTE_SOL) forge script CreateTokenScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: create-token
create-token:
	REMOTE_TOKEN=$(REMOTE_SPL) forge script CreateTokenScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv

.PHONY: bridge-sol-to-solana
bridge-sol-to-solana:
	LOCAL_TOKEN=$(LOCAL_SOL) \
	REMOTE_TOKEN=$(REMOTE_SOL) \
	TO=0x9827993b7b317eb6c74279d49074a13b33a3495a30637c86a7513922014ba424 \
	AMOUNT=1000000 \
	forge script BridgeTokensToSolanaScript --account testnet-admin --rpc-url $(BASE_RPC) --broadcast -vvvv
