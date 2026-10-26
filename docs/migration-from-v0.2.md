# Migrating from spel-admin-authority v0.2

For a program on [spel-admin-authority](https://github.com/mmlado/spel-admin-authority) v0.2.x moving to LEZ 0.4. Nothing migrates in place: LEZ 0.4 replaces instructions and account lists with actors and messages, programs are redeployed, and a program's state starts empty on the new chain. What carries over is the model, and most of the names.

## What changed underneath

| LEZ v0.2 | LEZ 0.4 |
| --- | --- |
| A program exposes instructions, each with an account list | A program is one handler that runs at an actor, the pair of an account and the program, and receives messages |
| Every account in the transaction comes with `is_authorized` | Only the receiving account's authorization is visible, at its own actor |
| Instructions are dispatched by SPEL from the IDL | The program decodes its own message enum, and there is no IDL yet |
| Programs call each other by chained calls with account lists | Actors send each other sync and async messages, and a PDA's seed travels as a grant on a sync message |
| Deployment is bytecode only, and the first caller of `admin_initialize` self-elects | The program id is the loader header account, whose key the deployer holds |
| Errors are `SpelError::Custom` with a code the client sees | A refusal is a panic whose text reaches the sequencer log, and the client reads state back |

## Concept map

| v0.2 | 0.4 | Note |
| --- | --- | --- |
| `#[admin_authority]` on the module | `HasSlot<Admin>` on your config, `Hosts` on your message enum | no macro at level 1 |
| `#[require_admin]` on an instruction | `ctx.require_admin(&config.admin)` first in the arm | same place, same meaning |
| `#[admin_slot]` on the embedded field | `HasSlot<Admin>` pointing at the field | embedded is the only mode |
| `AdminConfig`, 64 bytes, admin then nominee | `AuthoritySlot`, a Borsh enum: unset, held with its nominee, renounced | the holder carries its `via` |
| The Config PDA at `(program_id, "admin_config")` | Your config account at `(program, your seed)`, holding your whole config | one account, your layout |
| Embedded mode with `offset` | The default and only shape | no offset, Borsh finds the field |
| `admin_initialize`, self-election | `AdminOp::Init` from the deployer's seat | the init gate closes the race |
| `SPEL_ADMIN_GENESIS` at build time | `InitGate::Genesis(AccountId)` in code | compiled in, part of the program id |
| `admin_transfer` with `AdminCandidate` and a co-signer | gone, use nominate and accept | a PDA cannot co-sign |
| `admin_nominate`, `admin_accept` | `AdminOp::Nominate { next }`, `AdminOp::Accept { pda_seed }` | the PDA accept carries its seed |
| `admin_renounce` | `AdminOp::Renounce` | terminal, as before |
| Five injected instructions in the IDL | Four operations hosted in one variant of your enum, `Admin(AdminOp)` | plus `Forwarded`, and `Hub` with targets |
| `AdminError`, `6000 + code` on the client | `AuthorityError`, `lez-authority/admin E<code>: <text>` in the sequencer log | numbers kept where the meaning is the same |
| `caller-pda-seeds` on a chained call | `pda_seeds` on the owning program's sync message | the seat sees the grant as authorization |
| `UnownedSigner`, `wallet auth-transfer init` | no counterpart | fresh keys signed repeatedly on the dev stack |
| `spel` CLI, `--co-signer`, `--export` | no CLI yet, the LEZ wallet library and the runner | a client crate is issue #1 |

## Side by side

A gated instruction in v0.2:

```rust
#[instruction]
#[require_admin]
pub fn update_value(
    #[account(pda = literal("admin_config"))] admin_config: AccountWithMetadata,
    #[account(signer)] caller: AccountWithMetadata,
    #[account(mut, pda = literal("program_config"))] mut config: AccountWithMetadata,
    new_value: u64,
) -> SpelResult {
    // the admin check ran before this
}
```

The same in 0.4, one arm of the handler:

```rust
Message::UpdateValue(new_value) => {
    ctx.require_admin(&config.admin);
    config.value = new_value;
}
```

The accounts are gone from the signature: the config is the state the handler receives, and the caller is what the context knows from the hop. The client, instead of listing `admin_config`, `caller` and `config`, lists the admin's seat and the config account as the actors of one transaction.

## Error codes

Numbers up to 116 follow the v0.2 table where a refusal has the same meaning. The rest of the v0.2 table is retired, and new refusals start at 117.

| Code | v0.2 | 0.4 |
| --- | --- | --- |
| 101 | `NotInitialized` | `NotInitialized` |
| 102 | `Renounced` | `Renounced` |
| 103 | `NotAdmin` | `NotHolder` |
| 104 | `MissingSignature` | `MissingSignature` |
| 105 | `InvalidCandidate` | retired, there is no candidate |
| 106 | `UndeployedPda` | retired, a PDA proves itself by accepting through its program |
| 107 | `CandidateMismatch` | `SeedProofFailed` |
| 108 | `EncodingFailed` | `MessageDoesNotDecode` |
| 109 | `DecodingFailed` | `ConfigDoesNotDecode` |
| 110 | `AccountDataTooLarge` | retired |
| 111 | `SlotOutOfBounds` | retired, there is no offset |
| 112 | `UnownedSigner` | retired |
| 113 | `NoNomination` | `NoNomination` |
| 114 | `NotNominee` | `NotNominee` |
| 115 | `NotGenesisAdmin` | `NotGenesis` |
| 116 | `ConfigNotOwned` | retired, the config account is the program's own actor |
| 117 to 133 | | new, the routing refusals: see the [table](https://github.com/mmlado/lez-authority#refusals) |

## Steps

1. **One config struct.** Collect what the program kept on its accounts into one Borsh struct with `Default`, and add the `AuthoritySlot` field with `HasSlot<Admin>`. State that belongs to other accounts, such as a per-user value, stays on those accounts as a target state, written through the hub.
2. **One message enum.** One variant per instruction, with the instruction's arguments as fields and no accounts. Add `Admin(AdminOp)` and `Forwarded(Forwarded)`, and `Hub(Hub)` if step 1 left targets. Implement `Hosts`.
3. **The handler.** One `match` arm per variant. Where the instruction had `#[require_admin]`, call `ctx.require_admin(&config.admin)` first. Where it wrote another account, call `ctx.send(target, &message)` after the check and handle the message in the target handler.
4. **The entry.** `guard` or `guard_with_targets` with your seed and the init gate, then `actor_entrypoint!`.
5. **Drop SPEL.** `spel-framework`, `admin-authority` and the macros go. The dependencies are `borsh`, `lee_core` at the pinned commit, and `lez-admin-authority`.
6. **Build the guest** through a methods crate with `risc0_build::embed_methods`, as `sample/methods` does, with the risc0 toolchain at 3.0.5.
7. **Deploy immutable and send `Init`** from the deployer's seat. Then nominate the admin the program should have, and have it accept.
8. **Clients.** Replace IDL-driven calls with transactions from the LEZ wallet library: the root actor, the actors reached, the signer, and the Borsh-encoded message. The README's transaction shapes and the runner show each one. Read the config back after each call: a refusal is a reverted, charged transaction with its text in the sequencer log.

## Not there yet

A one-step transfer, a CLI and an IDL, macros, the SPEL shim, and the freeze role. See the README's [Not in this release](../README.md#not-in-this-release).
