//! Drives the sample on a local dev sequencer: deploy, `Init` through the deployer's seat, the
//! open setter, the admin setter, a stranger's call that must be refused, then the handover,
//! nominate and accept, with the old admin refused afterwards, and renounce. The config is read
//! back after each step. The wallet comes from the environment, as LEZ's own example runners do.
//!
//! Usage: sample-runner <sample.bin> <payer account id>

use anyhow::{Context, Result, anyhow};
use lee_core::account::{AccountId, Actor};
use lez_admin_authority::{AdminOp, AuthoritySlot, Holder};
use sample_core::{CONFIG_SEED, Config, Message, config_account};
use wallet::{
    AccountIdentity, AccountMention, WalletCore, program_facades::program_loader::ProgramLoader,
};

#[tokio::main]
async fn main() -> Result<()> {
    let usage = "usage: sample-runner <sample.bin> <payer account id>";
    let mut args = std::env::args().skip(1);
    let bin = args.next().context(usage)?;
    let payer: AccountId = args
        .next()
        .context(usage)?
        .parse()
        .map_err(|error| anyhow!("payer account id: {error:?}"))?;
    let mut wallet = WalletCore::from_env()?;

    // Deploy. The header account is the program's id, and its key is the deployer's.
    let bytecode = std::fs::read(&bin).with_context(|| format!("reading {bin}"))?;
    let user_elf_len = risc0_binfmt::ProgramBinary::decode(&bytecode)
        .map_err(|error| anyhow!("not a program binary: {error:?}"))?
        .user_elf
        .len();
    let segment_count = user_elf_len.div_ceil(program_loader_core::MAX_SEGMENT_DATA_LEN);
    let deployer = wallet.create_new_account_public(None).0;
    let segments: Vec<AccountId> =
        std::iter::repeat_with(|| wallet.create_new_account_public(None).0)
            .take(segment_count)
            .collect();
    let stranger = wallet.create_new_account_public(None).0;
    let nominee = wallet.create_new_account_public(None).0;
    wallet.store_persistent_data()?;
    let program = ProgramLoader(&mut wallet)
        .deploy(deployer, &segments, bytecode, true, Some(payer))
        .await?;
    println!("program  {program:?}");
    println!("deployer {deployer:?}");
    println!("config   {:?}", config_account(program));
    println!("stranger {stranger:?}");
    println!("nominee  {nominee:?}");

    let config = || {
        AccountIdentity::PublicPda {
            program,
            seed: CONFIG_SEED,
        }
        .select_program_actor_state(program)
        .without_authorization()
    };
    let seat = |account: AccountId| AccountIdentity::Public(account).select_program_actor_state(program);

    // A stranger's Init first: the gate refuses it, the config account stays empty.
    call(&wallet, payer, vec![seat(stranger), config()], &Message::Admin(AdminOp::Init), "stranger init").await?;
    check(&wallet, program, None, "after the stranger's init, must be empty").await?;

    // Through the deployer's seat: the seat and the config, the deployer signs. The deployer
    // is the program's own account, so the gate lets it through.
    call(&wallet, payer, vec![seat(deployer), config()], &Message::Admin(AdminOp::Init), "init").await?;
    let expected = Config {
        open_value: 0,
        admin_value: 0,
        admin: AuthoritySlot::held(Holder {
            account: deployer,
            via: None,
        }),
    };
    check(&wallet, program, Some(&expected), "after init").await?;

    // Straight to the config: anyone, the payer signs for the fee only.
    call(&wallet, payer, vec![config()], &Message::SetOpenValue(7), "open setter").await?;
    let expected = Config {
        open_value: 7,
        ..expected
    };
    check(&wallet, program, Some(&expected), "after the open setter").await?;

    call(&wallet, payer, vec![seat(deployer), config()], &Message::SetAdminValue(9), "admin setter").await?;
    let expected = Config {
        admin_value: 9,
        ..expected
    };
    check(&wallet, program, Some(&expected), "after the admin setter").await?;

    // A stranger through its own seat. The transaction is included and reverted, so the config
    // must read back unchanged. The refusal text is only in the sequencer's log.
    call(&wallet, payer, vec![seat(stranger), config()], &Message::SetAdminValue(1), "stranger").await?;
    check(&wallet, program, Some(&expected), "after the stranger, must be unchanged").await?;

    // Handover: the admin nominates, nothing moves, the nominee accepts through its own seat.
    let nominate = Message::Admin(AdminOp::Nominate { next: nominee });
    call(&wallet, payer, vec![seat(deployer), config()], &nominate, "nominate").await?;
    let expected = Config {
        admin: AuthoritySlot::Held {
            holder: Holder {
                account: deployer,
                via: None,
            },
            nominee: Some(nominee),
        },
        ..expected
    };
    check(&wallet, program, Some(&expected), "after nominate").await?;

    call(&wallet, payer, vec![seat(nominee), config()], &Message::SetAdminValue(1), "nominee before accept").await?;
    check(&wallet, program, Some(&expected), "after the nominee's early call, must be unchanged").await?;

    let accept = Message::Admin(AdminOp::Accept { pda_seed: None });
    call(&wallet, payer, vec![seat(nominee), config()], &accept, "accept").await?;
    let expected = Config {
        admin: AuthoritySlot::held(Holder {
            account: nominee,
            via: None,
        }),
        ..expected
    };
    check(&wallet, program, Some(&expected), "after accept").await?;

    call(&wallet, payer, vec![seat(nominee), config()], &Message::SetAdminValue(11), "new admin setter").await?;
    let expected = Config {
        admin_value: 11,
        ..expected
    };
    check(&wallet, program, Some(&expected), "after the new admin's setter").await?;

    call(&wallet, payer, vec![seat(deployer), config()], &Message::SetAdminValue(2), "old admin").await?;
    check(&wallet, program, Some(&expected), "after the old admin, must be unchanged").await?;

    call(&wallet, payer, vec![seat(nominee), config()], &Message::Admin(AdminOp::Renounce), "renounce").await?;
    let expected = Config {
        admin: AuthoritySlot::Renounced,
        ..expected
    };
    check(&wallet, program, Some(&expected), "after renounce").await?;

    call(&wallet, payer, vec![seat(nominee), config()], &Message::SetAdminValue(3), "after renounce").await?;
    check(&wallet, program, Some(&expected), "after the renounced admin's call, must be unchanged").await?;

    println!("all steps behaved. The refused calls are: stranger init, stranger, nominee before accept, old admin, after renounce. Their text is in the sequencer log.");
    Ok(())
}

