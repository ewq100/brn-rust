# Measuring development feedback

Method used by the development-speed experiments; read it only when timing build or test changes.

Freeze the source revision, toolchain/lockfile, target, feature set, Cargo jobs and
`CARGO_INCREMENTAL` value for a comparison. CI currently sets incremental to `0`;
local incremental timings do not establish hosted CI gains. Use task-owned target
roots and reserve the host against competing builds. Preserve failed and
contaminated attempts rather than rerunning until green.

Separate test compilation from execution: compile with
`cargo test --workspace --no-run --locked --offline --timings --message-format=json`,
then run the emitted test executables with the same filters/thread count in both
arms. Cargo's artifact messages record features, profile checks, freshness and
executable identity. A selected probe must recompile its intended target and run
nonzero tests; the full applicable suite remains a separate gate.

For reversible edit trials, restore source only when its bytes differ, compile
the restoration, and require a fully fresh no-op before the next probe group.
Rewriting identical bytes after that build changes modification time and can
contaminate the next probe. Compare the rebuilt artifact sets as well as wall
time. Record cold and warm conditions, medians/ranges, compile-plus-execution
feedback, and a fresh confirmation against the original baseline. Keep model
assistance outside timing windows or provide equivalent assistance to both arms.

Build both document helpers with the maintained one-job script before comparing
suite execution. Keep the same verified helper artifacts outside the comparison
target roots, record their hashes, and export both `BRN_XBERG_HELPER` and
`BRN_XBERG_HELPER_BRN`. Also copy those same hash-verified helpers beside the
built CLI for fixture children that deliberately clear `BRN_*` variables. Missing helpers and skipped tests cannot qualify a faster
suite. Hosted queue/setup time, required-check completion and aggregate runner
work are distinct from compilation or test execution.

Alternate arms (for example ABBA) for execution timings and reuse the same built
artifacts. Before attributing a difference to configuration, rerun identical
binaries across slots and verify their hashes. A process guard limited to Cargo
and compiler processes does not establish that the host was otherwise quiet.

Separate target roots do not isolate source timestamps in a shared checkout.
After restoring an edit, an inactive root can rebuild unchanged earlier-probe
dependencies during the next probe. Use independent source checkouts or warm
every affected root against the restored source. Require freshness immediately
before each arm and probe, including its build, lint, test and helper graphs;
an initial no-op or matching executable hashes alone is insufficient.

Freeze the effective native build environment too: compiler/linker selection,
`PKG_CONFIG_PATH`, include paths and library paths can invalidate caches despite
identical Rust versions. Activation scripts may append paths when sourced again.
Check freshness in the same environment that executes the measured commands.

A package-only test invocation can resolve a different dependency-feature graph
from the complete workspace gate. Compare actual Cargo artifact feature/profile
messages for common units, not only package names or executable filenames. A
selected filter must execute nonzero tests. A focused-first experiment must still
invoke the normal full gate after a focused failure and preserve a nonzero result
from either stage. The [cloud Cycle 2 record](development-speed-255-cycle2.md)
reports the observed gains, rejected/incomplete qualification and no retained
workflow change; focused checks were already supported and are not a new default.
