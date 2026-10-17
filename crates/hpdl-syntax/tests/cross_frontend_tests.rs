//! Cross-frontend equivalence tests (feature 03, Task 11).
//!
//! Both the native HPDL parser (`hpdl-syntax`) and the RON adapter (`hpdl-ron`)
//! converge on the same `hpdl_model::PlanetDefinition` and the same `hpdl-sema`
//! validator. These tests assert that *equivalent* source in the two formats
//! produces:
//!   1. identical `PlanetDefinition` models, and
//!   2. identical semantic-validation outcomes (diagnostic codes, messages, and
//!      order), or identical success.
//!
//! ## Placement rationale
//!
//! The feature-03 task originally named `hpdl-sema/tests/cross_frontend_tests.rs`,
//! but `hpdl-sema` must NOT depend on either frontend (both depend on it; the
//! reverse would be a dependency cycle). This test therefore lives in
//! `hpdl-syntax`, which can take `hpdl-ron` as a dev-dependency safely because
//! nothing depends on `hpdl-syntax`.
//!
//! ## Equivalence surface
//!
//! Native HPDL grammar v1.0 is ratified at **full parity** with the serde-reachable
//! model surface (ADR 0010): `id`, `physical`, `traits`, `modules`, `statuses`
//! (incl. the optional vitality fields), `actions` (incl. capabilities),
//! `migrations`, and the optional `identity`/`parent`/`orbit`/`rotation`/
//! `environment` blocks. The `comprehensive_parity_*` tests exercise all of them.
//!
//! World laws are out of both serialized frontends (`#[serde(skip)]`, ADR 0005 /
//! ADR 0010), so they are not part of the equivalence surface.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use hpdl_model::PlanetDefinition;
use hpdl_ron::{RonLoader, SourceDocument};
use hpdl_sema::{Diagnostic, Validator};
use hpdl_syntax::{parse_file, SourceFile};

/// Parse the HPDL form into a model.
fn via_hpdl(src: &str) -> PlanetDefinition {
    let source = SourceFile::from_string("equiv.hpdl", src.to_string());
    parse_file(&source).expect("hpdl source should parse")
}

/// Parse the RON form into a model.
fn via_ron(src: &str) -> PlanetDefinition {
    let loader = RonLoader::new();
    let source = SourceDocument::from_string("equiv.ron", src.to_string());
    loader
        .parse(&source)
        .expect("ron source should parse")
        .into_definition()
}

/// Render a validation result to a comparable form: `Ok` or the ordered list of
/// `(code, message)` pairs. Diagnostics are already sorted deterministically by
/// the validator, so equal inputs yield equal rendered output.
fn validate_rendered(def: PlanetDefinition) -> Result<(), Vec<(String, String)>> {
    let validator = Validator::default_config();
    match validator.validate(def) {
        Ok(_) => Ok(()),
        Err(diags) => Err(diags
            .iter()
            .map(|d: &Diagnostic| (d.code.code().to_string(), d.message.clone()))
            .collect()),
    }
}

/// A valid Earth-like planet expressed in both formats. The two sources are
/// written to be semantically identical over the shared subset.
mod earth {
    pub const HPDL: &str = r#"
        language hpdl "1.0";
        planet terra {
            id: "terra",
            physical: {
                mass_kg: 5972000000000000000000000,
                radius_m: 6371000,
                surface_gravity_mms2: 9810,
                orbital_period_s: 31557600,
                rotation_period_s: 86164,
            },
            traits: { tags: ["terrestrial", "habitable-zone"] },
        }
    "#;

    pub const RON: &str = r#"
        PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("terra"),
                properties: PhysicalProperties(
                    mass_kg: Some(5972000000000000000000000),
                    radius_m: Some(6371000),
                    surface_gravity_mms2: Some(9810),
                    orbital_period_s: Some(31557600),
                    rotation_period_s: Some(86164),
                ),
                traits: TraitSet(["terrestrial", "habitable-zone"]),
                modules: [],
                statuses: [],
                actions: [],
                migrations: [],
            )
        )
    "#;
}

/// A semantically-invalid planet (zero mass) in both formats.
mod zero_mass {
    pub const HPDL: &str = r#"
        language hpdl "1.0";
        planet void {
            id: "void",
            physical: { mass_kg: 0 },
        }
    "#;

