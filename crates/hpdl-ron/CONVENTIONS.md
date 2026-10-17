# RON Authoring Conventions

HPDL RON files mirror the `hpdl_model` type shapes via serde. The top-level value
is a `PlanetDefinition` containing a single `CelestialBody`.

## Shape

```ron
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("my-planet"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),  // kilograms, or None if undeclared
            radius_m: Some(6371000),                    // metres
            surface_gravity_mms2: Some(9810),           // milli-m/s^2
            orbital_period_s: Some(31557600),           // seconds
            rotation_period_s: Some(86164),             // seconds; negative = retrograde
        ),
        traits: TraitSet(["terrestrial", "habitable-zone"]),
        modules: [
            ModuleRef(name: "hpdl/stdlib/atmospheric", version: "1.0"),
        ],
        statuses: [
            StatusDeclaration(name: "AtmosphericStability", scale_min: 0, scale_max: 100),
        ],
        actions: [
            ActionDeclaration(
                name: "AdjustGreenhouse",
                reads: ["state.atmosphere.co2"],
                writes: ["state.atmosphere.co2"],
                // Every mutating write must be covered by a declared capability
                // (feature 10). Capability is a newtype, so RON needs the
                // `Capability("...")` form, not a bare string.
                capabilities: [Capability("state.atmosphere.write")],
            ),
        ],
        migrations: [
            MigrationDeclaration(name: "add-cryosphere", from_version: 1, to_version: 2),
        ],
        // The following orbital fields (feature 07) are optional and default to
        // absent if omitted:
        // identity: Some(Planet),
        // parent: Some(ParentRef(target: BodyId("sol"), frame: EclipticJ2000)),
        // orbit: Some(KeplerianElements(...)),
        // rotation: Some(RotationModel(...)),
    )
)
```

## Rules of thumb

- **No Earth defaults.** An omitted or `None` property means *undeclared*, not a
  default value. Only declare what you mean.
- **Numbers are integers.** Continuous quantities use scaled integers (no floats):
  gravity in milli-m/s², eccentricity as `e × 1e6`, angles in milli-degrees.
- **Statuses are derived.** You may declare a `StatusDeclaration` (name + scale),
  but actions must never list a `status.*` field in `writes` — the validator rejects
  it.
- **Retrograde rotation** is a negative `rotation_period_s` / sidereal period.

## Extension fields

Tool-specific metadata uses the namespaced `_ext::` prefix and is ignored by
semantic validation and PIR generation. See
[`docs/rfcs/0002-ron-extension-fields.md`](../../docs/rfcs/0002-ron-extension-fields.md)
for the full design.

## Validation vs. parsing

Parsing checks only that the RON is well-formed and matches the model shape.
Semantic correctness (physical constraints, status thresholds, parent-graph
acyclicity, …) is enforced separately by `ParsedDefinition::validate()`, which runs
the shared `hpdl-sema` pipeline. A file can parse yet fail validation.
