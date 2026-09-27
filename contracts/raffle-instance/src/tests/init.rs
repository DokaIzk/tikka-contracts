//! Config validation and init rejection branches (claim lockup, bounds, metadata hash).

use super::*;

#[test]
fn test_init_claim_lockup_seconds_at_bound_succeeds() {
    let (env, contract_id, factory, admin, creator, payment_token) = init_bounds_env();
    let client = ContractClient::new(&env, &contract_id);
    let mut config = base_config(&env, &payment_token);
    config.claim_lockup_seconds = Some(MAX_CLAIM_LOCKUP_SECONDS);

    client.init(&factory, &admin, &creator, &config);

    assert_eq!(client.get_raffle().claim_lockup_seconds, MAX_CLAIM_LOCKUP_SECONDS);
}

#[test]
fn test_init_claim_lockup_seconds_above_bound_rejected() {
    let (env, contract_id, factory, admin, creator, payment_token) = init_bounds_env();
    let client = ContractClient::new(&env, &contract_id);
    let mut config = base_config(&env, &payment_token);
    config.claim_lockup_seconds = Some(MAX_CLAIM_LOCKUP_SECONDS + 1);

    assert_eq!(
        client.try_init(&factory, &admin, &creator, &config),
        Err(Ok(Error::InvalidParameters))
    );
}

#[test]
#[cfg(any())]
fn test_init_claim_lockup_seconds_mid_range_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let payment_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token = StellarAssetClient::new(&env, &payment_token);
    let token_ro = soroban_sdk::token::Client::new(&env, &payment_token);
    token.mint(&creator, &100_000_000);
    token.mint(&buyer_a, &1_000_000);
    token.mint(&buyer_b, &1_000_000);
    token.mint(&buyer_c, &1_000_000);

    // 2. Init the raffle (factory-mediated init emulated directly).
    let config = lifecycle_config(&env, &payment_token, &treasury);
    let expected_metadata_hash = config.metadata_hash.clone();
    let prize_amount = config.prize_amount;
    let ticket_price = config.ticket_price;
    let fee_bp = config.protocol_fee_bp as i128;
    client.init(&creator_factory_addr(&env), &admin, &creator, &config);
    assert_metadata_hash(&client, &expected_metadata_hash);

    // Skip the factory cross-contract calls in buy_tickets.
    env.as_contract(&contract_id, || {
        env.storage().instance().remove(&DataKey::Factory);
    });

    assert_eq!(client.get_raffle().status, RaffleStatus::PendingPrize);
    assert_metadata_hash(&client, &expected_metadata_hash);

    // 3. Creator deposits the prize → status becomes Active.
    let creator_balance_before_deposit = token_ro.balance(&creator);
    let start_events = env.events().all().len();
    client.deposit_prize();
    assert_event(
        &env,
        &client.address,
        "prize_deposited",
        events::PrizeDeposited {
            creator: creator.clone(),
            amount: prize_amount,
            token: payment_token.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );
    assert_eq!(env.events().all().len(), start_events + 1);
    assert_eq!(client.get_raffle().status, RaffleStatus::Active);
    assert_metadata_hash(&client, &expected_metadata_hash);
    assert_eq!(
        token_ro.balance(&creator),
        creator_balance_before_deposit - prize_amount
    );
    assert_eq!(token_ro.balance(&contract_id), prize_amount);

    // 4. Three buyers each purchase one ticket. Each pays ticket_price; the
    //    protocol fee (1%) is forwarded to the treasury on each purchase.
    let per_ticket_fee = ticket_price * fee_bp / 10_000;
    for (idx, buyer) in [&buyer_a, &buyer_b, &buyer_c].iter().enumerate() {
        let start_events = env.events().all().len();
        let sold = client.buy_tickets(buyer, &1);
        
        assert_event(
            &env,
            &client.address,
            "ticket_purchased",
            events::TicketPurchased {
                buyer: buyer.clone(),
                ticket_ids: soroban_sdk::vec![&env, sold],
                quantity: 1,
                ticket_price: ticket_price,
                effective_ticket_price: ticket_price,
                total_paid: ticket_price,
                protocol_fee: ticket_price * fee_bp / 10_000,
                timestamp: env.ledger().timestamp(),
            },
        );
        // We know buy_tickets also emits a StatusChanged event when it sells out (the last iteration).
        if idx < 2 {
            assert_eq!(env.events().all().len(), start_events + 1);
        } else {
            assert_eq!(env.events().all().len(), start_events + 2); // StatusChanged emitted on the last one!
        }
        
        assert_eq!(sold, idx as u32 + 1);
        assert_metadata_hash(&client, &expected_metadata_hash);
    }
    let raffle_after_sales = client.get_raffle();
    assert_eq!(raffle_after_sales.tickets_sold, 3);
    // Selling out flips the raffle into Drawing.
    assert_eq!(raffle_after_sales.status, RaffleStatus::Drawing);
    assert_eq!(token_ro.balance(&treasury), per_ticket_fee * 3);

    // 5. Finalize → internal randomness picks the winner synchronously.
    let start_events = env.events().all().len();
    client.finalize_raffle();
    
    assert_event(
        &env,
        &client.address,
        "raffle_finalized",
        events::RaffleFinalized {
            raffle_id: client.address.clone(),
            winners: soroban_sdk::vec![&env, client.get_raffle().winners.get(0).unwrap().address],
            winning_ticket_ids: soroban_sdk::vec![&env, 1], // buyer_a is first ticket
            total_tickets_sold: 3,
            randomness_source: RandomnessSource::Internal,
            randomness_type: raffle_shared::RandomnessType::PseudoRandom,
            finalized_at: env.ledger().timestamp(),
        },
    );
    let finalized = client.get_raffle();
    assert_eq!(finalized.status, RaffleStatus::Finalized);
    assert_metadata_hash(&client, &expected_metadata_hash);
    assert_eq!(finalized.winners.len(), 1);
}

