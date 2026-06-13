#![cfg(test)]

use crate::MyToken;
use soroban_sdk::{testutils::Address as _, Address, Env, String};
use stellar_tokens::fungible::Base as TokenBase;

// ─── Helpers ──────────────────────────────────────────────

fn setup_env() -> (Env, Address, Address) {
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
    (env, contract_id, user)
}

// ─── Basic storage operations ─────────────────────────────

#[test]
fn test_token_mint_and_balance() {
    let (env, contract_id, user) = setup_env();

    env.as_contract(&contract_id, || {
        TokenBase::mint(&env, &user, 1000);
    });

    let bal: i128 = env.as_contract(&contract_id, || TokenBase::balance(&env, &user));
    assert_eq!(bal, 1000);

    let supply: i128 = env.as_contract(&contract_id, || TokenBase::total_supply(&env));
    assert_eq!(supply, 1000);
}

#[test]
fn test_token_metadata() {
    let (env, contract_id, ..) = setup_env();
    let client = crate::MyTokenClient::new(&env, &contract_id);

    assert_eq!(client.name(), String::from_str(&env, "MyToken"));
    assert_eq!(client.symbol(), String::from_str(&env, "MTK"));
    assert_eq!(client.decimals(), 7);
}

// ─── Adversarial: cumulative mint / supply tracking ──────

#[test]
fn test_mint_multiple_same_user() {
    let (env, contract_id, user) = setup_env();

    env.as_contract(&contract_id, || {
        TokenBase::mint(&env, &user, 100);
        TokenBase::mint(&env, &user, 200);
        TokenBase::mint(&env, &user, 300);
    });

    let bal: i128 = env.as_contract(&contract_id, || TokenBase::balance(&env, &user));
    assert_eq!(bal, 600);

    let supply: i128 = env.as_contract(&contract_id, || TokenBase::total_supply(&env));
    assert_eq!(supply, 600);
}

#[test]
fn test_mint_to_different_users() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    let contract_id = env.register(
        MyToken,
        (
            owner.clone(),
            String::from_str(&env, "MyToken"),
            String::from_str(&env, "MTK"),
            7u32,
        ),
    );

    env.as_contract(&contract_id, || {
        TokenBase::mint(&env, &alice, 500);
        TokenBase::mint(&env, &bob, 1500);
    });

    assert_eq!(
        env.as_contract(&contract_id, || TokenBase::balance(&env, &alice)),
        500
    );
    assert_eq!(
        env.as_contract(&contract_id, || TokenBase::balance(&env, &bob)),
        1500
    );
    assert_eq!(
        env.as_contract(&contract_id, || TokenBase::total_supply(&env)),
        2000
    );
}

#[test]
#[should_panic]
fn test_mint_overflow_panics() {
    // Minting i128::MAX should succeed; minting more causes overflow.
    let (env, contract_id, user) = setup_env();

    env.as_contract(&contract_id, || {
        TokenBase::mint(&env, &user, i128::MAX);
    });

    env.as_contract(&contract_id, || {
        TokenBase::mint(&env, &user, 1);
    });
}

#[test]
fn test_zero_balance_default() {
    let (env, contract_id, ..) = setup_env();
    let nobody = Address::generate(&env);

    let bal: i128 = env.as_contract(&contract_id, || TokenBase::balance(&env, &nobody));
    assert_eq!(bal, 0);
}
