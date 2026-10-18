//! Emit canonical PIR content digests for a fixed set of definitions.
//!
//! Used by the CI `digest-parity` job: each OS runs this example and the
//! resulting `name=<hash>` lines are diffed across platforms to prove that
//! equivalent semantics yield byte-identical canonical PIR everywhere
//! (determinism steering: "compare PIR and replay hashes across OS jobs").
//!
//! The emitter proves three distinct invariants, each on every platform:
//!
//! 1. **Cross-OS byte identity.** Every emitted `name=<hash>` line is diffed
//!    across the Linux / macOS / Windows CI legs (`digest-parity-compare`).
//! 2. **Cross-frontend byte identity.** Fixtures suffixed `*_frontends` are
//!    authored in BOTH native `.hpdl` and RON; the emitter compiles both and
//!    asserts their canonical payload bytes and hashes are identical *before*
//!    emitting a single line. A frontend divergence fails the emit step on
//!    every OS, not just the cross-OS diff (ADR 0010).
//! 3. **No silent canonical-byte drift.** Every emitted line is checked against
//!    the committed golden vector `tests/golden/digest_parity.txt`. If a change
//!    shifts canonical bytes — even identically on all three platforms — the
//!    example fails. Regenerating the vector is a deliberate, reviewed act
//!    (determinism steering: "changing canonical bytes requires a format-version
//!    decision and a reviewed vector update; never silently regenerate").
//!
//! The fixtures deliberately exercise the determinism-sensitive paths: the
//! `composition` gas map (whose key order must be canonicalized, not left to
//! `HashMap` iteration), the full orbital + rotation + thermal + hydrosphere
//! surface, large planetary magnitudes near the fixed-point storage width, and
//! a build resolved against a locked package closure.
//!
//! Output is deterministic: fixtures are compiled in a fixed order and each line
//! is `<name>=<64-hex content hash>`. No wall-clock, locale, filesystem order, or
//! hashmap iteration reaches the output.
//!
//! ## Regenerating the golden vector
//!
//! When canonical bytes change *intentionally* (a format-version decision, per
//! the determinism steering), regenerate the committed vector with:
//!
//! ```text
//! HPDL_BLESS_DIGESTS=1 cargo run -p hpdl-compiler --example emit_digests
//! ```
//!
//! and review the diff to `tests/golden/digest_parity.txt` before committing.

use std::fmt::Write as _;
use std::path::Path;

use hpdl_compiler::{CompileOptions, Compiler, Lockfile};

/// Path to the committed golden vector, relative to this crate's manifest dir.
const GOLDEN_REL: &str = "tests/golden/digest_parity.txt";

/// A minimal planet with no module references — the historical baseline.
const MINIMAL: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("earth"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),
            radius_m: Some(6371000),
            surface_gravity_mms2: Some(9810),
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

/// A planet carrying traits and a full set of physical fields, including a
/// signed (retrograde) rotation period and large magnitudes near the storage
/// width — exercising the representability guard on every platform.
const TRAITS: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("mars"),
        properties: PhysicalProperties(
            mass_kg: Some(641710000000000000000000),
            radius_m: Some(3389500),
            surface_gravity_mms2: Some(3720),
            orbital_period_s: Some(59354294),
            rotation_period_s: Some(88642),
        ),
        traits: TraitSet(["rocky", "thin-atmosphere"]),
        modules: [],
        statuses: [],
        actions: [],
        migrations: [],
    )
)
"#;

/// The determinism-sensitive fixture, authored in native HPDL.
///
/// Exercises the gas `composition` map (canonical key order), the full orbital /
/// rotation / thermal / hydrosphere / cryosphere surface, statuses, actions with
/// capabilities, and migrations. Paired byte-for-byte with [`ENV_RON`].
const ENV_HPDL: &str = r#"
language hpdl "1.0";
planet terra {
    id: "terra",
    physical: {
        mass_kg: 5972000000000000000000000,
        radius_m: 6371000,
    },
    traits: { tags: ["terrestrial"] },
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

/// The same planet as [`ENV_HPDL`], authored in RON. Compiling both and
/// comparing proves cross-frontend byte identity on every platform.
const ENV_RON: &str = r#"
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
        modules: [],
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

/// A planet declaring a module reference (`hpdl.climate`), compiled against a
/// locked package closure. Resolution does not change PIR bytes, but it drives
/// the deterministic package resolver on every platform.
const WITH_CLIMATE: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("earth"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),
            radius_m: Some(6371000),
            surface_gravity_mms2: Some(9810),
            orbital_period_s: None,
            rotation_period_s: None,
        ),
        traits: TraitSet([]),
        modules: [ModuleRef(name: "hpdl.climate", version: "2.0.0")],
        statuses: [],
        actions: [],
        migrations: [],
    )
)
"#;

