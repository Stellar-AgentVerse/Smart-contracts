use soroban_sdk::{contractevent, Address, MuxedAddress};

#[contractevent(data_format = "single-value", topics = ["mint"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MintEvent {
    #[topic]
    pub admin: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent(data_format = "single-value", topics = ["transfer"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: MuxedAddress,
    pub amount: i128,
}

#[contractevent(data_format = "single-value", topics = ["burn"])]
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BurnEvent {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contractevent(data_format = "single-value", topics = ["sell"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SellEvent {
    #[topic]
    pub seller: Address,
    pub amount: i128,
}
