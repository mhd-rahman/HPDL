//! Integration tests for the HPDL native parser: parse example `.hpdl` files,
//! convert to the model, and route through the shared semantic validator.
//!
//! These tests exercise native HPDL grammar v1.0 (ratified; see GRAMMAR.md /
//! ADR 0010), which parses the full RON-equivalent surface.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use hpdl_syntax::{
    parse_and_validate, parse_file, parse_file_with_limits, Error, ParseLimits, SourceFile,
};

fn example(name: &str) -> SourceFile {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    SourceFile::from_string(name, content)
}

#[test]
fn minimal_example_parses() {
    let def = parse_file(&example("minimal.hpdl")).expect("minimal.hpdl should parse");
    assert_eq!(def.language_version(), "1.0");
    assert_eq!(def.body().id().as_str(), "proxima-b");
    assert!(
        def.body().properties().mass_kg.is_none(),
        "no Earth defaults"
    );
}

#[test]
fn earth_example_parses_with_properties_and_traits() {
    let def = parse_file(&example("earth.hpdl")).expect("earth.hpdl should parse");
    let body = def.body();
    assert_eq!(body.id().as_str(), "terra-nova");
    assert_eq!(
        body.properties().mass_kg,
        Some(5_972_000_000_000_000_000_000_000)
    );
    assert_eq!(body.properties().radius_m, Some(6_371_000));
    assert_eq!(body.properties().rotation_period_s, Some(86_164));
    assert_eq!(body.traits().len(), 2);
}

#[test]
fn gas_giant_example_parses_with_block_comment_and_units() {
    let def = parse_file(&example("gas_giant.hpdl")).expect("gas_giant.hpdl should parse");
    assert_eq!(def.body().id().as_str(), "jove-prime");
    assert_eq!(
        def.body().properties().mass_kg,
        Some(1_898_000_000_000_000_000_000_000_000)
    );
}

#[test]
fn examples_pass_semantic_validation() {
    for name in [
        "minimal.hpdl",
        "earth.hpdl",
        "gas_giant.hpdl",
        "comprehensive.hpdl",
    ] {
        let validated = parse_and_validate(&example(name))
            .unwrap_or_else(|e| panic!("{name} should parse+validate, got: {e}"));
        assert_eq!(validated.definition().language_version(), "1.0");
    }
}

#[test]
fn comprehensive_example_extracts_full_surface() {
    let def = parse_file(&example("comprehensive.hpdl")).expect("comprehensive.hpdl should parse");
    let body = def.body();
    assert_eq!(body.id().as_str(), "terra-nova");
    assert_eq!(body.statuses().len(), 1);
    assert_eq!(body.actions().len(), 1);
    assert_eq!(body.migrations().len(), 1);
    assert!(body.identity().is_some());
    assert!(body.parent().is_some());
    assert!(body.orbit().is_some());
    assert!(body.rotation().is_some());
    assert!(body.environment().is_some());
}

#[test]
fn missing_language_decl_is_parse_error() {
    let src = SourceFile::from_string("bad.hpdl", "planet x { id: \"x\" }".to_string());
    assert!(matches!(parse_file(&src), Err(Error::Parse(_))));
}

#[test]
fn oversized_source_hits_limit() {
    let src = example("earth.hpdl");
    let tiny = ParseLimits {
        max_source_bytes: 16,
        ..ParseLimits::default()
    };
    assert!(matches!(
        parse_file_with_limits(&src, &tiny),
        Err(Error::Security(_))
    ));
}

/// Build a `.hpdl` source whose `id` field value is a list nested `depth` levels
/// deep: `id: [[[…]]]`. The innermost value is an empty list. This is the shape
/// that drives the recursive-descent parser `depth` levels into `parse_value`.
fn deeply_nested_source(depth: usize) -> SourceFile {
    let mut body = String::with_capacity(depth * 2 + 64);
    body.push_str("language hpdl \"1.0\";\nplanet deep {\n    id: ");
    for _ in 0..depth {
        body.push('[');
    }
    for _ in 0..depth {
        body.push(']');
    }
    body.push_str("\n}\n");
    SourceFile::from_string("deep.hpdl", body)
}

