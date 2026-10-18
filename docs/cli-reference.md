# HPDL CLI Reference

This is the user-facing reference for the `hpdl` command-line tool. It covers the
commands, options, exit codes, and diagnostic output formats you interact with
when checking, building, inspecting, and exporting HPDL planet definitions.

For the internal engineering design, see `docs/design/15-cli-tooling.md`. This
document describes observable behavior only.

## Overview

`hpdl` compiles an HPDL planet definition into a canonical, versioned Planetary
Intermediate Representation (PIR). It takes one source file, runs the full
compile pipeline (decode → semantic validation → module/package resolution →
fixed-point lowering → canonical PIR emission), and reports the result.

The tool never simulates a planet and never mutates your source. It produces
immutable artifacts: canonical PIR bytes and a reproducible build manifest.

### Source formats

The frontend is chosen by file extension:

| Extension      | Frontend          |
| -------------- | ----------------- |
| `.hpdl`        | native HPDL parser |
| anything else  | RON (`.ron`, and any unrecognized extension) |

The extension match is case-insensitive, so `planet.HPDL` uses the native
frontend. Semantically equivalent `.hpdl` and `.ron` inputs compile to
byte-identical PIR.

## Usage

```
hpdl <COMMAND> [OPTIONS]
```

Running `hpdl` with no command, or with `-h` / `--help`, prints the help text.
Running `hpdl -V` / `--version` prints the tool version.

## Commands

### `check`

```
hpdl check <file> [--format text|json]
```

Compiles the source and reports success or the diagnostics that caused failure.
Use this for fast validation in editors and CI; it does not write any files.

On success (text format) it prints a single line with the source name and the
first 12 hex characters of the canonical content hash:

```
ok: `earth` compiled (3f2a1b9c04de)
```

The source name is derived from the file stem (`earth.ron` → `earth`).

### `build`

```
hpdl build <file> [--lock <lock>] [--format text|json]
```

Compiles the source and prints the **build manifest** — a deterministic record
of what the compile produced. In text format the manifest is a stable,
line-oriented form:

```
source_name = earth
compiler_semantics_version = 0.1.0
pir_version = 2.0
content_hash = 3f2a1b9c...
module = hpdl.atmosphere 1.2.0 aaaa...
```

Manifest fields:

| Field                        | Meaning                                            |
| ---------------------------- | -------------------------------------------------- |
| `source_name`                | Logical name derived from the file stem            |
| `compiler_semantics_version` | Version of the compiler that produced the artifact |
| `pir_version`                | PIR format version of the emitted envelope         |
| `content_hash`               | Canonical content hash of the PIR (full hex)       |
| `module`                     | One line per resolved module: `name version hash`, sorted by name |

`build` does not write the PIR to disk — use `export` for that.

### `inspect`

```
hpdl inspect <file> [--lock <lock>]
```

Prints a human-readable compile summary. `inspect` is always text; a `--format`
flag, if passed, is ignored:

```
source:          earth
compiler:        0.1.0
pir_version:     2.0
content_hash:    3f2a1b9c...
modules:         (none)
```

When modules are resolved, `modules:` shows a count followed by one indented
line per module (`name version hash`). With no modules it prints `(none)`.

### `export`

```
hpdl export <file> [--lock <lock>] [--out <path>]
```

Writes the canonical PIR bytes.

- With `--out <path>`, the raw PIR bytes are written to that file and a
  confirmation is printed: `wrote N bytes of canonical PIR to \`<path>\``.
- Without `--out`, the PIR is printed to standard output as lowercase hex.

`export` is the command to use when you need the actual artifact a runtime
consumes. Equivalent definitions always export identical bytes, so the hex
output is safe to diff across machines and frontends.

## Options

| Option            | Applies to                     | Description |
| ----------------- | ------------------------------ | ----------- |
| `--format text\|json` | `check`, `build`           | Output format. Defaults to `text`. |
| `--lock <path>`   | `build`, `inspect`, `export`   | Path to an authoritative package lockfile. When given, the full transitive package closure is resolved and content-verified offline. |
| `--out <path>`    | `export`                       | Destination file for raw PIR bytes. Omit to print hex to stdout. |
| `-h`, `--help`    | top level                      | Print help. |
| `-V`, `--version` | top level                      | Print the tool version. |

Notes:

- Each command takes exactly one `<file>`. A second positional argument is a
  usage error.
- `--format json` is honored by `check` and `build` only. `inspect` and
  `export` ignore the flag if passed (inspect is always text; export is bytes
  or hex).

## Exit codes

Exit codes are a stable contract. Scripts and CI can rely on them:

| Code | Name    | Meaning |
| ---- | ------- | ------- |
| `0`  | success | The command completed. For `check`/`build`/`inspect`/`export`, the source compiled. |
| `1`  | failure | A valid command whose compile failed — a semantic validation error or another compiler error. |
| `2`  | usage   | A usage or I/O error: unknown command or flag, missing argument, bad `--format` value, an unreadable or non-UTF-8 input file, or an unreadable/invalid lockfile. |

The distinction matters: code `1` means your input reached the compiler and was
rejected (fix the definition); code `2` means the command itself was malformed
or an input file could not be read (fix the invocation or the file path).

Success output goes to **stdout**. Failure (`1`) and usage (`2`) messages go to
**stderr**.

## Diagnostic format

When a command fails, `hpdl` reports why. The format depends on `--format`.

### Text diagnostics

Compiler errors are prefixed with `error:`. A semantic validation failure first
prints a count, then one line per diagnostic, sorted deterministically:

