# HPDL Native Syntax — Implemented Grammar (NOT ratified)

> **Status: illustrative.** The HPDL language grammar is **not yet ratified**. This
> document describes only the subset of syntax the current `hpdl-syntax` parser
> actually accepts, so examples and tests have something concrete to target. It is
> **not** a normative language specification and may change. Do not treat any
> construct here as a committed language feature.

## Lexical structure

- **Encoding:** UTF-8 source text.
- **Whitespace:** spaces, tabs, carriage returns, and newlines are insignificant
  except as token separators.
- **Comments:** line comments `// ... <eol>` and block comments `/* ... */`.
- **Keywords:** `language`, `hpdl`, `planet`, `body`, `true`, `false` (and the other
  reserved words recognized by the lexer).
- **Identifiers:** `[A-Za-z_][A-Za-z0-9_-]*`.
- **String literals:** double-quoted, with escapes `\n \t \r \\ \"`.
- **Integer literals:** decimal digits with optional `_` separators (e.g.
  `1_000_000`). Parsed as `i128` to preserve planetary magnitudes exactly.
- **Float literals:** digits with a decimal point and/or scientific notation
  (`3.14`, `1e10`). (Floats are accepted lexically but canonical storage is integer;
  see note below.)
- **Punctuation:** `:` `,` `;` `{` `}` `[` `]` `(` `)`.

## Grammatical structure (EBNF-ish)

```
file            = language_decl planet_decl EOF ;
language_decl   = "language" "hpdl" string ";" ;
planet_decl     = "planet" identifier "{" field_list "}" ;
field_list      = [ field { "," field } [ "," ] ] ;   // trailing comma allowed
field           = identifier ":" value ;
value           = string
                | number [ unit ]
                | boolean
                | list
                | object ;
list            = "[" [ value { "," value } [ "," ] ] "]" ;
object          = "{" field_list "}" ;
unit            = identifier ;                          // optional suffix after a number
boolean         = "true" | "false" ;
```

## Model mapping (what conversion currently reads)

The AST is converted to `hpdl_model::PlanetDefinition`. The fields currently
recognized on the top-level `planet` object are:

| HPDL field | Model target | Notes |
|---|---|---|
| `id` (string) | `BodyId` | required |
| `physical` (object) | `PhysicalProperties` | optional; absent → all `None` |
| `physical.mass_kg` (int) | `mass_kg: u128` | optional |
| `physical.radius_m` (int) | `radius_m: u128` | optional |
| `physical.surface_gravity_mms2` (int) | `surface_gravity_mms2: u64` | optional |
| `physical.orbital_period_s` (int) | `orbital_period_s: u64` | optional |
| `physical.rotation_period_s` (int) | `rotation_period_s: i64` | optional; negative = retrograde |
| `traits` (object with `tags: [string]`) | `TraitSet` | optional |

### Not yet wired through conversion

`modules`, `statuses`, `actions`, and `migrations` are **not** currently extracted
from HPDL source (the converter returns empty collections for them). This is a known
limitation, not a language decision. The RON frontend supports these today; the native
syntax will gain them once the grammar for those blocks is ratified.

### Units and floats

A unit identifier may follow a number (e.g. `6371000 m`). The current converter stores
canonical **scaled integers** and does not perform unit conversion; the unit suffix is
accepted but not yet used to rescale. Floats are rejected by the integer-typed physical
fields — physical quantities must be given as integers in canonical units. This keeps
canonical output deterministic (no floats on the canonical path).

## Example

```hpdl
language hpdl "1.0";

planet terra_nova {
    id: "terra-nova",
    physical: {
        mass_kg: 5972000000000000000000000,
        radius_m: 6371000,
    },
    traits: {
        tags: ["terrestrial", "habitable-zone"],
    },
}
```

See `examples/*.hpdl` for runnable samples and `tests/integration_tests.rs` for the
behaviour these guarantee.
