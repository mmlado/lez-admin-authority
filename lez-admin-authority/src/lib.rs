//! Single-admin authority for LEZ actor programs: one admin per program, installed by
//! `Init`, handed over by `Nominate` and `Accept`, renounceable, and checked before the
//! handler for every message marked `#[only(Admin)]`, the way `onlyOwner` marks a Solidity
//! function.
//!
//! The program stays a plain LEZ program. Put [`admin`] on its handler, derive `HasSlot` on
//! its config with `#[slot(Admin)]` on the slot field, and derive `Gated` on its message
//! enum. The guard, the slot and the routing live in `lez-authority`.

#![warn(missing_docs)]

/// The crate underneath, which the macros reach through this one.
pub use lez_authority;
pub use lez_authority::{
    AuthorityError, AuthoritySlot, Caller, Gated, HasSlot, Holder, InitGate, Policy, Refusal, Role,
    Wire, config_account, guard, seed,
};

/// The admin's operations: `Init`, `Nominate`, `Accept` and `Renounce`. A client sends them
/// with [`admin_op`], the guard runs them, the handler never sees them.
pub use lez_authority::Op as AdminOp;

pub use lez_admin_authority_macros::admin;

/// The admin role. One holder, renounce is terminal.
pub struct Admin;

impl Role for Admin {
    const NAME: &'static str = "admin";
}

/// The admin answers to nobody: the deployer or the genesis key installs it, the holder
/// nominates and renounces, and renounce is for good.
impl<C: HasSlot<Admin>> Policy<C> for Admin {}

/// A client's message for one of the admin operations, to send to the sender's own actor.
#[must_use]
pub fn admin_op<M>(op: AdminOp) -> Wire<M> {
    Wire::op::<Admin>(op)
}
