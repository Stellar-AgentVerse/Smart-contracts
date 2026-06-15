use soroban_sdk::{Address, Env, MuxedAddress, String};
use stellar_tokens::fungible::Base;
use stellar_access::ownable;

use crate::events::{BurnEvent, MintEvent, SellEvent, TransferEvent};

pub struct TokenManager;

impl TokenManager {
    pub fn initialize(e: &Env, owner: Address, name: String, symbol: String, decimals: u32) {
        Base::set_metadata(e, decimals, name, symbol);
        ownable::set_owner(e, &owner);
    }

    pub fn mint(e: &Env, to: &Address, amount: i128) {
        ownable::enforce_owner_auth(e);
        Base::mint(e, to, amount);
        MintEvent {
            admin: ownable::get_owner(e).unwrap(),
            to: to.clone(),
            amount,
        }
        .publish(e);
    }

    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        Base::transfer(e, from, to, amount);
        TransferEvent {
            from: from.clone(),
            to: to.clone(),
            amount,
        }
        .publish(e);
    }

    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Base::transfer_from(e, spender, from, to, amount);
    }

    pub fn burn(e: &Env, from: &Address, amount: i128) {
        Base::burn(e, from, amount);
        BurnEvent {
            from: from.clone(),
            amount,
        }
        .publish(e);
    }

    pub fn sell(e: &Env, seller: &Address, amount: i128) {
        seller.require_auth();
        Base::burn(e, seller, amount);
        SellEvent {
            seller: seller.clone(),
            amount,
        }
        .publish(e);
    }
}
