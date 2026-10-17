---
inclusion: always
---

# HPDL — Product

## Identity
HPDL (HORIZON Planet Definition Language) is an independent, Rust-native, declarative language for defining persistent planetary and celestial worlds. HORIZON consumes its compiled, versioned Planetary Intermediate Representation (PIR).

HPDL is NOT a CUE wrapper, a RON dialect, a game engine, or a simulation solver.

## Core principles
- A planet definition declares identity, initial conditions, constraints, processes, statuses, actions and world-law contracts. A separate authoritative simulation engine evolves mutable state.
- No Earth-centric assumptions. Alternative physics and magic are explicit, versioned, deterministic modules.
- **Status is derived, never written.** No action, AI agent or definition may assign a status score directly.
- Status dimensions are independent: physical state, native ecosystem vitality, civilization viability, optional supernatural stability. More may be registered. A missing ecosystem is "not applicable", never "zero".
- Definitions and runtime state are distinct and separately versioned. History is preserved, reproducible and auditable.

## v1 architectural scope (16 domains)
1 Language foundations · 2 Celestial identities · 3 Astronomical properties · 4 Orbits · 5 Classification/traits · 6 Geology/terrain · 7 Atmosphere/climate · 8 Hydrosphere/cryosphere · 9 Clouds/weather · 10 Biosphere · 11 Asset references · 12 Vitality/process contracts · 13 Canonical PIR compilation · 14 Status scales · 15 Action impact contracts · 16 World laws & custom reality (magic, alternative physics)

## Scope discipline
- Architectural scope ≠ implemented solvers. Declaring a domain does not mean simulating it.
- Unsupported executable features MUST fail explicitly or be marked non-executable. Never silently behave as if simulated.
- Do not invent deadlines, schedules or product priorities. Mark unresolved decisions clearly.
