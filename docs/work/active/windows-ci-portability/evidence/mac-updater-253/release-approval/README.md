# Proposed command-enabled alpha pair (not approved)

A: v0.2.0-alpha.1/build3. B: v0.2.0-alpha.2/build4. Both use native-ui, macos-arm64, minimum15.0, normal ad-hoc signing. These supporting manifest copies are outside release/preview.json. The draft approval proposal separately changes that file for A; neither draft nor local artifact grants publication authority.

Selected runtime is a921e32533dc03134ed8a2001bd82f27194c3302 (264/265). Local A/B candidates built and qualified from that exact clean source; see the current alpha-a921e325 evidence. Actual published A source will be the later owner-approved current-main manifest commit. Prepare an approval PR changing release/preview.json and actual release notes; record full source SHA and locally qualified ZIP/manifest/SHA256SUMS identities. Owner release-manifest approval precedes that PR merge/publication. Existing exact-main/required-CI/immutable-asset workflow must publish A. B is a later increasing approved manifest commit, with final source identity and same checks, then the unchanged shipped A updater requests real B over fixed-repository HTTPS.

Rebuild from each actual approved source, not from these placeholder preparations. No source SHA/artifact hash is invented here. Install A once from its announced ZIP; legacy published previews cannot self-update. Native acceptance remains one coordinated clean-profile normal Gatekeeper/Copilot session.

## Source and artifact selection

| Identity | A | B |
|---|---|---|
| Proposed version/build | v0.2.0-alpha.1 / 3 | v0.2.0-alpha.2 / 4 |
| Exact source | Selected runtime/local candidatea921e325; actual release source pending approved manifest merge | Pending subsequent approved increasing manifest commit |
| Desktop features/platform | native-ui / macos-arm64 / minimum15.0 | Same |
| Bundled helpers | Locked stock Xberg1.3.10; existing BRN-adapted copy | Same unless separately reviewed |
| Copilot dependency | Existing Rig transport; no additional CLI/SDK binary | Same |
| ZIP/manifest/SHA256SUMS hashes | Local identities recorded in ../alpha-a921e325/package-identity.json; hosted identities pending approval/workflow | Same |
| Release state | Not approved or published | Not approved or published |

The intended pair can differ only in its approved version/build and receipts;
any runtime delta must be recorded explicitly. A successful version replacement
does not establish a new runtime behavior. Keep the local substituted-transport
witness separate from the published stock witness.

[Draft announcement/release notes](notes-draft.md) are preparation text. Copy the
approved version into the workflow's existing notes file only in the relevant
release-manifest approval PR. Local source/artifact identities are now [qualified and recorded](../alpha-a921e325/package-identity.json);
actual hosted source/artifact identities remain pending the approved workflow. The
workflow rebuilds from the exact approved main commit; record its published
artifact identities independently, without claiming local and hosted ZIPs are
byte-identical.

## Stock HTTPS witness after approved publication

Run from a clean checkout of the selected integrated source with its matching
test-only CLI. Both directories below must be absent. These literal versions
remain proposals until their separate manifest approvals and publication finish.

```sh
python3 scripts/prepare-published-macos-update.py \
  --version-a v0.2.0-alpha.1 --version-b v0.2.0-alpha.2 \
  --output "$PWD/work/approved-published-pair"
python3 scripts/tests/test_macos_updater.py --qualify-update --stock-transport \
  --delivery-a "$PWD/work/approved-published-pair/A" \
  --delivery-b "$PWD/work/approved-published-pair/B" \
  --host-cli "$PWD/target/release/brn" \
  --evidence "$PWD/work/published-stock-update.json"
```

Retain preparation.json, both published manifests and checksums, the stock
updater identity, update output and the synthetic-state/backup-restore report.
Preparation refuses legacy or unpublished artifacts. It proves no update by
itself. Native installation, ordinary Gatekeeper approval and Copilot continuity
are recorded separately in the single coordinated session.
