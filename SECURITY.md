# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| `main`  | ✅ Active development |

Only the latest commit on `main` receives security fixes during pre-1.0 development.

The full release and support policy — supported version windows for crate APIs and
PIR format versions, and the PIR format-version deprecation policy that takes effect
at 1.0 — lives in [`docs/RELEASE_POLICY.md`](docs/RELEASE_POLICY.md). This file
remains the source of truth for the disclosure process below.

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Please report security issues by emailing the project maintainers directly.
Include as much detail as possible:

- A description of the vulnerability and its potential impact
- Steps to reproduce or a minimal proof-of-concept
- The affected crate(s) and version(s)
- Any known mitigations

You will receive an acknowledgement within **5 business days**. We aim to
release a fix or mitigation within **30 days** for confirmed high-severity issues.

## Scope

The following are in scope:

- All active crates in this workspace: `hpdl-model`, `hpdl-sema`, `hpdl-ron`,
  `hpdl-syntax`, `hpdl-pir`, `hpdl-compiler`, `hpdl-cli`, and `hpdl-runtime-api`.
- Parsing and deserialization of untrusted inputs: `.hpdl` and `.ron` source,
  canonical PIR blobs, `hpdl.lock` lockfiles, and migration / package payloads.
- Determinism or canonicalization bypass attacks (any input that makes equivalent
  semantics produce divergent canonical PIR bytes or hashes).
- Denial of service via malformed input (unbounded allocation, nesting, recursion,
  or decode work).
- Integrity bypass of content-addressed references (package or asset digest mismatch
  accepted as valid).

The external HORIZON simulation runtime, its executor, and the validator network are
separate projects and out of scope here. `hpdl-runtime-api` is in scope only as the
published *contract* surface (types, receipts, binding hashes); it ships no executor.

## Disclosure Policy

We follow coordinated disclosure. Fixes will be released before full public
disclosure. CVE assignment will be requested for confirmed vulnerabilities.