/// #485: with unique_winners, two buyers with multiple tickets each win at most one tier.

#[test]
fn test_init_claim_lockup_seconds_mid_range_succeeds() {
    let (env, contract_id, factory, admin, creator, payment_token) = init_bounds_env();
    let client = ContractClient::new(&env, &contract_id);
    let lockup_seconds = MAX_CLAIM_LOCKUP_SECONDS / 2;
    let mut config = base_config(&env, &payment_token);
    config.claim_lockup_seconds = Some(lockup_seconds);

    client.init(&factory, &admin, &creator, &config);

    assert_eq!(client.get_raffle().claim_lockup_seconds, lockup_seconds);
}

#[test]
fn init_accepts_min_ticket_price_and_rejects_below_it() {
    let (env, contract_id, factory, admin, creator, payment_token) = init_bounds_env();
    let client = ContractClient::new(&env, &contract_id);
    let prizes = soroban_sdk::vec![&env, 10000u32];

    let config = init_bounds_config(
        &env,
        &payment_token,
        String::from_str(&env, "min ticket price"),
        5,
        MIN_TICKET_PRICE,
        MIN_TICKET_PRICE * 5,
        prizes.clone(),
    );
    client.init(&factory, &admin, &creator, &config);

    let invalid = init_bounds_config(
        &env,
        &payment_token,
        String::from_str(&env, "min ticket price"),
        5,
        MIN_TICKET_PRICE - 1,
        MIN_TICKET_PRICE * 5,
        prizes,
    );
    assert_eq!(
        client.try_init(&factory, &admin, &creator, &invalid),
        Err(Ok(Error::InvalidParameters))
    );
}

#[test]
fn init_accepts_max_prize_amount_and_rejects_above_it() {
    let (env, contract_id, factory, admin, creator, payment_token) = init_bounds_env();
    let client = ContractClient::new(&env, &contract_id);
    let prizes = soroban_sdk::vec![&env, 10000u32];

    let config = init_bounds_config(
        &env,
        &payment_token,
        String::from_str(&env, "max prize amount"),
        5,
        MIN_TICKET_PRICE,
        MAX_PRIZE_AMOUNT,
        prizes.clone(),
    );
    client.init(&factory, &admin, &creator, &config);

    let invalid = init_bounds_config(
        &env,
        &payment_token,
        String::from_str(&env, "max prize amount"),
        5,
        MIN_TICKET_PRICE,
        MAX_PRIZE_AMOUNT + 1,
        prizes,
    );
    assert_eq!(
        client.try_init(&factory, &admin, &creator, &invalid),
        Err(Ok(Error::InvalidParameters))
    );
}

