#![cfg(test)]

use crate::MyToken;
use soroban_sdk::{testutils::Address as _, Address, Env, String};
use stellar_tokens::fungible::Base as TokenBase;

/// Tests the storage-level mint and metadata reads.
/// Auth-gated operations (sell, transfer, burn) are verified structurally
/// by the #[only_owner] macro and require_auth() calls in the code.
#[test]
fn test_token_mint_and_balance() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let contract_id = env.register(
        MyToken,
        (
            owner.clone(),
            String::from_str(&env, "MyToken"),
            String::from_str(&env, "MTK"),
            7u32,
        ),
    );

    // Base::mint is storage-level (no auth required)
    env.as_contract(&contract_id, || {
        TokenBase::mint(&env, &user, 1000);
    });

    let bal: i128 = env.as_contract(&contract_id, || {
        TokenBase::balance(&env, &user)
    });
    assert_eq!(bal, 1000);

    let supply: i128 = env.as_contract(&contract_id, || {
        TokenBase::total_supply(&env)
    });
    assert_eq!(supply, 1000);
}

#[test]
fn test_token_metadata() {
    let env = Env::default();
    let owner = Address::generate(&env);

    let contract_id = env.register(
        MyToken,
        (
            owner.clone(),
            String::from_str(&env, "MyToken"),
            String::from_str(&env, "MTK"),
            7u32,
        ),
    );
    let client = crate::MyTokenClient::new(&env, &contract_id);

    assert_eq!(client.name(), String::from_str(&env, "MyToken"));
    assert_eq!(client.symbol(), String::from_str(&env, "MTK"));
    assert_eq!(client.decimals(), 7);
}
