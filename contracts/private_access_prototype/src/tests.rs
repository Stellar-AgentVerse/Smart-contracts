#![cfg(all(test, not(target_family = "wasm")))]
#![allow(deprecated, unused_imports)]
extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, BytesN, Env,
};

#[test]
fn test_feasibility_and_cost() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PrivateAccessPrototype);
    let client = PrivateAccessPrototypeClient::new(&env, &contract_id);

    let buyer = Address::generate(&env);

    // Simulating a deterministic hash with domain separation and salt
    let mut hash_data = [0u8; 32];
    hash_data[0] = 1; // dummy hash
    let prompt_hash = BytesN::from_array(&env, &hash_data);

    assert!(!client.has_access(&buyer, &prompt_hash));

    // Measure resource cost
    let pre_cpu = env.budget().cpu_instruction_cost();
    let pre_mem = env.budget().memory_bytes_cost();

    client.buy_access(&buyer, &prompt_hash);

    let post_cpu = env.budget().cpu_instruction_cost();
    let post_mem = env.budget().memory_bytes_cost();

    let cost_cpu = post_cpu - pre_cpu;
    let cost_mem = post_mem - pre_mem;

    std::println!("Prototype CPU Cost: {}", cost_cpu);
    std::println!("Prototype Memory Cost: {}", cost_mem);

    // Feasibility assertions
    assert!(client.has_access(&buyer, &prompt_hash));

    // Verify replay protection (cannot buy same hash twice)
    let res = client.try_buy_access(&buyer, &prompt_hash);
    assert!(res.is_err());
}
