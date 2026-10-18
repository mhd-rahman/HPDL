# 0011 — Compile-time Module Registry (existence, kind, integrity)

**Status:** Accepted
**Date:** 2026-10-18
**Feature:** `14-stdlib-packages` (registry concern; the "module registry is deferred" note in features 09/11/12)

## Context

Five model sites wrap a versioned `hpdl_model::ModuleRef` and one carries a
content-addressed `Digest`; each names an implementation HPDL does not execute:

| Reference | Model site | Non-executable flag today |
|---|---|---|
| Evaluator module | `EvaluatorRef.module` (`status_vitality.rs`) | `E9202 UnsupportedEvaluator` (error) |
| Climate model | `AtmosphereProfile.climate_model` (`environment.rs`) | `E9103 UnsupportedEnvironmentModule` (error) |
| World-law module | `WorldLaw.module` (`world_law.rs`) | `E6005 UnsupportedWorldLaw` (**warning**) |
| Magic-bridge module | `MagicBridge.module` (`world_law.rs`) | *(none — gap)* |
| Library/body module | `CelestialBody.modules` (`celestial_body.rs`) | resolved vs `hpdl.lock` (ADR 0008) |
| Baseline artifact | `BaselineRef.digest` (`status_vitality.rs`) | *(none — digest shape-checked only)* |

Every one of these diagnostics fires unconditionally because **no registry
exists**: there is nothing a declaration can be resolved *against*, so a
perfectly well-formed evaluator/climate/world-law declaration can never succeed.
The audit (this session) also surfaced two **silent gaps**: a magic-bridge module
is never flagged, and a baseline digest is never resolved — both are paths that
could later behave as if backed when they are not.

This ADR adds the missing resolution layer: a **compile-time module registry**
that answers, for each reference, *"is there a registered, correctly-typed,
content-verified implementation?"* It resolves and verifies; it never executes.

## Decision

Introduce a **`ModuleRegistry`** owned by `hpdl-compiler`, consulted by the single
validator in `hpdl-sema` through a small trait so sema gains no compiler
dependency. Scope is the **full set** (all six reference kinds above). The
registry verifies exactly three things and nothing more:

