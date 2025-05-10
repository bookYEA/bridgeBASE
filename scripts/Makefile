.PHONY: build
build:
	rm -rf target
	cd ../solana && anchor clean && anchor build
	mkdir target
	cp -r ../solana/target .

.PHONY: bridge-sol-to-base
bridge-sol-to-base:
	bun run index.ts
