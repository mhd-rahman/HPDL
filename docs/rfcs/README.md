# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records for HPDL. Each ADR
documents a significant design or engineering decision so future contributors
understand not just *what* was chosen, but *why*, and what the alternatives were.

## File naming

```
NNNN-short-title.md
```

- `NNNN` is a zero-padded four-digit sequence number.
- The short title uses lowercase kebab-case.

## Template

Each ADR must contain the following sections:

```markdown
# NNNN — Title

**Status:** Pending | Accepted | Superseded by NNNN | Rejected
**Date:** YYYY-MM-DD
**Feature:** <feature-id>

## Context

What is the situation that forces this decision?

## Decision

What was decided?

## Rationale

Why this option over the alternatives?

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| ...    | ...         |

## Compatibility impact

SemVer, PIR format-version, grammar version, or schema-version consequences.

## Test consequences

What tests need to be added, changed, or removed as a result?
```

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-workspace-open-decisions.md) | Workspace Foundation — Open Decisions | Accepted |
| [0002](0002-ron-extension-fields.md) | RON Extension Fields | Accepted |
| [0003](0003-pir-v2-orbital-fields.md) | PIR v2 Orbital Fields | Accepted |
| [0004](0004-activate-hpdl-runtime-api.md) | Activate `hpdl-runtime-api` | Accepted |
| [0005](0005-world-laws-and-magic.md) | World Laws & Magic | Accepted |
| [0006](0006-module-runtime-contracts.md) | Module Runtime Contracts (Vitality & Process) | Accepted |
| [0007](0007-schema-migration-execution.md) | Schema Migration Execution Contracts | Accepted |
| [0008](0008-stdlib-packages-lockfile.md) | Immutable Standard-Library Packages & Lockfile | Accepted |
| [0009](0009-cli-tooling.md) | HPDL CLI (`hpdl`) Tooling | Accepted |
