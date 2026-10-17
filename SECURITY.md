# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| `main`  | ✅ Active development |

Only the latest commit on `main` receives security fixes during pre-1.0 development.

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

- All crates in this workspace (`hpdl-model`, `hpdl-sema`, `hpdl-pir`,
  `hpdl-ron`, `hpdl-compiler`, `hpdl-cli`)
- Parsing and deserialization of untrusted `.hpdl`, `.ron`, PIR, and lockfile inputs
- Determinism or canonicalization bypass attacks
- Denial of service via malformed input (unbounded allocation, nesting, recursion)

The HORIZON simulation runtime is a separate project and out of scope here.

## Disclosure Policy

We follow coordinated disclosure. Fixes will be released before full public
disclosure. CVE assignment will be requested for confirmed vulnerabilities.
