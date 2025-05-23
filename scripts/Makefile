.PHONY: build
build:
	rm -rf target
	cd ../solana && anchor clean && anchor build
	mkdir target
	cp -r ../solana/target .

.PHONY: init
init:
	bun run init.ts

.PHONY: bridge-sol-to-base
bridge-sol-to-base:
	bun run index.ts

.PHONY: create-mint
create-mint:
	bun run src/createMint.ts

.PHONY: mint-tokens
mint-tokens:
	bun run src/mintTokens.ts

.PHONY: bridge-tokens-to-base
bridge-tokens-to-base:
	bun run src/bridgeTokensToBase.ts

.PHONY: bridge-to-solana
bridge-to-solana:
	bun run src/orchestrateBridge.ts

.PHONY: check-solana-spl-balance
check-solana-spl-balance:
	bun run src/checkSolanaBalance.ts

.PHONY: check-base-balance
check-base-balance:
	bun run src/checkBaseBalance.ts
