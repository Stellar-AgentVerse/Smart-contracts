use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};
use stellar_access::ownable::Ownable;
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{only_owner, when_not_paused};
use stellar_tokens::fungible::burnable::FungibleBurnable;
use stellar_tokens::fungible::{Base, FungibleToken};

use crate::core::token::TokenManager;

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

    #[only_owner]
    #[when_not_paused]
    pub fn mint(e: &Env, to: Address, amount: i128) {
        TokenManager::mint(e, &to, amount);
    }

    #[when_not_paused]
    pub fn sell(e: &Env, seller: Address, amount: i128) {
        TokenManager::sell(e, &seller, amount);
    }

    /// Same as `sell` but without `require_auth`. Intended for contract-to-contract
    /// calls (e.g. marketplace) where auth is forwarded from the root invocation.
    #[when_not_paused]
    pub fn sell_forwarded(e: &Env, seller: Address, amount: i128) {
        TokenManager::sell_forwarded(e, &seller, amount);
    }

    /// Same as `mint` but without `require_auth`. Intended for contract-to-contract
    /// calls (e.g. marketplace `remint`) where auth is forwarded from the root.
    #[when_not_paused]
    pub fn mint_forwarded(e: &Env, to: Address, amount: i128) {
        TokenManager::mint(e, &to, amount);
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
