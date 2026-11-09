# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The `admin` attribute, in the new `lez-admin-authority-macros` crate: on a plain LEZ handler, `fn(&ReceiveInput, Message) -> Response` under `lee_core::actor_entrypoint!`, it puts lez-authority's guard in front with the admin as the role. `config`, `seed`, and `genesis` for a genesis key.
- `admin_op`, a client's message for one of the admin's operations.
- `Policy` for `Admin`, with lez-authority's default rules: the deployer or the genesis key installs, the admin nominates and renounces, renounce is for good.
- lez-authority's `guard`, derives and `Wire`, re-exported, and `pub use lez_authority` for the macros to find it.
- Sample tests for a message that came through a seat sending to another program, and for an open message sent to a seat.
- E135 in the refusal table, from lez-authority: wherever the handler runs, its response may not send the config account a library message.

### Changed

- The samples are plain LEZ programs: their own enum and state, a handler under `actor_entrypoint!`, `#[only(Admin)]` on the admin-only messages, `#[from_config]` on the message the config sends to a target, and the attribute on the handler.
- The admin's operations are library messages behind the byte 255, not variants of the program's enum. The runner sends them with `admin_op`.
- A message that needs no admin runs wherever it is sent.
- The overhead numbers in `docs/overhead.md`, measured again: a message at the config costs a few thousand cycles more, for the guard's second read of the config, and the admin's operations less.

### Removed

- `Ctx` and `AdminOp` as hosted variants, `guard_with_targets`, and the handler's context with `require_admin`, `install_admin`, `nominate`, `accept`, `renounce` and `send`.

## [0.1.0-alpha.1] - 2026-10-27

### Added

- `Admin`, the admin role of lez-authority, and `AdminOp`, the operations a program hosts in its message enum: `Init`, `Nominate`, `Accept`, `Renounce`.
- `Ctx` with `caller`, `require_admin`, `install_admin`, `nominate`, `accept`, `renounce` and `send`.
- `guard` and `try_guard` for programs whose state lives at the config account, `guard_with_targets` for programs with targets.
- The sample: two values on the config, an open setter, an admin-only setter, and a per-account value the admin sets through the hub, with the runner that drives it on a local dev sequencer and the methods crate that builds its program binary.
- The minimal sample: one file, no targets.
- The overhead harness, `sample/methods/tests/overhead.rs`, and the cycle counts per hop in `docs/overhead.md`.
- The README with the integration steps, the transaction shapes and the error table, `docs/authority-lifecycle.md`, `docs/requirement-test-matrix.md` and `docs/migration-from-v0.2.md`.

[Unreleased]: https://github.com/mmlado/lez-admin-authority/compare/v0.1.0-alpha.1...HEAD
[0.1.0-alpha.1]: https://github.com/mmlado/lez-admin-authority/releases/tag/v0.1.0-alpha.1
