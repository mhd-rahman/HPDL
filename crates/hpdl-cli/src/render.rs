//! Presentation of compiler results and diagnostics (text and JSON).
//!
//! This module formats only; it holds no compilation logic. Diagnostics are sorted
//! for deterministic output; JSON is hand-built (a small fixed shape) so no
//! `serde_json` runtime dependency is required (OD-15.2).

use hpdl_compiler::{BuildManifest, CompileResult, Diagnostic, Error, Severity};

use crate::cli::Format;

/// Render a compiler [`Error`] in the requested format.
#[must_use]
pub fn render_error(err: &Error, format: Format) -> String {
    match format {
        Format::Text => render_error_text(err),
        Format::Json => render_error_json(err),
    }
}

fn render_error_text(err: &Error) -> String {
    match err {
        Error::Validation(diags) => {
            let mut sorted = diags.clone();
            sorted.sort();
            let mut out = format!(
                "error: semantic validation failed with {} diagnostic(s)\n",
                sorted.len()
            );
            for d in &sorted {
                out.push_str(&format!("{d}\n"));
            }
            out
        }
        other => format!("error: {other}\n"),
    }
}

fn render_error_json(err: &Error) -> String {
    match err {
        Error::Validation(diags) => {
            let mut sorted = diags.clone();
            sorted.sort();
            let items: Vec<String> = sorted.iter().map(diagnostic_json).collect();
            format!(
                "{{\"error\":\"validation\",\"diagnostics\":[{}]}}",
                items.join(",")
            )
        }
        other => format!(
            "{{\"error\":\"{}\",\"message\":\"{}\"}}",
            error_kind(other),
            escape_json(&other.to_string())
        ),
    }
}

fn error_kind(err: &Error) -> &'static str {
    match err {
        Error::Frontend { .. } => "frontend",
        Error::Validation(_) => "validation",
        Error::UnresolvedModule { .. } => "unresolved_module",
        Error::ModuleVersionMismatch { .. } => "module_version_mismatch",
        Error::LockParse { .. } => "lock_parse",
        Error::LockFormat { .. } => "lock_format",
        Error::UnresolvedPackage { .. } => "unresolved_package",
        Error::PackageVersionMismatch { .. } => "package_version_mismatch",
        Error::PackageIntegrity { .. } => "package_integrity",
        Error::DependencyCycle(_) => "dependency_cycle",
        Error::Lowering { .. } => "lowering",
        Error::Emission(_) => "emission",
        Error::Limit(_) => "limit",
    }
}

fn diagnostic_json(d: &Diagnostic) -> String {
    let span = match &d.span {
        Some(s) => format!(
            "{{\"start\":{},\"end\":{},\"source\":{}}}",
            s.start,
            s.end,
            match &s.source {
                Some(src) => format!("\"{}\"", escape_json(src)),
                None => "null".to_string(),
            }
        ),
        None => "null".to_string(),
    };
    format!(
        "{{\"code\":\"{}\",\"severity\":\"{}\",\"message\":\"{}\",\"span\":{}}}",
        d.code.code(),
        severity_str(d.severity),
        escape_json(&d.message),
        span,
    )
}

fn severity_str(sev: Severity) -> &'static str {
    match sev {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

/// Minimal JSON string escaping for the fixed diagnostic shape.
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Render the `check` success line.
#[must_use]
pub fn render_check_ok(result: &CompileResult, name: &str, format: Format) -> String {
    let hash = result.content_hash_hex();
    let prefix = hash.get(..12).unwrap_or(hash.as_str());
    match format {
        Format::Text => format!("ok: `{name}` compiled ({prefix})\n"),
        Format::Json => format!(
            "{{\"ok\":true,\"name\":\"{}\",\"content_hash\":\"{}\"}}",
            escape_json(name),
            hash
        ),
    }
}

/// Render the `build` output: the stable manifest text, or a JSON summary.
#[must_use]
pub fn render_build(result: &CompileResult, format: Format) -> String {
    match format {
        Format::Text => result.manifest().to_text(),
        Format::Json => manifest_json(result.manifest()),
    }
}

fn manifest_json(manifest: &BuildManifest) -> String {
    let modules: Vec<String> = manifest
        .modules
        .iter()
        .map(|m| {
            format!(
                "{{\"name\":\"{}\",\"version\":\"{}\",\"content_hash\":\"{}\"}}",
                escape_json(&m.name),
                escape_json(&m.version),
                escape_json(&m.content_hash)
            )
        })
        .collect();
    format!(
        "{{\"source_name\":\"{}\",\"pir_version\":\"{}\",\"content_hash\":\"{}\",\"modules\":[{}]}}",
        escape_json(&manifest.source_name),
        manifest.pir_version,
        escape_json(&manifest.content_hash),
        modules.join(",")
    )
}

/// Render the `inspect` human-readable summary.
#[must_use]
pub fn render_inspect(result: &CompileResult) -> String {
    let m = result.manifest();
    let mut out = String::new();
    out.push_str(&format!("source:          {}\n", m.source_name));
    out.push_str(&format!(
        "compiler:        {}\n",
        m.compiler_semantics_version
    ));
    out.push_str(&format!("pir_version:     {}\n", m.pir_version));
    out.push_str(&format!("content_hash:    {}\n", m.content_hash));
    if m.modules.is_empty() {
        out.push_str("modules:         (none)\n");
    } else {
        out.push_str(&format!("modules:         {}\n", m.modules.len()));
        for module in &m.modules {
            out.push_str(&format!(
                "  - {} {} {}\n",
                module.name, module.version, module.content_hash
            ));
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn escape_handles_control_and_quotes() {
        assert_eq!(escape_json("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }

    #[test]
    fn error_json_is_object() {
        let err = Error::Emission("boom".to_string());
        let json = render_error(&err, Format::Json);
        assert!(json.starts_with("{\"error\":\"emission\""));
        assert!(json.contains("boom"));
    }

    #[test]
    fn error_text_prefixes_error() {
        let err = Error::Limit("too big".to_string());
        assert!(render_error(&err, Format::Text).starts_with("error: "));
    }
}
