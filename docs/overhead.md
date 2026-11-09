# Overhead

Cycle counts of the sample's guest, measured 2026-10-29 with `sample/methods/tests/overhead.rs`: each hop's `ReceiveInput` fed to the real `sample.bin` under the risc0 executor, the way the sequencer runs it. LEZ at `7e3f7f7`, risc0 3.0.5, program binary 350,648 bytes.

```sh
RISC0_DEV_MODE=1 cargo test --manifest-path sample/methods/Cargo.toml --test overhead -- --nocapture
```

## Per hop

| Hop | Cycles |
| --- | ---: |
| seat hop, `Init` | 10,314 |
| config hop, `Init` installs the admin | 13,547 |
| config, `SetOpenValue`, no hop | 16,355 |
| seat hop, `SetAdminValue` | 9,955 |
| config hop, `SetAdminValue` | 19,022 |
| config hop, `Nominate` | 16,100 |
| config hop, `SetTargetValue`, sends to the target | 17,182 |
| target hop, `TargetValue` | 10,542 |

## Per operation

| Operation | Hops | Cycles |
| --- | --- | ---: |
| open setter | 1 | 16,355 |
| init | 2 | 23,861 |
| admin setter | 2 | 28,977 |
| nominate | 2 | 26,055 |
| target setter | 3 | 37,679 |

## Reading the numbers

- About 10,000 cycles is the floor of any execution of this guest: decoding the input frame, the Borsh enum, encoding the response. The seat hop and the target hop are that floor plus a little.
- The admin check costs about 2,700 cycles over an open write at the config: `SetOpenValue` at 16,355 against `SetAdminValue` at 19,022.
- A program message at the config pays for the guard's two reads of the config, one before the handler to check the declared holders and one after it to check the slot, on top of the handler's own. The admin's operations skip the handler, so `Init` and `Nominate` cost less than an open write.
- A hop costs one execution, so a gated write is two executions and a write to another account three. The transaction is still one, atomic, and these counts do not include the fee program's own executions around it, which the sequencer runs for every charged transaction.
- Against the budgets: the wallet's default `gas_limit` is 2,000,000 cycles, so the most expensive operation here uses about 2 percent of it. Kanon's M0 report puts one RedStone price update at about 1.9 million cycles. The admin is noise next to real work.
