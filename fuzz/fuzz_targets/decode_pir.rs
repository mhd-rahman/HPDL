#![no_main]
//! Fuzz target for the canonical PIR decoder.
//!
//! The decoder MUST be total on untrusted input: it may return `Err`, but it
//! must never panic, over-allocate, or loop unbounded (rust-standards +
//! determinism steering). This target feeds arbitrary bytes to
//! `decode_planet_definition` and lets libfuzzer surface any panic / UB.
//!
//! Run with the nightly toolchain:
//!     cargo +nightly fuzz run decode_pir
//! Keep interesting inputs and crash reproducers under `fuzz/corpus/` and
//! `fuzz/artifacts/` as a regression corpus.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // A `Result` is fine; a panic or hang is a bug. Ignore the Ok/Err outcome.
    let _ = hpdl_pir::decode_planet_definition(data);
});
