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

## Running locally

```sh
# Short smoke run (120s), seeded from fuzz/seeds/:
mkdir -p fuzz/corpus/decode_pir
cp -n fuzz/seeds/* fuzz/corpus/decode_pir/
cargo +nightly fuzz run decode_pir -- -max_total_time=120 -rss_limit_mb=2048
```

## Corpus & regressions

- `fuzz/seeds/` — committed, well-formed inputs the fuzzer mutates from.
- `fuzz/corpus/` — the growing corpus (git-ignored; regenerated from seeds).
- `fuzz/artifacts/` — crash reproducers. **Commit these** as regression cases when a
  crash is found, and add a matching unit/integration test against the real decoder.

## Scope

Fuzzing supplements, but does not replace, the test suite (testing-ci steering). The
decoder's totality is asserted by unit tests; the fuzzer searches for inputs those
tests missed.
