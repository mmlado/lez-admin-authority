//! Types of the sample program. No guest code here, so wallets, tests and other
//! programs can depend on it.

use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::{account::AccountId, program::PdaSeed};
use lez_admin_authority::{Admin, AdminOp, AuthoritySlot, Forwarded, HasSlot, Hosts};

/// The seed of the config account, the program's one well-known account.
pub const CONFIG_SEED: PdaSeed = lez_admin_authority::seed(b"lez-admin-sample/config/v1");

/// The account the config lives on, for this program.
#[must_use]
pub fn config_account(program: AccountId) -> AccountId {
    lez_admin_authority::config_account(program, &CONFIG_SEED)
}

/// The state kept on the config account. The admin slot is one more field.
#[derive(Debug, Clone, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Config {
    pub open_value: u64,
    pub admin_value: u64,
    pub admin: AuthoritySlot,
}

impl HasSlot<Admin> for Config {
    fn slot(&self) -> &AuthoritySlot {
        &self.admin
    }
    fn slot_mut(&mut self) -> &mut AuthoritySlot {
        &mut self.admin
    }
}

/// The messages the program accepts. The last two are the library's, hosted here so they
/// travel in the same enum: `Admin` carries init and renounce, `Forwarded` is the hop.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum Message {
    /// Anyone.
    SetOpenValue(u64),
    /// The admin only.
    SetAdminValue(u64),
    /// `Init` makes the sender the admin, `Renounce` gives the role up for good.
    Admin(AdminOp),
    /// The hop from the sender's own actor to the config. Clients never send it.
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
