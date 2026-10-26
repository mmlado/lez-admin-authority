# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Admin`, the admin role of lez-authority, and `AdminOp`, the operations a program hosts in its message enum: `Init`, `Nominate`, `Accept`, `Renounce`.
- `Ctx` with `caller`, `require_admin`, `install_admin`, `nominate`, `accept`, `renounce` and `send`.
- `guard` and `try_guard` for programs whose state lives at the config account, `guard_with_targets` for programs with targets.
- The sample: two values on the config, an open setter, an admin-only setter, and a per-account value the admin sets through the hub, with the runner that drives it on a local dev sequencer and the methods crate that builds its program binary.
- The minimal sample: one file, no targets.
