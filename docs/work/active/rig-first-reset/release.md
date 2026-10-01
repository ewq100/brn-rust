# Rig Reset Retirement and macOS Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline execution, or superpowers:subagent-driven-development when explicitly selected. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove superseded production assumptions and qualify a sidecar-free macOS distribution only after both providers and retrieval are accepted.

**Architecture:** One shared workflow with direct Rig subscriptions and separate derived SQLite retrieval. Historical data/experiments remain inspectable without an executable compatibility backend. Production .app packaging uses relocatable verified assets, not an absolute-path development launcher.

**Tech Stack:** Qualified Rust/default/native graphs, macOS executable/resource bundles, deterministic shell fixtures, codesign/notarytool/stapler only under separate explicit authorization.

**Spec:** [Approved reset](../../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md); [master plan](plan.md).

## Global constraints

- Inherit master authorization, data-preservation and verification constraints.
- D1 requires delivered N4/A4 and accepted N5/G4. A build is not retrieval acceptance.
- D2 implementation can produce an unsigned artifact; signing, notarization, account login, distribution and release each need their own authorization.
- No Windows implementation or universal-Mac claim without the relevant targets/machines.
- No special OpenAI/GitHub approval process is mandated. Review terms/support risks separately from technical success.
- Preserve original vaults/data/credentials; no migration, installation or sweeping cleanup by packaging tests.

---

## D1: Retire App Server and Lance production surfaces

**Prerequisite:** N4/N5/A4 accepted at their bounded gates; G1-G4 evidence reviewed.

**Files:**
- Remove from production workspace: `crates/brn-provider` source/manifest/tests and root membership.
- Modify: workflow Cargo/lib/worker/main; CLI ask/status/error/mod and provider-dependent tests; desktop main/native/settings/centre/history.
- Modify: retrieval Cargo/lib/native and root lockfile; frozen comparison experiment remains self-contained.
- Modify: `scripts/{make-macos-app.sh,test-make-macos-app.sh,verify-end-to-end.sh,verify-desktop-shell.sh,report-sizes.sh}`.
- Create: `scripts/verify-rig-reset.sh`.
- Modify: root/crate READMEs, docs architecture overview/invariants/dependencies, development setup/verification, status/roadmap/index.
- Modify historical index references if removed provider README paths were linked; preserve actual historical observations.

**Consumes:** Qualified Rig ask/candidate/current-note interfaces and accepted SQLite vector search.

**Produces:** No production sidecar/Lance dependency or fallback; updated commands/contracts and deterministic whole-reset check.

- [ ] **1. Write failing retirement tests first:** CLI `--codex`/`--codex-home` rejected as unknown options; status contains selected AI identity/capabilities and no `codex_configured`; local launcher rejects obsolete options. Production manifest assertion requires no `brn-provider`/`lancedb` dependency and no sidecar child-process startup in ask/settings. Named ChatGPT/Codex models remain valid; do not ban the word `codex`.

Run `cargo test -p brn --locked --test cli_basic` and `bash scripts/test-make-macos-app.sh`; expect the new legacy-option assertions to fail against current accepted options.

- [ ] **2. Remove obsolete production code:** delete App Server client/thread/config/process paths, fake-thread assumptions and old answer-copy candidate mutation. Remove provider crate from the workspace and its production dependencies; keep V1-V5 schema/read-only historical inspection without translating records. Keep brn-core's unrelated sample lifecycle/headless checks.

Replace old sidecar-dependent CLI/workflow tests with A3 replay fixtures while preserving signal/deadline/output-loss assertions. No unguarded ambient live calls. Historical App Server experiment already has its own manifest and no dependency on the production provider crate; mark its operational guidance historical, not a fallback.

- [ ] **3. Remove Lance/Arrow dependency paths:** production retrieval uses selected SQLite/vector/embedding backend. Preserve the frozen comparison's old Lance implementation in `experiments/retrieval-comparison/src/lance_baseline.rs`, pinned to baseline origin, with its own standalone manifest/lockfile. No production old-backend feature or default fallback remains.

Update generation errors to explicit rebuild-needed; do not auto-delete old generations, unfinished work or credentials. The accepted scope has no Codex-thread compatibility conversion.

- [ ] **4. Add integration script:** `scripts/verify-rig-reset.sh` has deterministic default and optional `--native`, never a live mode. It runs shared fmt/Clippy/tests once, N1-N4/A1-A4 fixtures, standalone replay checks and packaging behavior tests. Native mode compiles/tests the selected native graphs and reports asset-gated checks that did not execute. It does not automatically fetch models, launch login or sign artifacts.

Update existing integrated import/search fixtures to register/open/approve a disposable `.md` note; legacy imports stay inspectable but cannot act as normal AI current knowledge. Update local unsigned launcher to pass selected data/model/config paths without a Codex executable.

- [ ] **5. Search/audit intentionally retained references:**

```sh
rg -n 'brn-provider|lancedb|codex_home|codex_configured|--codex' crates scripts Cargo.toml Cargo.lock
cargo tree --workspace --locked
```

Classify each result. Old migration columns/history labels can remain; active execution/config/build requirements cannot. Historical standalone lockfiles do not establish a production dependency. Reject token/default-home/environment fallback routing as well as explicit sidecar fallbacks.

