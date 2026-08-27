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

/// A registered private prompt that stores only opaque hash commitments.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivatePrompt {
    pub price: i128,
    pub owner: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataKey {
    Admin,
    Token,
    Prompt(String),
    Purchase(Address, String),
    PrivatePrompt(BytesN<32>),
    PrivatePurchase(Address, BytesN<32>),
}
