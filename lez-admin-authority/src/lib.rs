//! Single-admin authority for LEZ actor programs: one admin per program, installed by
//! the hosted `Init` operation, handed over by `Nominate` and `Accept`, renounceable,
//! checked wherever the program asks.
//!
//! The hop and the slot check live in `lez-authority`. This crate names the role and
//! gives the program a context with admin-named methods.

#![warn(missing_docs)]

use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::program::{PdaSeed, ReceiveInput, Response};
pub use lez_authority::{
    AuthorityError, AuthoritySlot, Caller, Forwarded, HasSlot, Holder, Hosts, Hub, InitGate,
    Role, config_account, seed,
};

/// The operations the program hosts in its message enum: `Init`, `Nominate`, `Accept` and
/// `Renounce`. The guard runs them, the handler never sees them.
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
    /// the admin. Call it first in every arm that is the admin's. Afterwards the arm may
    /// send to targets.
    pub fn require_admin(&mut self, slot: &AuthoritySlot) {
        self.0.require(slot);
    }

    /// Sends `message` to `target`'s own actor under the program, after the handler
    /// returns. The target receives it as a hub message and knows it came from the config.
    /// Panics unless `require_admin` passed first.
    pub fn send<M: Hosts>(&mut self, target: lee_core::account::AccountId, message: &M) {
        self.0.send(target, message);
    }

    /// Installs the sender as the admin of an unset slot, if the sender passes the init gate.
    /// The hosted `AdminOp::Init` does this without the handler; call it yourself from an
    /// init message of your own.
    pub fn install_admin(&mut self, slot: &mut AuthoritySlot) {
        self.0.install(slot);
    }

    /// The sender, who must be the admin, offers the role to `next`. The all-zero id cancels.
    pub fn nominate(&mut self, slot: &mut AuthoritySlot, next: lee_core::account::AccountId) {
        self.0.nominate(slot, next);
    }

    /// The sender, who must be the pending nominee, takes the role. A key at top level needs
    /// no seed. A PDA reached through its program carries the seed that derives it there.
    pub fn accept(&mut self, slot: &mut AuthoritySlot, pda_seed: Option<PdaSeed>) {
        self.0.accept(slot, pda_seed);
    }

    /// Empties the slot for good, nominee included. The sender must be the admin.
    pub fn renounce(&mut self, slot: &mut AuthoritySlot) {
        self.0.renounce(slot);
    }
}

/// Runs `handler` behind the hop and the slot check. `gate` says who may send the first
/// `Init`: the deployer, or a genesis key. See `lez_authority::guard`.
///
/// # Panics
/// On every refusal, with the reason as the text.
pub fn guard<C, M>(
    input: &ReceiveInput,
    seed: &PdaSeed,
    gate: InitGate,
    message: M,
    handler: impl FnOnce(&mut Ctx<'_>, &mut C, M),
) -> Response
where
    C: BorshSerialize + BorshDeserialize + Default + HasSlot<Admin>,
    M: Hosts,
{
    lez_authority::guard::<Admin, C, M>(input, seed, gate, message, |ctx, config, message| {
        handler(&mut Ctx(ctx), config, message);
    })
}

/// [`guard`] for a program with targets: `target_handler` runs at a target over the
/// target's own state `T`, for hub messages that came from the config account itself.
///
/// # Panics
/// On every refusal, with the reason as the text.
pub fn guard_with_targets<C, T, M>(
    input: &ReceiveInput,
    seed: &PdaSeed,
    gate: InitGate,
    message: M,
    handler: impl FnOnce(&mut Ctx<'_>, &mut C, M),
    target_handler: impl FnOnce(&mut T, M),
) -> Response
where
    C: BorshSerialize + BorshDeserialize + Default + HasSlot<Admin>,
    T: BorshSerialize + BorshDeserialize + Default,
    M: Hosts,
{
    lez_authority::guard_with_targets::<Admin, C, T, M>(
        input,
        seed,
        gate,
        message,
        |ctx, config, message| handler(&mut Ctx(ctx), config, message),
        target_handler,
    )
}

/// [`guard`] with the guard's own refusals as a value. The context still panics.
///
/// # Errors
/// See [`AuthorityError`].
pub fn try_guard<C, M>(
    input: &ReceiveInput,
    seed: &PdaSeed,
    gate: InitGate,
    message: M,
    handler: impl FnOnce(&mut Ctx<'_>, &mut C, M),
) -> Result<Response, AuthorityError>
where
    C: BorshSerialize + BorshDeserialize + Default + HasSlot<Admin>,
    M: Hosts,
{
    lez_authority::try_guard::<Admin, C, M>(input, seed, gate, message, |ctx, config, message| {
        handler(&mut Ctx(ctx), config, message);
    })
}
