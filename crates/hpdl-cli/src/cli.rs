//! Command surface and a hand-rolled argument parser (no third-party arg parser).
//!
//! The command surface is small, so a hand-rolled parser keeps `hpdl-cli`
//! dependency-light (OD-15.1) and the behavior fully deterministic.

/// The output format for diagnostics and summaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Human-readable text (default).
    Text,
    /// Deterministic machine-readable JSON.
    Json,
}

/// A parsed command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `check <file> [--format text|json]` — compile and report success/diagnostics.
    Check {
        /// Source file path.
        file: String,
        /// Output format.
        format: Format,
    },
    /// `build <file> [--lock <lock>] [--format text|json]` — compile and emit the manifest.
    Build {
        /// Source file path.
        file: String,
        /// Optional authoritative package lockfile path.
        lock: Option<String>,
        /// Output format.
        format: Format,
    },
    /// `inspect <file> [--lock <lock>]` — print a human-readable compile summary.
    Inspect {
        /// Source file path.
        file: String,
        /// Optional authoritative package lockfile path.
        lock: Option<String>,
    },
    /// `export <file> [--lock <lock>] [--out <path>]` — write canonical PIR bytes.
    Export {
        /// Source file path.
        file: String,
        /// Optional authoritative package lockfile path.
        lock: Option<String>,
        /// Optional output path (hex to stdout if omitted).
        out: Option<String>,
    },
    /// `--help` / `-h` or no subcommand.
    Help,
    /// `--version` / `-V`.
    Version,
}

/// A usage error with a human-readable reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageError(pub String);

/// The `--help` text.
pub const HELP: &str = "\
hpdl — HPDL developer CLI

USAGE:
    hpdl <COMMAND> [OPTIONS]

COMMANDS:
    check    <file> [--format text|json]              Compile and report success or diagnostics
    build    <file> [--lock <lock>] [--format ...]    Compile and print the build manifest
    inspect  <file> [--lock <lock>]                   Print a compile summary (version, hash, modules)
    export   <file> [--lock <lock>] [--out <path>]    Write canonical PIR bytes (hex to stdout if no --out)

OPTIONS:
    -h, --help       Print this help
    -V, --version    Print the version

EXIT CODES:
    0  success
    1  compilation or diagnostic failure
    2  usage error (unknown command/flag, missing argument, unreadable file)
";

/// Parse CLI arguments (excluding the program name) into a [`Command`].
///
/// # Errors
///
/// Returns a [`UsageError`] on an unknown subcommand or flag, a missing file
/// argument, a missing flag value, or a bad `--format` value.
pub fn parse(args: &[String]) -> Result<Command, UsageError> {
    let first = match args.first() {
        None => return Ok(Command::Help),
        Some(a) => a.as_str(),
    };

    match first {
        "-h" | "--help" | "help" => return Ok(Command::Help),
        "-V" | "--version" | "version" => return Ok(Command::Version),
        _ => {}
    }

    let rest = args.get(1..).unwrap_or(&[]);
    let parsed = ParsedArgs::parse(rest)?;

    match first {
        "check" => Ok(Command::Check {
            file: parsed.require_file()?,
            format: parsed.format()?,
        }),
        "build" => Ok(Command::Build {
            file: parsed.require_file()?,
            lock: parsed.lock.clone(),
            format: parsed.format()?,
        }),
        "inspect" => Ok(Command::Inspect {
            file: parsed.require_file()?,
            lock: parsed.lock.clone(),
        }),
        "export" => Ok(Command::Export {
            file: parsed.require_file()?,
            lock: parsed.lock.clone(),
            out: parsed.out.clone(),
        }),
        other => Err(UsageError(format!("unknown command `{other}`"))),
    }
}

/// Flags and positionals collected from a subcommand's arguments.
struct ParsedArgs {
    file: Option<String>,
    lock: Option<String>,
    out: Option<String>,
    format: Option<String>,
}

