//! Single-admin authority for LEZ actor programs: one admin per program, installed by
//! the hosted `Init` operation, renounceable, checked wherever the program asks.
//!
//! The hop and the slot check live in `lez-authority`. This crate names the role and
//! gives the program a context with admin-named methods.

#![warn(missing_docs)]

use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::program::{PdaSeed, ReceiveInput, Response};
pub use lez_authority::{
    AuthorityError, AuthoritySlot, Caller, Forwarded, HasSlot, Holder, Hosts, Role,
    config_account, seed,
};

/// The operations the program hosts in its message enum: `Init` and `Renounce`. The guard
/// runs them, the handler never sees them.
pub use lez_authority::Op as AdminOp;

/// The admin role. One holder, renounce is terminal.
pub struct Admin;

impl Role for Admin {
    const NAME: &'static str = "admin";
}

/// What the guard hands the handler. Every method panics on refusal, which is how a
/// handler fails on LEZ.
pub struct Ctx<'a>(&'a mut lez_authority::Ctx);

impl Ctx<'_> {
    /// The sender, when the message came through the sender's own actor.
    #[must_use]
    pub fn caller(&self) -> Option<&Caller> {
        self.0.caller()
    }

    /// Panics unless the message came through the sender's own actor and the sender is
    /// the admin. Call it first in every arm that is the admin's.
    pub fn require_admin(&self, slot: &AuthoritySlot) {
        self.0.require(slot);
    }

    /// Installs the sender as the admin of an unset slot. The hosted `AdminOp::Init` does
    /// this without the handler; call it yourself from an init message of your own.
    pub fn install_admin(&mut self, slot: &mut AuthoritySlot) {
        self.0.install(slot);
    }

    /// Empties the slot for good. The sender must be the admin.
    pub fn renounce(&mut self, slot: &mut AuthoritySlot) {
        self.0.renounce(slot);
    }
}

/// Runs `handler` behind the hop and the slot check. See `lez_authority::guard`.
///
/// # Panics
/// On every refusal, with the reason as the text.
pub fn guard<C, M>(
    input: &ReceiveInput,
    seed: &PdaSeed,
    message: M,
    handler: impl FnOnce(&mut Ctx<'_>, &mut C, M),
) -> Response
where
    C: BorshSerialize + BorshDeserialize + Default + HasSlot<Admin>,
    M: Hosts,
{
    lez_authority::guard::<Admin, C, M>(input, seed, message, |ctx, config, message| {
        handler(&mut Ctx(ctx), config, message);
    })
}

/// [`guard`] with the guard's own refusals as a value. The context still panics.
///
/// # Errors
/// See [`AuthorityError`].
pub fn try_guard<C, M>(
    input: &ReceiveInput,
    seed: &PdaSeed,
    message: M,
    handler: impl FnOnce(&mut Ctx<'_>, &mut C, M),
) -> Result<Response, AuthorityError>
where
    C: BorshSerialize + BorshDeserialize + Default + HasSlot<Admin>,
    M: Hosts,
{
    lez_authority::try_guard::<Admin, C, M>(input, seed, message, |ctx, config, message| {
        handler(&mut Ctx(ctx), config, message);
    })
}
