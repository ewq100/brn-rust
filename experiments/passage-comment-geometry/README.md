# Passage/comment geometry on BRN's existing viewer

A bounded synthetic host using the same `EditorState`/`Editor` and
`TextViewState`/`TextView` as BRN at the recorded baseline. One selected comment
is painted beside its passage in Read. Clicking a comment uses the existing
viewer reveal API. The host is not BRN's full desktop or a new editor.

[PUBLICATION.md](PUBLICATION.md) records the separately authorized experiment-only
publication, exact reconciled check tree and remaining gates. Production/source/
ownership approval is still absent.

Read [INTEGRATION-REVIEW.md](INTEGRATION-REVIEW.md) for the corrected reference
`c20bda2`, 21 passing tests, independent review, the 150-test retained production
fallback result, and current blockers. Owner reply: “no c yet”; production
integration has no approved ref/ownership/toolkit source. Native acceptance is
pending. [PROPOSED-INTEGRATION.md](PROPOSED-INTEGRATION.md) is the bounded B/C
handoff. [HANDOFF.md](HANDOFF.md) preserves the historical 14-test qualification;
[REPORT.md](REPORT.md) retains the initial prototype investigation. The isolated toolkit change is
[the patch](patches/gpui-base-0.7.3-range-start.patch); its source and license
are recorded in [UPSTREAM.md](UPSTREAM.md).

From the worktree root, with the installed Rust 1.98.1 toolchain:

```sh
python3 experiments/passage-comment-geometry/run.py prepare python3 experiments/passage-comment-geometry/prepare-toolkit.py
python3 experiments/passage-comment-geometry/run.py headless cargo test --locked --manifest-path experiments/passage-comment-geometry/Cargo.toml -- --nocapture
python3 experiments/passage-comment-geometry/run.py build cargo build --locked --manifest-path experiments/passage-comment-geometry/Cargo.toml
python3 experiments/passage-comment-geometry/run.py lint cargo clippy --locked --manifest-path experiments/passage-comment-geometry/Cargo.toml --all-targets -- -D warnings
python3 experiments/passage-comment-geometry/run.py format cargo fmt --manifest-path experiments/passage-comment-geometry/Cargo.toml -- --check
python3 experiments/passage-comment-geometry/run.py audit python3 experiments/passage-comment-geometry/audit.py
python3 experiments/passage-comment-geometry/run.py integration-audit python3 experiments/passage-comment-geometry/integration-audit.py
```

`run.py` selects this Mac's installed pinned compiler and
`CARGO_TARGET_DIR=/private/tmp/brn-passage-geometry-target`. It records commands,
exit status, timings, output and competing build processes under `evidence/`.
Use a new evidence label to keep earlier runs. Python 3.12+ is needed for the
bootstrap's safe tar extraction. No toolchain installation is performed.

`prepare-toolkit.py` verifies the published archive checksum and applies the
patch only under this experiment's ignored `.vendor/`. It uses Cargo's cached
archive, or downloads only that pinned source archive if absent. To recreate
a changed patch, remove only this experiment's generated `.vendor/` and rerun.
The copied root lockfile is resolved for this standalone package; no shared
crate version changed. The patch override exists only in this Cargo.toml.

`src/main.rs` is a compilable synthetic native host with Reveal comment 1/2
and Read / Write controls. It was **not launched**. The automated tests use
GPUI's test platform, not an interactive window. A native launch and owner
acceptance are separate follow-up checks in the handoff.
