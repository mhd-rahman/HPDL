# HPDL CLI (`hpdl`) — Requirements

## Overview

This feature covers **domain-15 work: the developer-facing command-line tool**. It implements
the `hpdl` binary (crate `hpdl-cli`) with `check`, `build`, `inspect`, and `export` subcommands,
structured diagnostic presentation with stable exit codes, and an optional machine-readable
(JSON) diagnostic format. Per the architecture steering, **`hpdl-cli` owns commands and
diagnostic presentation and must NOT contain logic** — all compilation delegates to
`hpdl-compiler`.

The compiler (feature 06) already exposes everything the CLI needs: a `Compiler` entry point,
`CompileOptions` (with optional provisional and authoritative package lockfiles), a
`CompileResult` (PIR envelope + build manifest), and a typed `Error` enum re-exporting the
stable `hpdl-sema` diagnostic types (`Diagnostic`, `DiagnosticCode`, `Severity`, `SourceSpan`)
with their stable codes.

Primary crate: **`hpdl-cli`**. No change to `hpdl-compiler` or any contract crate.

> **Critical framing:** The CLI reads source/lock files, dispatches to the compiler, and
> presents results and diagnostics. It performs no compilation logic, no simulation, and no
> state execution. It never pretends an unsupported command or an unimplemented capability
> succeeded; an unknown subcommand or flag fails with a non-zero exit code and a usage message.

## Goals

1. **Subcommand dispatch**: a hand-rolled, dependency-light arg parser dispatching `check`,
   `build`, `inspect`, `export`, plus `--help`/`-h` and `--version`/`-V`. Keep the crate's
   dependencies minimal (no new third-party arg parser).
2. **`check`**: compile a `.ron` source, report success or render diagnostics; exit code
   reflects success/failure.
3. **`build`**: compile and emit the deterministic build manifest (and content hash) for a
   source, optionally using a lockfile; exit code reflects success/failure.
4. **`inspect`**: print a human-readable summary of a successful compile — PIR version, content
   hash, resolved-module closure — from the build manifest.
5. **`export`**: write the canonical PIR bytes of a successful compile to a file (or stdout),
   so the artifact can be handed to the external engine.
6. **Diagnostic presentation**: render `hpdl-sema` diagnostics (code, severity, message, span,
   notes) deterministically; support a `--format json` option for machine-readable output.
7. **Stable exit codes**: documented, stable exit codes distinguishing success, a usage error,
   and a compilation/diagnostic failure.

## Scope boundaries

- **No compilation logic in the CLI.** All model/validate/resolve/lower/emit work is in
  `hpdl-compiler`; the CLI only reads files, builds `CompileOptions`, calls `Compiler`, and
  formats the result/diagnostics.
- **No simulation / state execution.** The CLI never runs an engine, process, or migration; a
  `test`/`run` style command is out of scope here (the design lists `test`/`fmt` as future).
- **No native `.hpdl` parsing.** Only RON source is compiled (the native parser is feature 05);
  the CLI reads `.ron` (or any text) and calls `compile_ron_str`.
- **No PIR / format change.** The CLI emits the compiler's canonical PIR bytes unchanged; it
  introduces no new canonical bytes.
- **No network / registry access.** File I/O only; resolution is offline (feature 14).

## Functional Requirements

### FR1: Arg parsing & dispatch

**FR1.1**: `hpdl <subcommand> [args]` dispatches to `check`, `build`, `inspect`, or `export`.
`hpdl` with no subcommand, `hpdl --help`/`-h`, and an unknown subcommand print usage; `hpdl
--version`/`-V` prints the crate version.

**FR1.2**: Unknown flags or missing required arguments produce a clear usage error on stderr and
a usage exit code (never a panic, never a silent default).

**FR1.3**: Arg parsing is hand-rolled (no new third-party dependency); the crate's strict lints
(`forbid(unsafe_code)`, no `unwrap`/`expect`/indexing on input paths) are honored.

### FR2: `check <file>`

**FR2.1**: Read the file as UTF-8, call `Compiler::compile_ron_str(name, source)`. On success,
print a concise success line; on failure, render the error/diagnostics (FR6) and exit non-zero.

### FR3: `build <file> [--lock <lockfile>] [--format text|json]`

**FR3.1**: Compile the source (optionally loading an authoritative `Lockfile` via `--lock`). On
success, print the build manifest's stable text (or JSON summary with `--format json`) including
the content hash. On failure, render diagnostics and exit non-zero.

### FR4: `inspect <file> [--lock <lockfile>]`

**FR4.1**: Compile the source and print a human-readable summary from the build manifest: source
name, compiler-semantics version, PIR version, content hash, and the resolved-module closure
(name, version, content hash), each on its own line, deterministically ordered.

### FR5: `export <file> [--lock <lockfile>] [--out <path>]`

**FR5.1**: Compile the source and write the canonical PIR payload bytes to `--out <path>` (or to
stdout as hex when no path is given). The bytes are the compiler's canonical PIR, unchanged.

### FR6: Diagnostic presentation & exit codes

**FR6.1**: Render each compiler `Error` variant actionably. For `Error::Validation(diags)`,
render every `Diagnostic` deterministically (sorted) using its code, severity, message, span,
and notes. Other variants render their `Display` message.

**FR6.2**: Support `--format text` (default) and `--format json`. JSON output is a
deterministic, machine-readable object (array of diagnostics with code/severity/message/span,
or a single error object) suitable for tooling.