#[test]
fn init_accepts_max_description_length_and_rejects_above_it() {
    let (env, contract_id, factory, admin, creator, payment_token) = init_bounds_env();
    let client = ContractClient::new(&env, &contract_id);
    let prizes = soroban_sdk::vec![&env, 10000u32];
    let inside_description = String::from_str(&env, &"a".repeat(MAX_DESCRIPTION_LENGTH as usize));
    let outside_description =
        String::from_str(&env, &"a".repeat(MAX_DESCRIPTION_LENGTH as usize + 1));

    let config = init_bounds_config(
        &env,
        &payment_token,
        inside_description,
        5,
        MIN_TICKET_PRICE,
        MIN_TICKET_PRICE * 5,
        prizes.clone(),
    );
    client.init(&factory, &admin, &creator, &config);

    let invalid = init_bounds_config(
        &env,
        &payment_token,
        outside_description,
        5,
        MIN_TICKET_PRICE,
        MIN_TICKET_PRICE * 5,
        prizes,
    );
    assert_eq!(
        client.try_init(&factory, &admin, &creator, &invalid),
        Err(Ok(Error::InvalidParameters))
    );
}

#[test]
fn init_accepts_max_prizes_and_rejects_above_it() {
    let (env, contract_id, factory, admin, creator, payment_token) = init_bounds_env();
    let client = ContractClient::new(&env, &contract_id);
    let mut inside_prizes = soroban_sdk::Vec::new(&env);
    for _ in 0..MAX_PRIZES {
        inside_prizes.push_back(100u32);
    }
    let mut outside_prizes = soroban_sdk::Vec::new(&env);
    for _ in 0..(MAX_PRIZES + 1) {
        outside_prizes.push_back(100u32);
    }

    let config = init_bounds_config(
        &env,
        &payment_token,
        String::from_str(&env, "max prizes"),
        MAX_PRIZES,
        MIN_TICKET_PRICE,
        MIN_TICKET_PRICE * 5,
        inside_prizes,
    );
    client.init(&factory, &admin, &creator, &config);

    let invalid = init_bounds_config(
        &env,
        &payment_token,
        String::from_str(&env, "max prizes"),
        MAX_PRIZES,
        MIN_TICKET_PRICE,
        MIN_TICKET_PRICE * 5,
        outside_prizes,
    );
    assert_eq!(
        client.try_init(&factory, &admin, &creator, &invalid),
        Err(Ok(Error::TooManyPrizes))
    );
}

#[test]
fn init_accepts_max_tickets_limit_and_rejects_above_it() {
    let (env, contract_id, factory, admin, creator, payment_token) = init_bounds_env();
    let client = ContractClient::new(&env, &contract_id);
    let prizes = soroban_sdk::vec![&env, 10000u32];

    let config = init_bounds_config(
        &env,
        &payment_token,
        String::from_str(&env, "max tickets"),
        MAX_TICKETS_LIMIT,
        MIN_TICKET_PRICE,
        MIN_TICKET_PRICE * 5,
        prizes.clone(),
    );
    client.init(&factory, &admin, &creator, &config);

    let invalid = init_bounds_config(
        &env,
        &payment_token,
        String::from_str(&env, "max tickets"),
        MAX_TICKETS_LIMIT + 1,
        MIN_TICKET_PRICE,
        MIN_TICKET_PRICE * 5,
        prizes,
    );
    assert_eq!(
        client.try_init(&factory, &admin, &creator, &invalid),
        Err(Ok(Error::InvalidParameters))
    );
}

// ===========================================================================
// buy_tickets budget benchmark near maximum ticket count (#449)
//
// Confirms a full `max_tickets_per_tx` batch of purchases near the 100_000
// ticket ceiling stays within Soroban's per-invocation CPU/memory limits and
// still triggers the transition into `Drawing` on the final batch.
//
// NOTE: The companion `get_tickets_page_is_efficient_for_large_raffles` test
// from #449 is intentionally omitted — the raffle instance does not currently
// expose a `get_tickets_page` view, so there is no function to benchmark. It
// should be added alongside a paginated ticket-read view in a follow-up.
// ===========================================================================

