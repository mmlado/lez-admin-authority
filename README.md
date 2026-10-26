# lez-admin-authority

Single-admin authority for LEZ actor programs (LEZ 0.4): one admin per program, handed over by nominate and accept, renounceable, with admin-gated actions. Three levels for the integrator: a pure Rust library, macros on top of it, and a SPEL shim. Built on [lez-authority](https://github.com/mmlado/lez-authority).

Status: pre-alpha, nothing released yet. Pinned to logos-execution-zone `dev` at 7e3f7f7 (2026-10-24).

The v0.2.4 line lives in [spel-admin-authority](https://github.com/mmlado/spel-admin-authority) and stays as it is.

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE2) at the consumer's option.

## Developing against a local lez-authority

The workspace pins `lez-authority` to a commit on GitHub. To build against the sibling checkout instead, add an untracked `.cargo/config.toml`:

```toml
[patch."https://github.com/mmlado/lez-authority"]
lez-authority = { path = "../lez-authority" }
```
