//! # hpdl — HPDL developer CLI (binary entry point)
//!
//! A thin wrapper over [`hpdl_cli::dispatch`]: parse args, print the buffered
//! output, and exit with the resulting code. All logic lives in the `hpdl_cli`
//! library and, below it, in [`hpdl_compiler`].

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = hpdl_cli::dispatch(&args);
    if !outcome.stdout.is_empty() {
        print!("{}", outcome.stdout);
    }
    if !outcome.stderr.is_empty() {
        eprint!("{}", outcome.stderr);
    }
    std::process::exit(outcome.code);
}