**FR6.3**: Exit codes are stable and documented: `0` success; `1` compilation/diagnostic
failure; `2` usage error (unknown subcommand/flag, missing argument, unreadable file). No panic
path produces an exit code.

### FR7: Determinism & security

**FR7.1**: Output is deterministic — diagnostics sorted, manifest text already stable, no
hashmap iteration, no wall-clock/locale in output. The same inputs produce the same output and
exit code on every platform.

**FR7.2**: Treat file contents as untrusted; rely on the compiler's size/structure limits; never
panic on malformed input, a missing file, or a non-UTF-8 file — fail closed with a usage/IO
error and a non-zero exit code.

## Non-Functional Requirements

### NFR1 — Thin crate: `hpdl-cli` contains only arg parsing, file I/O, dispatch, and
presentation; no compilation logic. It depends on `hpdl-compiler` (and `thiserror`); no new
third-party arg parser.

### NFR2 — Determinism: identical inputs ⇒ identical stdout/stderr/exit code cross-platform;
diagnostics are sorted; manifest/PIR bytes come from the compiler unchanged.

### NFR3 — Security: no panics on untrusted input/missing files; stable exit codes; no network.

### NFR4 — Documentation: rustdoc on public items (CLI is `publish = false`, but modules carry
docs); `--help` text documents every subcommand and flag; CHANGELOG updated.

### NFR5 — Tests: integration tests invoking the built `hpdl` binary (or a testable dispatch
entry point) covering: `check` success/failure, `build` manifest output, `inspect` summary,
`export` bytes, unknown subcommand/flag usage error + exit code, `--help`/`--version`,
`--format json`, and a frozen golden CLI output fixture.

## Constraints

- `hpdl-cli → hpdl-compiler` only (plus `thiserror`); no direct `hpdl-pir`/`hpdl-sema`/`hpdl-ron`
  dependency (the compiler re-exports the diagnostic + envelope types the CLI needs). No logic
  in the CLI. No new third-party arg parser.
- No simulation / state execution; no native `.hpdl` parsing; no PIR change; no network.

## Dependency & versioning implications (surfaced up front)

- **No new runtime dependency** beyond what `hpdl-cli` already declares (`hpdl-compiler`,
  `thiserror`). Arg parsing is hand-rolled, matching the project's dependency-light style
  (OD-15.1). A `tests`-only dependency may be added for binary-invocation integration tests.
- **No PIR / format change.** The CLI emits the compiler's canonical bytes unchanged.
- **Exit codes are a stability surface.** The documented codes (`0`/`1`/`2`) are part of the
  CLI's contract and change only deliberately.

## Open decisions / approvals required — resolved below (ADR 0009)

- **OD-15.1 — Arg parser — RESOLVED: hand-rolled, no `clap`.** The command surface is small and
  the workspace has no `clap` dependency; a hand-rolled parser keeps the crate dependency-light
  (rust-standards: minimal dependencies) and the behavior fully deterministic. If the surface
  grows substantially, adopting `clap` is a future, separately-reviewed step.
- **OD-15.2 — Diagnostic formats — RESOLVED: `text` (default) + `json`.** Text reuses the
  `Diagnostic` `Display`; JSON is a hand-serialized deterministic object (no `serde_json`
  runtime dependency needed for the small, fixed shape).
- **OD-15.3 — Exit codes — RESOLVED: `0` ok, `1` compile/diagnostic failure, `2` usage/IO
  error.** Stable and documented in `--help` and the ADR.
- **OD-15.4 — RON frontend diagnostics — RESOLVED (scope): rendered as the compiler surfaces
  them.** The compiler flattens RON parse errors into `Error::Frontend { name, reason }` (a
  string); the CLI renders that string. Recovering RON spans/excerpts through the compiler is a
  future compiler-side change, not a CLI concern.

## Success Criteria

1. ✅ `hpdl` dispatches `check`, `build`, `inspect`, `export`, `--help`/`-h`, `--version`/`-V`;
   an unknown subcommand/flag or missing arg is a usage error with exit code `2`.
2. ✅ `check`/`build`/`inspect`/`export` delegate entirely to `hpdl-compiler`; the CLI contains
   no compilation logic.
3. ✅ Diagnostics render deterministically in `text` and `json`; exit codes are `0`/`1`/`2` as
   documented; no panic path.
4. ✅ `export` writes the compiler's canonical PIR bytes unchanged; `inspect`/`build` print the
   stable manifest; the same inputs ⇒ the same output cross-platform.
5. ✅ Integration tests + a golden CLI output fixture pass; `--help`/`--version` covered.
6. ✅ CI passes; CHANGELOG + ADR 0009 updated; no new third-party arg-parser dependency.

## Out of Scope

- Any compilation logic in the CLI; simulation / state execution; a `run`/`test`/`fmt` command
  (future).
- Native `.hpdl` parsing (feature 05); recovering RON spans through the compiler.
- Any PIR / canonical-bytes change; network / registry access.

## References

- Steering: `architecture.md` (`hpdl-cli` owns commands + diagnostic presentation; must NOT
  contain logic; dependency direction), `rust-standards.md` (minimal dependencies; no panics on
  untrusted input), `determinism.md` (deterministic output; no hashmap iteration),
  `testing-ci.md` (CLI golden output; invalid-input exit codes).
- Builds on: feature 06 (`Compiler`, `CompileOptions`, `CompileResult`, `Error`, re-exported
  `Diagnostic`/`Severity`/`PirEnvelope`) and feature 14 (`Lockfile`, `--lock`).
- Design: `docs/design/15-cli-tooling.md`.
