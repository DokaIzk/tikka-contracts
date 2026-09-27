#![cfg(test)]

extern crate std;
use std::vec;

use crate::*;
use raffle_shared::RaffleConfigBuilder;
pub(crate) use crate::{RaffleInstance as Contract, RaffleInstanceClient as ContractClient};
use soroban_sdk::{
	contract, contractimpl,
	testutils::{budget::Budget, Address as _, Events, Ledger, Register},
	token::StellarAssetClient,
	Address, BytesN, Env, String,
};
use crate::events;

#[contract]
pub struct MockFactory;

#[contractimpl]
impl MockFactory {
	pub fn is_global_paused(_env: Env) -> bool {
		false
	}

	pub fn record_volume(_env: Env, _token: Address, _amount: i128) {}

	pub fn track_participant(_env: Env, _participant: Address) {}
}

pub(crate) fn create_token<'a>(
	env: &'a Env,
	admin: &Address,
) -> (Address, StellarAssetClient<'a>) {
	let payment_token = env
		.register_stellar_asset_contract_v2(admin.clone())
		.address();
	(
		payment_token.clone(),
		StellarAssetClient::new(env, &payment_token),
	)
}

pub(crate) fn base_config(env: &Env, payment_token: &Address) -> RaffleConfig {
	RaffleConfigBuilder::new(env, payment_token.clone())
		.build()
		.expect("valid base raffle config")
}

pub(crate) fn setup_instance(
	env: &Env,
) -> (
	ContractClient<'_>,
	Address,
	Address,
	Address,
	Address,
	Address,
) {
	let contract_id = env.register(Contract, ());
	let client = ContractClient::new(env, &contract_id);
	let factory = Address::generate(env);
	let admin = Address::generate(env);
	let creator = Address::generate(env);
	let (payment_token, _) = create_token(env, &Address::generate(env));
	(client, contract_id, factory, admin, creator, payment_token)
}

pub(crate) fn setup_active_raffle(
	env: &Env,
) -> (
	ContractClient<'_>,
	Address,
	Address,
	Address,
	Address,
	StellarAssetClient<'_>,
) {
	env.mock_all_auths();
	env.ledger().set_timestamp(1_000);
	let contract_id = env.register(Contract, ());
	let client = ContractClient::new(env, &contract_id);
	let factory = env.register(MockFactory, ());
	let admin = Address::generate(env);
	let creator = Address::generate(env);
	let buyer = Address::generate(env);
	let (payment_token, token) = create_token(env, &Address::generate(env));
	token.mint(&creator, &1_000_000);
	token.mint(&buyer, &1_000_000);

	let mut config = base_config(env, &payment_token);
	config.max_tickets = 10;
	config.max_tickets_per_tx = 10;
	config.prize_amount = 10 * raffle_shared::constants::MIN_TICKET_PRICE;
	client.init(&factory, &admin, &creator, &config);
	client.deposit_prize();
	(client, admin, creator, buyer, factory, token)
}

pub(crate) fn creator_factory_addr(env: &Env) -> Address {
	Address::generate(env)
}

pub(crate) fn assert_drawing_lock_cleared(env: &Env, contract_id: &Address) {
	env.as_contract(contract_id, || {
		assert!(!env.storage().instance().has(&DataKey::DrawingLock));
	});
}

pub(crate) fn assert_metadata_hash(client: &ContractClient<'_>, expected: &BytesN<32>) {
	assert_eq!(client.get_raffle().metadata_hash, *expected);
}

pub(crate) fn init_bounds_env() -> (Env, Address, Address, Address, Address, Address) {
	let env = Env::default();
	env.mock_all_auths();
	env.ledger().set_timestamp(1_000);

	let contract_id = env.register(Contract, ());
	let factory = Address::generate(&env);
	let admin = Address::generate(&env);
	let creator = Address::generate(&env);
	let (payment_token, _) = create_token(&env, &Address::generate(&env));
	(env, contract_id, factory, admin, creator, payment_token)
}

pub(crate) fn init_bounds_config(
	env: &Env,
	payment_token: &Address,
	description: String,
	max_tickets: u32,
	ticket_price: i128,
	prize_amount: i128,
	prizes: soroban_sdk::Vec<u32>,
) -> RaffleConfig {
	RaffleConfig {
		description,
		end_time: 0,
		no_deadline: true,
		max_tickets,
		max_tickets_per_tx: max_tickets,
		max_tickets_per_address: 0,
		min_tickets: 1,
		allow_multiple: true,
		ticket_price,
		payment_token: payment_token.clone(),
		prize_amount,
		prizes,
		randomness_source: RandomnessSource::Internal,
		oracle_address: None,
		protocol_fee_bp: 0,
		treasury_address: None,
		swap_router: None,
		tikka_token: None,
		metadata_hash: BytesN::from_array(env, &[72u8; 32]),
		claim_lockup_seconds: None,
		claim_expiry_seconds: None,
		swap_deadline_seconds: None,
		early_bird_ticket_percentage: 0,
		early_bird_discount_bp: 0,
		category: None,
		unique_winners: false,
		bundles: soroban_sdk::Vec::new(env),
		prize_token: None,
		nft_contract: None,
	}
}

pub mod budget;
pub mod fairness;
pub mod draw;
pub mod invariants;
pub mod ttl;
pub mod claim_state;
pub mod claim;
pub mod init;
pub mod admin;
pub mod tickets;
