#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    PrivatePurchase(Address, BytesN<32>),
}

#[contract]
pub struct PrivateAccessPrototype;

#[contractimpl]
impl PrivateAccessPrototype {
    /// Grants access strictly bound to the opaque hash.
    /// To ensure high entropy and domain separation, the `prompt_hash` must be derived off-chain as:
    /// `SHA256("PMPT_V1" || prompt_id || user_salt)`
    pub fn buy_access(e: Env, buyer: Address, prompt_hash: BytesN<32>) {
        buyer.require_auth();

        let access_key = DataKey::PrivatePurchase(buyer.clone(), prompt_hash.clone());
        if e.storage().persistent().has(&access_key) {
            panic!("already purchased");
        }

        // Simulate atomic token deduction here
        // TokenClient::new(&e, &token_address).marketplace_burn(...);

        // Store the exact opaque hash. No plaintext prompt_id is recorded.
        e.storage().persistent().set(&access_key, &true);
    }

    pub fn has_access(e: Env, buyer: Address, prompt_hash: BytesN<32>) -> bool {
        let access_key = DataKey::PrivatePurchase(buyer, prompt_hash);
        e.storage().persistent().has(&access_key)
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests;
