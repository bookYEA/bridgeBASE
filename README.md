# Base <> alt-L1 Bridge

## Test the bridge

1. Ensure you have followed the `Setup` instructions for both the [oracle](oracle/README.md) and [scripts](scripts/README.md) directories.

2. On your terminal, start the oracle

```bash
cd oracle && make run-dev
```

3. Invoke the bridge script

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

4. Check the deployed addresses file in `base/deployments` for the new addresses

5. Enter the scripts directory

```bash
cd ../scripts
```

6. Convert the messenger address to a number array

```bash
ADDRESS=<messenger address from base/deployments file> make convert-address
```

7. Copy the output of that command into [`solana/programs/bridge/src/constants.rs`](solana/programs/bridge/src/constants.rs) for `OTHER_MESSENGER`

8. Copy the same output into [`solana/tests/ixs/messenger.ts`](solana/tests/ixs/messenger.ts) for `otherMessengerAddress` and into [`solana/tests/ixs/standard_bridge.ts`](solana/tests/ixs/standard_bridge.ts) for `otherMessengerAddress`.

9. Convert the bridge address to a number array

```bash
ADDRESS=<bridge address from base/deployments file> make convert-address
```

10. Copy the output of that command into [`solana/programs/bridge/src/constants.rs`](solana/programs/bridge/src/constants.rs) for `OTHER_BRIDGE`

11. Copy the same output into [`solana/tests/ixs/standard_bridge.ts`](solana/tests/ixs/standard_bridge.ts) for `otherBridgeAddress`.

12. Enter solana directory

```bash
cd ../solana
```

13. Re-build the program

```bash
anchor build
```

14. Run tests to ensure they all still pass

```bash
anchor test
```

15. Set target cluster in [`Anchor.toml`](solana/Anchor.toml) to `Devnet`

```toml
[provider]
cluster = "Devnet"
```

16. Re-deploy program

```bash
anchor deploy
```

17. Reset target cluster in [`Anchor.toml`](solana/Anchor.toml) back to `Localnet`.

```toml
[provider]
cluster = "Localnet"
```
