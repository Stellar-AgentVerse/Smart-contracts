#![cfg(all(test, not(target_family = "wasm")))]

use crate::contract::{
    PrivatePromptPurchased, PrivatePromptRegistered, PromptMarketplace, PromptMarketplaceClient,
    PromptPurchased, TokensReminted,
};
use my_token::MyToken;
use soroban_sdk::{
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    Address, Bytes, BytesN, Env, Event, IntoVal, String,
};
use stellar_tokens::fungible::Base as TokenBase;

struct Ctx {
    env: Env,
    token_id: Address,
    mkt: PromptMarketplaceClient<'static>,
    mkt_id: Address,
    admin: Address,
    creator: Address,
    buyer: Address,
    uri: String,
}

fn setup_env() -> Ctx {
    let env = Env::default();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let buyer = Address::generate(&env);

    // Deploy MyToken
    let token_id = env.register(
        MyToken,
        (
            admin.clone(),
            String::from_str(&env, "PromptToken"),
            String::from_str(&env, "PRMPT"),
            7u32,
        ),
    );

    // Deploy Marketplace
    let mkt_id = env.register(PromptMarketplace, (admin.clone(), token_id.clone()));
    let token = my_token::MyTokenClient::new(&env, &token_id);
    token
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &token_id,
                fn_name: "set_marketplace",
                args: (&mkt_id,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_marketplace(&mkt_id);

    let mkt = PromptMarketplaceClient::new(&env, &mkt_id);
    let uri = String::from_str(&env, "ipfs://QmTest");

    Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
    }
}

// ─── Marketplace storage logic (via client + mock_auths) ────

#[test]
fn test_register_and_query_prompt() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "alpha");

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 500i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &500, &creator, &uri);

    assert_eq!(mkt.get_price(&pid), 500);
    assert_eq!(mkt.get_owner(&pid), creator);
    assert_eq!(mkt.get_content_uri(&pid), uri);
}

#[test]
#[should_panic(expected = "prompt already registered")]
fn test_duplicate_registration_panics() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "dup");

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 100i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &100, &creator, &uri);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 200i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &200, &creator, &uri);
}

#[test]
fn test_update_price() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "dynamic");

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 100i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &100, &creator, &uri);

    assert_eq!(mkt.get_price(&pid), 100);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "update_price",
            args: (&pid, 250i128).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .update_price(&pid, &250);

    assert_eq!(mkt.get_price(&pid), 250);
}

#[test]
fn test_remove_prompt() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "temp");

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 50i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &50, &creator, &uri);

    assert!(mkt.get_price(&pid) > 0);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "remove_prompt",
            args: (&pid,).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .remove_prompt(&pid);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "remove_prompt",
            args: (&pid,).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .remove_prompt(&pid);
}

#[test]
fn test_multiple_prompts_independent() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let pid_a = String::from_str(&env, "a");
    let pid_b = String::from_str(&env, "b");

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid_a, 100i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid_a, &100, &creator, &uri);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid_b, 200i128, &buyer, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid_b, &200, &buyer, &uri);

    assert_eq!(mkt.get_price(&pid_a), 100);
    assert_eq!(mkt.get_price(&pid_b), 200);
    assert_eq!(mkt.get_owner(&pid_a), creator);
    assert_eq!(mkt.get_owner(&pid_b), buyer);
}

#[test]
#[should_panic(expected = "prompt not found")]
fn test_get_price_unregistered_panics() {
    let Ctx { env, mkt, .. } = setup_env();
    mkt.get_price(&String::from_str(&env, "ghost"));
}

// ─── Adversarial: auth boundaries ─────────────────────────

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_register() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        buyer,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "hack");

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 100i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &100, &creator, &uri);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_update_price() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "guarded");

    // Admin registers
    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 100i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &100, &creator, &uri);

    // Non-admin tries to update
    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "update_price",
            args: (&pid, 999i128).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .update_price(&pid, &999);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_remove() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "protected");

    // Admin registers
    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 100i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &100, &creator, &uri);

    // Non-admin tries to remove
    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "remove_prompt",
            args: (&pid,).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .remove_prompt(&pid);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_remint() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        buyer,
        ..
    } = setup_env();

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "remint",
            args: (&buyer, 100i128).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .remint(&buyer, &100);
}

// ─── Adversarial: edge values and state transitions ────

#[test]
#[should_panic(expected = "price must be positive")]
fn test_register_zero_price_panics() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "free");

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 0i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &0, &creator, &uri);
}

