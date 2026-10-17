---
inclusion: fileMatch
fileMatchPattern: ["crates/hpdl-pir/**", "crates/hpdl-compiler/**", "crates/hpdl-runtime-api/**", "tests/canonicalization/**", "tests/fixed_point/**", "tests/conformance/**", "tests/golden/**"]
---

# HPDL — Determinism, Numerics & PIR

## Canonical PIR
Versioned, strictly specified binary format. Define: field order, integer endianness, optional-value encoding, sorted map keys, string normalization, numeric representation, duplicate-field rejection, unknown-field behavior.
- The decoder enforces canonical form. One semantic value has exactly one valid byte encoding.
- Reject unsupported major versions and unknown critical extensions.
- Hash canonical bytes only, with a named algorithm and a domain separator that includes protocol ID and format version.
- NEVER hash Rust `Debug` output, in-memory layout, or `HashMap` iteration order.

## Golden vectors
Frozen files under `tests/golden/`: input semantics, exact bytes, format version, expected digest, expected decode result.
Changing canonical bytes requires a format-version decision and a reviewed vector update. A crate release alone is not sufficient. Never silently regenerate vectors to make tests pass.

## Fixed-point numerics (Amendments A2, A2.1)
- Compile time: exact decimal/rational arithmetic.
- Every PIR numeric field declares: dimension, canonical unit, storage width, quantum, rounding mode (`ties_to_even` default), overflow policy, max lowering error.
- Runtime: fixed-point integers with checked operations.
- Mixed-quantum operations (Pa + kPa, m × mm) use a widened intermediate (`i128`) and a specified rescale-then-round order.
- Large magnitudes (e.g. planetary mass ~6e24 kg) need a coarser quantum or wider storage; verify representability explicitly.
- Unrepresentable values or out-of-tolerance conversions fail. No silent saturation or clamping unless the field contract says so and tests it.

```
numeric PressureRuntime {
  dimension: Pressure  canonical_unit: Pa  storage: i64
  quantum: 1 Pa  rounding: ties_to_even  overflow: error
}
```

## Environment independence
No wall-clock time, locale, filesystem order, network, unseeded randomness or architecture-dependent behavior in compilation or state transitions. No floats in consensus-critical paths.

## Reproducibility
Same locked inputs + compiler semantics version + format version ⇒ byte-identical PIR on Linux, macOS and Windows.
Same initial state + ordered actions + module hashes + tick sequence ⇒ identical state/event hashes.
Keep language/PIR conformance separate from runtime replay conformance.
