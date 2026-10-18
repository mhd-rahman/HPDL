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

## Untrusted-input resource limits

Every untrusted-input parse/decode path is expected to be **total** — it may reject
input with a typed error, but must never panic, hang, overflow the stack, or allocate
unbounded memory. The enforced bounds today:

| Input | Size cap | Nesting depth | Collection / length | Other |
|-------|----------|---------------|---------------------|-------|
| `.hpdl` source | 10 MiB | `ParseLimits::max_nesting_depth` (recursive-descent guard) | identifier length | fails closed with `Error::Security` |
| `.ron` source | 10 MiB | `SecurityConfig::max_nesting_depth` (structural pre-scan) | `max_collection_length` (pre-scan) | path-traversal (`..`) rejected |
| PIR blob | 10 MiB envelope cap (checked before and after envelope decode) | — | inner length prefixes are read incrementally (see below) | content hash + format version verified before the inner decode |
| `hpdl.lock` / provisional `BuildLock` | byte-size cap (`ResourceLimits::max_source_bytes`) | — | resolved-closure size (`max_modules`); dependency cycles rejected | malformed/format/duplicate entries rejected |
| migration payloads | n/a (contracts only; no decoder ships here) | — | policy-id ≤ 128 chars | forward-only schema enforced |

### PIR allocation-bomb resistance

The PIR decoder caps the whole envelope at `MAX_PIR_SIZE` (10 MiB) and verifies the
content hash and format version **before** decoding the inner `PlanetDefinition`. It
does **not** impose explicit per-field element-count or string-length caps; instead it
relies on bincode 1.x's default behavior of reading length-delimited data incrementally
rather than pre-reserving the full declared length. A hostile envelope that inflates an
inner length prefix therefore fails with `Error::MalformedData` instead of triggering a
large up-front allocation. This reliance is pinned by regression tests
(`test_decode_rejects_inflated_payload_length_prefix` and
`test_decode_rejects_moderately_inflated_payload_prefix`); a bincode version or config
change that pre-reserves capacity would fail those tests. A future hardening step may add
explicit per-field count/length caps rather than depending on the dependency's behavior.

### Package archives (not yet implemented — required protections)

HPDL's package system is currently **offline and lock/hash-based only**: resolution
walks an authoritative lockfile's declared dependency graph, verifying content digests
and rejecting cycles. There is **no archive extraction anywhere** — the compiler never
opens, decompresses, or unpacks a `.tar`/`.zip`/other archive, and never writes package
contents to the filesystem.

Consequently, zip-bomb, decompression-ratio, extract-time path-traversal, symlink-escape,
and entry-count attacks are **not applicable today** because no code path can trigger
them. If archive fetching/extraction is ever added, it MUST, before being merged, enforce
all of the following (these do not exist yet):

- **Verify the content hash of the archive bytes before extracting anything.**
- **Decompressed-size cap** and a **compression-ratio cap** to bound zip-bomb expansion.
- **Entry-count cap** on the number of archive members.
- **Path-traversal rejection**: refuse any entry whose normalized path escapes the
  destination root (`..` components, absolute paths, drive-relative paths on Windows).
- **Symlink-escape rejection**: refuse symlinks/hardlinks (or resolve and confirm they
  stay within the package root); never follow a link during extraction.
- Fail closed with a typed error on any violation; never partially extract.

Until that work lands, reviewers should treat any PR that introduces archive handling
without these guards as a security regression.

## Disclosure Policy

We follow coordinated disclosure. Fixes will be released before full public
disclosure. CVE assignment will be requested for confirmed vulnerabilities.
