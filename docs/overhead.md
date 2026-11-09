# Overhead

Cycle counts of the sample's guest, measured 2026-11-09 with `sample/methods/tests/overhead.rs`: each hop's `ReceiveInput` fed to the real `sample.bin` under the risc0 executor, the way the sequencer runs it. LEZ at `7e3f7f7`, lez-authority at `7361e00`, risc0 3.0.5, program binary 350,416 bytes.

```sh
RISC0_DEV_MODE=1 cargo test --manifest-path sample/methods/Cargo.toml --test overhead -- --nocapture
```

## Per hop

| Hop | Cycles |
| --- | ---: |
| seat hop, `Init` | 10,320 |
| config hop, `Init` installs the admin | 13,571 |
| config, `SetOpenValue`, no hop | 16,784 |
| seat hop, `SetAdminValue` | 9,907 |
| config hop, `SetAdminValue` | 19,514 |
| config hop, `Nominate` | 16,124 |
| config hop, `SetTargetValue`, sends to the target | 17,296 |
| target hop, `TargetValue` | 10,728 |

## Per operation

| Operation | Hops | Cycles |
| --- | --- | ---: |
| open setter | 1 | 16,784 |
| init | 2 | 23,891 |
| admin setter | 2 | 29,421 |
| nominate | 2 | 26,031 |
| target setter | 3 | 37,931 |

## Reading the numbers

- About 10,000 cycles is the floor of any execution of this guest: decoding the input frame, the Borsh enum, encoding the response. The seat hop and the target hop are that floor plus a little.
- The admin check costs about 2,700 cycles over an open write at the config: `SetOpenValue` at 16,784 against `SetAdminValue` at 19,514.
- A program message at the config pays for the guard's two reads of the config, one before the handler to check the declared holders and one after it to check the slot, on top of the handler's own. The admin's operations skip the handler, so `Init` and `Nominate` cost less than an open write. Each read grows with the config's size, so a program keeps large data on its other actors, not in the config.
- Against `v0.1.0-alpha.1`, which decoded the config once and handed it to the handler, an open write at the config costs about 3,700 cycles more and an admin write about 3,500 more. `Init`, `Nominate` and the target setter cost less.
- Against 2026-10-29, before the guard refused a handler's library message to the config (E135) and stopped refusing a send to another program (E134), every hop that runs the handler costs 100 to 500 cycles more, for the check around the handler. The admin's operations and the seat hops are unchanged within 50 cycles.
- A hop costs one execution, so a gated write is two executions and a write to another account three. The transaction is still one, atomic, and these counts do not include the fee program's own executions around it, which the sequencer runs for every charged transaction.
- Against the budgets: the wallet's default `gas_limit` is 2,000,000 cycles, so the most expensive operation here uses about 2 percent of it. A public transaction pays its executed cycles times the execution base fee, so these counts are what an operation costs in execution fee. Kanon's M0 report puts one RedStone price update at about 1.9 million cycles. The admin is noise next to real work.
