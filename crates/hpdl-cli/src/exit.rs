//! Stable, documented process exit codes and the [`Outcome`] a subcommand produces.
//!
//! Exit codes are part of the CLI's contract (see `--help` and ADR 0009):
//! `0` success, `1` compilation/diagnostic failure, `2` usage/IO error.

/// Successful completion.
pub const OK: i32 = 0;

/// Compilation or diagnostic failure (a valid command whose compile failed).
pub const FAILURE: i32 = 1;

/// Usage error: unknown subcommand/flag, a missing argument, or an unreadable /
/// non-UTF-8 input file.
pub const USAGE: i32 = 2;

/// The result of dispatching a command: buffered output and an exit code.
///
/// Buffering (rather than printing directly) keeps dispatch pure and testable —
/// integration tests assert on `stdout`/`stderr`/`code` without spawning a process.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Text to print to standard output.
    pub stdout: String,
    /// Text to print to standard error.
    pub stderr: String,
    /// The process exit code.
    pub code: i32,
}

impl Outcome {
    /// A success outcome printing `stdout` to standard output.
    pub fn ok(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: String::new(),
            code: OK,
        }
    }

    /// A compilation/diagnostic failure printing `stderr` to standard error.
    pub fn failure(stderr: impl Into<String>) -> Self {
        Self {
            stdout: String::new(),
            stderr: stderr.into(),
            code: FAILURE,
        }
    }

    /// A usage/IO error printing `stderr` to standard error.
    pub fn usage(stderr: impl Into<String>) -> Self {
        Self {
            stdout: String::new(),
            stderr: stderr.into(),
            code: USAGE,
        }
    }
}
