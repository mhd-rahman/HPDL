---
inclusion: always
---

# HPDL — Architecture

## Repository policy
One `horizon-hpdl` Git repo, one Cargo workspace. The simulation engine, validator network, world content and clients live in separate repos and consume only published contract crates.

## Crate ownership
| Crate | Owns | Must NOT |
|---|---|---|
| `hpdl-model` | Source-independent typed domain model, migration *declarations*, module references | Parse source, encode PIR |
| `hpdl-sema` | The ONE semantic validator: constraints, dimensions, statuses, world-law checks | Be duplicated per frontend |
| `hpdl-ron` | RON input adapter → model | Contain semantic rules |
| `hpdl-pir` | Published PIR types, canonical encode/decode, strict decoder, domain-separated hashing | Depend on `hpdl-compiler`, `hpdl-sema`, or exact-decimal code |
| `hpdl-compiler` | Orchestration, package resolution, unit normalization, fixed-point lowering, PIR emission | — |
| `hpdl-cli` | Commands and diagnostic presentation | Contain logic |
| `hpdl-runtime-api` | Published state/module/event/migration-execution contracts | Depend on parser or compiler internals |
| `hpdl-syntax` | Native `.hpdl` lexer/parser/AST (later) | — |
| `hpdl-lsp` | Deferred until useful | — |

## Dependency direction
```
hpdl-cli → hpdl-compiler → { hpdl-ron, hpdl-syntax } → hpdl-model
                         → hpdl-sema → hpdl-model
                         → hpdl-pir   (leaf)
hpdl-runtime-api → hpdl-pir (+ selected stable model types)
```
No cycles. Keep public APIs small and independently versionable.

## Canonical pipeline
Source (HPDL or RON) → decode → model → **one** semantic validator → exact-unit normalization → fixed-point lowering → canonical PIR bytes → hash → runtime.
Equivalent semantics MUST yield identical PIR bytes regardless of frontend.

## Standard library
Immutable, content-addressed, independently versioned packages — not tied to compiler releases. `hpdl.lock` pins version, content hash and transitive closure. Never resolve a mutable branch or label.

## Contracts vs implementations
HPDL defines module contracts; runtime modules are separate versioned implementations. Unsupported modules yield explicit validation/activation errors.
