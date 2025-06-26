.PHONY: deps
deps:
	yarn install

.PHONY: build-localnet
build-localnet:
	anchor build -- --features localnet

.PHONY: build-devnet
build-devnet:
	anchor build -- --features devnet

.PHONY: test
test:
	cargo test

.PHONY: deploy
deploy:
	anchor deploy
