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

/// The frontend to use for a source file, chosen by extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Frontend {
    /// Native `.hpdl` source.
    Hpdl,
    /// RON source (the default for any non-`.hpdl` extension).
    Ron,
}

/// Pick the frontend from the file extension: `.hpdl` → native, everything else
/// (`.ron` and anything unrecognized) → RON. Case-insensitive on the extension.
fn frontend_for(path: &str) -> Frontend {
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("hpdl") => Frontend::Hpdl,
        _ => Frontend::Ron,
    }
}

/// Compile a source file, dispatching to the native or RON frontend by extension
/// and returning either the result or a rendered failure.
fn compile(file: &str, lock: Option<&str>, format: Format) -> Result<CompileResult, Outcome> {
    let source = read_source(file)?;
    let options = options_with_lock(lock)?;
    let name = source_name(file);
    let compiler = Compiler::new(options);
    let compiled = match frontend_for(file) {
        Frontend::Hpdl => compiler.compile_hpdl_str(&name, &source),
        Frontend::Ron => compiler.compile_ron_str(&name, &source),
    };
    compiled.map_err(|e: Error| Outcome::failure(render::render_error(&e, format)))
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn frontend_dispatch_by_extension() {
        assert_eq!(frontend_for("planet.hpdl"), Frontend::Hpdl);
        assert_eq!(frontend_for("planet.HPDL"), Frontend::Hpdl);
        assert_eq!(frontend_for("dir/sub/earth.hpdl"), Frontend::Hpdl);
        assert_eq!(frontend_for("planet.ron"), Frontend::Ron);
        assert_eq!(frontend_for("planet.txt"), Frontend::Ron);
        assert_eq!(frontend_for("noext"), Frontend::Ron);
    }

    #[test]
    fn hpdl_source_compiles_through_cli_compile() {
        const HPDL: &str = r#"
            language hpdl "1.0";
            planet p {
                id: "p",
                physical: { mass_kg: 1000000000000000000000000, radius_m: 1000000 },
            }
        "#;
        // Write to a temp .hpdl file so the extension dispatch picks the native path.
        let dir = std::env::temp_dir();
        let path = dir.join("hpdl_cli_dispatch_test.hpdl");
        std::fs::write(&path, HPDL).unwrap();
        let path_str = path.to_str().unwrap();

        let result = compile(path_str, None, Format::Text).expect("hpdl should compile via CLI");
        assert!(result.envelope().verify_hash());

        std::fs::remove_file(&path).ok();
    }
}
