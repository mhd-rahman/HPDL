#![no_main]
//! Fuzz target for the native `.hpdl` parser.
//!
//! `hpdl-syntax` lexes and parses untrusted `.hpdl` authoring input and must be
//! **total**: it may return `Err` (lex error, syntax error, security-limit
//! exceeded, failed model conversion), but it must never panic, over-allocate,
//! or loop/recurse unbounded (rust-standards + determinism steering). The parser
//! is recursive descent, so this target is the primary guard against a
//! stack-overflow abort on deeply nested input — the default [`ParseLimits`]
//! depth bound must turn that into a typed error rather than a crash.
//!
//! Both the strict and the error-recovering entry points are exercised, since
//! they drive the parser differently.
//!
//! Run with the nightly toolchain (gnu target, per the ASan requirement):
//!     cargo +nightly fuzz run parse_hpdl --target x86_64-unknown-linux-gnu
//! Keep interesting inputs under `fuzz/corpus/` and crash reproducers under
//! `fuzz/artifacts/` as a regression corpus.

use libfuzzer_sys::fuzz_target;

use hpdl_syntax::{parse_file, parse_source_collect_errors, SourceFile};

fuzz_target!(|data: &[u8]| {
    // `.hpdl` source is UTF-8 text. Non-UTF-8 inputs are skipped (the real
    // frontend only ever sees decoded text); every valid UTF-8 string is fed to
    // both parse entry points under default limits. A `Result` either way is
    // fine; a panic, hang, or stack overflow is a bug.
    if let Ok(text) = std::str::from_utf8(data) {
        let source = SourceFile::from_string("fuzz.hpdl".to_string(), text.to_string());
        let _ = parse_file(&source);
        let _ = parse_source_collect_errors(&source);
    }
});
