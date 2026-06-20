#![cfg(all(test, not(target_family = "wasm")))]

use crate::contract::{PromptMarketplace, PromptMarketplaceClient};
use my_token::MyToken;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, BytesN, Env, IntoVal, String,
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
    let Ctx { env, mkt, mkt_id, admin, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, buyer, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, buyer, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, buyer, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, buyer, uri, .. } = setup_env();
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

// ─── Adversarial: edge values and state transitions ────

#[test]
#[should_panic(expected = "price must be positive")]
fn test_register_zero_price_panics() {
    let Ctx { env, mkt, mkt_id, admin, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, .. } = setup_env();
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
    let Ctx { env, mkt, mkt_id, admin, creator, uri, .. } = setup_env();
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
    let Ctx { env, mkt, buyer, .. } = setup_env();
    let pid = String::from_str(&env, "unknown");
    assert!(!mkt.has_access(&buyer, &pid));
}

// ─── Token storage tests (via as_contract, no auth needed) ─

#[test]
fn test_token_mint_and_balance() {
    let Ctx { env, token_id, buyer, .. } = setup_env();

    env.as_contract(&token_id, || {
        TokenBase::mint(&env, &buyer, 1000);
    });

    let bal: i128 = env.as_contract(&token_id, || {
        TokenBase::balance(&env, &buyer)
    });
    assert_eq!(bal, 1000);
}

// ─── Cross-contract auth note ─────────────────────────────
// buy_prompt and remint involve cross-contract require_auth calls
// (marketplace → token.sell / token.mint). Soroban v25 mock auth
// cannot satisfy nested require_auth in sub-invocations — the
// __check_auth returns Void but the host rejects with InvalidAction.
// These flows are verified via deploy-and-invoke on testnet CLI only.

// ─── Privacy: AccessRegistry ──────────────────────────────

fn register_policy_helper(
    env: &Env,
    mkt: &PromptMarketplaceClient,
    mkt_id: &Address,
    admin: &Address,
    commitment: &BytesN<32>,
    price: i128,
) -> u64 {
    mkt.mock_auths(&[MockAuth {
        address: admin,
        invoke: &MockAuthInvoke {
            contract: mkt_id,
            fn_name: "register_policy",
            args: (commitment, price).into_val(env),
            sub_invokes: &[],
        },
    }])
    .register_policy(commitment, &price)
}

#[test]
fn test_register_policy_returns_incremental_ids() {
    let Ctx { env, mkt, mkt_id, admin, .. } = setup_env();
    let c0 = BytesN::from_array(&env, &[1u8; 32]);
    let c1 = BytesN::from_array(&env, &[2u8; 32]);

    let id0 = register_policy_helper(&env, &mkt, &mkt_id, &admin, &c0, 100);
    let id1 = register_policy_helper(&env, &mkt, &mkt_id, &admin, &c1, 200);

    assert_eq!(id0, 0u64);
    assert_eq!(id1, 1u64);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_register_policy() {
    let Ctx { env, mkt, mkt_id, buyer, .. } = setup_env();
    let commitment = BytesN::from_array(&env, &[9u8; 32]);

    mkt.mock_auths(&[MockAuth {
        address: &buyer,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "register_policy",
            args: (&commitment, 100i128).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .register_policy(&commitment, &100);
}

#[test]
#[should_panic(expected = "price must be positive")]
fn test_register_policy_zero_price_panics() {
    let Ctx { env, mkt, mkt_id, admin, .. } = setup_env();
    let commitment = BytesN::from_array(&env, &[3u8; 32]);

    register_policy_helper(&env, &mkt, &mkt_id, &admin, &commitment, 0);
}

#[test]
fn test_issue_access_updates_root_and_returns_leaf() {
    let Ctx { env, mkt, mkt_id, admin, .. } = setup_env();
    let commitment = BytesN::from_array(&env, &[0xAAu8; 32]);
    let session = BytesN::from_array(&env, &[0xBBu8; 32]);

    let policy_id = register_policy_helper(&env, &mkt, &mkt_id, &admin, &commitment, 50);

    let root_before = mkt.root();

    let leaf = mkt
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &mkt_id,
                fn_name: "issue_access",
                args: (&policy_id, &session).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .issue_access(&policy_id, &session);

    let root_after = mkt.root();

    // Root must change after issuing access
    assert_ne!(root_before, root_after);
    // Leaf must be deterministic for same inputs
    let leaf2 = mkt
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &mkt_id,
                fn_name: "issue_access",
                args: (&policy_id, &session).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .issue_access(&policy_id, &session);
    // Different calls produce different roots (each updates accumulator)
    // but identical (policy_id, session) produce the same leaf value
    assert_eq!(leaf, leaf2);
}

#[test]
#[should_panic(expected = "policy not found")]
fn test_issue_access_unknown_policy_panics() {
    let Ctx { env, mkt, mkt_id, admin, .. } = setup_env();
    let session = BytesN::from_array(&env, &[0xCCu8; 32]);

    mkt.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &mkt_id,
            fn_name: "issue_access",
            args: (&99u64, &session).into_val(&env),
            sub_invokes: &[],
        },
    }])
    .issue_access(&99, &session);
}

