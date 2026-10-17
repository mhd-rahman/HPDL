//! # hpdl-cli
//!
//! Library backing the `hpdl` developer CLI. It owns argument parsing, file I/O,
//! command dispatch, and diagnostic presentation only — all compilation delegates
//! to [`hpdl_compiler`] (architecture steering: `hpdl-cli` must not contain logic).
//!
//! The binary (`src/main.rs`) is a thin wrapper over [`dispatch`], which is kept
//! pure and testable: it performs file I/O but prints nothing and never exits the
//! process, returning a buffered [`exit::Outcome`] instead.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

pub mod cli;
pub mod exit;
pub mod render;
pub mod run;

use cli::{Command, UsageError};
use exit::Outcome;

/// Parse and execute a command line (excluding the program name), returning a
/// buffered [`Outcome`]. Pure and testable — it performs file I/O but prints
/// nothing and never exits the process.
#[must_use]
pub fn dispatch(args: &[String]) -> Outcome {
    match cli::parse(args) {
        Ok(Command::Help) => Outcome::ok(cli::HELP.to_string()),
        Ok(Command::Version) => Outcome::ok(format!("hpdl {}\n", env!("CARGO_PKG_VERSION"))),
        Ok(Command::Check { file, format }) => run::run_check(&file, format),
        Ok(Command::Build { file, lock, format }) => run::run_build(&file, lock.as_deref(), format),
        Ok(Command::Inspect { file, lock }) => run::run_inspect(&file, lock.as_deref()),
        Ok(Command::Export { file, lock, out }) => {
            run::run_export(&file, lock.as_deref(), out.as_deref())
        }
        Err(UsageError(reason)) => Outcome::usage(format!("error: {reason}\n\n{}", cli::HELP)),
    }
}
