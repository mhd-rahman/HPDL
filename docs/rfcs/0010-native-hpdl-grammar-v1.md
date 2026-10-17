# 0010 — Native HPDL Grammar v1.0 (ratification)

**Status:** Accepted
**Date:** 2026-10-18
**Feature:** `05-hpdl-language-parser` (grammar ratification, scope A2 — full RON parity)

## Context

The native `.hpdl` frontend (`hpdl-syntax`) shipped with an explicitly
**unratified** grammar: `GRAMMAR.md` was labelled "illustrative," and the AST→model
converter (`convert.rs`) wired only `id`, `physical`, and `traits`. `modules`,
`statuses`, `actions`, and `migrations` were stubs returning empty collections, and
the orbital/environment surface was never read. The RON frontend, by contrast,
reaches the entire serde-derived model surface for free.

This left native HPDL strictly less capable than RON and the cross-frontend
equivalence suite guaranteed sameness only over `{id, physical, traits}`. For the
language to be a production contract, equivalent semantics authored in `.hpdl` and
`.ron` must produce **identical `PlanetDefinition` models, identical validator
diagnostics, and identical canonical PIR bytes** (the determinism steering's
"equivalent semantics ⇒ identical PIR" invariant).

The lexer and parser were already general: the lexer emits `( )` tokens, and the
value grammar already accepts strings, integers/floats (with optional unit suffix),
booleans, lists, and nested objects with arbitrary `name: value` fields. The gap was
(a) a native spelling for closed-enum variants and newtypes — which RON gets for
free from serde (`Some(Planet)`, `Present(Microbial)`, `Custom("…")`,
`Capability("…")`) but the native AST had no form for — and (b) converter wiring for
the richer blocks.

## Decision

Ratify the native HPDL grammar at **grammar version `1.0`** (the string in
`language hpdl "1.0";`), scoped to **full parity with the serde-reachable model
surface**. Specifically:

1. **Add an identifier-variant value production** to the grammar and AST
   (`ValueNode::Variant { name, args }`), mirroring RON's enum/newtype syntax:
   - **Unit variant:** a bare capitalized/identifier value, e.g. `Planet`,
     `EclipticJ2000`, `Absent`, `Microbial`.
   - **Applied variant / newtype:** `Name( value, … )`, e.g. `Custom("arcane")`,
     `Present(Microbial)`, `Capability("state.atmosphere.write")`,
     `ModuleRef(name: "hpdl/stdlib/atmospheric", version: "1.0")`.
   - A variant's arguments are a comma-separated list of values (positional) **or**
     a field list (`name: value`) for struct-like variants, matching how the model
     types are shaped. This reuses the existing value and field-list productions;
     no new lexer tokens are required.

2. **Wire the converter for the full surface** so native HPDL can author everything
   RON can: `modules`, `statuses` (including the optional `dimension`, `baseline`,
   `evaluator`, `cut_points`, `window`), `actions` (including `reads`, `writes`,
   `capabilities`, with the `status.*` write prohibition enforced identically),
   `migrations`, and the optional `identity`, `parent`, `orbit`, `rotation`,
   `environment` fields.

3. **Resolve the recorded "Open Questions"** (`.kiro/specs/05-hpdl-language-parser/
   requirements.md`) as follows:
   - **Unit suffix:** optional whitespace between number and unit (`6371000 m` and
     `6371000m` both accepted). The unit is validated against the field's canonical
     unit and is advisory only: physical quantities MUST already be in canonical
     scaled-integer units; the unit suffix never triggers rescaling (no float path).
     A unit that contradicts the field's canonical unit is rejected, not silently
     coerced.
   - **String escapes:** `\n \t \r \\ \"` only (the implemented set). `\uXXXX` is
     **deferred** (reserved, rejected with a typed error) to avoid committing to a
     Unicode-escape normalization story that could affect canonical bytes.
   - **Number bases:** decimal only. Hex/binary/octal are **rejected** in v1.0
     (reserved). Decimal keeps one canonical textual form per value.
   - **Trailing commas:** optional (accepted but not required) in field lists,
     objects, lists, and variant argument lists. Already the implemented behavior.

4. **World laws are explicitly OUT of native grammar v1.0** (see Alternatives). This
   matches RON, which also cannot express `world_laws` because
   `CelestialBody.world_laws` is `#[serde(skip)]` (ADR 0005, OD-11.2). Bringing world
   laws into either serialized frontend is coupled to a PIR format-version decision
   and is deferred to a future grammar minor version.

## Rationale

