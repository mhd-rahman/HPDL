# HPDL Native Syntax — Grammar v1.0 (RATIFIED)

> **Status: ratified.** This is the normative grammar for native `.hpdl` source at
> language version `1.0` (the string in `language hpdl "1.0";`). It was ratified in
> [ADR 0010](../../docs/rfcs/0010-native-hpdl-grammar-v1.md) at full parity with the
> RON frontend's serde-reachable model surface: an `.hpdl` source and its RON
> equivalent produce an identical `PlanetDefinition`, identical validator
> diagnostics, and byte-identical canonical PIR.
>
> Changes to this grammar are versioned: additive syntax is a grammar **minor** bump
> (`1.1`), a breaking change is `2.0`. World laws are intentionally **not** part of
> v1.0 (see "Out of scope").

## Lexical structure

- **Encoding:** UTF-8 source text.
- **Whitespace:** spaces, tabs, carriage returns, and newlines are insignificant
  except as token separators.
- **Comments:** line comments `// … <eol>` and block comments `/* … */`.
- **Keywords:** `language`, `hpdl`, `planet`, `body`, `physical`, `traits`,
  `status`, `action`, `module`, `migration`, `true`, `false`. The contextual
  keywords may also appear as field names and as variant/newtype names.
- **Identifiers:** `[A-Za-z_][A-Za-z0-9_-]*`.
- **String literals:** double-quoted, with escapes `\n \t \r \\ \"`. `\uXXXX` and
  other escapes are **reserved** (rejected) in v1.0.
- **Integer literals:** **decimal** digits with optional `_` separators (e.g.
  `1_000_000`). Parsed as `i128` so planetary magnitudes survive exactly. Hex,
  binary, and octal bases are **reserved** (rejected) in v1.0.
- **Float literals:** accepted lexically (`3.14`, `1e10`) but **rejected during
  conversion** for every canonical numeric field — physical quantities must be given
  as integers in canonical units (no floats on the canonical path).
- **Punctuation:** `:` `,` `;` `{` `}` `[` `]` `(` `)`.

## Grammatical structure (EBNF)

```
file            = language_decl planet_decl EOF ;
language_decl   = "language" "hpdl" string ";" ;
planet_decl     = "planet" identifier "{" field_list "}" ;
field_list      = [ field { "," field } [ "," ] ] ;      // trailing comma allowed
field           = field_name ":" value ;
field_name      = identifier | contextual_keyword ;

value           = string
                | number [ unit ]
                | boolean
                | list
                | tuple
                | object
                | variant ;

list            = "[" [ value { "," value } [ "," ] ] "]" ;
tuple           = "(" [ value { "," value } [ "," ] ] ")" ;
object          = "{" field_list "}" ;

variant         = variant_name [ "(" variant_args ")" ] ;
variant_name    = identifier | contextual_keyword ;
variant_args    = field_list                              // struct variant: Name(f: v, …)
                | value { "," value } [ "," ] ;           // tuple/newtype: Name(v, …)

unit            = identifier ;                            // optional suffix after a number
boolean         = "true" | "false" ;
```

### Variants and newtypes (RON parity)

The model's closed enums and validated newtypes are written exactly as in RON:

| Form | Example | Meaning |
|---|---|---|
| Unit variant | `Planet`, `EclipticJ2000`, `Absent` | an enum variant with no payload |
| Newtype / tuple variant | `Custom("arcane")`, `Present(Microbial)`, `Capability("state.x.write")` | a variant carrying positional values |
| Struct variant | `ModuleRef(name: "a", version: "1.0")` | a variant carrying named fields |
| Optional sugar | `None`, `Some(Planet)` | an absent / present optional |
| Pair | `(25, "degraded")`, `(GasId("n2"), 780000)` | a two-element tuple |

A bare capitalized word in value position is a unit variant; the same word
followed by `(…)` is an applied variant. `None`/`Some(x)` are recognized for
optional fields. Digests may be written either as `Digest("…")` or as a bare
string literal.

## Model mapping (the full v1.0 surface)

Conversion targets `hpdl_model::PlanetDefinition`. All fields on the top-level
`planet` object:

