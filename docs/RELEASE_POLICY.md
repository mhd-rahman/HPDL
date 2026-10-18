# HPDL — Release & Support Policy

This document defines how HPDL is versioned, which releases receive fixes, and
how PIR format versions are deprecated. It is the production-facing complement to
[`SECURITY.md`](../SECURITY.md), which owns the vulnerability disclosure process.

The binding windows and rules below are ratified in
[ADR 0013](rfcs/0013-release-support-policy.md).

Precedence note: safety and deterministic-replay invariants outrank everything
here. A support window never obliges us to ship a fix that would break canonical
PIR byte-for-byte reproducibility; such a fix goes through a format-version
decision instead (see "PIR format-version deprecation").

---

## Current status: pre-1.0

Every crate is below `1.0` (`hpdl-pir` is `0.2.0`; the rest are `0.1.0`). During
pre-1.0 the support surface is intentionally narrow:

- **Only the latest commit on `main` is supported.** There are no maintained
  release branches, no backports, and no guaranteed upgrade path between pre-1.0
  versions. This matches the table in `SECURITY.md`.
- Breaking changes may land on `main` at any time, including to the published
  contract crates and the PIR format, accompanied by an ADR and a CHANGELOG entry.
- The sections below define the policy that **takes effect at the first `1.0`
  release** of each surface. They are published now so downstream consumers can
  plan, but they do not retroactively bind pre-1.0 versions.

---

## Versioning model

HPDL has three independently versioned surfaces. Do not conflate them.

| Surface | Scheme | Source of truth |
|---|---|---|
| Crate APIs (`hpdl-pir`, `hpdl-runtime-api`, and the rest) | Cargo SemVer (`MAJOR.MINOR.PATCH`) | each crate's `Cargo.toml` |
| PIR canonical binary format | `MAJOR.MINOR` | `hpdl_pir::PirVersion::CURRENT` |
| Native `.hpdl` grammar | `MAJOR.MINOR` | ADR 0010 |

The published **contract crates** — `hpdl-pir` and `hpdl-runtime-api` — are held
to SemVer mechanically by the `cargo semver-checks` CI job. A breaking change to
either requires a major bump and an ADR.

A crate version bump and a PIR format bump are **separate events**. Releasing a new
`hpdl-pir` crate does *not* by itself change the format version, and the canonical
bytes must not change without a deliberate format-version decision (see
"Determinism" steering: a crate release alone is never sufficient to change
golden vectors).

---

## Supported version windows (effective at 1.0)

Windows are expressed **relative to releases**, not calendar dates. We do not
commit to fixed-date EOLs; we commit to how many releases back remain supported.

### Crate APIs

- **Current major, latest minor** — full support: features, bug fixes, and
  security fixes.
- **Current major, previous minor** — security and critical-correctness fixes
  only, until the next minor supersedes it.
- **Previous major** — security fixes only, for **two** subsequent minor releases
  of the new major, then end-of-life. This gives consumers a bounded but real
  migration window across a breaking API change.
- **Older majors** — unsupported.

"Critical-correctness" means a defect that produces wrong PIR, breaks
deterministic replay, or corrupts state — triaged at the same priority as a
security fix.

### PIR format versions

Because the PIR is a persisted, hashed, replayable artifact, its support window is
stricter and decoder-centric:

- The decoder in the current `hpdl-pir` **must read every minor version within the
  current major** (`is_compatible`: same major, encoded minor ≤ decoder minor).
  Minor versions are additive and are **not** independently deprecated — they
  remain readable for the life of their major.
- **At most one previous major** may be supported at a time, and only for
  decoding (reading), to allow migration of stored artifacts. Encoding always
  targets `PirVersion::CURRENT`.
- When a new PIR major is introduced, the prior major enters a **read-only
  deprecation window of two crate minor releases** of `hpdl-pir`, during which a
  documented migration path to the new major exists. After that window the old
  major is rejected at decode time with `Error::IncompatibleVersion`.

---

## PIR format-version deprecation policy

The current ratified PIR format is **v2.0** (locked by
[ADR 0014](rfcs/0014-pir-v2-ratified.md)); the "fold into unreleased v2.0"
allowance from ADR 0003 is closed, so the next canonical-byte change is a v3.0
major bump under the rules below.

The PIR format version increments under exactly these rules; all are enforced by
frozen golden vectors under `crates/hpdl-pir/tests/golden/`.

1. **Minor bump (`x.y` → `x.(y+1)`)** — strictly additive, backward-compatible
   changes: new optional fields that older-minor encoders never produced. A
   current-major decoder reads all earlier minors unchanged. No migration needed.
2. **Major bump (`x.*` → `(x+1).0`)** — any change to field order, integer
   encoding, optional-value encoding, map-key sorting, string normalization,
   numeric representation, or duplicate/unknown-field handling. The byte layout
   changes, so a decoder of the new major **must reject** the old major's bytes
   unless an explicit migration is provided. The hash domain separator is bound to
   the major version, so v(N) and v(N+1) of the same semantic value hash
   differently by construction.

Every bump requires, in one reviewed change:

- an **ADR** in `docs/rfcs/` recording the format change, rationale, alternatives,
  and compatibility impact (precedent: ADR 0003 introduced PIR v2.0);
- a **deliberate golden-vector update** — never a silent regeneration to make tests
  pass;
- a CHANGELOG entry under the `hpdl-pir` release;
- for a major bump, a stated **migration path** and the start of the previous
  major's read-only deprecation window.

Deterministic-replay invariant: the same locked inputs + compiler semantics
version + format version must yield byte-identical PIR across Linux, macOS, and
Windows. Deprecating a format version never changes the bytes of a version still
in its window.

---

## Deprecation signalling

- API items slated for removal are marked `#[deprecated]` with the target removal
  version and the replacement, for at least one minor release before removal.
- PIR major deprecations are announced in the CHANGELOG and the relevant ADR at the
  moment the superseding major ships, stating the read-only window and the removal
  release.
- Grammar deprecations follow ADR 0010's own `MAJOR.MINOR` contract (additive →
  minor, breaking → major) and are called out in the CHANGELOG.

---

## CVE / vulnerability disclosure

The disclosure process, acknowledgement and fix-timeline targets, scope, and
coordinated-disclosure / CVE-assignment commitments live in
[`SECURITY.md`](../SECURITY.md) and are not duplicated here. In summary:

- Report privately by email; do not open a public issue.
- Acknowledgement within 5 business days; target fix or mitigation within 30 days
  for confirmed high-severity issues.
- Coordinated disclosure: a fix ships before full public disclosure, and CVE
  assignment is requested for confirmed vulnerabilities.

Interaction with support windows: a confirmed vulnerability is fixed on the current
supported release first, then backported to any other release still inside its
support window per the tables above. Releases outside their window do not receive
backports; the remediation is to upgrade.