#[test]
#[should_panic(expected = "price must be positive")]
fn test_update_price_zero_panics() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "discount");

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 100i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &100, &creator, &uri);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "update_price",
            args: (&pid, 0i128).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .update_price(&pid, &0);
}

#[test]
fn test_register_max_price() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "max");
    let max_price = i128::MAX;

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, max_price, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &max_price, &creator, &uri);

    assert_eq!(mkt.get_price(&pid), max_price);
}

#[test]
#[should_panic(expected = "prompt not found")]
fn test_update_unregistered_prompt_panics() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "phantom");

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "update_price",
            args: (&pid, 500i128).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .update_price(&pid, &500);
}

#[test]
fn test_register_after_remove() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "reborn");

    // Register
    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 100i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &100, &creator, &uri);

    // Remove
    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "remove_prompt",
            args: (&pid,).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .remove_prompt(&pid);

    // Re-register
    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 200i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &200, &creator, &uri);

    assert_eq!(mkt.get_price(&pid), 200);
}

#[test]
fn test_has_access_unregistered() {
    let Ctx {
        env, mkt, buyer, ..
    } = setup_env();
    let pid = String::from_str(&env, "unknown");
    assert!(!mkt.has_access(&buyer, &pid));
}

// ─── Token storage tests (via as_contract, no auth needed) ─

#[test]
fn test_token_mint_and_balance() {
    let Ctx {
        env,
        token_id,
        buyer,
        ..
    } = setup_env();

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 1000);
    });

    let bal: i128 = env.as_contract(&token_id, || TokenBase::balance(&env, &buyer));
    assert_eq!(bal, 1000);
}

// ─── Cross-contract auth note ─────────────────────────────
//
// Soroban v25's mock auth cannot satisfy a SECOND require_auth() for the
// SAME address within one call tree: if a root invocation calls
// `buyer.require_auth()` and then a sub-invocation (e.g. marketplace →
// token via `invoke_contract`) also calls `require_auth()` for `buyer`,
// the host rejects it with `Error(Auth, ExistingValue)` — mock auth has
// no way to represent "this address already authorized higher up."
//
// The root call authenticates the buyer/admin. The marketplace then uses
// `authorize_as_current_contract` to authorize the exact nested token call.
// MyToken accepts that call only from its one-time configured marketplace.
//
// `scripts/validate-testnet.sh` additionally exercises the same flow with
// real Testnet signatures and fresh contract IDs.

#[test]
fn test_buy_prompt_cross_contract() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "cross-contract");

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 1000);
    });

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 500i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &500, &creator, &uri);

    // Real cross-contract call: marketplace.buy_prompt() → invoke_contract
    // → token.marketplace_burn().
    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_prompt",
            args: (&buyer, &pid).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_prompt(&buyer, &pid);

    let bal: i128 = env.as_contract(&token_id, || TokenBase::balance(&env, &buyer));
    assert_eq!(
        bal, 500,
        "buyer's tokens must be burned via marketplace_burn"
    );
}

#[test]
#[should_panic(expected = "prompt not found")]
fn test_buy_prompt_unregistered_panics() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        buyer,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "missing-prompt");

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_prompt",
            args: (&buyer, &pid).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_prompt(&buyer, &pid);
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn test_buy_prompt_insufficient_balance_panics() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "expensive-prompt");

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 100);
    });

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 500i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &500, &creator, &uri);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_prompt",
            args: (&buyer, &pid).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_prompt(&buyer, &pid);
}

#[test]
#[should_panic(expected = "already purchased")]
fn test_buy_prompt_same_prompt_twice_panics() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "one-time-prompt");

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 1000);
    });

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 250i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &250, &creator, &uri);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_prompt",
            args: (&buyer, &pid).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_prompt(&buyer, &pid);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_prompt",
            args: (&buyer, &pid).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_prompt(&buyer, &pid);
}

#[test]
fn test_has_access_after_buy() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "access-flow");

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 1000);
    });

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 200i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &200, &creator, &uri);

    assert!(!mkt.has_access(&buyer, &pid), "no access before purchase");

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_prompt",
            args: (&buyer, &pid).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_prompt(&buyer, &pid);

    assert!(
        mkt.has_access(&buyer, &pid),
        "access granted after purchase"
    );
}

