# ADR 0001: Private Access Threat Model

## Status
Approved and Implemented

## Context
Before implementing a private-access registry in the Soroban smart contracts for the Stellar-AgentVerse platform, it is crucial to strictly define the security architecture and evaluate trade-offs. The current system handles prompt sales via contracts (`PromptMarketplace`, `MyToken`). We need to guarantee the privacy of the prompts accessed by authorized buyers, minimizing exposure on the blockchain. Since this analysis focuses on the **Smart Contracts** layer, we evaluate how to design the contracts to support secure private access without leaking sensitive information on the public Stellar network.

## 1. Threat Model

### Actors and Capabilities
*   **Blockchain Observers**: Anyone monitoring the public Stellar ledger. They can see all transactions, contract invocations, arguments, state changes, events, and execution timing.
*   **Backend / Indexers**: Off-chain infrastructure that interacts with contracts or reads events. They might attempt to correlate on-chain activity with specific users.
*   **Admins**: Platform operators who manage the marketplace.
*   **Buyers**: Users purchasing access to prompts. They might act maliciously by attempting to access unpaid prompts, reuse access (replay attacks), or bypass contract controls.

## 2. Leakage Analysis

In the context of Soroban smart contracts, information can leak in several ways:

*   **Arguments**: Passing plaintext prompt IDs or access keys in contract function arguments (e.g., `buy_prompt(id="secret-alpha")`) will leak this data on the ledger.
*   **Auth Entries**: Soroban `require_auth()` payloads expose the addresses and the exact contract functions being authorized, revealing who is participating and in what.
*   **Storage**: On-chain state is public. Storing unencrypted prompt content or plaintext access keys is a critical vulnerability.
*   **Events**: Emitting events like `PromptPurchased` with detailed data (`buyer`, `prompt_id`, `price`) leaves a permanent public trail of user activity, leaking their interests.
*   **Token Activity**: Token transfers (`MyToken`), such as mints and burns, correlate with purchases and reveal the economic value exchanged.
*   **Timing**: The time between on-chain purchase execution and off-chain access claim can help correlate identities if not obfuscated.

## 3. Solution Trade-offs

We compare four potential technical approaches for the smart contracts:

| Approach | Privacy Guarantees | Cost (Fees / Compute) | Soroban Feasibility | Operational Overhead |
| :--- | :--- | :--- | :--- | :--- |
| **Opaque Access Records (Hashes/Commitments)** | **Moderate**. Hides specific IDs on-chain using hashes (e.g., SHA-256). Buying patterns remain visible. | **Low**. Hashing is efficient and cheap in Soroban. | **High**. Very simple to implement in Rust/Soroban. | **Low**. Requires the client or backend to validate hashes. |
| **Encrypted Off-chain Delivery (On-chain tracking only for rights)** | **High**. The contract only records access rights. Content privacy relies on off-chain delivery. | **Low**. Minimal on-chain logic. | **High**. Keeps contracts simple. | **Moderate**. Requires robust off-chain infrastructure. |
| **Relayers (Meta-transactions)** | **High (for identity)**. Hides the buyer's address by subsidizing fees through a relayer. | **High**. Requires managing relayer accounts and complex signature delegation. | **Moderate**. Soroban supports auth delegation, but requires extra management. | **High**. Relayer infrastructure maintenance. |
| **Zero-Knowledge (ZK)** | **Very High**. Allows proving the purchase without revealing *who* bought *what*. | **Very High**. Expensive on-chain verification (compute limits). | **Low**. ZK support in Soroban is nascent and would require complex circuits. | **Very High**. Requires prover infrastructure and circuit setup. |

## 4. Security & Lifecycle Definitions

For the implementation of the access registry in Soroban, we define the following requirements:

### Anti-replay Mechanisms
If contracts must verify access claims, they must include nonce mechanisms per address or strict timestamps in delegated signatures to prevent a single authorization from being reused multiple times (replay attacks).

