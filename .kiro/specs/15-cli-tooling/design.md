# HPDL CLI (`hpdl`) — Design

## 1. Scope

Implement the `hpdl` binary in `hpdl-cli` with `check`, `build`, `inspect`, `export`, plus
`--help`/`--version`. The crate owns arg parsing, file I/O, dispatch, and diagnostic
presentation only; all compilation delegates to `hpdl-compiler`. No new third-party dependency;
hand-rolled parser (OD-15.1).

## 2. Module layout (`crates/hpdl-cli/src/`)

```
main.rs    -- thin: parse args, dispatch, map Outcome -> process::exit(code)
cli.rs     -- Command enum, arg parsing (hand-rolled), usage/help/version text
run.rs     -- run_* functions per subcommand; build CompileOptions, call Compiler
render.rs  -- diagnostic + error presentation (text + json), manifest/inspect formatting
exit.rs    -- ExitCode constants (0 ok, 1 failure, 2 usage) + an Outcome type
```

All logic that could belong to the compiler stays in the compiler; these modules only shuffle
inputs/outputs and format.

## 3. Arg model (`cli.rs`)

```rust
enum Command {
    Check { file: String, format: Format },
    Build { file: String, lock: Option<String>, format: Format },
    Inspect { file: String, lock: Option<String> },
    Export { file: String, lock: Option<String>, out: Option<String> },
    Help,
    Version,
}
enum Format { Text, Json }
fn parse(args: &[String]) -> Result<Command, UsageError>;
```
`parse` reads the first non-flag token as the subcommand, then its positional file and
`--lock`/`--out`/`--format` flags. An unknown subcommand/flag, a missing file arg, or a bad
`--format` value is a `UsageError` (→ exit 2). No panics; no `clap`.

## 4. Dispatch & run (`run.rs`)

Each `run_*` function: read the source file (UTF-8; a read/UTF-8 error → `Outcome::usage`),
optionally read + `Lockfile::parse` the `--lock` file, build `CompileOptions`
(`.with_package_lock` when a lock is given), call `Compiler::compile_ron_str(name, &source)`,
then:
- `check`: on `Ok`, print `ok: <name> compiled (<hash-prefix>)`; on `Err`, render + exit 1.
- `build`: on `Ok`, print `manifest.to_text()` (text) or a JSON summary; else render + exit 1.
- `inspect`: on `Ok`, print a labelled summary from the manifest; else render + exit 1.
- `export`: on `Ok`, write `envelope().payload` bytes to `--out` (or hex to stdout); else exit 1.

`name` is derived from the file path's stem. The CLI does no size pre-check — the compiler's
`max_source_bytes` limit governs (surfaced as `Error::Limit`).

## 5. Rendering (`render.rs`)

- `render_error(err: &hpdl_compiler::Error, format: Format) -> String`.
  - Text: for `Validation(diags)`, sort a clone of the diagnostics (their `Ord` is span→code→
    message) and print each via `Diagnostic`'s `Display` (`severity: [CODE] message (span)` +
    notes). Other variants print `err.to_string()`.
  - Json: a hand-built deterministic object — `{"error":"validation","diagnostics":[{"code":..,
    "severity":..,"message":..,"span":..}]}` or `{"error":"<variant>","message":".."}`. Strings
    are JSON-escaped by a small local helper (no `serde_json` runtime dep, OD-15.2).
- `render_manifest` / `render_inspect`: reuse `BuildManifest::to_text()` and labelled lines.

## 6. Exit codes (`exit.rs`)

```rust
pub const OK: i32 = 0;        // success
pub const FAILURE: i32 = 1;   // compilation / diagnostic failure
pub const USAGE: i32 = 2;     // unknown subcommand/flag, missing arg, unreadable/non-UTF-8 file
struct Outcome { stdout: String, stderr: String, code: i32 }
```
`main` prints `stdout`/`stderr` and calls `process::exit(code)`. A testable `dispatch(args) ->
Outcome` keeps `main` trivial and lets integration tests assert output + code without spawning a
process (plus one smoke test that spawns the real binary).

## 7. Determinism, security, forbidden list

- Deterministic: diagnostics sorted; manifest/PIR bytes come from the compiler unchanged; no
  hashmap iteration, wall-clock, or locale in output.
- Security: file contents untrusted → rely on compiler limits; never panic on a missing/non-UTF-8
  file or malformed source (fail closed with exit 2/1); honor `forbid(unsafe_code)` and the
  no-`unwrap`/`expect`/indexing lints.
- **Not** added: compilation logic, simulation, native `.hpdl` parsing, a `run`/`test`/`fmt`
  command, network/registry access, any PIR change, any new third-party dependency.

## 8. Tests

- `tests/cli.rs` (via the testable `dispatch(args) -> Outcome`): `check` success + failure;
  `build` manifest text + `--format json`; `inspect` summary; `export` to a temp file + hex
  stdout; unknown subcommand → exit 2; unknown flag → exit 2; missing file arg → exit 2;
  unreadable file → exit 2; `--help`/`--version` → exit 0; a validation failure → exit 1 with
  rendered diagnostics.
- A frozen golden fixture for `inspect`/`build` text output of a minimal planet.
- One smoke test spawning the built `hpdl` binary to confirm end-to-end wiring + exit code.

## 9. Compatibility

- `hpdl-cli` is `publish = false`; no contract-crate change; no PIR change.
- Exit codes `0/1/2` are the CLI's stability surface, documented in `--help` and ADR 0009.
- No new third-party dependency (OD-15.1); a dev-only dependency may back the binary smoke test.
