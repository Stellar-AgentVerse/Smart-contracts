#![cfg(test)]
extern crate std;

use crate::events::MintEvent;
use crate::{MyToken, MyTokenClient};
use soroban_sdk::{
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    Address, Env, Event, IntoVal, String,
};
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

#[test]
#[should_panic(expected = "marketplace not configured")]
fn test_marketplace_mint_requires_configuration() {
    let (env, contract_id, user) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);

    client.marketplace_mint(&user, &750);
}

#[test]
#[should_panic]
fn test_direct_marketplace_mint_is_rejected() {
    let (env, contract_id, user) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);
    let owner = env
        .as_contract(&contract_id, || ownable::get_owner(&env))
        .expect("owner must be set");
    let marketplace = Address::generate(&env);

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_marketplace",
                args: (&marketplace,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_marketplace(&marketplace);

    // A configured marketplace contract must authorize this call. A direct
    // external caller cannot authenticate as that contract address.
    client.marketplace_mint(&user, &750);
}

#[test]
#[should_panic(expected = "marketplace already configured")]
fn test_marketplace_cannot_be_reconfigured() {
    let (env, contract_id, ..) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);
    let owner = env
        .as_contract(&contract_id, || ownable::get_owner(&env))
        .expect("owner must be set");
    let first_marketplace = Address::generate(&env);
    let second_marketplace = Address::generate(&env);

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_marketplace",
                args: (&first_marketplace,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_marketplace(&first_marketplace);

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_marketplace",
                args: (&second_marketplace,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_marketplace(&second_marketplace);
}

#[test]
fn test_marketplace_mint_updates_balance() {
    let (env, contract_id, user) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);
    let marketplace = Address::generate(&env);
    let owner = env
        .as_contract(&contract_id, || ownable::get_owner(&env))
        .expect("owner must be set");

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_marketplace",
                args: (&marketplace,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_marketplace(&marketplace);

    client
        .mock_auths(&[MockAuth {
            address: &marketplace,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "marketplace_mint",
                args: (&user, 750i128).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .marketplace_mint(&user, &750);

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

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_marketplace_mint_rejects_non_positive_amount() {
    let (env, contract_id, user) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);
    let marketplace = Address::generate(&env);
    let owner = env
        .as_contract(&contract_id, || ownable::get_owner(&env))
        .expect("owner must be set");

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_marketplace",
                args: (&marketplace,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_marketplace(&marketplace);

    client
        .mock_auths(&[MockAuth {
            address: &marketplace,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "marketplace_mint",
                args: (&user, 0i128).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .marketplace_mint(&user, &0);
}

#[test]
#[should_panic]
fn test_direct_marketplace_burn_is_rejected() {
    let (env, contract_id, user) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);
    let owner = env
        .as_contract(&contract_id, || ownable::get_owner(&env))
        .expect("owner must be set");
    let marketplace = Address::generate(&env);

    env.as_contract(&contract_id, || {
        TokenBase::mint(&env, &user, 1000);
    });

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_marketplace",
                args: (&marketplace,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_marketplace(&marketplace);

    // A direct external caller cannot authenticate as the configured
    // marketplace contract.
    client.marketplace_burn(&user, &400);
}

#[test]
#[should_panic]
fn test_marketplace_mint_rejected_when_paused() {
    let (env, contract_id, user) = setup_env();
    let client = MyTokenClient::new(&env, &contract_id);
    let owner = env
        .as_contract(&contract_id, || ownable::get_owner(&env))
        .expect("owner must be set");
    let marketplace = Address::generate(&env);

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_marketplace",
                args: (&marketplace,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_marketplace(&marketplace);

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "pause",
                args: (&owner,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .pause(&owner);

    client
        .mock_auths(&[MockAuth {
            address: &marketplace,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "marketplace_mint",
                args: (&user, 750i128).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .marketplace_mint(&user, &750);
}
