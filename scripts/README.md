# Utility Scripts

Scripts for invoking the bridge

## Usage

### Setup

Install dependencies:

```bash
bun install
```

Generate solana build artifacts:

```bash
make build
```

Add a `.env` file with the following:

```env
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
ANCHOR_WALLET="id.json"
```

Add a `id.json` file with the private key of a funded Solana devnet account. It should look like this:

```json
[
  72, 9, 51, 70, 104, 152, 219, 94, 92, 127, 188, 189, 197, 25, 210, 38, 111,
  89, 242, 122, 167, 95, 236, 178, 26, 96, 180, 5, 59, 217, 15, 243
]
```

### Commands

Bridge SOL from Solana to Base:

```bash
make bridge-sol-to-base
```
