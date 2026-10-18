# Read-only merge simulation and the first combined-graph defect

17 October 2026, revised 18 October 2026 (round 1 corrections; see [plan](plan.md) and [response](opus1/response.md)). Static analysis of Git objects. No checkout of the merged result, no compile. The conclusions are about text and structure only.

## Method

```sh
git merge-tree --write-tree --name-only --messages work/int-w-20261017 73ea9ca7      # INT-W d2d84e66 x X2 73ea9ca7
git merge-tree --write-tree --name-only 73ea9ca7 57c710cc                            # X2 x origin/main
git merge-tree --write-tree --name-only work/int-w-20261017 57c710cc                 # INT-W x origin/main
git show "<tree>:<path>"                                                             # read the simulated result
```

`--write-tree` writes unreferenced tree and blob objects into the shared object store of the common `.git`. It creates no ref, index or working-tree change. `git gc` prunes them. The conflicted trees contain conflict markers in documentation files only.

| Merge | Result tree | Conflicts |
|---|---|---|
| INT-W `d2d84e66` × X2 `73ea9ca7` | `deb060ee0e809b0b906bc9aaf97626a4181ef7be` | `docs/status.md`, `docs/work/active/modular-foundation/plan.md` (one hunk each: both sides edit the top of the file) |
| X2 `73ea9ca7` × `origin/main` `57c710cc` | `ddf23ff933bbd714e5843688919d74c534e69ef6` | `docs/status.md` |
| INT-W `d2d84e66` × `origin/main` `57c710cc` | `b83b2374648d678bad5996fede265d00f2697423` | `docs/status.md`, `docs/architecture/dependencies.md` |

The first result's crates tree is `54285bd2bd5bd4d977063b90ab39558c495f55f2` (335 files). It differs from X2's crates in 47 files and from INT-W's in 110. The three-way with `origin/main` was not simulated as one step because it needs a commit object for the resolved first merge.

`docs/architecture/threads-behavior.md` auto-merged (the two sides edit different sections). `crates/brn-threads-app/src/lib.rs` auto-merged: INT-W adds one line near the top, the capability line adds `mod mcp`, `mod skills` and their re-exports.

## <a id="nf-1"></a>NF-1: `reviewed` is out of scope after the merge

Two accepted changes interact inside `crates/brn-desktop/src/threads_ui.rs`:

- INT-W B1b split the old `start_run` into `start_run` (builds a `ThreadRunStart` from the composer) and a shared `start_request` (prepare, spawn worker). The writing panel's retry (`retry_rewrite`) calls `start_request` directly.
- D4 added, inside the old single `start_run`, a prompt-bound MCP review: `take_mcp_review(&thread, &prompt, queued)` before preparation and `reviewed.attach(prepared)` after it.

Git merged the hunks without conflict, so in the simulated tree (`deb060ee…`) line positions are:

| Line | Content |
|---|---|
| 1899 | `fn start_run(...)` |
| 1914 | `let reviewed = match self.take_mcp_review(&thread, &prompt, queued.is_some()) { ... }` inside `start_run` |
| 1949 | `fn start_request(request, context_edit_note, continuation, completed, cx)` |
| 1983 | `match reviewed.attach(prepared)` inside `start_request` |

`reviewed` is a local of `start_run`; it is not a parameter of `start_request`. Compile error E0425 under `--features native-ui`. Only the native lanes see it: `threads_ui.rs` is not part of the default build. Only `start_run` and `retry_rewrite` (line 1860) call `start_request`.

`take_mcp_review` consumes the pending review and returns an error if a review exists but the run is queued, the stamp changed, the chat differs, or the prompt differs from the reviewed prompt. Absent a review it returns `None`. D4's reviewed input authorizes one invocation for the exact owner prompt only.

**Fix being specified (the full ordering, `queued` rule and real call-path cases are in [NF1-test-spec.md](NF1-test-spec.md)):** keep the X2 order on every start: `begin_run_start` busy check first (a busy refusal does not consume a review), then `refresh`, then `take_mcp_review` **before** `prepare_thread_run`, then `attach` after prepare. Move the take into `start_request` with a `queued: bool` equal to the queue-text match (`queued.is_some()` in `start_run`), not `request.queue_claim.is_some()`, because a matched queue item can have `dispatch == None`. `retry_rewrite` passes `false`. Its prompt is the fixed rewrite string X2 also passed, so a review for a different prompt is consumed and refused with the existing message, and an exactly equal review may attach, as in X2; `false` differs only if a queue item's text equals that string. Alternatives (pass `None` for writing, skip attach) leave a grant-bearing state unconsumed and need a stated reason.

