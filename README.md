# Base <> alt-L1 Bridge

## Test the bridge

1. Ensure you have followed the `Setup` instructions for both the [oracle](oracle/README.md) and [scripts](scripts/README.md) directories.

2. In your terminal, start the oracle

```bash
cd oracle && make run-dev
```

3. In a separate terminal, invoke the bridge script

```bash
cd scripts && make bridge-sol-to-base
```

## Useful Procedures

### Base Contracts Deployment

1. Enter base directory

```bash
cd base
```

2. Install dependencies

```bash
make deps
```

3. Deploy contracts

```bash
make deploy
```

4. Deploy wSOL

```bash
make create-wrapped-sol
```

5. Deploy wSPL

```bash
make create-token
```

6. Check the deployed addresses file in `base/deployments` for the new addresses

7. Copy the messenger and bridge addresses into [`solana/programs/bridge/src/constants.rs`](solana/programs/bridge/src/constants.rs) for `OTHER_MESSENGER` and `OTHER_BRIDGE`

8. Copy the messenger and bridge addresses into [`solana/tests/ixs/messenger.ts`](solana/tests/utils/constants.ts) for `otherMessengerAddress` and `otherBridgeAddress`.

9. Enter solana directory

```bash
cd ../solana
```

10. Re-build the program

```bash
anchor build
```

11. Run tests to ensure they all still pass

```bash
anchor test
```

12. Uncomment the `TRUSTED_ORACLE` constant for Devnet deployments in [`constants.rs`](solana/programs/bridge/src/constants.rs).

13. Build the program

```bash
anchor build
```

14. Set target cluster in [`Anchor.toml`](solana/Anchor.toml) to `Devnet`

```toml
[provider]
cluster = "Devnet"
```

15. Re-deploy program

```bash
anchor deploy
```

16. Reset the `TRUSTED_ORACLE` constant in [`constants.rs`](solana/programs/bridge/src/constants.rs).

17. Reset target cluster in [`Anchor.toml`](solana/Anchor.toml) back to `Localnet`.

```toml
[provider]
cluster = "Localnet"
```
