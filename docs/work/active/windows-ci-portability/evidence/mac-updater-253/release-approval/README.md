# Proposed command-enabled alpha pair (not approved)

A: v0.2.0-alpha.1/build3. B: v0.2.0-alpha.2/build4. Both use native-ui, macos-arm64, minimum15.0, normal ad-hoc signing. These are proposed manifest bytes, outside the actual release manifest; they grant no publication authority.

Select exact A source after updater258 and A response-limit fixes merge. Prepare an approval PR changing release/preview.json and actual release notes; record full source SHA and locally qualified ZIP/manifest/SHA256SUMS identities. Owner release-manifest approval precedes that PR merge/publication. Existing exact-main/required-CI/immutable-asset workflow must publish A. B is a later increasing approved manifest commit, with final source identity and same checks, then the unchanged shipped A updater requests real B over fixed-repository HTTPS.

Rebuild from each actual approved source, not from these placeholder preparations. No source SHA/artifact hash is invented here. Install A once from its announced ZIP; legacy published previews cannot self-update. Native acceptance remains one coordinated clean-profile normal Gatekeeper/Copilot session.
