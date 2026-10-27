# Isolated document-helper dependency cache

The [setup action](../../../../.github/actions/setup-rust/action.yml) now includes
Linux's isolated stock/adapted helper targets in the existing pinned rust-cache
step, matching macOS coverage. Both platforms share the configuration, but keep
separate OS, architecture and feature-lane keys. Windows remains uncached.
Normal locked helper builds and all existing tests still run on hits and misses.

## Measurement before implementation

Baseline `ed1e3979645e062454d95073a366c9d04c99ac66`, Linux, Rust 1.98.1,
one job, populated registry, fresh task-owned targets; one cold-target build then
one same-root warm build. No shared target eviction or helper execution occurred.
The unchanged [build script](../../../../scripts/build-document-helpers.sh) ran
with `--offline --jobs 1 --target-root <fresh absolute task target>` after locked
stock/adapted fetch and normal adaptation. A timing-only Cargo wrapper forwarded
all arguments and environment. High-resolution subprocess timings:

| Cargo stage | Cold target (seconds) | Same-root warm (seconds) |
|---|---:|---:|
| Stock fetch | 0.595 | 0.188 |
| Adapted fetch | 0.182 | 0.174 |
| Stock build | 262.944 | 0.227 |
| Adapted build | 267.308 | 0.223 |
| Both dependency graph recordings | 0.388 | 0.352 |

All eight outer stages and twelve Cargo invocations passed in 550.95 seconds.
The build script's observed wall upper bounds were 533.90/4.04 seconds; observer
polling adds up to about two seconds, so these are not precise subprocess times.
Adaptation/validation completed within one such interval each. Fetches used the
existing registry, not a cold network. Source stayed clean.

Stock/adapted targets totaled about 2.94 GiB before cache cleanup. Full task data
was 3.54 GiB and final disk free 9.68 GiB, within 30-minute/6-GiB/4-GiB-free limits.
Cold/warm output bytes were identical:

- Stock: 160,989,744 bytes, SHA256 `e8e1593c441f257a8b57b55b88853a7f8cf088de62d2ac9b66357f888efafd54`.
- Adapted: 161,153,496 bytes, SHA256 `0f237c37013442e0e0f1c434a4856f0428d006a27b5b311de3a8b1e1e1e489d1`.

Raw machine-local logs, controller and receipt are retained under
`/workspace/brn-next-tracks-evidence/helper-reuse/measurement-20261027T000032Z`.
The controller SHA256 is
`f2f6a8812aff06f5d72930108c7902e64e17ce9c60c88200929bb91c95aa6028`.
These paths are resumption pointers, not portable repository links.

## Identity, fallback and trust

The [pinned action documentation](https://github.com/Swatinem/rust-cache/blob/6323deb102c322ba6fcbdcafc7e3dddab59af2b6/README.md)
describes compiler/host/toolchain, manifest/lock/config/environment keying and
dependency-only cleanup. Explicit supplementary inputs cover root profiles, both
helper locks, manifest/build identity/build script, adapter generator and patch/
source-manifest globs. This key identifies reusable dependencies and adaptation inputs, not a complete
executable. Normal Cargo freshness checks/rebuilds current local helper and
brn-intake Rust source;
converter engine identity alone is never permission to reuse an executable.

Workspace crates, all-crate caching and Cargo-installed binary caching remain
explicitly disabled. Default cleanup removes local helper build products; the
0.450-second same-root result therefore **is not a hosted-cache forecast**.
CI also recreates the checkout-specific generated adapter workspace, which can
recompile its local dependency. No binary artifact pipeline is introduced.

Only main saves. PRs restore available base caches and run normal unprivileged
checks; no token, permission, trigger or credential changes. Missing/partial
cache entries take the normal build path. Generated-source cleanup/reproduction
and source-root guards remain intact. Existing stale/partial/symlink tests and a
new empty-cache test exercise regeneration; configuration tests check target/key
coverage, main-only dependency caching and unconditional locked helper builds.
They do not emulate hosted cache restoration.

## Validation and rollout gates

The focused adapter/cache suite passes 14 tests. Two cache contract checks fail
against the old action (missing Linux helper coverage/explicit input coverage),
then pass against the new configuration. Existing stale/partial source tests
remain passing. Markdown links and `git diff --check` pass. No Rust compilation
was repeated for this configuration-only change.

Hosted restore hits, pruned payload sizes and helper timings remain **unverified**.
A new key may populate only on an approved main run; draft CI cannot manufacture
that evidence. Lane duplication/storage pressure is a rollout gate: inspect
normal hosted cache sizes and hits before claiming net savings. If evictions
negate benefit, propose separate bounded cache sharing rather than changing
quotas or enabling broad executable reuse. No extra dispatch matrix is planned.
Independent review and exact-head CI remain integration gates.
