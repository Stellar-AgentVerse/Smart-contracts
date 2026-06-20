use soroban_sdk::{contracttype, Address, BytesN, String};

/// A registered prompt with its price (in MyToken units)
/// and the creator/owner who provided it.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub price: i128,
    pub owner: Address,
    pub content_uri: String,
}

/// Opaque policy record for the private access flow.
/// Only the commitment hash and token price are stored — content_uri and
/// prompt identity never touch the ledger in this path.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyData {
    pub commitment: BytesN<32>,
    pub price: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataKey {
    // ── Public marketplace ───────────────────────────────────
    Admin,
    Token,
    Prompt(String),
    Purchase(Address, String),

    // ── Private access flow (commitment-based) ───────────────
    PolicyCounter,          // u64 — next policy_id
    Policy(u64),            // PolicyData — opaque commitment + price
    AccessRoot,             // BytesN<32> — Merkle accumulator root
    Nullifier(BytesN<32>), // bool — consumed nullifiers
}
