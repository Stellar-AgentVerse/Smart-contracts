#![cfg(test)]
extern crate std;

use crate::events::{MintEvent, SellEvent};
use crate::{MyToken, MyTokenClient};
use soroban_sdk::{testutils::Address as _, testutils::Events as _, Address, Env, Event, String};
use stellar_access::ownable;
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

// ─── `*_forwarded` cross-contract trust boundary ───────────
//
// `sell_forwarded` and `mint_forwarded` deliberately skip `require_auth()`
// so the marketplace can call them after it already authorized the root
// invocation (see `contracts/marketplace/src/contract.rs::buy_prompt` /
// `remint`). That means these two functions are the highest-risk surface
// in the whole workspace: any caller, not just the marketplace, can invoke
// them directly with NO authorization check on `seller`/`to` whatsoever.
//
// These tests invoke them directly via the public client with no
// `mock_auths()` set up at all, proving (a) the burn/mint logic behaves
// correctly and (b) the functions truly require no auth to execute —
// which is exactly the property that makes them dangerous outside the
// marketplace's controlled call path.

#[test]
fn test_sell_forwarded_updates_balance() {
    let (env, contract_id, user) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);

    env.as_contract(&contract_id, || {
        TokenBase::mint(&env, &user, 1000);
    });

    // No mock_auths() anywhere — sell_forwarded must succeed without
    // the seller ever authorizing this call directly.
    client.sell_forwarded(&user, &400);

    // Events must be read before any further contract invocation — each
    // top-level call resets the recorded event buffer. `Base::update`
    // also publishes its own SEP-41 event, so check our custom `SellEvent`
    // is present rather than asserting on the full (implementation-coupled)
    // event list.
    let expected = SellEvent {
        seller: user.clone(),
        amount: 400,
    };
    assert!(env
        .events()
        .all()
        .events()
        .contains(&expected.to_xdr(&env, &contract_id)));

    assert_eq!(client.balance(&user), 600);
    assert_eq!(client.total_supply(), 600);
}

#[test]
fn test_mint_forwarded_mints_tokens() {
    let (env, contract_id, user) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);
    let owner = env
        .as_contract(&contract_id, || ownable::get_owner(&env))
        .expect("owner must be set");

    // No mock_auths() anywhere — mint_forwarded must succeed without
    // the contract owner authorizing this call directly.
    client.mint_forwarded(&user, &750);

    // Events must be read before any further contract invocation — each
    // top-level call resets the recorded event buffer. `Base::mint`
    // also publishes its own SEP-41 event, so check our custom `MintEvent`
    // is present rather than asserting on the full (implementation-coupled)
    // event list.
    let expected = MintEvent {
        admin: owner,
        to: user.clone(),
        amount: 750,
    };
    assert!(env
        .events()
        .all()
        .events()
        .contains(&expected.to_xdr(&env, &contract_id)));

    assert_eq!(client.balance(&user), 750);
    assert_eq!(client.total_supply(), 750);
}
