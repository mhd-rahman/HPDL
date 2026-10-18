# ADR 0013: Release & Support Policy

**Status:** Accepted
**Date:** 2026-10-18
**Author:** HPDL Team
**Feature:** `00-workspace-foundation` (production-readiness)

## Summary

Ratifies HPDL's release and support policy: the versioning model for its three
independent version surfaces, the supported version windows that take effect at
`1.0`, and the PIR format-version deprecation policy. The full prose policy lives
in [`docs/RELEASE_POLICY.md`](../RELEASE_POLICY.md); this ADR fixes the binding
numbers and rules that the doc describes, so they are a ratified decision rather
than an un-enforced suggestion.

The CVE/disclosure process is **not** re-decided here — it remains owned by
[`SECURITY.md`](../../SECURITY.md). This ADR only binds how disclosure interacts
with support windows (fix current, backport within-window only).

## Context

`SECURITY.md` previously documented only that the latest commit on `main` is
supported during pre-1.0 development, and nothing about what happens at and after
`1.0`. Production consumers of the published contract crates (`hpdl-pir`,
`hpdl-runtime-api`) and of persisted, hashed PIR artifacts need to know, before
they depend on HPDL:

- how long a given crate release keeps receiving fixes;
- how long a given PIR format major stays readable;
- what signals a deprecation and how much runway precedes a removal.

HPDL has three separately versioned surfaces (crate SemVer, PIR `MAJOR.MINOR` via
`hpdl_pir::PirVersion::CURRENT`, native grammar `MAJOR.MINOR` via ADR 0010). A
policy must not conflate them — in particular, a crate release is never on its own
a licence to change canonical PIR bytes (determinism steering; ADR 0003).

Everything is currently pre-1.0 (`hpdl-pir` `0.2.0`, the rest `0.1.0`). The windows
below are ratified now for planning but take effect at the first `1.0` of each
surface; they do not retroactively bind pre-1.0 versions, where `main`-only support
stands.

## Decision

1. **Adopt `docs/RELEASE_POLICY.md` as the governing policy**, with the binding
   values fixed here:

   - **Crate API windows (at 1.0):** current major/latest minor — full support;
     current major/previous minor — security + critical-correctness fixes until the
     next minor supersedes it; previous major — security fixes only for **two**
     subsequent minor releases of the new major, then EOL; older majors —
     unsupported.
   - **PIR format windows:** the current `hpdl-pir` decoder reads **every minor of
     the current major** (`is_compatible`: same major, encoded minor ≤ decoder
     minor); minors are additive and are not independently deprecated. **At most one
     previous major** is supported, decode-only, for a **read-only window of two
     `hpdl-pir` minor releases** after the new major ships, after which old-major
     bytes are rejected with `Error::IncompatibleVersion`. Encoding always targets
     `PirVersion::CURRENT`.

2. **Ratify the PIR format-version deprecation rules:** minor bump = strictly
   additive/backward-compatible; major bump = any canonical byte-layout change
   (field order, integer encoding, optional encoding, map-key sort, string
   normalization, numeric representation, duplicate/unknown-field handling), with
   the hash domain separator bound to the major version. Every bump requires, in one
   reviewed change: an ADR, a deliberate golden-vector update (never a silent
   regeneration), a CHANGELOG entry, and — for a major — a stated migration path and
   the start of the prior major's read-only window. (Consistent with ADR 0003.)

3. **Precedence:** safety and deterministic-replay invariants outrank support
   windows. A support obligation never forces a fix that would change the canonical
   bytes of a format version still in its window; such a change goes through a
   format-version decision instead.

4. **Deprecation signalling:** API removals are preceded by `#[deprecated]` (naming
   the removal version and replacement) for at least one minor; PIR major
   deprecations are announced in the CHANGELOG and ADR when the superseding major
   ships; grammar deprecations follow ADR 0010's own `MAJOR.MINOR` contract.

5. **Windows are release-relative, not calendar-dated.** Consistent with the product
   rule against inventing deadlines/schedules, the policy commits to "how many
   releases back" rather than fixed EOL dates.

## Rationale

Release-relative windows are honest for a pre-1.0 project with no fixed cadence: we
can commit to a bounded migration runway (two minors) without fabricating dates we
cannot guarantee. Binding the PIR decoder window to the format's own compatibility
rule (`is_compatible`) keeps the policy aligned with code that already exists rather
than inventing a parallel scheme. Keeping disclosure in `SECURITY.md` avoids two
sources of truth for the same process.

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| Calendar-dated EOLs (e.g. "18 months") | Invents a schedule the project hasn't committed to; violates the product rule against inventing deadlines. Release-relative windows give the same runway guarantee without fake dates. |
| Support all previous PIR majors indefinitely (decode-only) | Unbounded decoder-compatibility burden; every old major must keep passing frozen vectors forever. One previous major with a bounded read-only window is enough for migration. |
| Fold the whole policy into `SECURITY.md` | Overloads a disclosure document with release mechanics; the two concerns version and change at different cadences. Split with cross-links instead. |
| Independently deprecate PIR minors | Breaks the additive guarantee — a current-major decoder is required to read all earlier minors. Only majors are ever removed. |
| Leave the policy as un-ratified prose | Leaves the binding numbers as a silent suggestion; the workflow requires ratifying decisions as ADRs with rationale, alternatives, and compatibility impact. |

## Compatibility impact

- **No code or format change.** `PirVersion::CURRENT` stays `2.0`; no crate version
  changes; no canonical bytes change; no golden vector is touched. This ADR is
  documentation/governance only.
- **SemVer:** none. The contract crates' public APIs are unchanged, so
  `cargo semver-checks` is unaffected.
- **Forward effect:** future format and API changes must now cite this policy (and,
  for format majors, follow the ADR-0003-style bump procedure it ratifies).

## Test consequences

- No new unit/integration tests: this ADR adds no behavior and changes no bytes.
- Existing gates must remain green with the added docs: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `cargo deny check`. Markdown-only changes do not affect them, but they are the
  definition-of-done check for this change.
- The frozen PIR golden vectors are deliberately **unchanged**, which is itself the
  correct outcome for a governance-only ADR.
