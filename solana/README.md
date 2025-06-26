# Solana Programs for Base Bridge

This directory contains all Solana programs related to the Base Bridge. These programs are modeled after the OP Stack L1 bridge functionality.

## Usage

Install dependencies:

```bash
make deps
```

Build the programs:

```bash
make build-devnet
```

Run tests:

```bash
make test
```

Deploy to a live network:

1. Update the specified `cluster` in [Anchor.toml](./Anchor.toml) to the network you'd like to deploy to:

If deploying to a local network:

```toml
[provider]
cluster = "Localnet"
```

If deploying to a testnet:

```toml
[provider]
cluster = "Devnet"
```

If deploying to a mainnet:

```toml
[provider]
cluster = "Mainnet"
```

2. Deploy the program(s)

```bash
make deploy
```