#[test]
fn test_buy_prompt_emits_event() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let pid = String::from_str(&env, "event-flow");

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 1000);
    });

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&pid, 300i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&pid, &300, &creator, &uri);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_prompt",
            args: (&buyer, &pid).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_prompt(&buyer, &pid);

    // Read events immediately — the next contract invocation resets the
    // recorded buffer. `marketplace_burn` also publishes its own SellEvent
    // on the token contract, so check for PromptPurchased specifically
    // rather than asserting on the full event list.
    let expected = PromptPurchased {
        buyer: buyer.clone(),
        prompt_id: pid.clone(),
        price: 300,
    };
    assert!(env
        .events()
        .all()
        .events()
        .contains(&expected.to_xdr(&env, &mkt_id)));
}

#[test]
fn test_remint_cross_contract() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        buyer,
        ..
    } = setup_env();

    // Real cross-contract call: marketplace.remint() → invoke_contract →
    // token.marketplace_mint().
    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "remint",
            args: (&buyer, 2000i128).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .remint(&buyer, &2000);

    let bal: i128 = env.as_contract(&token_id, || TokenBase::balance(&env, &buyer));
    assert_eq!(bal, 2000, "tokens must be minted via marketplace_mint");
}

#[test]
fn test_remint_emits_event() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        buyer,
        ..
    } = setup_env();

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "remint",
            args: (&buyer, 1000i128).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .remint(&buyer, &1000);

    let expected = TokensReminted {
        admin: admin.clone(),
        to: buyer.clone(),
        amount: 1000,
    };
    assert!(env
        .events()
        .all()
        .events()
        .contains(&expected.to_xdr(&env, &mkt_id)));
}

// ─── Private Prompts (Opaque Access) ─────────────────────

// Test-only deterministic fixture for the documented commitment format:
// SHA256("PMPT_V1" || prompt_id || high-entropy salt). Production callers
// must generate a fresh cryptographically-random salt for every prompt.
fn private_commitment(env: &Env) -> BytesN<32> {
    env.crypto()
        .sha256(&Bytes::from_slice(
        env,
        b"PMPT_V1\0private-test-prompt\0\x9d\x8b\x3f\xa1\x72\x19\xc4\x5e\x0a\xb7\x61\xde\x34\x90\x28\xf6\x4c\x83\x15\xaa\xe9\x06\xd2\x7b\x11\x68\xbf\x42\x95\x3c\xf0\x7d",
    ))
        .into()
}

#[test]
fn test_register_and_buy_private_prompt() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        ..
    } = setup_env();
    let prompt_hash = private_commitment(&env);

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 1000);
    });

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_private_prompt",
            args: (&prompt_hash, 500i128, &creator).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_private_prompt(&prompt_hash, &500, &creator);

    assert!(!mkt.has_private_access(&buyer, &prompt_hash));

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_private_prompt",
            args: (&buyer, &prompt_hash).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_private_prompt(&buyer, &prompt_hash);

    // Read events immediately before the next contract invocation resets the buffer
    let expected_event = PrivatePromptPurchased {
        buyer: buyer.clone(),
        prompt_hash: prompt_hash.clone(),
        price: 500,
    };
    assert!(env
        .events()
        .all()
        .events()
        .contains(&expected_event.to_xdr(&env, &mkt_id)));

    assert!(mkt.has_private_access(&buyer, &prompt_hash));
    let bal: i128 = env.as_contract(&token_id, || TokenBase::balance(&env, &buyer));
    assert_eq!(bal, 500);
}

#[test]
#[should_panic(expected = "already purchased")]
fn test_buy_private_prompt_replay_panics() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        ..
    } = setup_env();
    let prompt_hash = private_commitment(&env);

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 1000);
    });

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_private_prompt",
            args: (&prompt_hash, 500i128, &creator).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_private_prompt(&prompt_hash, &500, &creator);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_private_prompt",
            args: (&buyer, &prompt_hash).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_private_prompt(&buyer, &prompt_hash);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_private_prompt",
            args: (&buyer, &prompt_hash).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_private_prompt(&buyer, &prompt_hash);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_unauthorized_private_prompt_registration() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        buyer,
        creator,
        ..
    } = setup_env();
    let prompt_hash = private_commitment(&env);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_private_prompt",
            args: (&prompt_hash, 500i128, &creator).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_private_prompt(&prompt_hash, &500, &creator);
}

#[test]
#[should_panic(expected = "prompt not found")]
fn test_buy_unregistered_private_prompt_panics() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        buyer,
        ..
    } = setup_env();
    let prompt_hash = private_commitment(&env);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_private_prompt",
            args: (&buyer, &prompt_hash).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_private_prompt(&buyer, &prompt_hash);
}

