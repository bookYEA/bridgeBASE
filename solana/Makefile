define build_bridge
	@echo "🔨 Building bridge for $(1) $(2)..."
	@echo "📁 Backing up Anchor.toml"
	cp Anchor.toml Anchor.toml.backup
	@echo "📁 Backing up lib.rs"
	cp programs/bridge/src/lib.rs programs/bridge/src/lib.rs.backup
	@if [ -f target/deploy/bridge-keypair.json ]; then echo "📁 Backing up bridge-keypair.json" && cp target/deploy/bridge-keypair.json target/deploy/bridge-keypair.json.backup; fi
	@echo "🔑 Using keypair: target/deploy/bridge.$(1).$(2).json"
	cp target/deploy/bridge.$(1).$(2).json target/deploy/bridge-keypair.json
	@echo "🔄 Syncing keys..."
	anchor keys sync
	@echo "🛠️  Building with features: $(1),$(2)"
	anchor build -- --features $(1),$(2)
	@if [ -f target/deploy/bridge-keypair.json.backup ]; then echo "📁 Restoring bridge-keypair.json" && mv target/deploy/bridge-keypair.json.backup target/deploy/bridge-keypair.json; else echo "🗑️  Removing bridge-keypair.json" && rm -f target/deploy/bridge-keypair.json; fi
	@echo "📁 Restoring lib.rs"
	mv programs/bridge/src/lib.rs.backup programs/bridge/src/lib.rs
	@echo "📁 Restoring Anchor.toml"
	mv Anchor.toml.backup Anchor.toml
	@echo "✅ Build completed for $(1) $(2)"
endef

define deploy_bridge
	@echo "🚀 Deploying bridge to $(1) $(2)..."
	@echo "🔑 Program keypair: target/deploy/bridge.$(1).$(2).json"
	@echo "📍 Deployer: $$(solana address --keypair ~/.config/solana/deployer.$(1).$(2).json)"
	anchor deploy \
		--program-name bridge \
		--program-keypair target/deploy/bridge.$(1).$(2).json \
		--provider.wallet ~/.config/solana/deployer.$(1).$(2).json
	@echo "✅ Deploy completed for $(1) $(2)"
endef

.PHONY: build-devnet-alpha deploy-devnet-alpha build-devnet-prod deploy-devnet-prod

build-devnet-alpha:
	$(call build_bridge,devnet,alpha)

deploy-devnet-alpha: build-devnet-alpha
	$(call deploy_bridge,devnet,alpha)

build-devnet-prod:
	$(call build_bridge,devnet,prod)

deploy-devnet-prod: build-devnet-prod
	$(call deploy_bridge,devnet,prod)

