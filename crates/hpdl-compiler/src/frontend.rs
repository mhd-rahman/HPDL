//! Frontend decode seam.
//!
//! The core pipeline operates on a frontend-neutral [`hpdl_model::PlanetDefinition`].
//! Both wired frontends — the RON adapter ([`hpdl_ron`]) and the native `.hpdl`
//! parser ([`hpdl_syntax`]) — decode into the same type, and nothing downstream
//! (resolution, lowering, emission) differs between them. Equivalent semantics in
//! either frontend therefore yield identical PIR bytes (ADR 0010).

use hpdl_model::PlanetDefinition;
use hpdl_ron::{RonLoader, SourceDocument};
use hpdl_syntax::{parse_file, SourceFile};

use crate::Error;

/// Decode RON source text into a [`PlanetDefinition`].
///
/// Delegates parsing to [`hpdl_ron`]; this crate never re-parses RON itself.
///
/// # Errors
///
/// Returns [`Error::Frontend`] if the RON frontend fails to parse the source.
pub(crate) fn decode_ron(name: &str, source: &str) -> Result<PlanetDefinition, Error> {
    let document = SourceDocument::from_string(name.to_string(), source.to_string());
    let loader = RonLoader::new();

    let parsed = loader.parse(&document).map_err(|e| Error::Frontend {
        name: name.to_string(),
        reason: e.to_string(),
    })?;

    Ok(parsed.into_definition())
}

/// Decode native `.hpdl` source text into a [`PlanetDefinition`].
///
/// Delegates parsing to [`hpdl_syntax`]; this crate never re-parses HPDL itself.
///
/// # Errors
///
/// Returns [`Error::Frontend`] if the native frontend fails to lex, parse, or
/// convert the source.
pub(crate) fn decode_hpdl(name: &str, source: &str) -> Result<PlanetDefinition, Error> {
    let file = SourceFile::from_string(name.to_string(), source.to_string());
    parse_file(&file).map_err(|e| Error::Frontend {
        name: name.to_string(),
        reason: e.to_string(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
    PlanetDefinition(
        language_version: "1.0",
        body: CelestialBody(
            id: BodyId("test-planet"),
            properties: PhysicalProperties(
                mass_kg: Some(1000000000000000000000000),
                radius_m: Some(1000000),
                surface_gravity_mms2: None,
                orbital_period_s: None,
                rotation_period_s: None,
            ),
            traits: TraitSet([]),
            modules: [],
            statuses: [],
            actions: [],
            migrations: [],
        )
    )
    "#;

    #[test]
    fn valid_ron_decodes() {
        let def = decode_ron("test", MINIMAL).unwrap();
        assert_eq!(def.language_version(), "1.0");
        assert_eq!(def.body().id().as_str(), "test-planet");
    }

    #[test]
    fn malformed_ron_is_frontend_error() {
        let err = decode_ron("bad", "this is not ron {{{").unwrap_err();
        match err {
            Error::Frontend { name, .. } => assert_eq!(name, "bad"),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