impl ParsedArgs {
    fn parse(args: &[String]) -> Result<Self, UsageError> {
        let mut file: Option<String> = None;
        let mut lock: Option<String> = None;
        let mut out: Option<String> = None;
        let mut format: Option<String> = None;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--lock" => lock = Some(take_value(&mut iter, "--lock")?),
                "--out" => out = Some(take_value(&mut iter, "--out")?),
                "--format" => format = Some(take_value(&mut iter, "--format")?),
                flag if flag.starts_with('-') => {
                    return Err(UsageError(format!("unknown flag `{flag}`")));
                }
                positional => {
                    if file.is_some() {
                        return Err(UsageError(format!(
                            "unexpected extra argument `{positional}`"
                        )));
                    }
                    file = Some(positional.to_string());
                }
            }
        }

        Ok(Self {
            file,
            lock,
            out,
            format,
        })
    }

    fn require_file(&self) -> Result<String, UsageError> {
        self.file
            .clone()
            .ok_or_else(|| UsageError("missing required <file> argument".to_string()))
    }

    fn format(&self) -> Result<Format, UsageError> {
        match self.format.as_deref() {
            None | Some("text") => Ok(Format::Text),
            Some("json") => Ok(Format::Json),
            Some(other) => Err(UsageError(format!(
                "invalid --format `{other}` (expected `text` or `json`)"
            ))),
        }
    }
}

fn take_value(iter: &mut std::slice::Iter<'_, String>, flag: &str) -> Result<String, UsageError> {
    iter.next()
        .cloned()
        .ok_or_else(|| UsageError(format!("flag `{flag}` requires a value")))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn no_args_is_help() {
        assert_eq!(parse(&[]).unwrap(), Command::Help);
    }

    #[test]
    fn help_and_version_flags() {
        assert_eq!(parse(&args(&["--help"])).unwrap(), Command::Help);
        assert_eq!(parse(&args(&["-h"])).unwrap(), Command::Help);
        assert_eq!(parse(&args(&["--version"])).unwrap(), Command::Version);
        assert_eq!(parse(&args(&["-V"])).unwrap(), Command::Version);
    }

    #[test]
    fn check_parses_file_and_format() {
        let cmd = parse(&args(&["check", "p.ron", "--format", "json"])).unwrap();
        assert_eq!(
            cmd,
            Command::Check {
                file: "p.ron".to_string(),
                format: Format::Json,
            }
        );
    }

    #[test]
    fn build_parses_lock() {
        let cmd = parse(&args(&["build", "p.ron", "--lock", "hpdl.lock"])).unwrap();
        assert_eq!(
            cmd,
            Command::Build {
                file: "p.ron".to_string(),
                lock: Some("hpdl.lock".to_string()),
                format: Format::Text,
            }
        );
    }

    #[test]
    fn export_parses_out() {
        let cmd = parse(&args(&["export", "p.ron", "--out", "p.pir"])).unwrap();
        assert_eq!(
            cmd,
            Command::Export {
                file: "p.ron".to_string(),
                lock: None,
                out: Some("p.pir".to_string()),
            }
        );
    }

    #[test]
    fn unknown_command_is_usage_error() {
        assert!(parse(&args(&["frobnicate", "p.ron"])).is_err());
    }

    #[test]
    fn unknown_flag_is_usage_error() {
        assert!(parse(&args(&["check", "p.ron", "--wat"])).is_err());
    }

    #[test]
    fn missing_file_is_usage_error() {
        assert!(parse(&args(&["check"])).is_err());
    }

    #[test]
    fn missing_flag_value_is_usage_error() {
        assert!(parse(&args(&["build", "p.ron", "--lock"])).is_err());
    }

    #[test]
    fn bad_format_is_usage_error() {
        assert!(parse(&args(&["check", "p.ron", "--format", "yaml"])).is_err());
    }

    #[test]
    fn extra_positional_is_usage_error() {
        assert!(parse(&args(&["check", "a.ron", "b.ron"])).is_err());
    }
}
