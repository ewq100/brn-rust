# CI helper cache implementation report

- Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`
- Branch: `codex/ci-document-helper-cache`
- Baseline: `ea223ef1c123c2402651236450df7541f825608a`
- Scope: only `.github/actions/setup-rust/action.yml`, `.github/workflows/ci.yml`, `.gitignore`; no commit or push.

## Implemented

- On macOS, the existing pinned Swatinem/rust-cache action now also receives two helper workspace entries whose target directories resolve to workspace-root siblings `.brn-ci-document-helpers/stock` and `/adapted`. Non-macOS continues using the prior root-only cache step. Both targets are disjoint and outside `target`, so the root target cleaner cannot traverse them.
- The macOS cache key retains runner OS/architecture, lane, root Cargo profile manifest hash, and the action's compiler/host/toolchain/lockfile dimensions. Its explicit `hashFiles` input includes the adapted lock, helper manifest/build script/stock build identity, `scripts/build-document-helpers.sh`, preparation script, all four accepted patch files, and all four corresponding source manifests. The preparation script contains the accepted archive digest and engine identity derivation; each source manifest binds selected source file identities.
- All CI helper target-root, artifact, and test target references now use `.brn-ci-document-helpers`; product target paths and adapted generated workspace location are unchanged. `.gitignore` ignores the new root sibling. Helper artifacts, receipts, and source-root guards remain siblings outside the two cached target directories.
- `save-if` remains main-only; no raw cache, new dependencies, generated workspace metadata, or check changes were added.

## Evidence

- Pinned action inspected at `/tmp/brn-rust-cache-pinned`, commit `6323deb102c322ba6fcbdcafc7e3dddab59af2b6`. Its `action.yml` documents newline-separated workspace-to-target mappings; `src/config.ts:145-150` splits and resolves entries with `path.join(root,target)`, and `:276` adds configured workspace target paths. Thus the helper mappings resolve to the intended repo-root sibling paths independently from root `target`.
- `src/save.ts:41-50` prunes configured targets in order. Since root `target` and both helper targets are siblings, root cleanup does not recurse into helper targets. No helper artifact/receipt/guard directory is configured as a cache target.
- Every one of the 14 explicit digest inputs exists. The resolved stock/adapted target paths were checked and are distinct under repo-root `.brn-ci-document-helpers`.
- `git diff --check`: PASS after final key-input update.
- Ruby Psych parsed `.github/actions/setup-rust/action.yml` and `.github/workflows/ci.yml`: PASS after final key-input update. PyYAML was unavailable; no package was installed.
- `bash -n` over all `scripts/*.sh`: PASS after final key-input update.
- No build, download, cache API, native/account/credential action, or CI run was performed.

## Changed-file SHA-256

- `.github/actions/setup-rust/action.yml`: `b938d4368e7f2a9397c2ab43fa130662cd49f960294a927beea36c7e0853b526`
- `.github/workflows/ci.yml`: `3db9b97ad975600efdd2dc4500f76bb13fd860e2e30d8c2798580f7d9cd4ead3`
- `.gitignore`: `cce1b0b0da95beb3d85a97b03192e28b22324dea6a883c923dc10cb6de096f41`
- Combined current diff SHA-256: `d3fc9e711c79ecc90ed7ac483faa5fef1dc4a3389f5a501b05034595d2775678`

## Limitations

This is a configuration implementation only. A real macOS workflow must establish restore hits, resulting pruned archive contents, cache size, and helper preparation time. The Ruby parse validates YAML syntax but does not evaluate GitHub expressions or execute the action. The action's recursive cleaner behavior is source-inspected; no workflow cache was produced or inspected.
