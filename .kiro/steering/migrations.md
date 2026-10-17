---
inclusion: fileMatch
fileMatchPattern: ["**/*migration*", "**/*governance*", "**/*authoriz*", "tests/migrations/**"]
---

# HPDL — Migrations & Governance (Amendments A3, A3.1, A3.2)

## Ownership
| Layer | Owner |
|---|---|
| Declaration (versions, prerequisites, invariants) | `hpdl-model` |
| Canonical serialization of declarations and records | `hpdl-pir` |
| Execution interfaces, receipts, failure contracts | `hpdl-runtime-api` |
| Snapshots, authorization enforcement, transformation, replay | external simulation repo |

## Version boundaries
Language semantics, planet definition, PIR format, stdlib packages, module implementation and state schema are versioned independently. Running planets stay pinned until an authorized migration activates. Upgrades are never implicit.

## Protocol
1. Proposal names: planet, source/target definition + schema hashes, migration implementation digest, invariants, activation tick, nonce, governance-policy ID.
2. Dry-run against a snapshot; validate invariants.
3. Apply atomically at a tick boundary. Pre-commit failure leaves prior state authoritative.
4. Commit a signed event: old/new state + definition hashes, implementation hash, tick, approvals, migration ID, result.
5. In-flight actions at the boundary are deterministically queued or rejected per declared policy.
6. Retries are idempotent via unique migration ID.

## Forward-fix only
A committed migration is immutable. A wrong-but-successful migration is corrected by a NEW authorized forward migration. Never erase, reorder or rewrite committed history. Disaster recovery must be an auditable recovery operation that preserves fork/recovery evidence.

## Authorization
- Versioned `MigrationAuthorizationPolicy`: operator key(s), validator set or commitment, signature suite, threshold, signature domain, rotation and revocation rules, policy hash.
- Standalone worlds MAY be operator-only; federated worlds MUST meet validator attestation.
- Signatures bind world ID, migration ID, old/new hashes, implementation hash, activation tick and policy hash.
- Reject missing, invalid, expired, duplicate or policy-incompatible approvals.
- Never hardcode a quorum. Approval proves consent, not correctness.

## Replay
Replay selects the module, schema and authorization-policy versions active at each historical tick. Preserve historical module binaries, hashes and snapshots.

## ⚠️ Unresolved — surface, do not decide in code
- **Approval expiry basis:** must be simulation ticks (wall clock breaks replay) or the validity result must be recorded in the signed event. Needs an ADR.
- **Revoked-key handling** for pending and in-flight approvals.
- **Emergency policy** when validator quorum is unreachable (e.g. world pause).
