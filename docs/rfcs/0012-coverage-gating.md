# 0012 — Code-coverage gating (ratchet floors, region metric for critical crates)

**Status:** Accepted
**Date:** 2026-10-18
**Feature:** CI / repo hygiene (`coverage.yml`)

## Context

`coverage.yml` collected `cargo llvm-cov` data, published it to Codecov, and
uploaded an lcov artifact, but **enforced nothing** — coverage could regress to
zero without failing CI. The request was to add gating: a workspace-wide floor
plus tighter floors on the two coverage-critical crates, the "decoder"
(`hpdl-pir`) and the "validator" (`hpdl-sema`).

A real baseline was measured before choosing any numbers (never assume):

| Scope | Metric | Measured baseline |
|---|---|---|
| workspace | lines | **88.40%** |
| `hpdl-pir` | lines / regions | 96.4% / **97.35%** |
| `hpdl-sema` | lines / regions | 92.2% / ~**94.2%** (after added tests) |

### The line-coverage ceiling (the key finding)

An initial target of 95% workspace / 97% per-crate **line** coverage is not
reachable on `hpdl-pir` / `hpdl-sema` by adding honest tests. Tracing every
uncovered line showed the residue is overwhelmingly **mandated defense-in-depth
error handling** — the rust-standards rule "fail closed with typed errors" and
the determinism rule "malformed input never panics" require these re-checks, yet
the `hpdl-model` constructors reject the malformed inputs *before* the validator
or decoder re-check ever executes. Examples:

- `environment_validator.rs`: zero-pressure, inverted-temperature,
  coverage-over-range, malformed-digest arms — all pre-rejected by
  `AtmosphereProfile`/`ThermalProfile`/`Hydrosphere`/`Digest` constructors.
- `type_checker.rs`: empty/over-long name and empty-version arms — pre-rejected
  by `BodyId`/`StatusDeclaration`/`PlanetDefinition`.
- `status_validator.rs`: the `status.`-write branch — `ActionDeclaration::new`
  rejects status writes at construction (`E` StatusWriteForbidden).
- `hpdl-pir` `decoding.rs`/`encoding.rs`: oversized-payload and
  serialize-failure arms — the encoder enforces `MAX_PIR_SIZE` and a valid
  `PlanetDefinition` always serializes.

Reaching high-90s **line** coverage would require either (a) deleting required
defensive code, or (b) constructing illegal models via a test-only bypass to
brush the branches. Both are rejected: (a) violates steering + rust-standards;
(b) produces dishonest tests the workflow rules forbid ("never invent passing
test results").

## Decision

Gate CI on **measured floors** (ratchet model), using **region coverage** for
the two critical crates and **line coverage** for the published workspace run.

| Gate | Metric | Floor | Measured | Mechanism |
|---|---|---|---|---|
| workspace | lines | **85%** | ≥88.4% | `cargo llvm-cov --workspace --fail-under-lines 85` |
| `hpdl-pir` | regions | **97%** | 97.35% | `cargo llvm-cov -p hpdl-pir --fail-under-regions 97` |
| `hpdl-sema` | regions | **93%** | 94.23% | `cargo llvm-cov -p hpdl-sema --fail-under-regions 93` |

Floors sit a few points below the observed values so ordinary churn does not red
CI; they are a **ratchet** — raised deliberately as coverage improves, never
lowered silently. The Codecov upload and lcov artifact are retained unchanged for
the published 95%-visibility report.

Per-crate gates are **separate `cargo llvm-cov` invocations** scoped with
`--package`, because `--fail-under-regions` applies to the whole run; scoping by
package is how an individual crate is gated. This costs extra CI minutes but
fails explicitly and unambiguously, matching the project's fail-closed discipline.

### Why region coverage for the critical crates

Region coverage is the metric that reflects *tested behavior* without being
distorted by deliberately-unreachable defensive arms. Line coverage counts each
defensive line it can never enter; region coverage is far less sensitive to this,
so a region floor is both honest and meaningful. `hpdl-pir` clears 97% regions
with no exclusions; `hpdl-sema` tops out ~94% regions and is gated at 93%.

## Rationale

- Gating on measured floors keeps CI green on the first run (REQ: never ship a
  gate that fails) while still catching real regressions.
- Choosing region over line coverage is the honest response to mandated
  defensive code, rather than relaxing the defensive-code rules or faking tests.
- Keeping Codecov + lcov intact preserves the existing published report.

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| 95% workspace / 97% per-crate **line** gates | Not reachable without deleting mandated defensive code or faking illegal-model tests. |
| 97% region on sema with `--ignore-filename-regex` excluding the defensive files | Viable, but hides whole files from the metric; the ratchet floor is simpler and keeps every file counted. (Chosen floor = option B of the design discussion.) |
| Report-only (`continue-on-error`) gates | Does not actually gate; the request was enforcement. |
| Rely on Codecov's own status thresholds | Less deterministic than an in-job `cargo llvm-cov` exit code; kept Codecov for visibility only. |

## Compatibility impact

- CI-only change. No crate source, public API, or PIR-format impact.
- `coverage.yml` now fails the job on coverage regression below the floors.
- New tests added this session (`hpdl-sema` resolver/diagnostic/status/orbital/
  environment paths; ~30 tests) are pure additions; all pass.

## Test consequences

- No golden-vector changes; no PIR change.
- The gates are enforced by `cargo llvm-cov` exit codes in CI, not by unit tests.
- Thresholds must be re-measured and the floors raised whenever substantial new
  tested code lands; this ADR is the record of the current measured ceiling.

## Deferred — surfaced, NOT decided here

- **Raising `hpdl-sema` toward 97% regions.** Requires either refactoring the
  model/validator boundary so the defensive re-checks become reachable, or a
  deliberate `--ignore-filename-regex` exclusion of the pure-defensive files.
  Either is a separate, explicitly-approved change.
- **Branch coverage.** `cargo llvm-cov` branch coverage is nightly-only and
  unstable; not adopted.
