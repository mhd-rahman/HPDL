#![no_main]
//! Fuzz target for the RON frontend loader.
//!
//! `hpdl-ron` parses untrusted `.ron` authoring input and must be **total**: it
//! may return `Err` (syntax error, wrong shape, security-limit exceeded), but it
//! must never panic, over-allocate, or loop unbounded (rust-standards +
//! determinism steering). This target feeds arbitrary UTF-8 to the loader's
//! `parse` entry point — the same path the compiler frontend uses — and lets
//! libfuzzer surface any panic / UB / hang.
//!
//! Run with the nightly toolchain (gnu target, per the ASan requirement):
//!     cargo +nightly fuzz run decode_ron --target x86_64-unknown-linux-gnu
//! Keep interesting inputs under `fuzz/corpus/` and crash reproducers under
//! `fuzz/artifacts/` as a regression corpus.

use libfuzzer_sys::fuzz_target;

use hpdl_ron::{RonLoader, SourceDocument};

fuzz_target!(|data: &[u8]| {
    // RON source is UTF-8 text. Non-UTF-8 inputs are simply skipped (the real
    // frontend only ever sees decoded text); every valid UTF-8 string is fed to
    // the loader. A `Result` is fine; a panic or hang is a bug.
    if let Ok(text) = std::str::from_utf8(data) {
        let source = SourceDocument::from_string("fuzz".to_string(), text.to_string());
        let loader = RonLoader::new();
        let _ = loader.parse(&source);
    }
});
