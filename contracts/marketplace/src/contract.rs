use soroban_sdk::{contract, contractevent, contractimpl, contracterror, vec, Address, Bytes, Env, IntoVal, String, Symbol, Val, Vec};

use crate::privacy::Bytes32;
use crate::storage::types::{DataKey, PolicyData, Prompt};

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

// ─── Events ────────────────────────────────────────────────

/// Emitted when an admin registers a new prompt.
#[contractevent(data_format = "map", topics = ["prompt_registered"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptRegistered {
    #[topic]
    pub admin: Address,
    #[topic]
    pub prompt_id: String,
    pub price: i128,
    pub owner: Address,
    pub content_uri: String,
}

/// Emitted when an admin updates a prompt's price.
#[contractevent(data_format = "single-value", topics = ["prompt_price_updated"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptPriceUpdated {
    #[topic]
    pub admin: Address,
    #[topic]
    pub prompt_id: String,
    pub new_price: i128,
}

/// Emitted when an admin removes a prompt.
#[contractevent(data_format = "single-value", topics = ["prompt_removed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptRemoved {
    #[topic]
    pub admin: Address,
    #[topic]
    pub prompt_id: String,
}

/// Emitted when a user buys a prompt (tokens burned).
#[contractevent(data_format = "single-value", topics = ["prompt_purchased"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptPurchased {
    #[topic]
    pub buyer: Address,
    #[topic]
    pub prompt_id: String,
    pub price: i128,
}

/// Emitted when admin re-mints tokens into circulation.
#[contractevent(data_format = "single-value", topics = ["tokens_reminted"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokensReminted {
    #[topic]
    pub admin: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

// ─── Privacy Events ────────────────────────────────────────

/// Emitted when admin registers a new opaque policy commitment.
/// Only the policy_id and commitment hash are exposed — never the content.
#[contractevent(data_format = "map", topics = ["policy_registered"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyRegistered {
    #[topic]
    pub policy_id: u64,
    pub commitment: Bytes32,
}

/// Emitted when an access leaf is issued into the Merkle accumulator.
/// No buyer address or prompt identity is included.
#[contractevent(data_format = "map", topics = ["access_issued"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessIssued {
    pub leaf: Bytes32,
    pub new_root: Bytes32,
}