| HPDL field | Model target | Notes |
|---|---|---|
| `id` (string) | `BodyId` | **required** |
| `physical` (object) | `PhysicalProperties` | optional; integer fields `mass_kg`, `radius_m`, `surface_gravity_mms2`, `orbital_period_s`, `rotation_period_s` |
| `traits` (object with `tags: [string]`) | `TraitSet` | optional |
| `modules` (list of `ModuleRef(…)`) | `Vec<ModuleRef>` | optional |
| `statuses` (list of `StatusDeclaration(…)`) | `Vec<StatusDeclaration>` | optional; supports `dimension`, `baseline`, `evaluator`, `cut_points`, `window` |
| `actions` (list of `ActionDeclaration(…)`) | `Vec<ActionDeclaration>` | optional; `reads`, `writes`, `capabilities`; a `status.*` write is rejected |
| `migrations` (list of `MigrationDeclaration(…)`) | `Vec<MigrationDeclaration>` | optional; forward-only |
| `identity` (`CelestialIdentity`) | `Option<CelestialIdentity>` | optional |
| `parent` (`ParentRef(…)`) | `Option<ParentRef>` | optional |
| `orbit` (`KeplerianElements(…)`) | `Option<KeplerianElements>` | optional; scaled integers |
| `rotation` (`RotationModel(…)`) | `Option<RotationModel>` | optional |
| `environment` (`EnvironmentDefinition(…)`) | `Option<EnvironmentDefinition>` | optional; atmosphere, thermal, hydro/cryosphere, biosphere, assets |

Absence means **undeclared**, never an Earth default. All continuous quantities are
scaled integers (gravity in milli-m/s², eccentricity as `e × 1e6`, angles in
milli-degrees, pressure in Pa, temperature in milli-kelvin, coverage/mole fractions
in ppm).

### Units and floats

A unit identifier may follow a number (e.g. `6371000 m`). The unit is **advisory**:
the converter stores canonical scaled integers and does not rescale from the suffix.
A unit that contradicts the field's canonical unit is rejected. Floats are rejected
by the integer-typed fields — this keeps canonical output deterministic.

## Resource limits (untrusted input)

Source size ≤ 10 MiB, nesting depth ≤ 64, collection length ≤ 10 000, identifier
length ≤ 256. Exceeding a limit fails closed with a typed `Error::Security`.

## Example

```hpdl
language hpdl "1.0";

planet terra_nova {
    id: "terra-nova",
    physical: {
        mass_kg: 5972000000000000000000000,
        radius_m: 6371000,
    },
    traits: { tags: ["terrestrial", "habitable-zone"] },
    identity: Planet,
    parent: ParentRef(target: BodyId("sol"), frame: EclipticJ2000),
    statuses: [
        StatusDeclaration(name: "Vitality", scale_min: 0, scale_max: 100),
    ],
    actions: [
        ActionDeclaration(
            name: "AdjustGreenhouse",
            reads: ["state.atmosphere.co2"],
            writes: ["state.atmosphere.co2"],
            capabilities: [ Capability("state.atmosphere.write") ],
        ),
    ],
    environment: EnvironmentDefinition(
        biosphere: Present(Microbial),
        atmosphere: AtmosphereProfile(
            surface_pressure_pa: 101325,
            composition: Composition(
                components: [ (GasId("n2"), 780000), (GasId("o2"), 220000) ],
                tolerance_ppm: 0,
            ),
        ),
    ),
}
```

See `examples/*.hpdl` for runnable samples, `tests/cross_frontend_tests.rs` for the
HPDL↔RON model/diagnostic parity guarantees, and
`../hpdl-compiler/tests/hpdl_ron_equivalence.rs` for the byte-identical PIR guarantee.

## Out of scope (v1.0)

- **World laws / magic / alternative physics.** The model's `world_laws` field is
  `#[serde(skip)]` (ADR 0005) and expressible in neither serialized frontend;
  bringing it into native syntax is coupled to a PIR format-version decision and is
  deferred to a later grammar minor version.
- **`\uXXXX` string escapes, non-decimal number bases.** Reserved; rejected in v1.0.
- A canonical formatter and an LSP remain out of scope.
