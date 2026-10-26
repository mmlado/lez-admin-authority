# Running the sample on a local dev sequencer

What this run proves: the header key signs after deployment, so the deployer gate is possible; the wallet supplies the config PDA's evidence from a `PublicPda` mention; the default fee covers a two-hop call; and whether a refusal's text reaches the sequencer log.

## 1. Sequencer

In a LEZ checkout at `7e3f7f765371c4110a1f7f37baf520cd253fb339` (branch `dev`), inside `nix develop`:

```sh
just clean
RISC0_DEV_MODE=1 just run-sequencer-standalone     # 127.0.0.1:3040, no proving
```

Leave it running. Refusal text appears only in this terminal.

## 2. Wallet

In a second terminal, same checkout, same `nix develop`:

```sh
export LEE_WALLET_HOME_DIR=$PWD/lez/wallet/configs/debug
just run-wallet check-health                 # sets the wallet password after `just clean`
just wallet-import-test-accounts             # two genesis-funded accounts, take the first as PAYER
```

## 3. Guest binary

From this repo, with the risc0 toolchain installed through `rzup`:

```sh
cargo build --release --manifest-path sample/methods/Cargo.toml
ls sample/methods/target/riscv-guest/sample-methods/sample/riscv32im-risc0-zkvm-elf/release/sample.bin
```

## 4. Run

```sh
cargo run --release -p sample-runner -- \
  sample/methods/target/riscv-guest/sample-methods/sample/riscv32im-risc0-zkvm-elf/release/sample.bin \
  <PAYER>
```

The runner prints the program, deployer, config and stranger account ids, one `included in block` line per step, and the config read back after each step. It stops at the first step whose read-back differs from what the step should have left.

## 5. What to look for

- Four `included in block` lines after the deployment's own.
- `after init`: the admin slot held by the deployer, both values 0.
- `after the stranger, must be unchanged`: `admin_value` still 9.
- In the sequencer terminal, the stranger's transaction with the text `the signing account is not the admin`. If the text is not there, note what is: that answers whether refusal text survives.

## If something fails

- `SegmentCountMismatch`: the runner counts segments from the user ELF inside the program binary. Report the numbers.
- `UnadmittedPublicActor`: the wallet did not attach the config PDA's evidence. Report it, that is a finding about `PublicPda` mentions.
- `MissingFeeDeclaration` or a fee error: the wallet's default `gas_limit` is 2,000,000. Report it.
- An `init` read-back of `None`: the call reverted. The sequencer log has the reason.
