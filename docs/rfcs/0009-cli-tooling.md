# 0009 — HPDL CLI (`hpdl`) Tooling

**Status:** Accepted
**Date:** 2026-10-17
**Feature:** `15-cli-tooling` (OD-15.1 … OD-15.4)

## Context

The architecture steering assigns `hpdl-cli` "commands and diagnostic presentation" and states
it "must NOT contain logic". Feature 06 exposed everything the CLI needs: a `Compiler` entry
point, `CompileOptions` (with provisional and authoritative-package lockfiles), a
`CompileResult` (PIR envelope + build manifest), and a typed `Error` enum re-exporting the
stable `hpdl-sema` diagnostic types. The CLI crate shipped only a placeholder `main` that
exited 1.

## Decision

Implement the `hpdl` binary with `check`, `build`, `inspect`, and `export` subcommands (plus
`--help`/`--version`), structured diagnostic presentation in text and JSON, and stable exit
codes — all delegating compilation to `hpdl-compiler`. The crate gains a small library
(`hpdl_cli`) exposing a pure, testable `dispatch(args) -> Outcome`; `main` is a thin wrapper.

- **OD-15.1 — Hand-rolled arg parser, no `clap`.** The command surface is small and the
  workspace has no `clap` dependency; a hand-rolled parser keeps the crate dependency-light
  (rust-standards: minimal dependencies) and fully deterministic. Adopting `clap` later is a
  separately-reviewed step.
- **OD-15.2 — Diagnostic formats: `text` (default) + `json`.** Text reuses the `Diagnostic`
  `Display`; JSON is a hand-built deterministic object with a local string-escape helper, so no
  `serde_json` runtime dependency is needed for the small fixed shape.
- **OD-15.3 — Exit codes: `0` ok, `1` compile/diagnostic failure, `2` usage/IO error.** Stable
  and documented in `--help` and this ADR; part of the CLI's contract.
- **OD-15.4 — RON frontend diagnostics rendered as the compiler surfaces them.** The compiler
  flattens RON parse errors into `Error::Frontend { name, reason }` (a string); the CLI renders
  that string. Recovering RON spans/excerpts through the compiler is a future compiler-side
  change, not a CLI concern.

## Rationale

The implementation keeps `hpdl-cli` a thin presentation layer: it reads files, builds
`CompileOptions`, calls the compiler, and formats the result/diagnostics into a buffered
`Outcome`. No compilation logic, no simulation, no new third-party dependency. Output is
deterministic (sorted diagnostics; the compiler's stable manifest/PIR bytes), exit codes are
stable, and no path panics on untrusted input or a missing/non-UTF-8 file.

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| Use `clap` for arg parsing | Adds a third-party dependency for a tiny command surface; against the minimal-dependency standard. |
| Use `serde_json` for JSON output | Unnecessary runtime dependency for a small fixed JSON shape; a local escape helper suffices. |
| Keep all code in `main.rs` | A bin-only crate can't be imported by integration tests; a thin `lib` with a pure `dispatch` makes dispatch testable without spawning a process. |
| Put formatting helpers in `hpdl-compiler` | Presentation belongs to the CLI; the compiler already exposes stable diagnostic/manifest types. |

## Compatibility impact

- `hpdl-cli` is `publish = false`; it gains a `[lib]` target (`hpdl_cli`) and the `hpdl` binary
  is unchanged in name/path. No contract-crate change; no PIR change; no new third-party
  runtime dependency.
- Exit codes `0`/`1`/`2` are the CLI's stability surface, documented in `--help`.

## Test consequences

- Unit tests in `cli.rs` (arg parsing: subcommands, flags, usage errors) and `render.rs` (JSON
  escaping, error rendering).
- Integration tests in `tests/cli.rs` via `dispatch`: help/version, unknown
  subcommand/flag/missing-arg → exit 2, unreadable file → exit 2, `check` ok/fail, `build`
  text + JSON, `inspect` summary, `export` to file + hex stdout, invalid lock → exit 2, a
  golden `inspect` fixture (stable lines), and one binary smoke test spawning `hpdl`.
- No golden PIR vector changes; the CLI emits the compiler's canonical bytes unchanged.
