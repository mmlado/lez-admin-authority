# lez-admin-authority

[![CI](https://github.com/mmlado/lez-admin-authority/actions/workflows/ci.yml/badge.svg)](https://github.com/mmlado/lez-admin-authority/actions/workflows/ci.yml)
[![Consumer build](https://github.com/mmlado/lez-admin-authority/actions/workflows/consumer-build.yml/badge.svg)](https://github.com/mmlado/lez-admin-authority/actions/workflows/consumer-build.yml)
[![Audit](https://github.com/mmlado/lez-admin-authority/actions/workflows/audit.yml/badge.svg)](https://github.com/mmlado/lez-admin-authority/actions/workflows/audit.yml)
[![Version](https://img.shields.io/github/v/tag/mmlado/lez-admin-authority?include_prereleases&sort=semver&label=version)](https://github.com/mmlado/lez-admin-authority/releases)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.98.1-93450a?logo=rust)](rust-toolchain.toml)

Single-admin authority for LEZ actor programs (LEZ 0.4): one admin per program, installed by the deployer, handed over by nominate and accept, renounceable, and checked wherever the program's handler asks. A pure Rust library with no macros and no SPEL dependency, built on [lez-authority](https://github.com/mmlado/lez-authority), which owns the routing and the slot. Macros on top of it (level 2) and a SPEL shim (level 3) are planned, each cutting the integrator's boilerplate further.

Status: alpha, `v0.1.0-alpha.1`. The API and the wire formats may still change before 0.1.0. Pinned to logos-execution-zone `dev` at 7e3f7f7 (2026-10-24) and lez-authority `v0.1.0-alpha.1`. The v0.2.4 line lives in [spel-admin-authority](https://github.com/mmlado/spel-admin-authority) and stays as it is. Coming from it: [docs/migration-from-v0.2.md](docs/migration-from-v0.2.md).

## What it does

The program keeps its config on one well-known account, with the admin slot as one more field. It keeps its own message enum, hosting the library's variants so they travel on the same wire, and its own handler, which asks the context for the admin where a message needs one. The guard wraps the handler: it routes, runs the admin operations, and checks that the slot changed only through the context.

```rust
use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::program::{PdaSeed, ReceiveInput, Response};
use lez_admin_authority::{
    Admin, AdminOp, AuthoritySlot, Ctx, Forwarded, HasSlot, Hosts, InitGate,
};

const CONFIG_SEED: PdaSeed = lez_admin_authority::seed(b"lez-admin-sample-minimal/v1");

#[derive(Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
struct Config {
    open_value: u64,
    admin_value: u64,
    admin: AuthoritySlot,
}

impl HasSlot<Admin> for Config {
    fn slot(&self) -> &AuthoritySlot {
        &self.admin
    }
    fn slot_mut(&mut self) -> &mut AuthoritySlot {
        &mut self.admin
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum Message {
    SetOpenValue(u64),
    SetAdminValue(u64),
    Admin(AdminOp),
    Forwarded(Forwarded),
}

impl Hosts for Message {
    fn forwarded(hop: Forwarded) -> Self {
        Message::Forwarded(hop)
    }
    fn into_forwarded(self) -> Result<Forwarded, Self> {
        match self {
            Message::Forwarded(hop) => Ok(hop),
            other => Err(other),
        }
    }
    fn into_op(self) -> Result<AdminOp, Self> {
        match self {
            Message::Admin(op) => Ok(op),
            other => Err(other),
        }
    }
}

fn handle_message(ctx: &mut Ctx, config: &mut Config, message: Message) {
    match message {
        Message::SetOpenValue(value) => config.open_value = value,
        Message::SetAdminValue(value) => {
            ctx.require_admin(&config.admin);
            config.admin_value = value;
        }
        Message::Admin(_) | Message::Forwarded(_) => unreachable!("the guard handles these"),
    }
}

fn entry(input: &ReceiveInput, message: Message) -> Response {
    lez_admin_authority::guard(
        input,
        &CONFIG_SEED,
        InitGate::Deployer,
        message,
        handle_message,
    )
}

lee_core::actor_entrypoint!(entry);
```

That is [`sample-minimal`](sample-minimal/src/main.rs) without its tests, a complete program. Compared with a program that has no admin, the integrator adds the slot field with its `HasSlot` impl, two hosted variants with their `Hosts` impl, one `require_admin` call per admin-only arm, and the guard around the handler. The hosted `Admin(..)` operations, `Init`, `Nominate`, `Accept` and `Renounce`, never reach the handler. `require_admin` is what `#[require_admin]` was in v0.2, and `HasSlot<Admin>` is what `#[admin_slot]` was.

## How a message travels

LEZ 0.4 tells a handler only whether the account it runs on authorized the transaction. So a message that needs the admin goes to the sender's own actor under the program first, the seat, which forwards it to the config account naming the program that reached it. An open message goes straight to the config.

```text
open:        client ──► (config, P)

admin-only:  client ──► (admin, P) ──forward──► (config, P)
                        signed?                 forward from an actor of P?
                                                from == admin, via matches?

with a       client ──► (admin, P) ──forward──► (config, P) ──hub──► (target, P)
target:                                         require_admin,        from == (config, P) exactly?
                                                then ctx.send
```

Identity comes from the engine's `from` at every hop, never from a field in the message. The admin is an account together with the program it acts through: none for a key signing at top level, the owning program for a PDA. Both must match. The hop-by-hop protocol, with every check and its code, is in [lez-authority's routing document](https://github.com/mmlado/lez-authority/blob/main/docs/routing.md).

## Layout

| Crate | Purpose |
| --- | --- |
| [`lez-admin-authority`](lez-admin-authority/) | The library: the `Admin` role, `AdminOp`, `Ctx` with the admin-named methods, `guard`, `guard_with_targets` and `try_guard`. Re-exports what a program needs from lez-authority. |
| [`sample`](sample/) | The reference program: an open setter, an admin-only setter, and a per-account value the admin sets through the hub. |
| [`sample/core`](sample/core/) | The sample's types without guest code, so the runner and the tests share them. |
| [`sample/methods`](sample/methods/) | Builds the sample into the program binary the loader deploys, and measures its [overhead](docs/overhead.md). A standalone workspace, needs the risc0 toolchain. |
| [`sample/runner`](sample/runner/) | Drives the sample on a local dev sequencer through the LEZ wallet library: the reference client. |
| [`sample-minimal`](sample-minimal/) | The program above: one file, no targets. |
| [`ci/consumer-fixture`](ci/consumer-fixture/) | A downstream program on the published library, checked in CI and built as a guest weekly. |

## Adding as a dependency

```toml
[dependencies]
borsh = { version = "1", features = ["derive"] }
lee_core = { git = "https://github.com/logos-blockchain/logos-execution-zone", rev = "7e3f7f765371c4110a1f7f37baf520cd253fb339" }
lez-admin-authority = { git = "https://github.com/mmlado/lez-admin-authority", tag = "v0.1.0-alpha.1" }
```

The `lee_core` source must match the one this repo pins, spelt the same way: cargo treats a tag and a rev as different sources even when they point at the same commit, and two copies of `lee_core` do not compile. `lez-authority` comes in transitively.

The guest builds with the risc0 toolchain, installed through `rzup`, at risc0 3.0.5 and Rust 1.98.1. [`ci/consumer-fixture`](ci/consumer-fixture/) is the smallest complete setup: a methods crate with `risc0_build::embed_methods` and a guest crate that depends on this library.

## Integration steps

1. **Put the slot in your config.** One `AuthoritySlot` field, `Default` on the struct, and `HasSlot<Admin>` pointing at the field. The config lives on the config account, a PDA of the program id and a seed you choose with `seed(b"..")`, at most 32 bytes.
2. **Host the library's variants.** Add `Admin(AdminOp)` and `Forwarded(Forwarded)` to your message enum, and `Hub(Hub)` when the program has targets. Implement `Hosts`: it only wraps and unwraps. The variants' positions in the enum are yours.
3. **Write the handler** as `fn(&mut Ctx, &mut Config, Message)`. Call `ctx.require_admin(&config.admin)` first in every admin-only arm. The hosted variants are unreachable there.
4. **Wrap it in the guard.** `guard` for a program whose state lives on the config account, `guard_with_targets` with a second handler for a program whose admin writes to other accounts. Pass `InitGate::Deployer`, or a [genesis key](#init-gate), and put `actor_entrypoint!` on the entry.
5. **Build and deploy** the guest through a methods crate, as `sample/methods` does, and deploy the program binary with the wallet's program loader, immutable. The header account becomes the program id, and the key that deployed it is the deployer's.
6. **Send `Init`** from the deployer's seat. Until it lands the config account is empty and every other message is refused, so there is no window for anyone else. The deployer is the first admin.
7. **Hand over or renounce** with `Nominate`, `Accept` and `Renounce`, see [Two-step handover](#two-step-handover).

[`docs/authority-lifecycle.md`](docs/authority-lifecycle.md) covers the states, the transitions with their checks, and the PDA admin.

## Sending messages

There is no IDL and no CLI for 0.4 yet. A client builds a transaction with the LEZ wallet library, naming the root actor, every actor reached, and the signers, and encodes the message with Borsh. The transaction shapes of the sample, as [the runner](sample/runner/src/main.rs) sends them:

| Step | Root actor | Actors | Signer | Message |
| --- | --- | --- | --- | --- |
| init | the deployer's seat | seat, config | the deployer | `Admin(Init)` |
| open setter | the config | config | nobody, the payer pays | `SetOpenValue(7)` |
| admin setter | the admin's seat | seat, config | the admin | `SetAdminValue(9)` |
| target setter | the admin's seat | seat, config, target | the admin | `SetTargetValue { target, value }` |
| nominate | the admin's seat | seat, config | the admin | `Admin(Nominate { next })` |
| accept | the nominee's seat | seat, config | the nominee | `Admin(Accept { pda_seed: None })` |
| renounce | the admin's seat | seat, config | the admin | `Admin(Renounce)` |

In wallet terms a seat is `AccountIdentity::Public(account).select_program_actor_state(program)`, the config is `AccountIdentity::PublicPda { program, seed }` on the same program without authorization, and a target the wallet holds the key for is `PublicNoSign`. The wallet supplies the config account's PDA evidence from the mention. A client crate that builds these for any program is [issue #1](https://github.com/mmlado/lez-admin-authority/issues/1).

A refused transaction is still included and charged, and the client learns nothing from it: read the config back with `get_account_view` on the config actor and decode your `Config`. The refusal text is in the sequencer log. [`sample/RUN.md`](sample/RUN.md) walks through a run on a local dev sequencer.

## Init gate

`InitGate::Deployer` admits `Init` only from the program's own account, through its seat at top level. On LEZ 0.4 the program id is the loader header account, and the deployer holds its key, so the race between deployment and the first `Init` that v0.2 had does not exist: a stranger's `Init` is refused with E117 and the config account stays empty. A program deployed with a mutable header keeps a super-admin in that key, because the header's holder can swap the code. Deploy immutable.

`InitGate::Genesis(AccountId)` is for a program whose header account is not a key. The key is written into the program, so it is part of the program id, and only that key installs itself as the first admin. There is no environment variable: the gate is code, and two builds with different keys are two programs.

Both gates refuse an account reached through a program. The first admin is always a key at top level. A PDA becomes admin by accepting a nomination.

## Two-step handover

The admin nominates, the nominee accepts, each in its own transaction with one signer. Nothing moves until the accept, so a mistyped id only costs a second nomination. A later nomination replaces an earlier one, the all-zero id cancels, and renounce drops a pending nomination.

A PDA nominee accepts through the program that derives it: that program sends `Accept { pda_seed: Some(seed) }` to the nominee's seat by a sync call with the seed as a grant, and the config checks that the seed derives the nominee under that program. From then on the PDA acts only through that program. A multisig vault is the expected case, with the vault account as the admin.

There is no one-step transfer. A PDA cannot co-sign a transaction, nothing on 0.4 exports a message for offline co-signing, and one shape serves keys and PDAs alike.

## Targets and the hub

A program whose admin writes to accounts other than the config hosts `Hub` and uses `guard_with_targets`. In an admin-only arm, after `require_admin`, the handler calls `ctx.send(target, &message)`. After the handler returns, the guard sends the message from the config account to the target's actor under the program, as a hub message. At the target, the second handler runs over the target's own state, and only for a hub message that came from the config account itself, by exact actor. That is three hops, each one guest execution. A program whose gated state lives in the same struct as the slot needs none of this and uses `guard`, like `sample-minimal`.

## Errors

A refusal is a panic with a stable code: `lez-authority/admin E103: the signing account is not the admin`. The text reaches the sequencer log verbatim, the transaction is included and reverted, and the client reads the state back. Codes are append-only. The full table, with each refusal's trigger, is in [lez-authority](https://github.com/mmlado/lez-authority#refusals).

| Code | Text |
| --- | --- |
| 101 | the config account is not initialized |
| 102 | the admin is renounced |
| 103 | the signing account is not the admin |
| 104 | the sender must authorize the change |
| 107 | a nominee reached through a program must prove itself with the seed that derives it under that program |
| 108 | the message does not decode |
| 109 | the config account does not decode |
| 113 | no admin nominee is pending |
| 114 | the signing account is not the nominated admin |
| 115 | only the genesis key installs the first admin, through its seat at top level |
| 117 | only the deployer installs the first admin: the program's own account, through its seat at top level |
| 118 | the admin is already installed |
| 119 | a forwarded message only comes from this program's own actor |
| 120 | the admin acts at top level, or through the program that made it the admin |
| 121 | forwarded messages are only handled at the config account |
| 122 | a message that needs the admin must go through the sender's own actor |
| 123 | a hub message comes only from the config account, to a target |
| 126 | a handler must not change the admin slot except through the context |
| 131 | no admin is installed |
| 132 | sending to a target needs the admin: require it first |
| 133 | the target's state does not decode |

## Overhead

Measured from the sample's guest under the risc0 executor, per guest execution. [`docs/overhead.md`](docs/overhead.md) has the harness and every hop.

| Operation | Hops | Cycles |
| --- | --- | ---: |
| open setter | 1 | 13,053 |
| init | 2 | 24,926 |
| admin setter | 2 | 26,177 |
| nominate | 2 | 27,913 |
| target setter | 3 | 39,034 |

About 10,000 cycles is the floor of any execution of this guest, the admin check itself costs about 3,000 over an open write, and each hop is one execution. The wallet's default `gas_limit` is 2,000,000 cycles, so the most expensive operation here uses about 2 percent of it. In transaction size, a gated message adds the sender's seat to the actor list, a pair of 32-byte ids, and the sender's signature. The message bytes are your enum's, unchanged.

## Security notes

- **Identity is the engine's.** The config takes the sender from `from`, the seat from `is_authorized`. A message cannot name its sender.
- **The admin's `via` is part of the admin.** A key admin acts at top level only, so a program that carries the admin's signature cannot act as the admin (E120). A PDA admin acts through its owning program only.
- **The config runs library code before yours.** A forward from outside the program (E119), a gated message sent straight to the config (E122), a hub message anywhere but a target (E123), and any message on an empty config other than `Init` (E101) are refused before the handler runs.
- **The handler cannot touch the slot.** The guard compares the slot before and after the handler, and refuses a change the context did not make (E126). A handler reaches other actors only through `ctx.send`, after `require_admin`, and only actors of its own program, so a PDA admin's inherited grant never reaches a foreign program.
- **Init is gated, re-init refused.** The deployer or the genesis key, at top level, on an empty config. A second `Init` is E118, and after renounce E102.
- **Renounce is terminal.** No recovery path, by design. An admin that loses its key leaves the program in the same state.
- **Deploy immutable.** The header key can swap a mutable program's code, which no admin check can see.
- **Public admins only.** A private actor may present a blinded alias as `from`, so the check fails closed.
- **History is not recorded.** The current admin and the pending nominee are readable from the config. Past handovers need an indexer.

## Not in this release

- A one-step transfer. Nominate and accept is the one handover.
- A client crate or CLI: [issue #1](https://github.com/mmlado/lez-admin-authority/issues/1). The runner is the reference client.
- Local messages, run at the actor that received them without a hop: [lez-authority#1](https://github.com/mmlado/lez-authority/issues/1).
- Level 2 macros and the level 3 SPEL shim.
- A freeze role. It comes as a second role of lez-authority, after the admin.

## Documentation

- [`docs/authority-lifecycle.md`](docs/authority-lifecycle.md): states, transitions, checks, the PDA admin.
- [`docs/requirement-test-matrix.md`](docs/requirement-test-matrix.md): RFP-001 requirements traced to tests and runs.
- [`docs/migration-from-v0.2.md`](docs/migration-from-v0.2.md): from spel-admin-authority to this.
- [`docs/overhead.md`](docs/overhead.md): cycle counts per hop.
- [`sample/RUN.md`](sample/RUN.md): the sample on a local dev sequencer.
- [`CONTEXT.md`](CONTEXT.md): the admin's language, on top of [lez-authority's](https://github.com/mmlado/lez-authority/blob/main/CONTEXT.md).
- [lez-authority's routing document](https://github.com/mmlado/lez-authority/blob/main/docs/routing.md) and [ADRs](https://github.com/mmlado/lez-authority/tree/main/docs/adr).

## Development

```bash
cargo fmt --check
cargo clippy --workspace --exclude sample-runner --all-targets --locked
cargo test --workspace --exclude sample-runner --locked
cargo check --locked --manifest-path ci/consumer-fixture/guest/Cargo.toml
```

The runner links the LEZ wallet stack, which needs `libpcsclite-dev` and is slow, so CI builds it weekly. The guest and the overhead numbers need the risc0 toolchain:

```bash
cargo build --release --manifest-path sample/methods/Cargo.toml
RISC0_DEV_MODE=1 cargo test --manifest-path sample/methods/Cargo.toml --test overhead -- --nocapture
```

## Versioning and stability

Semantic versioning covers the public Rust API and the on-chain wire formats: the Borsh encoding of `AuthoritySlot` inside your config, and of `AdminOp`, `Forwarded` and `Hub` inside your message enum. The variant positions in your enum are yours. The refusal codes and their meaning are covered too. While the version is 0.x, a minor bump may change any of them, and each such change is called out in the [changelog](CHANGELOG.md).

The `lee_core` dependency pins logos-execution-zone `dev` at commit `7e3f7f765371c4110a1f7f37baf520cd253fb339`, and lez-authority at its `v0.1.0-alpha.1` tag. Every re-pin is a minor release.

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE2) at the consumer's option.

## Developing against a local lez-authority

The workspace pins `lez-authority` to its tag on GitHub. To build against the sibling checkout instead, add an untracked `.cargo/config.toml`:

```toml
[patch."https://github.com/mmlado/lez-authority"]
lez-authority = { path = "../lez-authority" }
```
