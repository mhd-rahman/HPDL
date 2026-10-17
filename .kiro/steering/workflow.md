---
inclusion: always
---

# HPDL — Workflow

## Current state (update this section as crates land)
| Crate | Status |
|---|---|
| `hpdl-model` | active (feature 01) |
| `hpdl-sema` | active (feature 03) |
| `hpdl-ron` | active (feature 04) |
| `hpdl-pir` | active (feature 02) |
| `hpdl-compiler` | active (features 06, 14) — pipeline + authoritative package resolution |
| `hpdl-cli` | active (feature 15) — check/build/inspect/export, diagnostic presentation |
| `hpdl-runtime-api` | active (features 10, 12, 13) — runtime + migration contracts; executor is external (mocked in tests) |
| `hpdl-syntax` | active (feature 05) |
| `hpdl-lsp` | deferred |

Do NOT scaffold crates marked deferred unless the task explicitly asks.

## Commands (run before declaring any task done)
```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```
Published contract crates additionally: `cargo semver-checks -p hpdl-pir -p hpdl-runtime-api`.

## Working agreement
1. Read the relevant steering and existing crates before coding.
2. State dependency, versioning and PIR-format implications up front.
3. Surface unresolved design decisions; do not resolve them silently in code.
4. Make minimal coherent changes. Never replace HPDL with RON or CUE.

## Adding a domain
Model structs → shared validation in `hpdl-sema` → PIR mapping + canonical impact → runtime contract (if executable) → explicit unsupported-simulation behavior → tests and an example. RON and native HPDL must share identical semantics.

## Changing published formats or APIs
Assess SemVer and PIR format-version impact, update golden vectors deliberately, document compatibility, preserve artifacts needed for replay.

## Precedence when sources conflict
1. Safety and deterministic-replay invariants
2. Approved HPDL spec + normative amendments (amendments win over the base spec)
3. Steering files
4. Current feature design / illustrative examples

If a conflict remains, stop and surface it. Record decisions as ADRs in `docs/rfcs/` (status, rationale, alternatives, compatibility impact, test consequences).

## Definition of done
- Behavior, failure modes, compatibility impact documented
- Unit + negative tests added; golden vectors updated only when intended
- Malformed input never panics or does unbounded work
- Docs/examples match implemented behavior; unsupported capabilities labeled
- Never claim unimplemented syntax or simulation works. Never invent passing test results.
