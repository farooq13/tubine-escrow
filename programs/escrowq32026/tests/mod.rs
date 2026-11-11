use {
    anchor_lang::{
        prelude::Clock, solana_program::instruction::Instruction,
        solana_program::program_pack::Pack, AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::spl_token,
    },
    litesvm::LiteSVM,
    litesvm_token::{
        spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
    },
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

const DEPOSIT: u64 = 10_000_000;
const INITIAL_TAKER_BALANCE: u64 = 25_000_000;

struct TestContext {
    svm: LiteSVM,
    maker: Keypair,
    taker: Keypair,
    mint_a: Pubkey,
    mint_b: Pubkey,
    maker_ata_a: Pubkey,
    maker_ata_b: Pubkey,
    taker_ata_a: Pubkey,
    taker_ata_b: Pubkey,
}

impl TestContext {
    fn escrow(&self, seed: u64) -> Pubkey {
        Pubkey::find_program_address(
            &[b"escrow", self.maker.pubkey().as_ref(), &seed.to_le_bytes()],
            &escrowq32026::id(),
        )
        .0
    }

    fn vault(&self, seed: u64) -> Pubkey {
        associated_token::get_associated_token_address(&self.escrow(seed), &self.mint_a)
    }

    fn make_ix(
        &self,
        seed: u64,
        deposit: u64,
        receive: u64,
        unlock_at: i64,
        expiration: i64,
    ) -> Instruction {
        Instruction {
            program_id: escrowq32026::id(),
            accounts: escrowq32026::accounts::Make {
                maker: self.maker.pubkey(),
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                maker_ata_a: self.maker_ata_a,
                escrow: self.escrow(seed),
                vault: self.vault(seed),
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: anchor_lang::system_program::ID,
            }
            .to_account_metas(None),
            data: escrowq32026::instruction::Make {
                seed,
                deposit,
                receive,
                unlock_at,
                expiration,
            }
            .data(),
        }
    }

    fn update_ix(&self, seed: u64, receive: u64, unlock_at: i64, expiration: i64) -> Instruction {
        Instruction {
            program_id: escrowq32026::id(),
            accounts: escrowq32026::accounts::Update {
                maker: self.maker.pubkey(),
                escrow: self.escrow(seed),
            }
            .to_account_metas(None),
            data: escrowq32026::instruction::Update {
                receive,
                unlock_at,
                expiration,
            }
            .data(),
        }
    }

    fn take_ix(&self, seed: u64) -> Instruction {
        Instruction {
            program_id: escrowq32026::id(),
            accounts: escrowq32026::accounts::Take {
                taker: self.taker.pubkey(),
                maker: self.maker.pubkey(),
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                taker_ata_a: self.taker_ata_a,
                taker_ata_b: self.taker_ata_b,
                maker_ata_b: self.maker_ata_b,
                escrow: self.escrow(seed),
                vault: self.vault(seed),
                token_program: TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: escrowq32026::instruction::Take {}.data(),
        }
    }

    fn refund_ix(&self, seed: u64) -> Instruction {
        Instruction {
            program_id: escrowq32026::id(),
            accounts: escrowq32026::accounts::Refund {
                maker: self.maker.pubkey(),
                mint_a: self.mint_a,
                maker_ata_a: self.maker_ata_a,
                escrow: self.escrow(seed),
                vault: self.vault(seed),
                token_program: TOKEN_PROGRAM_ID,
                system_program: anchor_lang::system_program::ID,
            }
            .to_account_metas(None),
            data: escrowq32026::instruction::Refund {}.data(),
        }
    }

    fn token_balance(&self, address: &Pubkey) -> u64 {
        let account = self.svm.get_account(address).unwrap();
        spl_token::state::Account::unpack(&account.data)
            .unwrap()
            .amount
    }

    fn escrow_state(&self, seed: u64) -> escrowq32026::state::Escrow {
        let account = self.svm.get_account(&self.escrow(seed)).unwrap();
        escrowq32026::state::Escrow::try_deserialize(&mut account.data.as_ref()).unwrap()
    }

    fn now(&self) -> i64 {
        self.svm.get_sysvar::<Clock>().unix_timestamp
    }

    fn set_time(&mut self, unix_timestamp: i64) {
        let mut clock = self.svm.get_sysvar::<Clock>();
        clock.unix_timestamp = unix_timestamp;
        self.svm.set_sysvar(&clock);
        self.svm.expire_blockhash();
    }
}

fn setup() -> TestContext {
    let maker = Keypair::new();
    let taker = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/escrowq32026.so"
    ));
    svm.add_program(escrowq32026::id(), bytes).unwrap();
    svm.airdrop(&maker.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&taker.pubkey(), 1_000_000_000).unwrap();

    let mint_a = CreateMint::new(&mut svm, &maker)
        .decimals(6)
        .authority(&maker.pubkey())
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut svm, &maker)
        .decimals(6)
        .authority(&maker.pubkey())
        .send()
        .unwrap();
    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .owner(&maker.pubkey())
        .send()
        .unwrap();
    let maker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
        .owner(&maker.pubkey())
        .send()
        .unwrap();
    let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .owner(&taker.pubkey())
        .send()
        .unwrap();
    let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
        .owner(&taker.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, DEPOSIT * 2)
        .send()
        .unwrap();
    MintTo::new(
        &mut svm,
        &maker,
        &mint_b,
        &taker_ata_b,
        INITIAL_TAKER_BALANCE,
    )
    .send()
    .unwrap();

    TestContext {
        svm,
        maker,
        taker,
        mint_a,
        mint_b,
        maker_ata_a,
        maker_ata_b,
        taker_ata_a,
        taker_ata_b,
    }
}