**Witness:** the existing MCP widget tests call `take_mcp_review` and `attach` directly and do not reach `start_run` or `start_request`. The specification lists real call-path cases (`retry_rewrite` with a pending review and the preserved request, feedback and wrapped notice; a second retry that progresses; ordinary mismatched and queued starts; a matching positive judged on the synchronous `start_run` outcome (worker set, no refusal notice, review consumed, one new Run with the worker's id; no terminal-state assertion); busy does not consume). The compile error remains a static finding, not an executed red.

Not yet known: whether other accepted-line seams collide. The first `cargo check -p brn-desktop --features native-ui --all-targets` is the discovery step; report any further break before editing.

## <a id="lock-audit"></a>Lock audit

Compared package sets (name, version, checksum) of the simulated merged `Cargo.lock` against parents:

| Set | Packages |
|---|---|
| `8f768943`, CW2 | 952 |
| INT-W | 953 (`brn-writing-ui 0.1.0`) |
| X2 `73ea9ca7` | 965 (13 added, none removed) |
| Simulated merge | 966 = union of INT-W and X2, no extras, none missing |

No checksum differs from `8f768943` for any shared name and version. The only name with two versions newly present is `darling`/`darling_core`/`darling_macro` (0.20.11 and 0.23.0, from the capability line). The merged lock's SHA-256 is `72f9ced96cf938aec013932f31d60f213d811f2e4b1d876c8828b4d5201e6b58`. This proves set consistency only; Cargo may still want to rewrite dependency edges, which `cargo metadata --locked --offline` reveals. If it fails, allow one `cargo metadata --offline` rewrite, then require the same package set and checksums, and report the diff.

Reusable check (read-only, run against any lock blobs):

```python
import re
def pk(t):
    out = {}
    for b in t.split("\n[[package]]\n")[1:]:
        n = re.search(r'^name = "(.*)"', b, re.M).group(1)
        v = re.search(r'^version = "(.*)"', b, re.M).group(1)
        c = re.search(r'^checksum = "(.*)"', b, re.M)
        out[(n, v)] = c.group(1) if c else None
    return out
# compare pk(merged) to pk(parent_a) | pk(parent_b); then each Xberg addition to the audited source ledger
```

After Xberg, the expected set is **computed from the exact handoff lock, the common base and both parents**, not frozen. The current X2 HTML WIP (no pin; `Cargo.lock` SHA-256 `8b7a8ac0…`, read only, metadata run output SHA-256 `aa1a8b35…`) moves **six** existing packages: `serde_json` 1.0.151→1.0.154, `toml` 1.1.6+spec-1.1.0→1.1.8+spec-1.1.0, and the mandatory companions `serde_spanned` 1.1.1→1.1.2, `toml_datetime` 1.1.1+spec-1.1.0→1.1.2+spec-1.1.0, `toml_parser` 1.1.3+spec-1.1.0→1.1.5+spec-1.1.0, `toml_writer` 1.1.2+spec-1.1.0→1.1.3+spec-1.1.0, with 52 added identities (965 → 1,011), 6 removed, and zero same-version drift; `toml 0.8.23`, `serde_spanned 0.6.9`, `toml_datetime 0.6.11` coexist. Three-way block facts (parsed from Git blobs and that file): INT-W touches two blocks (new `brn-writing-ui`; `brn-desktop` gains `brn-writing-ui` and `rig`), and neither X2 delta nor the WIP delta touches either, so the merge is block-disjoint; `brn-writing-ui` lists plain `serde_json`, valid while one version exists. Provisional merged count 966 + 52 − 6 = 1,012. Procedure and stop rules: [plan section 5](plan.md#5-lock-reconciliation-expected-identity-set-not-a-frozen-number).

## Link-check failures on the X2 base (NF-6)

`python3 scripts/check-markdown-links.py` run in this checkout (X2 base plus only the files of this folder): `Markdown links: 359 files, 1703 local links, 82 failures`. Counts by file: `docs/work/active/modular-foundation/x2/source-audit/api.md` 42 and `.../source-audit/resources.md` 40; none in `final-integration/`. Each failure is a Markdown link whose target is an absolute path under `/Users/evokessler/.cargo/registry/src/...` or `/Users/evokessler/.t3/worktrees/brn-rust/t3-9527c407/...`, reported as "missing file". INT-D's tree recorded 258 files and 0 failures, so these arrived with the X2 audit docs. The CI `Documentation and tooling` job runs the same script. This is a read-only script run, not a build. The 82 is immutable history, not a target. The X2 worker's uncommitted HTML candidate reports 82 to 0 with the same unchanged checker (357 files, 1,611 links, 0 failures); that is a WIP receipt for that lane, not a pin and not a whole-root result. The X2 worker owns those docs; the integration re-runs the checker when a pin arrives and sends any failure back instead of editing them in duplicate. The old cd2-bound ledger hashes are not rewritten.

## Other static checks that found no break

- Struct literals for core `Run` (`loaded_skill_details`) and `WorkspaceSettings` (`mcp`, `skills`) added by the capability line: every literal site in INT-W-changed or INT-W-added code already goes through existing constructors or `Workspace::settings()`/`configure`; no INT-W-only literal is missing a field.
- `ThreadRunStart` is unchanged by the capability line; `brn-writing-ui` builds it from the same fields.
- `brn-ai`'s `scripted-runs` feature still exists, so INT-W's desktop dev-dependency on it resolves. Because that dev-dependency enables `scripted-runs` in workspace test builds, the SDK loopback tests may also run in the required Core lanes.
- `brn-writing-ui/examples/writing_host/fixture.rs` uses `use brn_threads_app::*;`; the capability line adds public names (`McpStatus`, `LoadedSkillDetails`, `SkillPackage`, …). A glob collision would be a compile error only if the example defined a same-named item; not detected statically.
- Capability-line hunks in `threads_state.rs`, `threads_tiles.rs`, `threads_chat.rs`, `threads_writing.rs` are additive and sit in regions INT-W did not remove (`.child(self.loaded_skill_details(id))` is present in the simulated `threads_writing.rs`).