    pub const RON: &str = r#"
        PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("void"),
                properties: PhysicalProperties(
                    mass_kg: Some(0),
                    radius_m: None,
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
}

#[test]
fn valid_definition_models_are_identical() {
    let from_hpdl = via_hpdl(earth::HPDL);
    let from_ron = via_ron(earth::RON);
    assert_eq!(
        from_hpdl, from_ron,
        "equivalent HPDL and RON sources must produce identical models"
    );
}

#[test]
fn valid_definition_validates_identically() {
    let hpdl_result = validate_rendered(via_hpdl(earth::HPDL));
    let ron_result = validate_rendered(via_ron(earth::RON));
    assert_eq!(hpdl_result, ron_result);
    assert!(
        hpdl_result.is_ok(),
        "the valid planet should validate clean"
    );
}

#[test]
fn invalid_definition_models_are_identical() {
    let from_hpdl = via_hpdl(zero_mass::HPDL);
    let from_ron = via_ron(zero_mass::RON);
    assert_eq!(from_hpdl, from_ron);
}

#[test]
fn invalid_definition_produces_identical_diagnostics() {
    let hpdl_result = validate_rendered(via_hpdl(zero_mass::HPDL));
    let ron_result = validate_rendered(via_ron(zero_mass::RON));

    // Both must fail, with the identical (code, message) sequence in the same order.
    assert_eq!(
        hpdl_result, ron_result,
        "equivalent invalid sources must yield identical diagnostics"
    );
    let diags = hpdl_result.expect_err("zero mass must fail validation");
    assert!(
        diags.iter().any(|(code, _)| code == "E2001"),
        "expected the NegativeMass diagnostic (E2001), got {diags:?}"
    );
}

#[test]
fn diagnostic_order_matches_across_frontends() {
    // A definition with no declared physical values is trivially valid in both;
    // the point here is that an identical (empty) diagnostic sequence is produced.
    const HPDL: &str = r#"
        language hpdl "1.0";
        planet bare { id: "bare" }
    "#;
    const RON: &str = r#"
        PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("bare"),
                properties: PhysicalProperties(
                    mass_kg: None, radius_m: None, surface_gravity_mms2: None,
                    orbital_period_s: None, rotation_period_s: None,
                ),
                traits: TraitSet([]),
                modules: [], statuses: [], actions: [], migrations: [],
            )
        )
    "#;

    assert_eq!(via_hpdl(HPDL), via_ron(RON));
    assert_eq!(
        validate_rendered(via_hpdl(HPDL)),
        validate_rendered(via_ron(RON))
    );
}

/// A comprehensive definition exercising every block native HPDL grammar v1.0
/// now supports, expressed in both formats. Proves full-parity extraction
/// (ADR 0010), replacing the old "shared subset" boundary test.
mod comprehensive {
    pub const HPDL: &str = r#"
        language hpdl "1.0";
        planet terra {
            id: "terra",
            physical: {
                mass_kg: 5972000000000000000000000,
                radius_m: 6371000,
            },
            traits: { tags: ["terrestrial"] },
            modules: [ ModuleRef(name: "hpdl/stdlib/atmo", version: "1.0") ],
            statuses: [
                StatusDeclaration(name: "Vitality", scale_min: 0, scale_max: 100),
            ],
            actions: [
                ActionDeclaration(
                    name: "Terraform",
                    reads: ["state.atmosphere.co2"],
                    writes: ["state.atmosphere.co2"],
                    capabilities: [ Capability("state.atmosphere.write") ],
                ),
            ],
            migrations: [
                MigrationDeclaration(name: "add-ocean", from_version: 1, to_version: 2),
            ],
            identity: Planet,
            parent: ParentRef(target: BodyId("sol"), frame: EclipticJ2000),
            orbit: KeplerianElements(
                semi_major_axis_m: 149598000000,
                eccentricity_micro: 16700,
                inclination_mdeg: 0,
                lon_ascending_node_mdeg: 174000,
                arg_periapsis_mdeg: 288000,
                mean_anomaly_epoch_mdeg: 0,
                frame: EclipticJ2000,
            ),
            rotation: RotationModel(
                sidereal_period_s: 86164,
                axial_tilt_mdeg: 23440,
                frame: EclipticJ2000,
            ),
            environment: EnvironmentDefinition(
                biosphere: Present(Microbial),
                atmosphere: AtmosphereProfile(
                    surface_pressure_pa: 101325,
                    composition: Composition(
                        components: [ (GasId("n2"), 780000), (GasId("o2"), 220000) ],
                        tolerance_ppm: 0,
                    ),
                ),
                thermal: ThermalProfile(mean_mk: 288000, min_mk: 184000, max_mk: 331000),
                hydrosphere: Hydrosphere(surface_coverage_ppm: 708000),
                cryosphere: Cryosphere(surface_coverage_ppm: 100000),
            ),
        }
    "#;

