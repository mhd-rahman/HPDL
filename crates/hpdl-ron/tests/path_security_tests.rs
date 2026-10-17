//! Path-security tests: loading from a file path must reject traversal attempts.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use hpdl_ron::{Error, RonLoader, SourceDocument};

#[test]
fn parent_dir_traversal_is_rejected_from_source_document() {
    let result = SourceDocument::from_file("../../../etc/passwd");
    assert!(matches!(result, Err(Error::PathTraversal(_))));
}

#[test]
fn parent_dir_traversal_is_rejected_from_loader() {
    let loader = RonLoader::new();
    let result = loader.load_file("../secrets.ron");
    assert!(matches!(result, Err(Error::PathTraversal(_))));
}

#[test]
fn nonexistent_plain_path_is_io_error_not_traversal() {
    // A path with no traversal components that doesn't exist should surface as an
    // I/O error, not a path-traversal error.
    let loader = RonLoader::new();
    let result = loader.load_file("definitely_missing_planet_file.ron");
    assert!(matches!(result, Err(Error::Io(_))));
}
