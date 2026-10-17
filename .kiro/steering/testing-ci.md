---
inclusion: manual
---

# HPDL — Testing & CI

Invoke with `/testing-ci` for test-writing, CI changes or release review.

## CI gates
- Matrix: pinned stable Rust on Linux, macOS, Windows.
- `fmt --check`, `clippy -D warnings`, unit + integration + doc tests.
- **Cross-platform digest parity:** compare PIR and replay hashes *across* OS jobs, not just pass/fail per job.
- `cargo deny check` (licenses, advisories).
- `cargo semver-checks` on published contract crates.
- MSRV job.
- Releases: changelog entry required; no unresolved critical security issues; no unexplained hash divergence.

## Test suites
| Suite | Covers |
|---|---|
| Unit | Model invariants, units, thresholds, lowering, decoder |
| Frontend equivalence | RON and HPDL → identical PIR bytes |
| Golden vectors | Frozen bytes + digests; changes need format review |
| Diagnostics | `insta` snapshots, reviewed on update |
| Property | Canonical round-trip, semantic equivalence, malformed-input rejection (`proptest`) |
| Fuzz | `cargo-fuzz` on PIR decoder (and parser once it exists); keep regression corpora |
| Replay | Deterministic action → event → state transitions |
| Migrations | See below |

## Required boundary cases
- Thresholds: `24.5`, `25`, `49.999`, `100`, `100.0001` (reject), `-0.0001` (reject).
- Fixed-point: Pa vs kPa, m vs mm; multiply/divide; negative ties; half-quantum `ties_to_even`; near-overflow; repeated conversion.
- PIR: malformed bytes, duplicate fields, non-canonical encodings, decode size/depth limits, unknown critical extension.

## Migration cases
Success; pre-commit failure; successful-but-wrong → forward-fix; attempted history rewrite; invalid operator signature; insufficient quorum; validator-set rotation across activation; duplicate signatures; duplicate migration ID; replay with historical policy versions; conflicting in-flight actions at the boundary.

Fuzzing supplements, but does not prove, safety.
