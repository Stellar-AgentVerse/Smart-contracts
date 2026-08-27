# ADR 0001: Private Access Threat Model and Stellar Feasibility

- **Status:** Proposed
- **Scope:** Soroban contracts and the minimum interface required for encrypted off-chain delivery
- **Gate:** No private-access registry implementation begins until this ADR is approved

## Context

The current marketplace stores `buyer -> prompt_id`, stores `content_uri`, and emits purchase events containing buyer and prompt identifiers. This is a public marketplace flow, not anonymous access. The product target is privacy against public blockchain observers while prompt delivery remains encrypted and off-chain.

## Questions This ADR Must Answer

1. What must be hidden: prompt content, buyer/prompt relationship, payer identity, purchase history, and replay attempts?
2. Which observers are in scope: chain observers, the backend, administrators, other buyers, or all of them?
3. Which data remains observable through arguments, authorization entries, storage, events, token transfers, transaction metadata, and timing?
4. What delivery protocol provides the buyer with encrypted content without publishing decryption material on-chain?
5. What replay protection, key lifecycle, revocation, and migration behavior are required?

## Options to Compare

| Option | Benefit | Limitation to measure |
|---|---|---|
| Encrypted off-chain content | Smallest on-chain change | May not hide payment/access correlation |
| Opaque commitments and nullifiers | Supports one-time access records | Does not automatically hide transaction or token identity |
| Relayer or payer/recipient separation | Reduces direct wallet linkage | Adds trust, fees, abuse controls, and operational complexity |
| ZK verification | Strongest unlinkability potential | Requires supported primitives, verifier cost, proof format, and audited circuits |

## Decision Gate

The approved option must document its privacy guarantees, known leaks, Soroban resource cost, proof/key dependencies, failure modes, and test strategy. Commitments, nullifiers, and ZK are candidates—not requirements—until those facts are demonstrated with a small feasibility prototype and adversarial tests.
