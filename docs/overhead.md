# Overhead

Cycle counts of the sample's guest, measured 2026-10-26 with `sample/methods/tests/overhead.rs`: each hop's `ReceiveInput` fed to the real `sample.bin` under the risc0 executor, the way the sequencer runs it. LEZ at `7e3f7f7`, risc0 3.0.5, program binary 362,244 bytes.

```sh
RISC0_DEV_MODE=1 cargo test --manifest-path sample/methods/Cargo.toml --test overhead -- --nocapture
```

## Per hop

| Hop | Cycles |
| --- | ---: |
| seat hop, `Init` | 10,158 |
| config hop, `Init` installs the admin | 14,768 |
| config, `SetOpenValue`, no hop | 13,053 |
| seat hop, `SetAdminValue` | 10,145 |
| config hop, `SetAdminValue` | 16,032 |
| config hop, `Nominate` | 17,768 |
| config hop, `SetTargetValue`, sends to the target | 17,896 |
| target hop, `TargetValue` | 10,993 |

## Per operation

| Operation | Hops | Cycles |
| --- | --- | ---: |
| open setter | 1 | 13,053 |
| init | 2 | 24,926 |
| admin setter | 2 | 26,177 |
| nominate | 2 | 27,913 |
| target setter | 3 | 39,034 |

## Reading the numbers

- About 10,000 cycles is the floor of any execution of this guest: decoding the input frame, the Borsh enum, encoding the response. The seat hop and the target hop are that floor plus a little.
- The admin check costs about 3,000 cycles over an open write at the config: `SetOpenValue` at 13,053 against `SetAdminValue` at 16,032. Nominate and the hub send are each a further 2,000.
- A hop costs one execution, so a gated write is twice an open one and a hub write three times. The transaction is still one, atomic, and these counts do not include the fee program's own executions around it, which the sequencer runs for every charged transaction.
- Against the budgets: the wallet's default `gas_limit` is 2,000,000 cycles, so the most expensive operation here uses about 2 percent of it. Kanon's M0 report puts one RedStone price update at about 1.9 million cycles. The admin is noise next to real work.
