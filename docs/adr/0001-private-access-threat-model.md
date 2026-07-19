# ADR 0001: Private Access Threat Model and Stellar Feasibility

## Status
Proposed

## Context
The repository currently implements a Soroban marketplace where buyers purchase prompts, token balances are burned, and access is recorded on-chain via `has_access`. Before adding any private-access registry, we must define what "private access" means for this system and choose a solution that fits Stellar/Soroban constraints.

This ADR focuses on:
- defining private access goals and attacker capabilities
- enumerating on-chain and off-chain leakage
- evaluating candidate architectures: encrypted off-chain delivery, opaque access records, relayers, and zero-knowledge proofs
- recommending a practical path for the current codebase
- defining anti-replay, key lifecycle, delivery, migration, and test evidence requirements

## Goal
Establish a formal threat model and privacy feasibility analysis for private access that can guide the design of the marketplace without prematurely committing to a specific on-chain protection mechanism.

## Problem
A buyer purchases a prompt using tokens, but the current contract flow exposes on-chain evidence of purchase and prompt identifiers. The system must answer: what guarantees can be offered, under which adversaries, and what tradeoffs are acceptable for the current Stellar-based implementation?

The key questions are:
- What does "private access" protect, and from whom?
- What types of data are visible to blockchain observers, backend operators, admins, and buyers?
- Which mechanisms are compatible with Stellar/Soroban today, and what costs/problems do they impose?
- What operational and lifecycle controls are needed for key management, delivery, and testing?

## Definitions
### Private Access
For this ADR, "private access" means:
- a buyer can obtain or prove access to a purchased prompt without creating a permanent, readable on-chain record that directly ties that buyer to the prompt content or prompt identifier
- the system minimizes leakage of buyer intent, prompt identifiers, and purchase timing to third-party observers
- the system avoids enabling backend/admin processes to deanonymize buyers or reconstruct prompt access beyond what is necessary for delivery and entitlement verification

This is intentionally narrower than "fully anonymous payments" or "confidential execution"; the primary focus is access authorization privacy for prompt content.

### Scope of Guarantees
This ADR treats privacy goals as layered by adversary type:
- Blockchain observer: sees on-chain contract calls, events, storage keys, gas/timing, and token movements
- Backend operator: sees off-chain request metadata, encrypted delivery payloads, server logs, user IPs, and database state
- Administrator: has privileged access to the backend and off-chain systems, but not necessarily the private keys of buyers
- Buyer: has access to their own prompts, keys, and any data delivered to them

### Threat Model Assumptions
Assume the following baseline properties of the current system:
- Soroban contract state and transactions are publicly visible to all observers
- buyers authenticate to the contract with their Stellar address/signature
- prompt pricing is paid in a fungible token where `sell_forwarded` burns the amount and the marketplace records access
- current marketplace access is binary and queryable via `has_access(address, prompt_id)`
- the backend may be trusted for prompt delivery but should not be a single source of privacy guarantees

## Threat Model
### Adversaries
#### 1. Blockchain Observer
Capabilities:
- read all on-chain transactions, events, contract storage, metadata, and transaction timing
- index operations by address, prompt IDs, and contract methods
- analyze token burn or transfer behavior

Goals:
- determine which address purchased which prompt
- correlate prompt IDs with buyer addresses or time windows
- infer buyer activity from market behavior

#### 2. Backend Operator
Capabilities:
- access off-chain database and logs
- inspect API request headers, IP addresses, session data, and backend storage
- intercept prompt delivery if the backend implements content distribution

Goals:
- link buyer identity to purchased prompts
- decrypt deliveries if they are stored or proxied by backend systems
- abuse privileged access to expose purchase histories

#### 3. Administrator
Capabilities:
- full backend access and operational control over the contract deployment environment
- may access backend keys, stored encrypted payloads, and service logs
- does not have buyers' private signing keys by default

Goals:
- reconstruct buyer purchase history
- recover prompt access data from server storage
- perform surveillance of access patterns

#### 4. Malicious Buyer
Capabilities:
- act as buyer on-chain, including purchase flow and post-purchase access checks
- possibly interact with the backend or relayer services
- may reuse or attempt to replay delivery proofs

Goals:
- claim access without paying, or reuse access proofs across prompt purchases
- infer the ownership or access rights of other buyers through protocol behavior

## Leakage Vectors
Evaluate where private-access information can leak in the current design and future variants.

### 1. Transaction Arguments
- `buy_prompt(buyer, prompt_id)` reveals the buyer address and the prompt identifier directly in the transaction payload
- `has_access(user, prompt_id)` reveals prompt IDs when queried by a buyer or external observer if arguments are visible in a contract call

