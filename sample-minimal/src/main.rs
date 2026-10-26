//! The smallest program with an admin. Everything it keeps lives on the config account, so
//! it uses `guard`, hosts no hub variant, and never sees a third hop. One file: the types
//! could move to a core crate for clients, as `sample` does, but nothing requires it.

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

#[cfg(test)]
mod tests {
    use lee_core::{
        account::{AccountId, Actor, ActorState},
        program::{SyncMessage, Transition},
    };
    use lez_admin_authority::Holder;

    use super::*;

    const PROGRAM: AccountId = AccountId::new([0xB0; 32]);
    const DEPLOYER: AccountId = PROGRAM;
    const STRANGER: AccountId = AccountId::new([2; 32]);

    fn config_actor() -> Actor {
        Actor::new(
            lez_admin_authority::config_account(PROGRAM, &CONFIG_SEED),
            PROGRAM,
        )
    }

    fn run(
        receiver: Actor,
        from: Option<Actor>,
        authorized: bool,
        pre: Vec<u8>,
        message: Message,
    ) -> Transition {
        let input = ReceiveInput {
            receiver,
            from,
            is_authorized: authorized,
            pre_state: ActorState::from(pre),
            message: borsh::to_vec(&message).unwrap(),
        };
        entry(&input, message).into_transition(input)
    }

    /// The sender's own actor forwards, then the config handles what it forwarded.
    fn through_seat(sender: AccountId, pre: Vec<u8>, message: Message) -> Transition {
        let hop = run(Actor::new(sender, PROGRAM), None, true, Vec::new(), message);
        let [SyncMessage { message, .. }] =
            <[SyncMessage; 1]>::try_from(hop.response.sync_messages).unwrap();
        run(
            config_actor(),
            Some(Actor::new(sender, PROGRAM)),
            false,
            pre,
            borsh::from_slice(&message).unwrap(),
        )
    }

    fn written(transition: &Transition) -> Config {
        borsh::from_slice(transition.response.post_state.as_ref().unwrap()).unwrap()
    }

    fn initialized() -> Config {
        Config {
            open_value: 0,
            admin_value: 0,
            admin: AuthoritySlot::held(Holder {
                account: DEPLOYER,
                via: None,
            }),
        }
    }

    #[test]
    fn the_deployer_initializes_anyone_sets_the_open_value_only_the_admin_the_other() {
        let init = through_seat(DEPLOYER, Vec::new(), Message::Admin(AdminOp::Init));
        assert_eq!(written(&init), initialized());

        let open = run(
            config_actor(),
            None,
            false,
            borsh::to_vec(&initialized()).unwrap(),
            Message::SetOpenValue(7),
        );
        assert_eq!(written(&open).open_value, 7);

        let admin = through_seat(
            DEPLOYER,
            borsh::to_vec(&initialized()).unwrap(),
            Message::SetAdminValue(9),
        );
        assert_eq!(written(&admin).admin_value, 9);
    }

    #[test]
    #[should_panic(expected = "E103: the signing account is not the admin")]
    fn a_stranger_is_refused() {
        let _transition = through_seat(
            STRANGER,
            borsh::to_vec(&initialized()).unwrap(),
            Message::SetAdminValue(1),
        );
    }
}