#[test]
fn update_metadata_hash_before_deposit_only() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let factory = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = sac.address();

    let mut config = base_config(&env, &token_addr);
    config.description = String::from_str(&env, "metadata");
    config.max_tickets = 5;
    config.max_tickets_per_tx = 5;
    config.prize_amount = 50_000;
    config.metadata_hash = BytesN::from_array(&env, &[1u8; 32]);

    client.init(&factory, &admin, &creator, &config);
    let new_hash = BytesN::from_array(&env, &[2u8; 32]);
    client.update_metadata_hash(&new_hash);
    assert_eq!(client.get_raffle().metadata_hash, new_hash);

    StellarAssetClient::new(&env, &token_addr).mint(&creator, &1_000_000);
    client.deposit_prize();
    assert_eq!(
        client.try_update_metadata_hash(&BytesN::from_array(&env, &[3u8; 32])),
        Err(Ok(Error::InvalidStatus))
    );
}

#[test]
fn test_explicit_zero_lockup_is_honored() {
    let env = Env::default();
    env.mock_all_auths();
    let factory = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let payment_token = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let mut config = base_config(&env, &payment_token);
    config.description = String::from_str(&env, "Zero Lockup");
    config.max_tickets = 1;
    config.max_tickets_per_tx = 1;
    config.claim_lockup_seconds = Some(0);
    config.swap_deadline_seconds = Some(0);
    config.metadata_hash = BytesN::from_array(&env, &[99; 32]);

    client.init(&factory, &admin, &creator, &config);
    let raffle = client.get_raffle();
    
    // Explicit 0 should be stored as 0, not DEFAULT_CLAIM_LOCKUP_SECONDS
    assert_eq!(raffle.claim_lockup_seconds, 0);
    assert_eq!(raffle.swap_deadline_seconds, 0);
}

#[test]
fn test_unset_lockup_gets_default() {
    let env = Env::default();
    env.mock_all_auths();
    let factory = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let payment_token = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let mut config = base_config(&env, &payment_token);
    config.description = String::from_str(&env, "Default Lockup");
    config.max_tickets = 1;
    config.max_tickets_per_tx = 1;
    config.metadata_hash = BytesN::from_array(&env, &[100; 32]);

    client.init(&factory, &admin, &creator, &config);
    let raffle = client.get_raffle();
    
    // None should be resolved to the defaults
    assert_eq!(raffle.claim_lockup_seconds, DEFAULT_CLAIM_LOCKUP_SECONDS);
    assert_eq!(raffle.swap_deadline_seconds, DEFAULT_SWAP_DEADLINE_SECONDS);
}

#[test]
fn init_rejects_quorum_k_zero() {
    let (env, _, factory, admin, creator, payment_token) = init_bounds_env();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let oracle = Address::generate(&env);
    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle);

    let mut config = base_config(&env, &payment_token);
    config.randomness_source = RandomnessSource::Quorum(QuorumConfig { k: 0, oracles });

    assert_eq!(
        client.try_init(&factory, &admin, &creator, &config),
        Err(Ok(Error::InvalidParameters))
    );
}

#[test]
fn init_rejects_quorum_k_greater_than_n() {
    let (env, _, factory, admin, creator, payment_token) = init_bounds_env();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let oracle = Address::generate(&env);
    let mut oracles = Vec::new(&env);
    oracles.push_back(oracle);

    let mut config = base_config(&env, &payment_token);
    config.randomness_source = RandomnessSource::Quorum(QuorumConfig { k: 2, oracles });

    assert_eq!(
        client.try_init(&factory, &admin, &creator, &config),
        Err(Ok(Error::InvalidParameters))
    );
}

#[test]
fn init_rejects_quorum_more_than_ten_oracles() {
    let (env, _, factory, admin, creator, payment_token) = init_bounds_env();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let mut oracles = Vec::new(&env);
    for _ in 0..11 {
        oracles.push_back(Address::generate(&env));
    }

    let mut config = base_config(&env, &payment_token);
    config.randomness_source = RandomnessSource::Quorum(QuorumConfig { k: 2, oracles });

    assert_eq!(
        client.try_init(&factory, &admin, &creator, &config),
        Err(Ok(Error::InvalidParameters))
    );
}

#[test]
fn init_rejects_quorum_oracle_equal_to_contract() {
    let (env, _, factory, admin, creator, payment_token) = init_bounds_env();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let mut oracles = Vec::new(&env);
    oracles.push_back(contract_id.clone());

    let mut config = base_config(&env, &payment_token);
    config.randomness_source = RandomnessSource::Quorum(QuorumConfig { k: 1, oracles });

    assert_eq!(
        client.try_init(&factory, &admin, &creator, &config),
        Err(Ok(Error::InvalidParameters))
    );
}

