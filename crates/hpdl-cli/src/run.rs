//! Subcommand execution: read inputs, delegate to `hpdl-compiler`, format output.
//!
//! These functions contain **no** compilation logic — they read files, build
//! [`CompileOptions`], call the [`Compiler`], and hand the result to `render`.

use std::path::Path;

use hpdl_compiler::{CompileOptions, CompileResult, Compiler, Error, Lockfile};

use crate::cli::Format;
use crate::exit::Outcome;
use crate::render;

/// Read a UTF-8 source file, returning a usage [`Outcome`] on an IO/UTF-8 error.
fn read_source(path: &str) -> Result<String, Outcome> {
    std::fs::read_to_string(path)
        .map_err(|e| Outcome::usage(format!("error: cannot read `{path}`: {e}\n")))
}

/// Derive a logical source name from a path's file stem.
fn source_name(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

/// Build [`CompileOptions`], loading an authoritative package lock if `--lock` was
/// given. Returns a usage [`Outcome`] on an unreadable/invalid lock.
fn options_with_lock(lock: Option<&str>) -> Result<CompileOptions, Outcome> {
    let options = CompileOptions::default();
    match lock {
        None => Ok(options),
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| Outcome::usage(format!("error: cannot read lock `{path}`: {e}\n")))?;
            let parsed = Lockfile::parse(&text)
                .map_err(|e| Outcome::usage(format!("error: invalid lockfile `{path}`: {e}\n")))?;
            Ok(options.with_package_lock(parsed))
        }
    }
}

/// Compile a source file, returning either the result or a rendered failure.
fn compile(file: &str, lock: Option<&str>, format: Format) -> Result<CompileResult, Outcome> {
    let source = read_source(file)?;
    let options = options_with_lock(lock)?;
    let name = source_name(file);
    Compiler::new(options)
        .compile_ron_str(&name, &source)
        .map_err(|e: Error| Outcome::failure(render::render_error(&e, format)))
}

/// `check <file>`.
#[must_use]
pub fn run_check(file: &str, format: Format) -> Outcome {
    match compile(file, None, format) {
        Ok(result) => Outcome::ok(render::render_check_ok(&result, &source_name(file), format)),
        Err(outcome) => outcome,
    }
}

/// `build <file> [--lock] [--format]`.
#[must_use]
pub fn run_build(file: &str, lock: Option<&str>, format: Format) -> Outcome {
    match compile(file, lock, format) {
        Ok(result) => Outcome::ok(render::render_build(&result, format)),
        Err(outcome) => outcome,
    }
}

/// `inspect <file> [--lock]`.
#[must_use]
pub fn run_inspect(file: &str, lock: Option<&str>) -> Outcome {
    match compile(file, lock, Format::Text) {
        Ok(result) => Outcome::ok(render::render_inspect(&result)),
        Err(outcome) => outcome,
    }
}

/// `export <file> [--lock] [--out]`.
#[must_use]
pub fn run_export(file: &str, lock: Option<&str>, out: Option<&str>) -> Outcome {
    let result = match compile(file, lock, Format::Text) {
        Ok(r) => r,
        Err(outcome) => return outcome,
    };
    let (envelope, _manifest) = result.into_parts();
    let bytes = &envelope.payload;
    match out {
        Some(path) => match std::fs::write(path, bytes) {
            Ok(()) => Outcome::ok(format!(
                "wrote {} bytes of canonical PIR to `{}`\n",
                bytes.len(),
                path
            )),
            Err(e) => Outcome::usage(format!("error: cannot write `{path}`: {e}\n")),
        },
        None => {
            let hex = to_hex(bytes);
            Outcome::ok(format!("{hex}\n"))
        }
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}
