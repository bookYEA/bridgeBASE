export MINT = EpGUaQN3ndd6LvY66kh4NxiStwmZHoApZWtwRMmn5SVS
export USER = 0x8C1a617BdB47342F9C17Ac8750E0b070c372C721
export ERC20_MINT = Ek8P6mDrJKVF7Nc2aaXvaBSoLuea4S9T6Xz99xi9iAjE

.PHONY: build
build:
	rm -rf target
	cd ../solana && anchor clean && anchor build
	mkdir target
	cp -r ../solana/target .
	cp -r ../base/deployments .

.PHONY: init
init:
	bun run src/initSolanaProgram.ts

.PHONY: create-mint
create-mint:
	bun run src/createMint.ts

.PHONY: create-bridge-token
create-bridge-token:
	bun run src/createBridgeToken.ts

.PHONY: create-bridge-token-user-ata
create-bridge-token-user-ata:
	bun run src/createBridgeTokenUserAccount.ts

.PHONY: mint-tokens
mint-tokens:
	bun run src/mintTokens.ts

.PHONY: bridge-sol-to-base
bridge-sol-to-base:
	bun run src/bridgeSolToBase.ts

.PHONY: bridge-spl-to-base
bridge-spl-to-base:
	bun run src/bridgeTokensToBase.ts

.PHONY: bridge-sol-to-solana
bridge-sol-to-solana:
	IS_SOL=true bun run src/orchestrateBridge.ts

.PHONY: bridge-spl-to-solana
bridge-spl-to-solana:
	bun run src/orchestrateBridge.ts

.PHONY: bridge-erc20-to-solana
bridge-erc20-to-solana:
	IS_ERC20=true bun run src/orchestrateBridge.ts

.PHONY: check-solana-spl-balance
check-solana-spl-balance:
	bun run src/checkSolanaBalance.ts

.PHONY: check-base-balance
check-base-balance:
	bun run src/checkBaseBalance.ts