### Key Lifecycle
The on-chain logic must not store direct decryption keys. If commitments are used, the contract must manage the commitment lifecycle (creation upon purchase, invalidation or expiration upon claim).

### Delivery Protocol (Contract Perspective)
1.  The buyer submits a purchase transaction on-chain sending an opaque **hash (commitment)** instead of the plaintext prompt ID.
2.  The contract burns the corresponding tokens and stores the commitment.
3.  The contract emits a generic or opaque event for indexing, without revealing the buyer's direct identity if delegations are used, or at least hiding the purchased asset.

### Migration Paths
If ZK or another advanced technology is adopted in the future, the contract must use upgradable patterns or storage delegation to allow migration of old access records to the new format without loss of rights.

### Required Test Evidence
*   **Unit Tests (Rust)**: Ensure unauthorized contract accesses fail and commitment (hash) verification is correct.
*   **Leakage Tests**: Validate in tests that neither events nor state storage include plaintext strings related to prompts.
*   **Integration Tests**: Simulate opaque purchase flows on testnet and verify correct token deduction.

## Decision
**Approved**.
We have implemented the **Opaque Access Records (Hashes/Commitments)** approach natively in `PromptMarketplace` using `BytesN<32>` hashes. This avoids massive ZK costs while sanitizing on-chain state and events of sensitive information.

### Implemented Evidence & Cryptography Assumptions

*   **Commitment Construction & Entropy**: Hashes (`BytesN<32>`) must be generated off-chain using a cryptographic hash function (e.g., SHA-256) applying domain separation and a high-entropy salt to prevent preimage/dictionary attacks. The required format is: `Hash = SHA256(Domain_Separator || Prompt_ID || High_Entropy_Salt)`. Example domain separator: `PMPT_V1`.
*   **Resource-Cost Measurements**: The feasibility test records host CPU and memory deltas for an opaque access write. Those numbers are not a Testnet fee quote: fees also depend on the network fee schedule, transaction footprint, and storage rent. A deployment decision must record a `simulateTransaction`/Testnet result for the exact deployed WASM.
*   **Privacy Guarantees**: Prompt content, URIs, and human-readable IDs are never exposed on-chain or in event schemas. Validated by storage and event inspection tests rejecting plaintext.
*   **Residual Metadata Leaks**: The transaction submitter (buyer), exact time of purchase, and the price (tokens burned) remain public. The opaque hash allows linkability if the same prompt is bought multiple times by different users.
*   **Failure Modes & Atomic Transitions**: Replay attacks are prevented via atomic storage updates (`PrivatePurchase` key mapping). If token deduction fails (e.g., insufficient balance), the entire transaction rolls back atomically, preventing partial entitlement. Unauthorized registrations are blocked by `require_auth`.
*   **Key-Lifecycle Assumptions**: The contract does not store or manage DEKs (Data Encryption Keys). The backend manages key rotation. The smart contract acts exclusively as an authorization ledger. A private access record is perpetual by default; an authorized administrator may invalidate it with `revoke_private_access`. No automatic cleanup is needed while a grant is valid; the contract instance TTL and upgrades remain the operational storage lifecycle.
*   **Delivery-Service Trust**: Buyers must trust the off-chain delivery service to respect the on-chain purchase event and securely deliver the decrypted prompt content. The contract cannot mathematically enforce the off-chain delivery (no ZK/escrow).

## Consequences
*   We will modify `PromptMarketplace` to accept opaque identifiers (hashes) instead of plaintext prompt IDs.
*   Emitted events are versioned (`*_v1`) for indexers. They expose the unavoidable buyer, commitment, and price metadata, but never a plaintext prompt identifier or key.
*   Changes will be required in how clients construct transactions: the trusted registration path must generate `SHA256("PMPT_V1" || prompt_id || fresh_32_byte_salt)` and retain the salt off-chain. The contract cannot validate this preimage because it intentionally never receives it; it only accepts the resulting opaque 32-byte commitment.