fn submit(svm: &mut LiteSVM, signer: &Keypair, instruction: Instruction) -> Result<(), String> {
    let message = Message::new(&[instruction], Some(&signer.pubkey()));
    let transaction = Transaction::new(&[signer], message, svm.latest_blockhash());
    svm.send_transaction(transaction)
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn create_escrow(
    context: &mut TestContext,
    seed: u64,
    receive: u64,
    unlock_at: i64,
    expiration: i64,
) {
    let ix = context.make_ix(seed, DEPOSIT, receive, unlock_at, expiration);
    submit(&mut context.svm, &context.maker, ix).unwrap();
}

#[test]
fn make_deposits_tokens_and_initializes_escrow() {
    let mut context = setup();
    let seed = 11;
    let unlock_at = context.now() + 100;
    let expiration = unlock_at + 1_000;
    let maker_balance_before = context.token_balance(&context.maker_ata_a);

    create_escrow(&mut context, seed, 7_000_000, unlock_at, expiration);

    assert_eq!(
        context.token_balance(&context.maker_ata_a),
        maker_balance_before - DEPOSIT
    );
    assert_eq!(context.token_balance(&context.vault(seed)), DEPOSIT);
    let escrow = context.escrow_state(seed);
    assert_eq!(escrow.seed, seed);
    assert_eq!(escrow.maker, context.maker.pubkey());
    assert_eq!(escrow.mint_a, context.mint_a);
    assert_eq!(escrow.mint_b, context.mint_b);
    assert_eq!(escrow.receive, 7_000_000);
    assert_eq!(escrow.unlock_at, unlock_at);
    assert_eq!(escrow.expiration, expiration);
}

#[test]
fn make_rejects_zero_amounts_and_invalid_time_windows() {
    let mut context = setup();
    let now = context.now();

    for (seed, deposit, receive, unlock_at, expiration) in [
        (21, 0, 1, now + 100, now + 1_000),
        (22, DEPOSIT, 0, now + 100, now + 1_000),
        (23, DEPOSIT, 1, now, now + 1_000),
        (24, DEPOSIT, 1, now + 1_000, now + 1_000),
    ] {
        let ix = context.make_ix(seed, deposit, receive, unlock_at, expiration);
        assert!(
            submit(&mut context.svm, &context.maker, ix).is_err(),
            "make should reject seed {seed}"
        );
        assert!(context.svm.get_account(&context.escrow(seed)).is_none());
        assert!(context.svm.get_account(&context.vault(seed)).is_none());
    }
}

#[test]
fn update_changes_the_requested_terms() {
    let mut context = setup();
    let seed = 31;
    let original_unlock_at = context.now() + 1_000;
    let original_expiration = original_unlock_at + 1_000;
    create_escrow(
        &mut context,
        seed,
        5_000_000,
        original_unlock_at,
        original_expiration,
    );

    let new_unlock_at = context.now() + 2_000;
    let new_expiration = new_unlock_at + 1_000;
    let ix = context.update_ix(seed, 8_000_000, new_unlock_at, new_expiration);
    submit(&mut context.svm, &context.maker, ix).unwrap();

    let escrow = context.escrow_state(seed);
    assert_eq!(escrow.receive, 8_000_000);
    assert_eq!(escrow.unlock_at, new_unlock_at);
    assert_eq!(escrow.expiration, new_expiration);
    assert_eq!(context.token_balance(&context.vault(seed)), DEPOSIT);
}

#[test]
fn update_rejects_non_maker_zero_amount_and_invalid_time_window() {
    let mut context = setup();
    let seed = 32;
    let unlock_at = context.now() + 1_000;
    let expiration = unlock_at + 1_000;
    create_escrow(&mut context, seed, 5_000_000, unlock_at, expiration);

    let unauthorized_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Update {
            maker: context.taker.pubkey(),
            escrow: context.escrow(seed),
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Update {
            receive: 8_000_000,
            unlock_at,
            expiration,
        }
        .data(),
    };
    assert!(submit(&mut context.svm, &context.taker, unauthorized_ix).is_err());

    let zero_amount_ix = context.update_ix(seed, 0, unlock_at, expiration);
    assert!(submit(&mut context.svm, &context.maker, zero_amount_ix).is_err());

    let invalid_window_ix = context.update_ix(seed, 8_000_000, expiration, expiration);
    assert!(submit(&mut context.svm, &context.maker, invalid_window_ix).is_err());

    let escrow = context.escrow_state(seed);
    assert_eq!(escrow.receive, 5_000_000);
    assert_eq!(escrow.unlock_at, unlock_at);
    assert_eq!(escrow.expiration, expiration);
}

#[test]
fn update_rejects_after_the_claim_window_opens() {
    let mut context = setup();
    let seed = 33;
    let unlock_at = context.now() + 10;
    let expiration = unlock_at + 1_000;
    create_escrow(&mut context, seed, 5_000_000, unlock_at, expiration);
    context.set_time(unlock_at);

    let ix = context.update_ix(seed, 8_000_000, unlock_at + 1, expiration + 1);
    assert!(submit(&mut context.svm, &context.maker, ix).is_err());
    let escrow = context.escrow_state(seed);
    assert_eq!(escrow.receive, 5_000_000);
    assert_eq!(escrow.unlock_at, unlock_at);
    assert_eq!(escrow.expiration, expiration);
}

#[test]
fn updated_unlock_time_controls_take_eligibility() {
    let mut context = setup();
    let seed = 34;
    let original_unlock_at = context.now() + 10;
    let original_expiration = original_unlock_at + 1_000;
    create_escrow(
        &mut context,
        seed,
        5_000_000,
        original_unlock_at,
        original_expiration,
    );

    let updated_unlock_at = original_unlock_at + 10;
    let updated_expiration = updated_unlock_at + 1_000;
    let update_ix = context.update_ix(seed, 8_000_000, updated_unlock_at, updated_expiration);
    submit(&mut context.svm, &context.maker, update_ix).unwrap();

    context.set_time(original_unlock_at);
    let early_take_ix = context.take_ix(seed);
    assert!(submit(&mut context.svm, &context.taker, early_take_ix).is_err());
    assert_eq!(context.token_balance(&context.vault(seed)), DEPOSIT);

    context.set_time(updated_unlock_at);
    let eligible_take_ix = context.take_ix(seed);
    submit(&mut context.svm, &context.taker, eligible_take_ix).unwrap();
    assert_eq!(context.token_balance(&context.taker_ata_a), DEPOSIT);
    assert_eq!(context.token_balance(&context.maker_ata_b), 8_000_000);
    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
}

#[test]
fn take_exchanges_tokens_and_closes_escrow_and_vault() {
    let mut context = setup();
    let seed = 41;
    let receive = 12_000_000;
    let unlock_at = context.now() + 10;
    let expiration = unlock_at + 1_000;
    create_escrow(&mut context, seed, receive, unlock_at, expiration);
    let maker_b_before = context.token_balance(&context.maker_ata_b);
    let taker_b_before = context.token_balance(&context.taker_ata_b);

    let too_early_ix = context.take_ix(seed);
    assert!(submit(&mut context.svm, &context.taker, too_early_ix).is_err());
    assert_eq!(context.token_balance(&context.taker_ata_b), taker_b_before);
    assert_eq!(context.token_balance(&context.vault(seed)), DEPOSIT);

    context.set_time(unlock_at);
    let ix = context.take_ix(seed);
    submit(&mut context.svm, &context.taker, ix).unwrap();

    assert_eq!(context.token_balance(&context.taker_ata_a), DEPOSIT);
    assert_eq!(
        context.token_balance(&context.maker_ata_b),
        maker_b_before + receive
    );
    assert_eq!(
        context.token_balance(&context.taker_ata_b),
        taker_b_before - receive
    );
    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
    assert!(context.svm.get_account(&context.vault(seed)).is_none());
}

#[test]
fn take_rejects_expired_escrow_without_moving_tokens() {
    let mut context = setup();
    let seed = 42;
    let unlock_at = context.now() + 1;
    let expiration = unlock_at + 1;
    create_escrow(&mut context, seed, 5_000_000, unlock_at, expiration);
    context.set_time(expiration);
    let taker_a_before = context.token_balance(&context.taker_ata_a);
    let taker_b_before = context.token_balance(&context.taker_ata_b);
    let maker_b_before = context.token_balance(&context.maker_ata_b);

    let ix = context.take_ix(seed);
    assert!(submit(&mut context.svm, &context.taker, ix).is_err());

    assert_eq!(context.token_balance(&context.taker_ata_a), taker_a_before);
    assert_eq!(context.token_balance(&context.taker_ata_b), taker_b_before);
    assert_eq!(context.token_balance(&context.maker_ata_b), maker_b_before);
    assert!(context.svm.get_account(&context.escrow(seed)).is_some());
    assert_eq!(context.token_balance(&context.vault(seed)), DEPOSIT);
}

#[test]
fn take_rejects_insufficient_requested_tokens_atomically() {
    let mut context = setup();
    let seed = 43;
    let unlock_at = context.now() + 10;
    let expiration = unlock_at + 1_000;
    create_escrow(
        &mut context,
        seed,
        INITIAL_TAKER_BALANCE + 1,
        unlock_at,
        expiration,
    );
    context.set_time(unlock_at);
    let taker_b_before = context.token_balance(&context.taker_ata_b);
    let maker_b_before = context.token_balance(&context.maker_ata_b);

    let ix = context.take_ix(seed);
    assert!(submit(&mut context.svm, &context.taker, ix).is_err());

    assert_eq!(context.token_balance(&context.taker_ata_b), taker_b_before);
    assert_eq!(context.token_balance(&context.maker_ata_b), maker_b_before);
    assert_eq!(context.token_balance(&context.vault(seed)), DEPOSIT);
    assert!(context.svm.get_account(&context.escrow(seed)).is_some());
}

#[test]
fn refund_returns_tokens_and_closes_escrow_even_after_expiration() {
    let mut context = setup();
    let seed = 51;
    let unlock_at = context.now() + 1;
    let expiration = unlock_at + 1;
    let maker_balance_before = context.token_balance(&context.maker_ata_a);
    create_escrow(&mut context, seed, 5_000_000, unlock_at, expiration);
    context.set_time(expiration);

    let ix = context.refund_ix(seed);
    submit(&mut context.svm, &context.maker, ix).unwrap();

    assert_eq!(
        context.token_balance(&context.maker_ata_a),
        maker_balance_before
    );
    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
    assert!(context.svm.get_account(&context.vault(seed)).is_none());
}

#[test]
fn refund_rejects_until_expiration() {
    let mut context = setup();
    let seed = 53;
    let unlock_at = context.now() + 10;
    let expiration = unlock_at + 100;
    let maker_balance_before = context.token_balance(&context.maker_ata_a);
    create_escrow(&mut context, seed, 5_000_000, unlock_at, expiration);

    let before_unlock_ix = context.refund_ix(seed);
    assert!(submit(&mut context.svm, &context.maker, before_unlock_ix).is_err());
    context.set_time(unlock_at);
    let during_window_ix = context.refund_ix(seed);
    assert!(submit(&mut context.svm, &context.maker, during_window_ix).is_err());

    assert_eq!(
        context.token_balance(&context.maker_ata_a),
        maker_balance_before - DEPOSIT
    );
    assert_eq!(context.token_balance(&context.vault(seed)), DEPOSIT);
    assert!(context.svm.get_account(&context.escrow(seed)).is_some());
}

#[test]
fn refund_rejects_non_maker() {
    let mut context = setup();
    let seed = 52;
    let unlock_at = context.now() + 100;
    let expiration = unlock_at + 1_000;
    create_escrow(&mut context, seed, 5_000_000, unlock_at, expiration);
    let maker_a_before = context.token_balance(&context.maker_ata_a);

    let unauthorized_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Refund {
            maker: context.taker.pubkey(),
            mint_a: context.mint_a,
            maker_ata_a: context.taker_ata_a,
            escrow: context.escrow(seed),
            vault: context.vault(seed),
            token_program: TOKEN_PROGRAM_ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Refund {}.data(),
    };
    assert!(submit(&mut context.svm, &context.taker, unauthorized_ix).is_err());

    assert_eq!(context.token_balance(&context.maker_ata_a), maker_a_before);
    assert_eq!(context.token_balance(&context.vault(seed)), DEPOSIT);
    assert!(context.svm.get_account(&context.escrow(seed)).is_some());
}