- **Parity requires a variant form.** The model is full of closed enums
  (`CelestialIdentity`, `ReferenceFrame`, `StatusDimension`, `Biosphere`,
  `ComplexityTier`, `AssetKind`, `LawMode`) and validated newtypes (`Capability`,
  `GasId`, `Digest`, `BodyId`, `ModuleRef`). RON expresses these as serde variants;
  without a matching native form the two frontends could never produce identical
  models. Choosing the **RON-identical spelling** (`Variant(args)` /
  `Variant(field: value)`) minimizes author cognitive load and makes the equivalence
  tests a direct transcription.
- **No lexer change, no new tokens.** `( )` already lex; the variant production is a
  pure parser/AST addition. Smaller surface, less risk, and the resource limits
  (depth 64, collection 10 000, source 10 MiB) already bound it.
- **Scoping world laws out is the honest choice.** The determinism steering forbids
  silently changing canonical PIR bytes; the language steering forbids inventing
  syntax to fit examples. Since world laws touch neither PIR nor RON today, giving
  them native syntax now would either desynchronize the frontends or force an
  unreviewed PIR bump. Deferring keeps native HPDL and RON exactly equivalent.
- **Conservative lexical choices** (decimal only, limited escapes) preserve the
  "one semantic value, one canonical textual intent" property and avoid locking in
  normalization behavior we have not yet tested against canonical bytes.

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| Keep grammar "illustrative," ratify only `{id, physical, traits}` (scope A1) | Leaves native HPDL permanently weaker than RON; equivalence guarantee stays partial. User selected full parity (A2). |
| Invent a non-RON variant syntax (e.g. `kind = planet` keywords) | Would diverge from the serde surface, duplicate vocabulary, and make equivalence tests a translation layer rather than a transcription. Violates "do not invent syntax." |
| Include world laws in native v1.0 grammar | Requires un-skipping `world_laws` serialization and a PIR format-version decision (ADR 0005, OD-11.2). Out of scope; would break the RON/HPDL symmetry (RON can't express them either). |
| Support `\uXXXX`, hex/binary literals now | Adds canonical-form normalization questions (Unicode escapes, base canonicalization) with no current authoring need. Reserved for a later grammar minor version. |
| Rescale values from unit suffixes (`6371 km` → metres) | Introduces a conversion path and float risk on the canonical path. The model is canonical scaled-integer by contract; units stay advisory. |

## Compatibility impact

- **Grammar version:** this is the **first ratified** native grammar, version `1.0`.
  The `language hpdl "1.0";` declaration is now a committed contract for the
  constructs described here. Future additive syntax (e.g. world laws, `\uXXXX`) will
  be a grammar **minor** bump (`1.1`); a breaking change would be `2.0`.
- **Rust API (`hpdl-syntax`):** additive. `ValueNode` gains a `Variant` variant
  (this is a breaking change to any external code that exhaustively matches
  `ValueNode`, but `hpdl-syntax` is `publish = false` / pre-1.0, so no SemVer
  guarantee is in force). No public function signatures change.
- **PIR format:** **no change.** Native HPDL produces the same `PlanetDefinition`
  the RON path does, which lowers to the existing PIR v2.0 bytes. `PirVersion::CURRENT`
  stays `2.0`; no golden PIR vector changes. (World laws remain out of PIR.)
- **`hpdl-model` / `hpdl-sema`:** unchanged. Native HPDL routes through the identical
  shared validator; no new diagnostic codes.

## Test consequences

- **`hpdl-syntax` unit tests:** lexer/parser coverage for the variant production
  (unit, positional-newtype, struct-variant, nested) plus negative cases (reserved
  `\uXXXX`, hex literals, contradictory unit suffix).
- **Converter tests:** positive + negative + boundary extraction for every newly
  wired block (`modules`, `statuses` incl. optional vitality fields, `actions` incl.
  capabilities + status-write ban, `migrations`, `identity`, `parent`, `orbit`,
  `rotation`, `environment`).
- **Cross-frontend equivalence (`cross_frontend_tests.rs`):** the
  `shared_subset_boundary_is_documented` test — which previously asserted native
  HPDL extracts *zero* statuses — is replaced by positive parity cases asserting
  identical models and identical validator diagnostics for a comprehensive
  definition exercising all newly wired blocks. This is the honest signal the old
  test was designed to force once the converter grew.
- **Golden fixtures:** a frozen `.hpdl` ↔ `.ron` equivalence fixture pair proving
  byte-identical PIR (`tests/golden/`), per the design-doc requirement to store
  canonical/user-facing outputs under golden.
- **No golden PIR vector regeneration** in `hpdl-pir` / `hpdl-compiler`.

## Notes

- The grammar version string (`"1.0"`) is the **language** version, independent of
  the `hpdl-syntax` crate version and the PIR format version — three separate
  version lines, per the architecture steering.
- The native fuzz target `decode_hpdl` mentioned in `fuzz/README.md` becomes
  appropriate once this grammar is ratified; adding it is tracked as a follow-up
  (production-readiness roadmap, fuzzing step).