### 2. Authorization Entries
- on-chain auth is tied to the buyer address and call origin; no hidden authorization exists
- contract methods such as `register_prompt` and `buy_prompt` require explicit `require_auth()` on buyer/admin addresses

### 3. Storage
- current `has_access` implementation writes a mapping from `(buyer, prompt_id)` to `true`
- prompt metadata (`uri`, owner, price) is likely stored in cleartext under prompt IDs
- any storage keys or values that combine buyer addresses and prompt IDs leak direct association

### 4. Events
- events emitted by `PromptPurchased` and other marketplace actions expose buyer, prompt ID, and price on-chain
- even if prompt IDs are opaque, repeated event emission can be correlated to buyers and timing

### 5. Token Activity
- burning tokens during `buy_prompt` is observable and connects the buyer address to a purchase action
- token balance changes may reveal which addresses are purchasing if an on-chain token is reused for private access

### 6. Metadata and Timing
- prompt registration time, purchase time, and event sequence can be correlated
- prompt IDs may be stable and linkable across operations
- contract invocation timing plus external delivery timing can enable cross-layer correlation

## Candidate Solutions
This section compares candidate privacy architectures by guarantees, cost, feasibility, and operations.

### Option A: Encrypted Off-Chain Delivery
Buyer purchases on-chain normally, while the prompt payload or access secret is delivered off-chain encrypted to the buyer.

Guarantees:
- on-chain purchase remains visible, but prompt content is not stored on-chain
- buyer-specific access is confidential if encryption keys remain private
- backend still learns the buyer-request semantics unless the request is anonymous or stored minimally

Cost and feasibility:
- low on-chain cost, because contract changes are small or none
- simple to implement with existing Stellar/Soroban infrastructure
- requires an off-chain key distribution system and secure storage of encrypted payloads

Operations:
- generate per-prompt or per-purchase encryption keys
- deliver encrypted payload via backend or client-side decryption
- rotate keys and support buyer key recovery/migration

Risks:
- backend operator / admin can read payloads if they control keys or plaintext delivery channels
- transaction-level purchase association remains visible to blockchain observers
- this does not provide anonymous access verification, only content confidentiality

### Option B: Opaque Access Records
Record access on-chain using commitments, hash pointers, or blind identifiers instead of explicit `(buyer, prompt_id)` pairs.

Guarantees:
- direct access linkage between buyer and prompt can be hidden if access records are opaque
- observers cannot trivially map buyer → prompt if commitments are computed securely and not reused across prompts
- the contract can still verify ownership through proof-of-knowledge or preimage reveal patterns

Cost and feasibility:
- moderate on-chain cost: storage of commitments/hashes and contract logic for verification
- compatible with Soroban contract storage and deterministic hashing, but careful design is needed to avoid leaking preimages through known prompt IDs
- requires off-chain coordination to generate and distribute commitment data or access tokens

Operations:
- buyer derives a commitment using a secret and prompt-specific data
- contract stores or checks commitments instead of explicit buyer IDs
- backend or client provides secrets for entitlement recovery or prompt delivery

Risks:
- if commitments are reused or simple, observers may still correlate repeated purchases
- if the backend delivers the preimage, the backend becomes a critical trust point
- on-chain events or storage keys may still reveal access patterns unless access records are carefully isolated

### Option C: Relayers
Use relayer services to obscure buyer transaction origination and reduce direct wallet-to-contract linkability.

Guarantees:
- transaction origin can be separated from the buyer if the buyer submits requests through a relayer or fee-bumping service
- does not hide the actual buyer address if the contract still includes buyer auth or if the buyer signs a transaction with their address
- can help with UX by abstracting fee payment and gas management

Cost and feasibility:
- low on-chain cost, because contracts remain mostly unchanged
- Stellar supports fee bump transactions and relayer patterns, but this is not a privacy feature by itself
- requires a trusted relayer or an anonymous relay network and a user protocol to assert authorization off-chain

Operations:
- buyer constructs a signed transaction or authorizes a relayer to submit on their behalf
- relayer pays fees and submits the transaction to the network
- anti-replay / nonce handling can be implemented at the transaction or application layer

Risks:
- the relayer learns transaction intents and can correlate buyer signature requests with prompt purchases
- if the relayer is compromised or malicious, it can deanonymize or censor users
- contract-level auth still binds the buyer address unless replaced with a more opaque mechanism