- [ ] **6. Update implemented contracts:** actual architecture boundaries, independent outcomes, currentness/eligibility, basic save limitations, auth cache policy, configured capabilities and CLI reference. Keep original provenance/recovery invariants. Mark old specifications/plans historical superseded, not retrospectively executed.

- [ ] **7. Green/commit:** `bash scripts/verify-rig-reset.sh`; `bash scripts/verify-rig-reset.sh --native`; `bash scripts/test-make-macos-app.sh`; `bash -n scripts/*.sh`; local Markdown links/`git diff --check`. Record resource/native limitations. Commit as `refactor: retire App Server and Lance production backends`. No release claim.

## D2: Relocatable app, explicit signing and clean-machine qualification

**Prerequisite:** D1; distribution implementation authorization. G5 closes only with separate live/signing/clean-machine evidence.

**Files:**
- Create: `scripts/{package-macos.sh,test-package-macos.sh}`.
- Create: `packaging/macos/{Info.plist,entitlements.plist,README.md}`.
- Create: `docs/development/macos-distribution.md`.
- Modify: desktop main/resource discovery, tests/cli; verification/setup/status/index.
- Modify: model manifest/third-party notices required by the selected assets/dependencies.

**Consumes:** Delivered default/native executable, verified local model artifacts, explicit supported Mac target and separate signing/notarization authorization.

**Produces:** Sidecar-free relocatable .app, non-live packaging tests, versioned artifact manifest and G5 qualification record.

- [ ] **1. Red packaging behavior tests:** define script flags:

```text
package-macos.sh --stage assemble --binary ABSOLUTE_BINARY --model-dir ABSOLUTE_VERIFIED_MODEL_DIR --output NEW_ABSOLUTE.app --version VERSION --target RUST_TARGET
package-macos.sh --stage sign --bundle ABSOLUTE.app --identity SELECTED_IDENTITY
package-macos.sh --stage notarize --bundle ABSOLUTE.app --keychain-profile SELECTED_PROFILE
package-macos.sh --stage verify --bundle ABSOLUTE.app
```

There is no default stage that signs, uploads, installs, logs in or overwrites. `test-package-macos.sh` builds synthetic executable/model files and stub signing tools under one resolved disposable directory. Assert unknown/repeated/missing flags, wrong target, occupied output, credential/cache paths, symlink resources and hash mismatch fail; assemble calls no signing/notary commands. Assert exact executable/model bytes and relative resource locations.

Run `bash scripts/test-package-macos.sh`; expect missing script/incorrect bundle behavior.

- [ ] **2. Implement assemble:** declare one actual supported target/minimum macOS version from tested GPUI/ONNX/native requirements. Don't invent Intel/universal support. Build with `cargo build -p brn-desktop --release --locked --features native-ui,native-retrieval --target TARGET` only after required dependencies/resources exist.

Bundle native executable, verified local assets and required dylibs/notices. Inspect `otool -L` output and runtime search paths; eliminate development/Homebrew/checkout absolute dependencies. Resolve packaged resources relative to bundle; put mutable data/recovery/index and owner-private credentials outside it and outside selected vaults. Protect caches with A1 policy.

Model acquisition/bundling requires authorized verified assets and distribution-license review. The binary cannot silently download its missing model. Unsupported/missing resources report explicit unavailable state. Include artifact/model hashes, target, minimum OS, versions, selected dependency graph and build commit in a nonsecret manifest.

- [ ] **3. Test relocation:** copy the synthetic app to a new path with spaces/unicode, remove access to original synthetic input paths, and launch the stub to assert bundled resources/new application-private state discovery. Test empty machine-config state and missing assets. No account interaction or original data.

- [ ] **4. Review distribution risks:** applicable provider terms, direct-route documentation/support uncertainty, account entitlements, model/dependency licenses and privacy disclosures. Record authoritative dated sources and outstanding material decisions. Vendor confirmation is optional evidence, not a presumed mandatory application/email.

- [ ] **5. Stop for target/signing authorization:** use selected identity/profile only through installed macOS tools, without reading/logging credential material. Sign nested binaries/resources in correct order with minimally justified entitlements; verify hardened runtime. Notarize the explicit artifact with selected profile, staple and verify codesign/spctl/stapler. A locally ad-hoc-signed build is not a distributable substitute.

- [ ] **6. Clean-machine checklist:** on the declared supported Mac with no Rust/Codex/Copilot CLI/developer model paths, install the authorized quarantined artifact; observe first launch, resource loading, relocation and restart. Use synthetic vault/data. Explicitly Connect/select each direct provider/model/account, completion/stream/tools/typed candidate generation, credential reuse/reconnect and disconnect. Verify no ambient credentials, default provider fallback or development path.

Observe note Save/conflict/recovery, candidate review/adoption/separate Save, current-only retrieval and stale-history new-conversation flow. Inspect sanitized failures and cache ownership/modes. Record OS/architecture/artifact hashes/scenarios; mark natural expiry/revocation or unobserved accessibility cases honestly.

- [ ] **7. Green/commit and separate release decision:** `bash scripts/test-package-macos.sh`; selected native build/tests; authorized signing verification and named clean-Mac observations. G5 is blocked if either provider, packaging or material distribution risk remains unresolved. Commit packaging/contracts/evidence as `build: qualify sidecar-free macOS distribution`. Publish/merge/release only with a new explicit instruction.
