# Rig agent safety patch

BRN uses the exact published `rig-agent 0.43.0` package with one change:
remove the unconditional `PARTIAL` stderr print in the streaming invalid-tool
branch, before invalid-call hooks. Typed errors, tools, requests, selection,
retries, limits and history remain upstream behavior. No stderr interception or
provider protocol replacement is used.

The cached crates.io archive SHA256 is
`eb11364e7ec4cb4cc20b7dda77a1963652baab01070f0ef2ac275aea3a6081f8`.
Its packaged VCS commit is `654567eb64274fca00cab86cdd32c86b9913769e`.
All88 package files (2,199,521 original bytes), including the normalized manifest,
original manifest, package lockfile and README, are retained. Every original and
vendored file hash is recorded in [the provenance manifest](rig-agent-provenance.json);
only `src/agent/engine.rs` differs, by exactly that deleted line. The archive
contains no standalone license file; the MIT declaration is unchanged, and
[the upstream MIT license](LICENSE-rig) is copied byte-exact from the same release's
cached `rig` facade archive, with separate provenance in the manifest.

Root Cargo.toml patches only `rig-agent` to this directory and excludes it from
BRN's six-crate workspace. The root lockfile retains all other packages and edges;
only rig-agent's registry source/checksum becomes a local path package. Do not
edit global Cargo configuration or registry caches. Upstream package style and
relative README links are retained as release evidence, not rewritten as BRN docs.

The `brn-ai` process-stderr regression exercises all three real Rig provider
routes with synthetic transports. Its isolated child proves fixed InvalidToolUse,
zero backend dispatch, no retry and complete execution; its parent checks that
unknown-tool arguments and prior partial text do not reach stderr. This changes
no account or network behavior. Verbose upstream tracing is a separate surface;
this patch is not a general logging audit.

Remove this override only after a reviewed compatible published release contains
the fix and the same regression passes. No newer published fix was identified
in the read-only2026-10-06 release check.