#[test]
fn test_register_private_prompt_emits_event() {
    let Ctx {
        env,
        mkt,
        mkt_id,
        admin,
        creator,
        ..
    } = setup_env();
    let prompt_hash = private_commitment(&env);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_private_prompt",
            args: (&prompt_hash, 500i128, &creator).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_private_prompt(&prompt_hash, &500, &creator);

    let expected = PrivatePromptRegistered {
        admin: admin.clone(),
        owner: creator,
        prompt_hash: prompt_hash.clone(),
        price: 500,
    };
    assert!(env
        .events()
        .all()
        .events()
        .contains(&expected.to_xdr(&env, &mkt_id)));
}

#[test]
fn test_cross_user_cannot_spend_another_buyers_authorization() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        ..
    } = setup_env();
    let buyer2 = Address::generate(&env);

    let prompt_hash = BytesN::from_array(&env, &[6u8; 32]);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_private_prompt",
            args: (&prompt_hash, 500i128, &creator).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_private_prompt(&prompt_hash, &500, &creator);

    env.as_contract(&token_id, || TokenBase::mint(&env, &buyer, 1000));

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_private_prompt",
            args: (&buyer, &prompt_hash).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_private_prompt(&buyer, &prompt_hash);

    assert!(mkt.has_private_access(&buyer, &prompt_hash));
    assert!(!mkt.has_private_access(&buyer2, &prompt_hash));

    let result = mkt
        .mock_auths(&[MockAuth {
            address: &buyer2,
            invoke: &MockAuthInvoke {
                contract: &mkt_id,
                fn_name: "buy_private_prompt",
                args: (&buyer, &prompt_hash).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_buy_private_prompt(&buyer, &prompt_hash);

    assert!(result.is_err());
    assert!(!mkt.has_private_access(&buyer2, &prompt_hash));
}

#[test]
fn test_atomicity_fail_burn() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        ..
    } = setup_env();
    let prompt_hash = private_commitment(&env);

    env.as_contract(&token_id, || TokenBase::mint(&env, &buyer, 100));

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_private_prompt",
            args: (&prompt_hash, 500i128, &creator).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_private_prompt(&prompt_hash, &500, &creator);

    let result = mkt
        .mock_auths(&[MockAuth {
            address: &buyer,
            invoke: &MockAuthInvoke {
                contract: &mkt_id,
                fn_name: "buy_private_prompt",
                args: (&buyer, &prompt_hash).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_buy_private_prompt(&buyer, &prompt_hash);

    assert!(result.is_err());
    assert!(!mkt.has_private_access(&buyer, &prompt_hash));
    let balance: i128 = env.as_contract(&token_id, || TokenBase::balance(&env, &buyer));
    assert_eq!(balance, 100);
}

#[test]
fn test_migration_compatibility() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        uri,
        ..
    } = setup_env();
    let prompt_id = String::from_str(&env, "legacy_prompt");
    let prompt_hash = private_commitment(&env);

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 2000);
    });

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_prompt",
            args: (&prompt_id, 500i128, &creator, &uri).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_prompt(&prompt_id, &500, &creator, &uri);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_private_prompt",
            args: (&prompt_hash, 500i128, &creator).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_private_prompt(&prompt_hash, &500, &creator);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_prompt",
            args: (&buyer, &prompt_id).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_prompt(&buyer, &prompt_id);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_private_prompt",
            args: (&buyer, &prompt_hash).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_private_prompt(&buyer, &prompt_hash);

    assert!(mkt.has_access(&buyer, &prompt_id));
    assert!(mkt.has_private_access(&buyer, &prompt_hash));
}

#[test]
fn test_admin_can_revoke_private_access() {
    let Ctx {
        env,
        token_id,
        mkt,
        mkt_id,
        admin,
        creator,
        buyer,
        ..
    } = setup_env();
    let prompt_hash = private_commitment(&env);

    env.as_contract(&token_id, || TokenBase::mint(&env, &buyer, 1000));
    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_private_prompt",
            args: (&prompt_hash, 500i128, &creator).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_private_prompt(&prompt_hash, &500, &creator);
    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "buy_private_prompt",
            args: (&buyer, &prompt_hash).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .buy_private_prompt(&buyer, &prompt_hash);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "revoke_private_access",
            args: (&buyer, &prompt_hash).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .revoke_private_access(&buyer, &prompt_hash);

    assert!(!mkt.has_private_access(&buyer, &prompt_hash));
}
