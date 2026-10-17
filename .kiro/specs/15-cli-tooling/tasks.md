# HPDL CLI (`hpdl`) — Tasks

> Scope: implement the `hpdl` binary (check/build/inspect/export + help/version), diagnostic
> presentation (text + json), stable exit codes. No compilation logic in the CLI, no
> simulation, no native `.hpdl` parsing, no PIR change, no new third-party arg parser.
> OD-15.1…OD-15.4 resolved in ADR 0009.

- [x] 1. `exit.rs` — exit-code constants + `Outcome`
  - `OK=0`, `FAILURE=1`, `USAGE=2`; `Outcome { stdout, stderr, code }` helpers.
  - **Files:** `crates/hpdl-cli/src/exit.rs`.

- [x] 2. `cli.rs` — `Command`/`Format` + hand-rolled `parse` + usage/help/version text
  - Dispatch subcommands, positional file, `--lock`/`--out`/`--format` flags; unknown
    subcommand/flag/missing arg → `UsageError`. No `clap`. Unit tests.
  - **Files:** `crates/hpdl-cli/src/cli.rs`.

- [x] 3. `render.rs` — diagnostic + error + manifest/inspect presentation (text + json)
  - Sorted deterministic diagnostics; hand-built JSON with a local escape helper; manifest
    text reuse; labelled inspect summary. Unit tests.
  - **Files:** `crates/hpdl-cli/src/render.rs`.

- [x] 4. `run.rs` — per-subcommand run functions
  - Read file(s), build `CompileOptions` (`--lock` → `Lockfile`), call `Compiler`, format
    result/diagnostics into an `Outcome`. No compilation logic.
  - **Files:** `crates/hpdl-cli/src/run.rs`.

- [x] 5. `main.rs` — thin dispatch
  - `dispatch(args) -> Outcome` (testable) + `main` that prints and `process::exit(code)`.
  - **Files:** `crates/hpdl-cli/src/main.rs`.

- [x] 6. Integration + golden tests
  - `tests/cli.rs`: check ok/fail, build text+json, inspect, export (file + hex), unknown
    subcommand/flag/missing-arg → 2, help/version → 0, validation failure → 1; golden
    `tests/golden/inspect_minimal.txt`; one binary smoke test.
  - **Files:** `crates/hpdl-cli/tests/cli.rs`, `tests/golden/inspect_minimal.txt`, `Cargo.toml`.

- [x] 7. ADR 0009 + CHANGELOG + README index
  - Record OD-15.1…OD-15.4. CHANGELOG section. Update `docs/rfcs/README.md` index (0001–0009).
  - **Files:** `docs/rfcs/0009-cli-tooling.md`, `CHANGELOG.md`, `docs/rfcs/README.md`.

- [x] 8. Verify: fmt, clippy -D warnings, test --workspace.

## Out of scope (explicit)
- Compilation logic in the CLI; simulation / state execution; `run`/`test`/`fmt` command.
- Native `.hpdl` parsing; recovering RON spans through the compiler.
- PIR change; network / registry; new third-party arg parser.
