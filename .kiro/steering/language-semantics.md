---
inclusion: fileMatch
fileMatchPattern: ["crates/hpdl-model/**", "crates/hpdl-sema/**", "crates/hpdl-ron/**", "crates/hpdl-syntax/**", "**/*.hpdl", "**/*.ron", "stdlib/**"]
---

# HPDL — Language & Semantics

## Source
`.hpdl`, UTF-8, explicit `language hpdl "1.0";`. Grammar is NOT yet ratified: treat all example syntax as illustrative. Do not invent syntax or bend semantics to fit an example.

## Absence
`Absent`, `Unknown` and `Generated` are distinct typed states. Missing values never imply Earth defaults.

## Units
- SI is canonical. `Temperature` and `TemperatureDelta` are distinct types.
- Custom dimensions (e.g. `arcane::Aether`) are namespaced and versioned, and never convert to SI without a declared world-law bridge.
- Reject NaN, infinities and implicit float coercion.

## Composition
Schema composition is intersection: compatible constraints narrow; contradictions fail compilation. No last-write-wins.

## Status thresholds (Amendment A1)
Ordered lower-bound cut points over a bounded scale. Each band is `[lower, next)`; the last band includes the maximum.

✅ Valid:
```
scale: 0..=100
thresholds { critical: 0  degraded: 25  stressed: 50  stable: 75 }
// [0,25) [25,50) [50,75) [75,100]
```
❌ Reject: unsorted, duplicate, out-of-range, or incomplete cut points (e.g. first cut ≠ scale minimum). Out-of-range scores are rejected, never clamped.

## Statuses
- Derived from state by versioned evaluators; never writable fields.
- Evaluators declare version, inputs, baseline, window, scope, thresholds, evidence output.
- Atmospheric stability ≠ habitability ≠ pollution.
- The 12 atmosphere components are inputs/subsystems, not 12 weighted scores. Never claim illustrative weights are scientifically validated.

## Actions
Declare `reads` and `writes`. Undeclared writes fail closed. Actions never assign status values.

❌ Reject at compile time:
```
action Bad { writes: [status.AtmosphericStability]; }
```

## World laws & magic
Optional, registered, versioned, deterministic modules. Modes: `additive` or explicit replacement of named domains. One authoritative law per exclusive domain. Magic affects physical state only through a declared bridge. No universal magic system.

## Immutable vs mutable
Definitions and PIR are immutable artifacts. A declaration never executes or simulates anything.