#[test]
fn test_root_stable_before_any_access() {
    let Ctx { mkt, .. } = setup_env();
    let r1 = mkt.root();
    let r2 = mkt.root();
    assert_eq!(r1, r2);
}

// ─── Privacy: AccessNullifiers ────────────────────────────

#[test]
fn test_used_returns_false_for_fresh_nullifier() {
    let Ctx { env, mkt, .. } = setup_env();
    let nullifier = BytesN::from_array(&env, &[0u8; 32]);
    assert!(!mkt.used(&nullifier));
}

#[test]
fn test_consume_marks_nullifier_used() {
    let Ctx { env, mkt, .. } = setup_env();
    let nullifier = BytesN::from_array(&env, &[42u8; 32]);

    assert!(!mkt.used(&nullifier));
    mkt.consume(&nullifier);
    assert!(mkt.used(&nullifier));
}

#[test]
fn test_distinct_nullifiers_are_independent() {
    let Ctx { env, mkt, .. } = setup_env();
    let n1 = BytesN::from_array(&env, &[1u8; 32]);
    let n2 = BytesN::from_array(&env, &[2u8; 32]);

    mkt.consume(&n1);

    assert!(mkt.used(&n1));
    assert!(!mkt.used(&n2));
}

#[test]
#[should_panic(expected = "nullifier already used")]
fn test_double_consume_panics() {
    let Ctx { env, mkt, .. } = setup_env();
    let nullifier = BytesN::from_array(&env, &[7u8; 32]);

    mkt.consume(&nullifier);
    mkt.consume(&nullifier); // must panic
}

// ─── Privacy: storage does not expose wallet+prompt ───────

#[test]
fn test_issue_access_storage_is_opaque() {
    // Verify that issuing access for two different (policy_id, session)
    // pairs produces distinct leaves — confirming session_commitment binds
    // the leaf, not just the policy_id.
    let Ctx { env, mkt, mkt_id, admin, .. } = setup_env();
    let commitment = BytesN::from_array(&env, &[0xDDu8; 32]);

    let pid = register_policy_helper(&env, &mkt, &mkt_id, &admin, &commitment, 10);

    let s1 = BytesN::from_array(&env, &[0x11u8; 32]);
    let s2 = BytesN::from_array(&env, &[0x22u8; 32]);

    let leaf1 = mkt
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &mkt_id,
                fn_name: "issue_access",
                args: (&pid, &s1).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .issue_access(&pid, &s1);

    let leaf2 = mkt
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &mkt_id,
                fn_name: "issue_access",
                args: (&pid, &s2).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .issue_access(&pid, &s2);

    // Different session commitments → different leaves (unlinkable)
    assert_ne!(leaf1, leaf2);
}
