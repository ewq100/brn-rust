# Experiment-only publication

The owner's latest 15 October 2026 instruction authorizes a normal push and
**draft PR only** to preserve this isolated experiment as a remote record.
It supersedes the earlier experiment-only merge authorization. **Do not merge
or release.** Draft [PR #149](https://github.com/ewq100/brn-rust/pull/149) uses
`work/passage-comment-geometry` and is titled “experiment: reviewed
passage-comment positioning prototype”. It does not authorize production
integration, toolkit adoption or assignment of the wider C role. C/source/ownership and native acceptance gates
in [INTEGRATION-REVIEW.md](INTEGRATION-REVIEW.md) remain open.

## Exact reviewed and tested identities

- Independently reviewed corrected reference:
  `c20bda2dda16257164f7c584f0c5fc20a301075f`.
- Review/handoff receipt: `1fa01165d2d567d0cfc26257eded29207a8d0f69`.
- Fresh publication base: `8bef439b133e91ad68b0b76792bcd155d3d06237`.
- Reconciled tested candidate: `32008c10fd7a636de56a2e3fc87fc6d846a1a7a9`, tree `4d06c5fbe8d6c04e5f207596b3456fc106a869d7`.
  Reconciliation merged main's two already-landed documentation changes; no
  conflict and no experiment implementation change. The later publication
  receipt changes only this experiment's documentation/evidence.
- Original Z1 qualification is unchanged and separate at
  `b9bd1a14862fe2efe246790e887797c949f13c50`.

## Full proposed diff and review applicability

[Full inherited-file audit](evidence/publication-full-diff-audit.stdout) reads
and hashes **all 192 inherited proposed files**, not only the latest commit.
Every file is under `experiments/passage-comment-geometry/` and matches the
handoff receipt byte for byte. Prototype source, manifests/standalone lock,
three patches, scripts, license and ignore rule also match the independently
reviewed reference. The receipt after that reference contains only the
recorded handoff/docs/evidence. No unrelated experiment, production/root
dependency edit, toolkit tree, binary/build output, secret credential or user
content is proposed. Evidence contains synthetic fixture output, compiler
messages and development command metadata. No provider/model/native GUI was run.
The branch had no unrelated task changes, so no replacement publication branch
or rewriting/discarding of existing work was needed.

The narrow gpui-base patch remains experiment build input only, reconstructed
from the checksum-verified 0.7.3 archive. Its SHA-256 remains
`de6dcb320028f4d1848a44c3b2141fc87cd8edd7d23adb930500e4f40839b6d4`.
The published prototype positions one selected saved comment beside the first
rendered glyph of its exact passage in Read; no full-range geometry, live Write
comments, collision layout or Zorite adoption is added.

[Independent re-review](review/independent-corrected.md) completed for the exact
isolated reference, resolved all five findings and independently reran 21 tests.
Byte-identical code and toolkit sources preserve that review. Lead inspection
covers only mechanical publication reconciliation/docs/evidence; no additional
behavior change or fresh implementation delegation. The lead retains the
selected runtime configuration, with no new helpers. Stop/reassess if a conflict
requires a production edit or dependency-source decision.

## Reconciled local checks

Normal root CI does **not** exercise this standalone experiment. Explicit local
commands below were run through `run.py` on the tested candidate above, with
Rust 1.98.1 and `CARGO_TARGET_DIR=/private/tmp/brn-passage-geometry-target`:

```sh
python3 experiments/passage-comment-geometry/prepare-toolkit.py
cargo test --manifest-path experiments/passage-comment-geometry/Cargo.toml --locked --offline --test geometry -- --nocapture
cargo build --manifest-path experiments/passage-comment-geometry/Cargo.toml --locked --offline
cargo clippy --manifest-path experiments/passage-comment-geometry/Cargo.toml --all-targets --locked --offline -- -D warnings
cargo fmt --manifest-path experiments/passage-comment-geometry/Cargo.toml --check
cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline -- --nocapture
```

All exited zero: **21 geometry tests and 150 existing desktop tests passed,
zero ignored**, plus build/strict own-target lint/format. Exact command vectors,
environment, timings and outputs are committed in
[evidence/commands.jsonl](evidence/commands.jsonl), labels `publication-*`,
[geometry output](evidence/publication-geometry.stdout) and
[desktop output](evidence/publication-desktop.stdout). Existing dependency
future-incompatibility and installed `rust-objcopy` stripping warnings remain
visible; they did not fail these commands.

The historical baseline audits intentionally pin the original pre-publication
tree. After reconciliation, use the publication full-diff audit (its complete
Python command is in `commands.jsonl`) for scope and archive reconstruction;
old baseline-to-working-tree assertions include main's later docs and are not
publication audits. No historical evidence was rewritten.

## Hosted gates and retained limits

[Actual main protection](evidence/publication-main-protection.json) requires
up-to-date branches and these four checks; admin enforcement is enabled and
force-pushes disabled:

- Core and CLI (ubuntu-24.04)
- Core and CLI (macos-15)
- Native UI build and state (macos-15)
- Native retrieval and combined build (macos-15)

No required reviewer count is configured; [applicable rules](evidence/publication-main-rules.json)
are empty. The completed independent review above remains the review evidence.
The earlier experiment-only publication created PR #149 at
`df3fe07b7e08c21a6c327ae96af5b28c3cdce6a1`; hosted checks were queued. The
latest instruction converts that same PR to draft and stops merge work. The
subsequent remote-record commit changes only this publication authority record;
reviewed source, tests, toolkit patch and all evidence remain unchanged.
Hosted checks may continue, but do not authorize merge. There is no merge or
release in this assignment; any later merge needs new applicable authority.

**Production integration: none. Toolkit source and production ownership: not
approved. Native acceptance: pending; Mac locked.** Existing production behavior
and original Z1 qualification remain unchanged. Production geometry assertions
and the consolidated native acceptance list remain future gates after C's
approved source/ref/ownership and an actual candidate. No release is assigned.
