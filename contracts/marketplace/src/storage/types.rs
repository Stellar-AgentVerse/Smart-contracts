use soroban_sdk::{contracttype, Address, String};

/// A registered prompt with its price (in MyToken units)
/// and the creator/owner who provided it.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
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
}
