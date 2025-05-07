# Base Bridge Oracle

This directory contains the Go application for the Base Bridge Oracle. The oracle is responsible for relaying information between Solana and Base (Sepolia for development/testing, Mainnet for production).

## Overview

The oracle monitors events or states on one chain (e.g., Solana) and posts relevant data to a target address on another chain (e.g., Base), or vice-versa. It uses RPC URLs to connect to the respective blockchain networks.

**For the first iteration of our bridge, this serves as a Coinbase-run trusted oracle. It is a temporary solution and will be quickly phased out with upcoming protocol-level updates in the OP stack.**

## Setup

### Prerequisites

- Go programming language installed.
- Funded EOA account on Base for submitting transactions from.

### Configuration

Key configuration parameters are managed via environment variables and command-line flags. The `Makefile` provides convenience targets that pass the necessary flags.

**Important:** You will need to create a `.env` file in the `oracle` directory with the following content:

```env
PRIVATE_KEY=your_32_byte_hex_encoded_private_key_without_the_0x_prefix
```

Replace `your_32_byte_hex_encoded_private_key_without_the_0x_prefix` with the private key of an EOA that is funded on the Base network (Sepolia for development, Mainnet for production). This account will be used to submit transactions.

Other environment variables that are typically set (e.g., in the `.env` file or exported in your shell) or passed via Makefile targets include:

> [!NOTE] The following variables are automatically handled by the Makefile and do not need to be set if you use the Makefile commands.

- `SOLANA_PROGRAM_ID`: The Solana program ID the oracle interacts with.
- `BASE_SEPOLIA_RPC_URL`: RPC URL for the Base Sepolia test network.
- `BASE_MAINNET_RPC_URL`: RPC URL for the Base main network.

The application uses `urfave/cli` for command-line argument parsing. The specific flags can be found in [`internal/flags/flags.go`](./internal/flags/flags.go).

### Install Dependencies

```bash
go mod tidy
```

## Running the Oracle

You can use the `Makefile` to run the oracle:

### Development/Testing (Base Sepolia)

```bash
make run-dev
```

### Production (Base Mainnet)

```bash
make run-prod
```

### Directly with `go run`

You can also run the oracle directly:

```bash
source .env
go run ./cmd --target-address <SOLANA_PROGRAM_ID> --base-rpc-url <BASE_RPC_URL> [--is-mainnet]
```

Replace placeholders with actual values. The `--is-mainnet` flag should be used when connecting to Base Mainnet.
