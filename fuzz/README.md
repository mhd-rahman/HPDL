# HPDL fuzzing

Coverage-guided fuzzing of the canonical PIR decoder (`decode_planet_definition`),
which must be **total** on untrusted input: it may return `Err`, but it must never
panic, over-allocate, or hang.

## Why this needs nightly

`cargo fuzz` compiles with `-Zsanitizer=address` and sancov instrumentation, which
are **nightly-only** flags. The workspace root pins a stable toolchain in
`rust-toolchain.toml`, and that pin is resolved from the **current working
directory** — so running `cargo fuzz` from the repo root otherwise picks up stable and
fails with:

```
error: the option `Z` is only accepted on the nightly compiler
```

### The fix

Always invoke cargo-fuzz with an explicit nightly proxy, which rustup resolves *before*
any `rust-toolchain.toml`:

```sh
rustup toolchain install nightly        # once
cargo +nightly fuzz run decode_pir
```

CI does exactly this (`.github/workflows/fuzz.yml`): it installs nightly, sets a
directory override, and calls `cargo +nightly fuzz run`.

## Targets

| Target | Entry point | Input |
|---|---|---|
| `decode_pir` | `hpdl_pir::decode_planet_definition` | raw PIR bytes |
| `decode_ron` | `hpdl_ron::RonLoader::parse` | UTF-8 RON source |

Both parse **untrusted** input and must be total (return `Err`, never panic/hang/
over-allocate).

## Running locally

```sh
# Short smoke run (120s each), seeded from fuzz/seeds/:
mkdir -p fuzz/corpus/decode_pir fuzz/corpus/decode_ron
cp -n fuzz/seeds/valid_envelope.bin fuzz/corpus/decode_pir/
cp -n fuzz/seeds/valid.ron          fuzz/corpus/decode_ron/
cargo +nightly fuzz run decode_pir --target x86_64-unknown-linux-gnu -- -max_total_time=120 -rss_limit_mb=2048
cargo +nightly fuzz run decode_ron --target x86_64-unknown-linux-gnu -- -max_total_time=120 -rss_limit_mb=2048
```

(`--target x86_64-unknown-linux-gnu` avoids cargo-fuzz's musl default, whose static
libc is incompatible with AddressSanitizer.)

## Corpus & regressions

- `fuzz/seeds/` — committed, well-formed inputs the fuzzer mutates from
  (`valid_envelope.bin` for PIR, `valid.ron` for RON).
- `fuzz/corpus/` — the growing corpus (git-ignored; regenerated from seeds).
- `fuzz/artifacts/<target>/` — crash reproducers. **Commit these** as regression
  cases when a crash is found. No test edit is required: the workspace test
  `crates/hpdl-compiler/tests/fuzz_regressions.rs` automatically replays every file
  under `fuzz/artifacts/decode_pir/` and `fuzz/artifacts/decode_ron/` through the
  real decoder entry points under `cargo test --workspace`, asserting totality (no
  panic/hang) on Linux, macOS, and Windows. Committing the reproducer turns it into
  an enforced regression.

## Running an extended session (hours)

The `Fuzz` workflow supports a manual `workflow_dispatch` with a real per-target
budget. From the GitHub Actions UI (or `gh workflow run fuzz.yml`), set:

- `max_total_time` — seconds **per target** (e.g. `14400` = 4h). Each target runs
  the full budget independently; they are not split.
- `target` — `both` (default), `decode_pir`, or `decode_ron`.
- `rss_limit_mb` — libFuzzer RSS cap (default `2048`).

The run uploads the evolved corpus (`fuzz-corpus-<target>`) on every run and, on a
crash, uploads the reproducer (`fuzz-artifacts-<target>`) and also prints it
base64-encoded in the job log for retrieval without runner access.

## Scope

Fuzzing supplements, but does not replace, the test suite (testing-ci steering). The
decoders' totality is asserted by unit tests; the fuzzer searches for inputs those
tests missed.

The native `.hpdl` parser (`hpdl-syntax`) is **not** fuzzed yet (spec 16, OD-16.3):
its grammar is unratified and the converter is a stub, so fuzzing it now would test
throwaway code. Add a `decode_hpdl` target once the grammar is ratified.