```
error: semantic validation failed with 2 diagnostic(s)
error: [E2001] mass must be non-negative (earth.ron:120:140)
error: [E4001] status thresholds must be sorted ascending (earth.ron:300:360)
```

Each diagnostic line has the shape:

```
<severity>: [<code>] <message> (<source>:<start>:<end>)
```

- `severity` is `error`, `warning`, or `info`.
- `code` is a stable diagnostic code (see below).
- The location is shown as `(source:start:end)` when a source is known, or
  `(@start:end)` when only offsets are known. It is omitted when there is no
  span. Notes, when present, appear on following lines prefixed with `note:`.

Other compiler errors (not semantic validation) print as a single
`error: <message>` line — for example an unresolved module, a package integrity
failure, a lowering error, or a resource-limit error.

### JSON diagnostics

With `--format json`, a semantic validation failure prints a single JSON object:

```json
{
  "error": "validation",
  "diagnostics": [
    {
      "code": "E2001",
      "severity": "error",
      "message": "mass must be non-negative",
      "span": { "start": 120, "end": 140, "source": "earth.ron" }
    }
  ]
}
```

`span` is `null` when the diagnostic has no location; within a span, `source`
is `null` when unknown. Diagnostics are sorted deterministically, so the JSON
is stable across runs and machines.

Non-validation errors use a flat shape with an error kind and message:

```json
{ "error": "unresolved_module", "message": "module `hpdl.climate` is not present in the lockfile" }
```

The `error` kind is one of: `frontend`, `validation`, `unresolved_module`,
`module_version_mismatch`, `lock_parse`, `lock_format`, `unresolved_package`,
`package_version_mismatch`, `package_integrity`, `dependency_cycle`,
`lowering`, `emission`, `limit`.

> Note: usage errors (exit code `2`), such as a missing file or an unreadable
> lockfile, are plain `error:` text on stderr even when `--format json` is
> requested. JSON formatting applies to compiler diagnostics, not to
> argument/IO errors.

## Diagnostic codes

Diagnostic codes are stable identifiers grouped by series. The table lists the
codes the semantic validator can emit.

| Code  | Meaning |
| ----- | ------- |
| E1001 | Missing required field |
| E1002 | Invalid field type |
| E1003 | Invalid enum variant |
| E2001 | Negative mass |
| E2002 | Invalid temperature range |
| E2003 | Inconsistent orbital parameters |
| E2004 | Contradictory constraints |
| E3001 | Missing unit |
| E3002 | Incompatible dimensions |
| E3003 | Invalid temperature delta |
| E3004 | Unregistered custom dimension |
| E3005 | Invalid numeric value |
| E3006 | Numeric overflow |
| E4001 | Status thresholds not sorted ascending |
| E4002 | Duplicate threshold |
| E4003 | Threshold out of scale range |
| E4004 | Invalid scale bounds |
| E4005 | Status appears in an action's write list |
| E4006 | First threshold does not match the scale minimum |
| E5001 | Invalid action reference |
| E5002 | Undeclared write |
| E6001 | Malformed module id |
| E6002 | Conflicting module versions |
| E6003 | Multiple world laws for one exclusive domain |
| E6004 | Missing magic bridge |
| E6005 | Unsupported (non-executable) world law declared |
| E6006 | Unsupported (non-executable) magic bridge declared |
| E7001 | Maximum nesting depth exceeded |
| E7002 | Maximum collection size exceeded |
| E7003 | Maximum string length exceeded |
| E8001 | Parent chain cycle (including self-parent) |
| E8002 | Reference-frame mismatch |
| E8003 | Invalid orbital element |
| E8004 | Unsupported (non-executable) orbital model |
| E9101 | Gas composition out of bounds |
| E9102 | Invalid environment value |
| E9103 | Unsupported (non-executable) environment module |
| E9104 | Missing or malformed asset digest |
| E9201 | Incomplete evaluator declaration |
| E9202 | Unsupported (non-executable) evaluator |
| E9203 | Unsupported (non-executable) baseline |
| E9301 | Malformed action field path |
| E9302 | Uncovered write (not backed by a declared capability) |
| E9999 | Generic validation error |

Codes in the E6000, E8000, E9100, and E9200 series marked "unsupported
(non-executable)" mean a declaration was accepted as a contract but has no
registered implementation. HPDL reports these explicitly rather than pretending
the capability is simulated.

## Lockfiles

A lockfile pins the exact version and content hash of every package in the
transitive closure. Supply one with `--lock` to `build`, `inspect`, or
`export`. The format is line-oriented:

```
lock_format_version = 1
package = hpdl.atmosphere 1.2.0 aaaaaaaa...
package = hpdl.climate 2.0.0 bbbbbbbb... hpdl.atmosphere
package = hpdl.ocean 0.4.1 cccccccc...
```

Each `package =` line is `name version content_hash [dependency ...]`. When a
lock is supplied, the compiler resolves the full closure offline and verifies
every content hash before use; a missing package, a version mismatch, a hash
mismatch, or a dependency cycle fails the build with the corresponding error
kind. Supplying a lock does not change the PIR bytes — resolution only confirms
the build's modules resolve deterministically.

An unreadable or malformed lockfile is a usage error (exit code `2`).

## Examples

Check a RON definition, machine-readable output:

```
hpdl check earth.ron --format json
```

Build a native HPDL definition against a lockfile and view the manifest:

```
hpdl build earth.hpdl --lock hpdl.lock
```

Inspect a compile summary:

```
hpdl inspect earth.ron
```

Export canonical PIR bytes to a file:

```
hpdl export earth.ron --out earth.pir
```

Export as hex to stdout (useful for diffing across machines or frontends):

```
hpdl export earth.ron
```
