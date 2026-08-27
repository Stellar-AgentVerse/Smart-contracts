use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};
use stellar_access::ownable::Ownable;
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{only_owner, when_not_paused};
use stellar_tokens::fungible::burnable::FungibleBurnable;
use stellar_tokens::fungible::{Base, FungibleToken};

use crate::{core::token::TokenManager, storage::types::DataKey};

// SEP-0046 contract metadata embedded in the WASM binary.
soroban_sdk::contractmeta!(
    key = "Description",
    val = "MyToken — SEP-0041 fungible token with owner-gated minting and pausability"
);
soroban_sdk::contractmeta!(key = "Version", val = "0.1.0");

#[contract]
pub struct MyToken;

#[contractimpl]
impl MyToken {
    /// One-time constructor (Protocol 22+ / CAP-0058).
    /// `decimals` is now a parameter instead of being hardcoded to 7 — use 7
    /// for XLM-compatible wallet display.
    pub fn __constructor(e: &Env, owner: Address, name: String, symbol: String, decimals: u32) {
        TokenManager::initialize(e, owner, name, symbol, decimals);
    }

    /// Configure the single marketplace allowed to mint and burn through
    /// contract-to-contract calls. This is intentionally one-time setup.
    #[only_owner]
    pub fn set_marketplace(e: &Env, marketplace: Address) {
        assert!(
            e.storage()
                .instance()
                .get::<_, Address>(&DataKey::Marketplace)
                .is_none(),
            "marketplace already configured"
        );
        e.storage()
            .instance()
            .set(&DataKey::Marketplace, &marketplace);
    }

    pub fn get_marketplace(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::Marketplace)
            .expect("marketplace not configured")
    }

    #[only_owner]
    #[when_not_paused]
    pub fn mint(e: &Env, to: Address, amount: i128) {
        TokenManager::mint(e, &to, amount);
    }

    #[when_not_paused]
    pub fn sell(e: &Env, seller: Address, amount: i128) {
        TokenManager::sell(e, &seller, amount);
    }

    /// Burn tokens on behalf of the configured marketplace.
    #[when_not_paused]
    pub fn marketplace_burn(e: &Env, seller: Address, amount: i128) {
        let marketplace = Self::get_marketplace(e);
        marketplace.require_auth();
        TokenManager::marketplace_burn(e, &seller, amount);
    }

    /// Mint tokens on behalf of the configured marketplace.
    #[when_not_paused]
    pub fn marketplace_mint(e: &Env, to: Address, amount: i128) {
        let marketplace = Self::get_marketplace(e);
        marketplace.require_auth();
        TokenManager::marketplace_mint(e, &to, amount);
    }
}

#[contractimpl]
impl FungibleToken for MyToken {
    type ContractType = Base;

    fn balance(e: &Env, account: Address) -> i128 {
        Base::balance(e, &account)
    }

    fn total_supply(e: &Env) -> i128 {
        Base::total_supply(e)
    }

    fn decimals(e: &Env) -> u32 {
        Base::decimals(e)
    }

    fn name(e: &Env) -> String {
        Base::name(e)
    }

    fn symbol(e: &Env) -> String {
        Base::symbol(e)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        Base::allowance(e, &owner, &spender)
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        Base::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    #[when_not_paused]
    fn transfer(e: &Env, from: Address, to: MuxedAddress, amount: i128) {
        TokenManager::transfer(e, &from, &to, amount);
    }

    #[when_not_paused]
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        TokenManager::transfer_from(e, &spender, &from, &to, amount);
    }
}

/// SEP-0041 requires `FungibleBurnable` for full compliance.
/// Both `burn` and `burn_from` are blocked while the contract is paused.
#[contractimpl]
impl FungibleBurnable for MyToken {
    #[when_not_paused]
    fn burn(e: &Env, from: Address, amount: i128) {
        TokenManager::burn(e, &from, amount);
    }

    #[when_not_paused]
    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        Base::burn_from(e, &spender, &from, amount);
    }
}

#[contractimpl]
impl Pausable for MyToken {
    fn paused(e: &Env) -> bool {
        pausable::paused(e)
    }

    #[only_owner]
    fn pause(e: &Env, _caller: Address) {
        pausable::pause(e);
    }

    #[only_owner]
    fn unpause(e: &Env, _caller: Address) {
        pausable::unpause(e);
    }
}

#[contractimpl]
impl Ownable for MyToken {}