#[test]
fn excessive_nesting_depth_fails_closed() {
    // A small depth cap must reject input nested beyond it with a typed
    // Security error rather than descending further.
    let limits = ParseLimits {
        max_nesting_depth: 8,
        ..ParseLimits::default()
    };
    let src = deeply_nested_source(64);
    assert!(matches!(
        parse_file_with_limits(&src, &limits),
        Err(Error::Security(_))
    ));
}

#[test]
fn nesting_within_depth_limit_is_accepted() {
    // Nesting at or below the cap must still parse (the value is a well-formed
    // list, so it fails later validation, but NOT with a Security/limit error).
    let limits = ParseLimits {
        max_nesting_depth: 8,
        ..ParseLimits::default()
    };
    let src = deeply_nested_source(4);
    assert!(
        !matches!(
            parse_file_with_limits(&src, &limits),
            Err(Error::Security(_))
        ),
        "nesting within the limit must not trip the depth guard"
    );
}

#[test]
fn pathological_nesting_does_not_overflow_the_stack() {
    // Regression for the audit finding: before the depth guard this input
    // recursed until the native stack overflowed and aborted the process.
    // Under the default limit it must now fail closed with a typed error and
    // the test process must survive to make this assertion.
    let src = deeply_nested_source(100_000);
    assert!(matches!(parse_file(&src), Err(Error::Security(_))));
}

#[test]
fn recovery_returns_ok_on_valid_source() {
    use hpdl_syntax::parse_source_collect_errors;
    let ast = parse_source_collect_errors(&example("earth.hpdl"))
        .expect("valid source should produce an AST with no errors");
    assert_eq!(ast.planet.identifier, "terra_nova");
}

#[test]
fn recovery_collects_multiple_field_errors() {
    use hpdl_syntax::parse_source_collect_errors;
    // Three independent field problems: `id` has no value, `foo` has a stray
    // token value, and `bar` is missing a value before the comma. A first-error
    // parser would only report one; recovery should surface several.
    let source = SourceFile::from_string(
        "multi.hpdl",
        r#"
            language hpdl "1.0";
            planet earth {
                id: ,
                mass: 42,
                foo: ],
                radius: 6371000
            }
        "#
        .to_string(),
    );

    let errors = parse_source_collect_errors(&source)
        .expect_err("source with multiple bad fields should fail");
    assert!(
        errors.len() >= 2,
        "expected multiple collected errors, got {}: {errors}",
        errors.len()
    );
    // Every error must carry a usable span (end >= start).
    for e in &errors.errors {
        assert!(e.span.end >= e.span.start);
    }
}

#[test]
fn recovery_reports_missing_language_decl() {
    use hpdl_syntax::parse_source_collect_errors;
    let source =
        SourceFile::from_string("nolang.hpdl", "planet earth { id: \"earth\" }".to_string());
    let errors = parse_source_collect_errors(&source).expect_err("missing language decl");
    assert!(!errors.is_empty());
}

#[test]
fn recovery_terminates_on_garbage() {
    use hpdl_syntax::parse_source_collect_errors;
    // Pathological input must not hang; it must return a bounded error set.
    let source = SourceFile::from_string(
        "garbage.hpdl",
        r#"language hpdl "1.0"; planet p { @ @ @ @ @ }"#.to_string(),
    );
    let result = parse_source_collect_errors(&source);
    assert!(result.is_err());
}

#[test]
fn zero_mass_source_fails_validation() {
    // Parses structurally, but zero mass fails semantic validation.
    let src = SourceFile::from_string(
        "void.hpdl",
        r#"
            language hpdl "1.0";
            planet void {
                id: "void",
                physical: { mass_kg: 0 },
            }
        "#
        .to_string(),
    );
    // Parsing succeeds...
    assert!(parse_file(&src).is_ok());
    // ...but validation rejects it.
    assert!(matches!(
        parse_and_validate(&src),
        Err(Error::Validation(_))
    ));
}