1. **Existence** — the exact `name@version` (or baseline `digest`) is registered.
   A mutable label is never resolvable (reuse `PackageId`'s exact-version rule).
2. **Kind** — the registered entry's declared `ModuleKind` matches the kind the
   site requires (an evaluator ref must resolve to an `Evaluator`, a climate
   model to a `ClimateModel`, a world law to a `WorldLaw`, a bridge to a
   `MagicBridge`, a body module to a `Library`). A kind mismatch is a typed error,
   so a climate model cannot masquerade as an evaluator.
3. **Integrity** — the entry carries a content `Digest` (`hpdl_model::Digest`,
   SHA-256); resolution compares the reference's pinned digest, when present,
   against the registered digest and rejects a mismatch. The registry never
   fetches or hashes bytes itself (reuses `verify_integrity`'s pure comparison).

### New types

- **`hpdl-model`:** `ModuleKind` enum (`Evaluator`, `ClimateModel`, `WorldLaw`,
  `MagicBridge`, `Library`) — pure declaration vocabulary. `RegisteredModule`
  descriptor: `{ id: PackageId, digest: Digest, kind: ModuleKind }`. These are
  source-independent declarations; they resolve nothing.
- **`hpdl-sema`:** a `ModuleResolver` trait (`resolve(name, version, expected_kind)
  -> Result<(), ModuleResolution>`) and a `ResolverError` describing not-found /
  wrong-kind / integrity-mismatch. The validator takes an `Option<&dyn
  ModuleResolver>`. **Absent resolver = today's behavior exactly** (flag every
  reference as non-executable). This preserves the "no registry ⇒ non-executable"
  default and keeps `hpdl-sema → hpdl-model` the only edge.
- **`hpdl-compiler`:** `ModuleRegistry` implementing `ModuleResolver`, built from
  a set of `RegisteredModule`s (and reusing the `Lockfile` for library modules).
  Deterministic, offline, fail-closed. `CompileOptions` gains an optional registry.

### Behavior change at the flags

With a registry supplied, each `Unsupported*` site becomes a real lookup:
- registered + kind matches + digest verifies → **no diagnostic** (declaration is backed);
- not registered / wrong kind / digest mismatch → the **same** diagnostic code,
  now a genuine resolution failure.

The magic-bridge gap is closed with a **new code `E6006 UnsupportedMagicBridge`**;
the baseline gap is closed by resolving `BaselineRef.digest` against the registry's
content entries (reusing `E9202`'s series intent via a new `E9203
UnsupportedBaseline`). Both are errors.

### Severity normalization (resolves the audit's inconsistency)

`E6005 UnsupportedWorldLaw` becomes an **error** (was a warning) **only in the
sense that an unresolved reference now means "not registered," which is a true
failure.** The warning existed solely because resolution could never succeed;
with a registry it can, so the remaining failure is a real error — matching
E9202/E9103. This supersedes ADR 0005 OD-11.4's warning choice. When **no
resolver is supplied**, world laws remain a non-blocking warning (unchanged
default), so existing definitions that declare laws still compile without a
registry.

## Rationale

- A registry is the correct fix for "no registered implementation," not a
  severity edit: it lets a declaration *succeed* and makes failure a true error.
- Verifying existence + kind + integrity — and nothing else — is the maximal
  honest guarantee available without a trust model (see Deferred) and without
  executing anything. It directly satisfies the rust-standards rule "verify
  content hashes before parsing packages" and the architecture rule "unsupported
  modules yield explicit validation/activation errors."
- Threading the registry as a trait *into* the validator (rather than sema
  reaching out) keeps `hpdl-sema` the single validator with no forbidden edge,
  and keeps `hpdl-pir`/`hpdl-runtime-api` untouched.

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| Resolve only the four currently-flagged refs | Leaves the magic-bridge and baseline silent gaps the audit found; user chose the full set. |
| Put the registry in `hpdl-sema` | sema would gain resolution/possibly I/O concerns; violates "single validator, model-only dep." The trait keeps sema pure. |
| Make `E6005` an unconditional hard error now, no registry | Breaks authoring of every world law until a registry exists — hostile, and the real missing piece is resolution, not severity. |
| Verify authenticity / trust roots now | Explicitly deferred by ADR 0008 OD-14.4; deciding a trust model in code is forbidden by steering. |
| Resolve a custom orbital reference frame (E8004) via the registry | A custom `ReferenceFrame` is a frame *variant*, not a `ModuleRef`; it is not a registry lookup. Left as-is (see Deferred). |

## Compatibility impact

- **`hpdl-model`:** additive public types (`ModuleKind`, `RegisteredModule`). Pre-1.0.
- **`hpdl-sema`:** additive `ModuleResolver` trait + two new diagnostic codes
  (`E6006`, `E9203`); `Validator` gains an optional resolver (existing
  constructors keep working, resolver defaults to `None`). Existing diagnostics
  unchanged in meaning.
- **`hpdl-compiler`:** additive `ModuleRegistry`, `CompileOptions.registry`.
- **No PIR change.** The registry is a build input; canonical PIR bytes for a
  given definition are unchanged (`PirVersion::CURRENT` and golden vectors
  untouched). Asserted by a test that a definition hashes identically with and
  without a registry.

## Test consequences

- `hpdl-model`: `ModuleKind`/`RegisteredModule` construction + mutable-label rejection.
- `hpdl-sema`: for each of the six kinds — resolved→clean, not-found→flag,
  wrong-kind→flag, digest-mismatch→flag; and the no-resolver path reproduces
  today's behavior exactly (world-law warning, others error).
- `hpdl-compiler`: registry build + resolution integration; PIR-unchanged-with-registry.
- No golden PIR vector regeneration.

## Deferred — surfaced, NOT decided in code

- **Authenticity / trust roots (ADR 0008 OD-14.4).** This registry verifies
  *integrity* (digest match), *existence*, and *kind*. It does **not** verify
  *authenticity*: who signed a module, which registries/trust roots are trusted,
  and how roots are established/rotated. A clearly-marked seam is left; wiring any
  trust model requires a dedicated ADR and explicit approval.
- **Custom orbital reference frame (`E8004`).** Not a module reference; a custom
  `ReferenceFrame` stays a non-executable frame-variant flag, out of registry
  scope. If custom frames should ever be registry-backed, that is a separate,
  explicitly-designed change.
- **Execution.** The registry resolves contracts; the external HORIZON engine
  alone executes them (`Error::UnsupportedModule` remains the runtime-side shape).
