use soroban_sdk::{contract, contractimpl, contracterror, symbol_short, vec, Address, Env, IntoVal, String, Val, Vec};

use crate::storage::types::{DataKey, Prompt};

/// Contract errors for prompt-marketplace operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MktError {
    PromptNotFound = 1,
    AlreadyPurchased = 2,
    PriceMustBePositive = 3,
    Unauthorized = 4,
    AlreadyRegistered = 5,
}

/// A Soroban contract that lets admins register prompts for sale,
/// users buy them (burning tokens), and admins re-mint tokens back
/// into circulation.
///
/// Flow:
///   1. Admin registers a prompt with a price in MyToken units
///   2. User buys the prompt → tokens are BURNED (via MyToken::sell)
///   3. Admin re-mints tokens via remint() to keep circulation healthy
///
/// Auth model:
///   - register_prompt / update_price / remint → admin only
///   - buy_prompt → buyer signs (pays tokens, gets access)
#[contract]
pub struct PromptMarketplace;

#[contractimpl]
impl PromptMarketplace {
    // ─── Lifecycle ───────────────────────────────────────────

    /// One-time constructor.
    /// `admin` is the address that can register prompts and re-mint tokens.
    /// `token` is the MyToken contract ID used for payment/burning.
    pub fn __constructor(e: &Env, admin: Address, token: Address) {
        e.storage().instance().set(&DataKey::Admin, &admin);
        e.storage().instance().set(&DataKey::Token, &token);
    }

    // ─── Admin: prompt management ────────────────────────────

    /// Register a new prompt for sale.
    /// `prompt_id` – unique identifier for the prompt.
    /// `price`     – how many tokens the buyer must burn to access it.
    /// `owner`     – the user who created/provided the prompt content.
    pub fn register_prompt(e: &Env, prompt_id: String, price: i128, owner: Address) {
        Self::enforce_admin(e);
        assert!(price > 0, "price must be positive");

        let key = DataKey::Prompt(prompt_id.clone());
        assert!(
            e.storage().instance().get::<_, Prompt>(&key).is_none(),
            "prompt already registered"
        );

        let prompt = Prompt { price, owner };
        e.storage().instance().set(&key, &prompt);
    }

    /// Change the price of an existing prompt.
    pub fn update_price(e: &Env, prompt_id: String, new_price: i128) {
        Self::enforce_admin(e);
        assert!(new_price > 0, "price must be positive");

        let key = DataKey::Prompt(prompt_id.clone());
        let mut prompt: Prompt = e
            .storage()
            .instance()
            .get(&key)
            .expect("prompt not found");
        prompt.price = new_price;
        e.storage().instance().set(&key, &prompt);
    }

    /// Remove a prompt from the marketplace entirely.
    pub fn remove_prompt(e: &Env, prompt_id: String) {
        Self::enforce_admin(e);
        let key = DataKey::Prompt(prompt_id);
        e.storage().instance().remove(&key);
    }

    // ─── User: purchase flow ─────────────────────────────────

    /// Buy a prompt. The buyer authenticates, their tokens are burned
    /// via `MyToken::sell`, and the buyer gains access to the prompt.
    ///
    /// This is idempotent in storage — buying again is a no-op
    /// (tokens are burned each time though, so the caller pays again).
    pub fn buy_prompt(e: &Env, buyer: Address, prompt_id: String) {
        buyer.require_auth();

        let key = DataKey::Prompt(prompt_id.clone());
        let prompt: Prompt = e
            .storage()
            .instance()
            .get(&key)
            .expect("prompt not found");

        // Burn tokens from the buyer via the MyToken sell function.
        // because `buyer` signs the top-level invocation, Soroban
        // forwards the auth to `MyToken::sell` → `seller.require_auth()`.
        let token = Self::get_token(e);
        let sell_sym = symbol_short!("sell");
        let sell_args: Vec<Val> = vec![&e, buyer.clone().into_val(e), prompt.price.into_val(e)];
        let _: () = e.invoke_contract(&token, &sell_sym, sell_args);

        // Mark the purchase so has_access returns true.
        let purchase_key = DataKey::Purchase(buyer, prompt_id);
        e.storage().instance().set(&purchase_key, &true);
    }

    // ─── Admin: re-mint tokens ───────────────────────────────

    /// Re-mint tokens back into circulation.
    /// The admin can put burned tokens back on the market.
    /// Only works if the admin is also the owner of the MyToken contract
    /// (or if the token's `only_owner` check passes via forwarded auth).
    pub fn remint(e: &Env, to: Address, amount: i128) {
        Self::enforce_admin(e);
        assert!(amount > 0, "amount must be positive");

        let token = Self::get_token(e);
        let mint_sym = symbol_short!("mint");
        let mint_args: Vec<Val> = vec![&e, to.into_val(e), amount.into_val(e)];
        let _: () = e.invoke_contract(&token, &mint_sym, mint_args);
    }

    // ─── Queries ─────────────────────────────────────────────

    /// Check whether a user has purchased a specific prompt.
    pub fn has_access(e: &Env, user: Address, prompt_id: String) -> bool {
        let key = DataKey::Purchase(user, prompt_id);
        e.storage().instance().get(&key).unwrap_or(false)
    }

    /// Read the price of a prompt.
    pub fn get_price(e: &Env, prompt_id: String) -> i128 {
        let key = DataKey::Prompt(prompt_id);
        let prompt: Prompt = e
            .storage()
            .instance()
            .get(&key)
            .expect("prompt not found");
        prompt.price
    }

    /// Read the prompt owner (creator).
    pub fn get_owner(e: &Env, prompt_id: String) -> Address {
        let key = DataKey::Prompt(prompt_id);
        let prompt: Prompt = e
            .storage()
            .instance()
            .get(&key)
            .expect("prompt not found");
        prompt.owner
    }

    /// Get the token contract address used by this marketplace.
    pub fn get_token(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized")
    }

    /// Get the admin address.
    pub fn get_admin(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized")
    }

    // ─── Internal ────────────────────────────────────────────

    fn enforce_admin(e: &Env) {
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();
    }
}