/// The locked package closure for [`WITH_CLIMATE`]: `hpdl.climate 2.0.0`
/// depending on `hpdl.atmosphere 1.2.0`. Digests are fixed hex strings so the
/// closure resolves identically and offline on every platform.
fn climate_lock() -> Result<Lockfile, String> {
    let a = "a".repeat(64);
    let b = "b".repeat(64);
    let text = format!(
        "lock_format_version = 1\n\
         package = hpdl.climate 2.0.0 {a} hpdl.atmosphere\n\
         package = hpdl.atmosphere 1.2.0 {b}\n"
    );
    Lockfile::parse(&text).map_err(|e| format!("climate lock parse: {e}"))
}

/// Compile a single-frontend fixture and return its content hash.
fn ron_digest(compiler: &Compiler, name: &str, source: &str) -> Result<String, String> {
    compiler
        .compile_ron_str(name, source)
        .map(|r| r.content_hash_hex())
        .map_err(|e| format!("error compiling `{name}` (RON): {e}"))
}

/// Compile a locked-package fixture and return its content hash.
fn locked_digest(name: &str, source: &str, lock: Lockfile) -> Result<String, String> {
    let compiler = Compiler::new(CompileOptions::default().with_package_lock(lock));
    compiler
        .compile_ron_str(name, source)
        .map(|r| r.content_hash_hex())
        .map_err(|e| format!("error compiling `{name}` (locked): {e}"))
}

/// Compile a dual-frontend fixture through BOTH frontends, assert byte identity,
/// and return the (single) content hash. Fails if the frontends diverge.
fn frontend_parity_digest(
    compiler: &Compiler,
    name: &str,
    hpdl_src: &str,
    ron_src: &str,
) -> Result<String, String> {
    let from_hpdl = compiler
        .compile_hpdl_str(name, hpdl_src)
        .map_err(|e| format!("error compiling `{name}` (HPDL): {e}"))?;
    let from_ron = compiler
        .compile_ron_str(name, ron_src)
        .map_err(|e| format!("error compiling `{name}` (RON): {e}"))?;

    if from_hpdl.envelope().payload != from_ron.envelope().payload {
        return Err(format!(
            "`{name}`: canonical PIR payload bytes differ between native HPDL and RON \
             frontends — cross-frontend determinism violated (ADR 0010)"
        ));
    }
    if from_hpdl.envelope().content_hash != from_ron.envelope().content_hash {
        return Err(format!(
            "`{name}`: content hashes differ between native HPDL and RON frontends"
        ));
    }
    Ok(from_hpdl.content_hash_hex())
}

/// Build the full, deterministically ordered digest report.
fn build_report() -> Result<String, String> {
    let compiler = Compiler::new(CompileOptions::default());

    // Fixed order — never a HashMap. Each entry yields exactly one output line.
    let lines = [
        ("minimal", ron_digest(&compiler, "minimal", MINIMAL)?),
        ("traits", ron_digest(&compiler, "traits", TRAITS)?),
        (
            "environment_frontends",
            frontend_parity_digest(&compiler, "terra", ENV_HPDL, ENV_RON)?,
        ),
        (
            "locked_closure",
            locked_digest("earth", WITH_CLIMATE, climate_lock()?)?,
        ),
    ];

    let mut report = String::new();
    for (name, hash) in lines {
        // Infallible write into a String.
        let _ = writeln!(report, "{name}={hash}");
    }
    Ok(report)
}

fn main() {
    let report = match build_report() {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // Always print the report to stdout: CI uploads and diffs it across OSes.
    print!("{report}");

    let golden_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN_REL);

    // Deliberate regeneration path (format-version decisions only).
    if std::env::var_os("HPDL_BLESS_DIGESTS").is_some() {
        if let Err(e) = std::fs::write(&golden_path, &report) {
            eprintln!(
                "failed to write golden vector {}: {e}",
                golden_path.display()
            );
            std::process::exit(1);
        }
        eprintln!("blessed golden vector: {}", golden_path.display());
        return;
    }

    // Self-check against the committed golden vector: catch canonical-byte drift
    // that is identical across all platforms (which the cross-OS diff cannot).
    let golden = match std::fs::read_to_string(&golden_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!(
                "failed to read golden vector {}: {e}\n\
                 (run with HPDL_BLESS_DIGESTS=1 to create it after a reviewed \
                 format-version decision)",
                golden_path.display()
            );
            std::process::exit(1);
        }
    };

    // Compare on normalized line endings so a checkout with CRLF on Windows does
    // not spuriously fail; canonical bytes are compared via the hashes themselves.
    if normalize(&report) != normalize(&golden) {
        eprintln!(
            "canonical digests drifted from the committed golden vector {}.\n\
             expected:\n{}\n---\nactual:\n{}\n---\n\
             If this change is intentional, it requires a format-version decision \
             (determinism steering). Re-bless deliberately with \
             HPDL_BLESS_DIGESTS=1 and review the diff.",
            golden_path.display(),
            golden.trim_end(),
            report.trim_end(),
        );
        std::process::exit(1);
    }
}

/// Normalize line endings for comparison (CRLF/CR -> LF).
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}