/// Sends one root call over `accounts`, paid by `payer`, and waits for inclusion.
async fn call(
    wallet: &WalletCore,
    payer: AccountId,
    accounts: Vec<AccountMention>,
    message: &Message,
    what: &str,
) -> Result<()> {
    let bytes = borsh::to_vec(message)?;
    let hash = wallet
        .send_pub_tx_paid_by(accounts, 0, bytes, Some(payer))
        .await
        .map_err(|error| anyhow!("{what}: {error:?}"))?;
    let (_tx, block) = wallet.poll_transaction(hash).await?;
    println!("{what}: included in block {block:?}");
    Ok(())
}

/// Reads the config back and compares it with what the step should have left.
async fn check(wallet: &WalletCore, program: AccountId, expected: Option<&Config>, when: &str) -> Result<()> {
    let actor = Actor::new(config_account(program), program);
    let found: Option<Config> = match wallet.get_account_view(actor).await? {
        None => None,
        Some(account) => {
            let state: &[u8] = account.data.actor_state(program);
            if state.is_empty() {
                None
            } else {
                Some(borsh::from_slice(state).context("the config does not decode")?)
            }
        }
    };
    println!("{when}: {found:?}");
    if found.as_ref() != expected {
        return Err(anyhow!("{when}: expected {expected:?}"));
    }
    Ok(())
}