### Option D: Zero-Knowledge Proofs
Use ZK proofs to validate access or purchase entitlement without revealing buyer/prompt linkage.

Guarantees:
- can provide strong privacy if proofs are constructed such that the verifier only learns a boolean access decision
- buyer identity and prompt ID can be hidden in the proof witness, depending on the protocol
- can also support anonymous credentials, nullifiers, and unlinkable access tokens

Cost and feasibility:
- high complexity and cost on Stellar today
- Soroban does not natively support general-purpose ZK proof verification efficiently; on-chain ZK verification would likely require large WASM footprints or external verification
- there is no mature, audited ZK library in this repository, and Stellar does not currently provide optimized ZK primitives for this use case

Operations:
- build a ZK circuit for prompt entitlement proofs
- generate and verify proofs off-chain or in a specialized oracle/relayer
- manage nullifiers, commitments, and proof reuse prevention

Risks:
- very high engineering and maintenance burden
- large performance/gas costs on Soroban contract execution
- likely premature for the current project and threat model unless absolutely required

## Recommendation
For the current Stellar/Soroban marketplace, the most feasible and practical private-access architecture is:

1. Keep the on-chain purchase flow simple and explicit for now.
2. Define "private access" as content confidentiality and minimized linkage rather than full on-chain anonymity.
3. Use encrypted off-chain delivery for prompt payloads or buyer-specific access secrets.
4. Avoid adding a private-access registry or complex ZK/relayer scheme until the ADR is reviewed and approved.

This means the system should first focus on:
- removing explicit prompt ownership or buyer prompt links from on-chain events and storage where possible
- delivering prompt content or access tokens off-chain in encrypted form
- using opaque identifiers for prompt content if prompt IDs are sensitive
- preserving the ability to verify entitlement on-chain without making the mapping trivially public

## Comparison Table
| Approach | Privacy guarantee | Stellar feasibility | On-chain cost | Operational burden | Notes |
|---|---|---|---|---|---|
| Encrypted off-chain delivery | content confidentiality; purchase linkage still visible | high | low | moderate | Best first step for current repo |
| Opaque access records | partial unlinkability if carefully designed | medium | medium | moderate | Good for future hardening, but needs clear design |
| Relayers | limited privacy benefit alone | high | low | moderate | Useful for UX/fees, not core privacy |
| ZK proofs | strongest privacy in principle | low | high | high | Premature for this repo today |

## Anti-Replay and Key Lifecycle
### Anti-Replay
Any off-chain or commitment-based access token must include:
- prompt-specific freshness (timestamp, counter, or unique prompt nonce)
- one-time use or nullifier handling if tokens are reused for proof
- delivery tokens must be bound to the buyer or the buyer's secret
- replay protection should be enforced off-chain and, when possible, on-chain via nonces or commitment revocation

### Key Lifecycle
- Buyers should use local keys or wallet-controlled secrets for decryption/encryption of prompt payloads
- The backend should never retain buyer private keys; it may hold public keys or ephemeral delivery keys only
- If buyer recovery is required, the system should support a key migration path where a buyer can re-encrypt or rekey content to a new valid secret
- Prompt content encryption keys should be rotated periodically, and old keys should be retired securely

### Delivery
Delivery should be designed as:
- buyer requests prompt via backend or peer-to-peer channel
- backend fetches encrypted payload or access secret from storage
- backend delivers ciphertext to buyer without storing plaintext long-term
- buyer decrypts locally using their key material

### Migration
If the system eventually adds an on-chain private-access registry:
- keep it separate from the current `has_access` boolean storage until approved
- design migration to opaque identifiers or hashed commitments rather than raw `(buyer, prompt_id)` tuples
- include a compatibility plan so existing buyers retain entitlement without leaking old access records

## Test Evidence Requirements
To approve this ADR for implementation, the following evidence should be produced:
- a threat model document that clearly enumerates adversaries and leakage vectors
- privacy evaluation of the current flow versus proposed designs
- a prototype or design sketch for encrypted off-chain delivery and/or opaque access records
- test cases that validate the following:
  - prompt content is not stored in plaintext on-chain after purchase
  - events do not emit buyer/prompt linkage when privacy definitions require it
  - off-chain delivery uses encryption and does not expose plaintext to backend logs
  - anti-replay protections prevent reuse of access tokens or secrets
- clearly documented assumptions and limitations in the ADR

## Decision
Do not implement a private-access registry until this ADR is approved.

The recommended next step is to adopt encrypted off-chain delivery and privacy-conscious prompt identifiers, then iterate toward opaque commitments only if requirements demand stronger on-chain unlinkability.