/// Emitted when a nullifier is consumed (one-time access proof used).
#[contractevent(data_format = "single-value", topics = ["nullifier_consumed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NullifierConsumed {
    pub nullifier: Bytes32,
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
    /// `prompt_id`    – unique identifier for the prompt.
    /// `price`        – how many tokens the buyer must burn to access it.
    /// `owner`        – the user who created/provided the prompt content.
    /// `content_uri`  – URI/IPFS hash pointing to the prompt content.
    pub fn register_prompt(
        e: &Env,
        prompt_id: String,
        price: i128,
        owner: Address,
        content_uri: String,
    ) {
        Self::enforce_admin(e);
        assert!(price > 0, "price must be positive");

        let key = DataKey::Prompt(prompt_id.clone());
        assert!(
            e.storage().instance().get::<_, Prompt>(&key).is_none(),
            "prompt already registered"
        );

        let prompt = Prompt {
            price,
            owner: owner.clone(),
            content_uri: content_uri.clone(),
        };
        e.storage().instance().set(&key, &prompt);

        PromptRegistered {
            admin: Self::get_admin(e),
            prompt_id,
            price: prompt.price,
            owner,
            content_uri,
        }
        .publish(e);
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

        PromptPriceUpdated {
            admin: Self::get_admin(e),
            prompt_id,
            new_price,
        }
        .publish(e);
    }

    /// Remove a prompt from the marketplace entirely.
    pub fn remove_prompt(e: &Env, prompt_id: String) {
        Self::enforce_admin(e);
        let key = DataKey::Prompt(prompt_id.clone());
        e.storage().instance().remove(&key);

        PromptRemoved {
            admin: Self::get_admin(e),
            prompt_id,
        }
        .publish(e);
    }

    // ─── User: purchase flow ─────────────────────────────────

    /// Buy a prompt. The buyer authenticates, their tokens are burned
    /// via `MyToken::sell_forwarded`, and the buyer gains access to the prompt.
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

        // Burn tokens from the buyer via `sell_forwarded` — this function
        // trusts the root invocation's auth (buyer.require_auth() above)
        // and does NOT call require_auth again, avoiding Soroban's
        // "frame is already authorized" error.
        let token = Self::get_token(e);
        let sell_sym = Symbol::new(e, "sell_forwarded");
        let sell_args: Vec<Val> = vec![&e, buyer.clone().into_val(e), prompt.price.into_val(e)];
        let _: () = e.invoke_contract(&token, &sell_sym, sell_args);

        // Mark the purchase so has_access returns true.
        let purchase_key = DataKey::Purchase(buyer.clone(), prompt_id.clone());
        e.storage().instance().set(&purchase_key, &true);

        PromptPurchased {
            buyer,
            prompt_id,
            price: prompt.price,
        }
        .publish(e);
    }

    // ─── Admin: re-mint tokens ───────────────────────────────

    /// Re-mint tokens back into circulation.
    /// The admin can put burned tokens back on the market.
    /// Calls `mint_forwarded` (no auth check) because `enforce_admin` above
    /// already verified the admin's authorization at the root level. Calling
    /// the regular `mint` (with `only_owner`) would trigger a double
    /// `require_auth` for the same address.
    pub fn remint(e: &Env, to: Address, amount: i128) {
        Self::enforce_admin(e);
        assert!(amount > 0, "amount must be positive");

        let token = Self::get_token(e);
        let mint_sym = Symbol::new(e, "mint_forwarded");
        let mint_args: Vec<Val> = vec![&e, to.clone().into_val(e), amount.into_val(e)];
        let _: () = e.invoke_contract(&token, &mint_sym, mint_args);

        TokensReminted {
            admin: Self::get_admin(e),
            to,
            amount,
        }
        .publish(e);
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

    /// Read the content URI for a prompt.
    pub fn get_content_uri(e: &Env, prompt_id: String) -> String {
        let key = DataKey::Prompt(prompt_id);
        let prompt: Prompt = e
            .storage()
            .instance()
            .get(&key)
            .expect("prompt not found");
        prompt.content_uri
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

    // ─── AccessRegistry ──────────────────────────────────────

    /// Register an opaque policy commitment. Only the hash and token price
    /// are stored — content_uri and prompt identity never touch the ledger.
    /// Returns a monotonically increasing policy_id.
    pub fn register_policy(e: &Env, policy_commitment: Bytes32, price: i128) -> u64 {
        Self::enforce_admin(e);
        assert!(price > 0, "price must be positive");

        let id: u64 = e
            .storage()
            .instance()
            .get(&DataKey::PolicyCounter)
            .unwrap_or(0u64);

        let policy = PolicyData {
            commitment: policy_commitment.clone(),
            price,
        };
        e.storage().instance().set(&DataKey::Policy(id), &policy);
        e.storage()
            .instance()
            .set(&DataKey::PolicyCounter, &(id + 1));

        PolicyRegistered {
            policy_id: id,
            commitment: policy_commitment,
        }
        .publish(e);

        id
    }

    /// Issue an opaque access leaf without recording buyer identity or
    /// prompt_id in storage. Updates the Merkle accumulator root and
    /// returns the leaf for the caller to retain as their access proof.
    ///
    /// Admin-gated: payment verification happens off-chain (or via a
    /// separate token-burn step) before the admin calls this function.
    pub fn issue_access(e: &Env, policy_id: u64, session_commitment: Bytes32) -> Bytes32 {
        Self::enforce_admin(e);

        let _: PolicyData = e
            .storage()
            .instance()
            .get(&DataKey::Policy(policy_id))
            .expect("policy not found");

        let leaf = Self::compute_leaf(e, policy_id, &session_commitment);

        let current_root: Bytes32 = e
            .storage()
            .instance()
            .get(&DataKey::AccessRoot)
            .unwrap_or_else(|| Self::genesis_root(e));

        let new_root: Bytes32 = {
            let mut buf = Bytes::new(e);
            let root_bytes = Bytes::from_array(e, &current_root.to_array());
            buf.append(&root_bytes);
            let leaf_bytes = Bytes::from_array(e, &leaf.to_array());
            buf.append(&leaf_bytes);
            e.crypto().sha256(&buf).into()
        };
        e.storage()
            .instance()
            .set(&DataKey::AccessRoot, &new_root);

        AccessIssued {
            leaf: leaf.clone(),
            new_root,
        }
        .publish(e);

        leaf
    }

    /// Return the current Merkle accumulator root over all issued access leaves.
    pub fn root(e: &Env) -> Bytes32 {
        e.storage()
            .instance()
            .get(&DataKey::AccessRoot)
            .unwrap_or_else(|| Self::genesis_root(e))
    }

    // ─── AccessNullifiers ─────────────────────────────────────

    /// Consume a nullifier, permanently preventing its reuse.
    /// Panics if the nullifier has already been consumed.
    pub fn consume(e: &Env, nullifier: Bytes32) {
        let key = DataKey::Nullifier(nullifier.clone());
        let already: bool = e.storage().instance().get(&key).unwrap_or(false);
        assert!(!already, "nullifier already used");
        e.storage().instance().set(&key, &true);

        NullifierConsumed { nullifier }.publish(e);
    }

    /// Check whether a nullifier has been consumed.
    pub fn used(e: &Env, nullifier: Bytes32) -> bool {
        e.storage()
            .instance()
            .get(&DataKey::Nullifier(nullifier))
            .unwrap_or(false)
    }

    // ─── Internal: privacy helpers ────────────────────────────

    /// leaf = SHA256(policy_id_be_bytes || session_commitment)
    fn compute_leaf(e: &Env, policy_id: u64, session_commitment: &Bytes32) -> Bytes32 {
        let mut buf = Bytes::new(e);
        buf.append(&Bytes::from_array(e, &policy_id.to_be_bytes()));
        buf.append(&Bytes::from_array(e, &session_commitment.to_array()));
        e.crypto().sha256(&buf).into()
    }

    /// Deterministic initial root so `root()` is stable before any access is issued.
    fn genesis_root(e: &Env) -> Bytes32 {
        e.crypto()
            .sha256(&Bytes::from_array(e, b"access_genesis"))
            .into()
    }
}