    pub const RON: &str = r#"
        PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("terra"),
                properties: PhysicalProperties(
                    mass_kg: Some(5972000000000000000000000),
                    radius_m: Some(6371000),
                    surface_gravity_mms2: None,
                    orbital_period_s: None,
                    rotation_period_s: None,
                ),
                traits: TraitSet(["terrestrial"]),
                modules: [ ModuleRef(name: "hpdl/stdlib/atmo", version: "1.0") ],
                statuses: [
                    StatusDeclaration(name: "Vitality", scale_min: 0, scale_max: 100),
                ],
                actions: [
                    ActionDeclaration(
                        name: "Terraform",
                        reads: ["state.atmosphere.co2"],
                        writes: ["state.atmosphere.co2"],
                        capabilities: [ Capability("state.atmosphere.write") ],
                    ),
                ],
                migrations: [
                    MigrationDeclaration(name: "add-ocean", from_version: 1, to_version: 2),
                ],
                identity: Some(Planet),
                parent: Some(ParentRef(target: BodyId("sol"), frame: EclipticJ2000)),
                orbit: Some(KeplerianElements(
                    semi_major_axis_m: 149598000000,
                    eccentricity_micro: 16700,
                    inclination_mdeg: 0,
                    lon_ascending_node_mdeg: 174000,
                    arg_periapsis_mdeg: 288000,
                    mean_anomaly_epoch_mdeg: 0,
                    frame: EclipticJ2000,
                )),
                rotation: Some(RotationModel(
                    sidereal_period_s: 86164,
                    axial_tilt_mdeg: 23440,
                    frame: EclipticJ2000,
                )),
                environment: Some(EnvironmentDefinition(
                    biosphere: Present(Microbial),
                    atmosphere: Some(AtmosphereProfile(
                        surface_pressure_pa: 101325,
                        composition: Composition(
                            components: { GasId("n2"): 780000, GasId("o2"): 220000 },
                            tolerance_ppm: 0,
                        ),
                    )),
                    thermal: Some(ThermalProfile(mean_mk: 288000, min_mk: 184000, max_mk: 331000)),
                    hydrosphere: Some(Hydrosphere(surface_coverage_ppm: 708000, dominant_liquid: None)),
                    cryosphere: Some(Cryosphere(surface_coverage_ppm: 100000)),
                    assets: {},
                )),
            )
        )
    "#;
}

#[test]
fn comprehensive_parity_models_are_identical() {
    let from_hpdl = via_hpdl(comprehensive::HPDL);
    let from_ron = via_ron(comprehensive::RON);
    assert_eq!(
        from_hpdl, from_ron,
        "full-surface HPDL and RON sources must produce identical models"
    );
}

#[test]
fn comprehensive_parity_validates_identically() {
    let hpdl_result = validate_rendered(via_hpdl(comprehensive::HPDL));
    let ron_result = validate_rendered(via_ron(comprehensive::RON));
    assert_eq!(
        hpdl_result, ron_result,
        "full-surface equivalent sources must validate identically"
    );
}

/// The native frontend extracts every block (the inverse of the retired
/// "native HPDL extracts zero statuses" boundary assertion).
#[test]
fn native_hpdl_extracts_the_full_surface() {
    let def = via_hpdl(comprehensive::HPDL);
    let body = def.body();
    assert_eq!(body.modules().len(), 1);
    assert_eq!(body.statuses().len(), 1);
    assert_eq!(body.actions().len(), 1);
    assert_eq!(body.migrations().len(), 1);
    assert!(body.identity().is_some());
    assert!(body.parent().is_some());
    assert!(body.orbit().is_some());
    assert!(body.rotation().is_some());
    assert!(body.environment().is_some());
}
