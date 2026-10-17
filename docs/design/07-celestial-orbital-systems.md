# Celestial and Orbital Systems — Technical Design

> **Status:** Proposed engineering design (not an assertion of implemented functionality).  
> **Scope:** HPDL v1 | Kiro feature `07-celestial-orbital-systems`  
> **Authority:** The HPDL v1 language definition and Kiro steering rules take precedence. Open decisions require explicit approval.

## 1. Purpose and boundaries

Body identities, physical properties, frames, Keplerian orbit declarations. This design covers the declared contracts and verifiable behavior; it does not assume a production implementation already exists.

**Primary crates:** `hpdl-model, hpdl-sema`.

## 2. Public data and interfaces

- `CelestialIdentity`
- `ParentRef`
- `ReferenceFrame`
- `KeplerianElements`
- `RotationModel`

Interface definitions should use Rust newtypes for identifiers and units where appropriate, `Result<T, Error>` for fallible operations, and stable public types only in published contract crates. Do not expose compiler AST or internal implementation structures to runtime consumers.

## 3. Processing and dependency design

1. Parse or deserialize the source into a frontend-neutral model where applicable.
2. Run the shared semantic validator; report structured diagnostics and fail on invalid declarations.
3. Resolve all versioned module/package references against the lockfile and approved registry.
4. Lower valid definitions into canonical PIR with explicit numeric representations.
5. Pass PIR and runtime contracts to the external HORIZON simulation, which alone executes state transitions.

Only the stages relevant to this feature should be implemented in its crate. Never introduce reverse dependencies from `hpdl-pir` or `hpdl-runtime-api` to `hpdl-compiler`.

## 4. Invariants and failure behavior

Validate parent graph acyclicity, unit dimensions, reference frame and eccentricity constraints; orbital declaration is not automatically an orbital solver

All errors must be explicit, typed where practical, and non-destructive. Unrecognized or unsupported executable declarations must fail rather than silently produce plausible-looking results. No fallback may change canonical output without a versioned specification.

## 5. Security and deterministic execution

- Validate untrusted lengths, nesting, references, and resource usage before allocating or executing.
- Do not use unordered iteration, locale-dependent formatting, implicit floating-point behavior, ambient clocks, or network state to determine canonical bytes.
- Bind behavior to versioned implementation identifiers; preserve historical hashes and compatible decoders.
- Reject ambiguous units, numeric overflows, incompatible schema versions, and incomplete authorization.
- Apply least-privilege capability checks for modules that read or write planetary state.

## 6. Verification and acceptance evidence

**Required test cases:** invalid parent cycle; reference mismatch; unit errors; supported and unsupported orbit model declarations.

Every relevant invariant requires positive, negative, and boundary cases. Store frozen fixtures under `tests/golden/` when the output is canonical or user-facing. Test the same definition and event sequence across Linux, macOS, and Windows wherever the contract promises cross-platform equivalence.

## 7. Integration contracts

- **Input:** Versioned source definitions, package locks, or PIR depending on the crate.
- **Output:** Validated model, canonical PIR, typed diagnostics, or deterministic runtime contract.
- **Compatibility:** SemVer for published Rust APIs; independently version PIR encoding, language grammar, package formats, and state schemas.
- **Observability:** Stable diagnostic codes, explicit source/version/hash references, and structured failure reporting.
- **Migration:** If state shape or canonical encoding changes, require an approved versioned migration or format bump; never rewrite committed history.

## 8. Open decisions / approvals required

- Confirm the exact Rust public API signatures and crate publication versions before stabilizing them.
- Confirm any feature-specific numerical precision, limits, and registry trust roots rather than guessing.
- Record unresolved choices in an RFC or architecture decision record, with explicit alternatives and compatibility consequences.

## 9. Definition of done

- The behavior above is implemented with unit and integration coverage.
- CI, security, compatibility, and relevant golden/replay tests pass.
- Public interfaces and failure cases are documented.
- No unsupported feature silently claims simulation or successful validation.
