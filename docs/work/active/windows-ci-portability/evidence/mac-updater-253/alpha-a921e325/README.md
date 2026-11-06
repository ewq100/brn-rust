# Combined local alpha qualification — 6 November 2026

A's handoff is resolved. Selected source is clean main
`a921e32533dc03134ed8a2001bd82f27194c3302`, including264/265. B normally reconciled
that commit into the existing acceptance branch, then built from a clean detached
checkout of that exact source, in B's own target. No A CLI/target was reused.

## Packages and provenance

| Local candidate | Build | ZIP SHA256 |
|---|---|---|
| v0.2.0-alpha.1 | 3 | `879d08655cac787f1c7703847723dae6ede06c25fdeece85f573440fac611c70` |
| v0.2.0-alpha.2 | 4 | `cfab37c9e9da0c43298e776d83e2892880a5772ee6eb50d4b75d64bb9dd7f7c7` |

Local deliverables: task outputs/mac-alpha-a921e325/A and B on the Mac mini.
Both have native-ui, arm64-only binaries, macOS15 minimum, ad-hoc signing, stock
Xberg1.3.10 and the existing adapted helper. No extra Copilot CLI/SDK binary,
embedding model or provider credential is bundled. A/B executable hashes are
identical; this pair changes version/build/receipts only.

[Exact package/helper identity](package-identity.json),
[build context/commands/slot](build-context.json),
[A manifest](package-A.manifest.json), [B manifest](package-B.manifest.json),
[A checksums](package-A.SHA256SUMS), [B checksums](package-B.SHA256SUMS).
The unchanged bundled updater is SHA256
`7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5`.

These are local proposed-version candidates. Sourcea921e325 still has the legacy
preview.2 approved manifest. They are not approved/published release assets and
cannot substitute for a source-approved published pair. Publication must rebuild
and record each actual approved current-main release commit and hosted asset
identity through the existing workflow. Local and hosted ZIP byte identity is
not assumed.

## Observed headless checks

- Release desktop and matching test-only CLI: passed from exacta921e325, pinned
  Rust1.98.1, deployment15.0, jobs4. Existing rust-objcopy/libLLVM debug-strip
  warnings retained; actual shipped library/signature checks passed.
- Both helper commands passed in B's own targets (stock cached matching inputs;
  adapted recompiled). First attempt reached direct Cargo, which cannot process
  the script's +1.98.1 selector. [Failure](build-helpers-first-path-failure.log.gz)
  retained; installed rustup shims fixed command resolution without a source,
  toolchain or lock change. [Successful log](build-helpers.log.gz).
- [32 packaging/updater/preparation tests](packaging-updater-tests.log.gz): all
  passed, including19 updater failure/concurrency/recovery cases.
- [Relocated package](packaged-qualification.log.gz): startup/reopen and
  foreign-data refusal; both helpers/all8 formats and16 CLI imports with AI Off
  passed. [Synthetic captures](packaged-fixture-captures.json.gz).
- [Actual local signed A→B](local-A-to-B.json): real replacement, unchanged
  external data/settings/credential sentinel bytes, B headless reopening and
  backup/restore passed. **Transport alone is substituted**, not stock HTTPS.
  Its generic historical “not final integrated Agent A candidate” limitation is
  retained raw; the package receipts now explicitly bind A's selecteda921e325.
  Native/live acceptance is still pending.
- [Full synthetic backup/restore](full-restore.json): notes, records (including
  figure Asset), history, settings, projects and chats match exactly. Draft bytes
  and metadata survive; holder changes from caller to desktop as required by
  restore's token reset, and the old caller token refuses without mutation.
  [Archival verifier source](verify-full-restore.py), not an in-place runner.
  Its captured execution location was
  `/Users/evokessler/Documents/Codex/2026-11-06/you-are-agent-b-for-ewq100/work/final-alpha-a921e325/verify-full-restore.py`;
  PACKAGE derives from that task-work layout, not this evidence directory. To
  reproduce, put the copy in that layout, select cleana921e325 in the specified
  REPO path and run it there with the selected CLI/packages. Keep output outside
  this immutable evidence directory. The original body is retained unchanged.
  The [initial blanket-equality failure](full-restore-first-equality-failure.json)
  exposed the expected holder change, not lost data. A second verifier typo used
  Rust enum names instead of public JSON spellings; its [failure log](full-restore-verifier-enum-failure.log.gz)
  is retained. Corrected expectations follow types.rs/restore.rs; final check passed.
- [Selected-main CI42294766975](https://github.com/ewq100/brn-rust/actions/runs/42294766975):
  all4 required contexts and Documentation pass. Three inherited optional
  Windows failures remain visible; A's TEXT_DELTA issue is fixed by265.

## Remaining authority and native gates

Mac reports locked; no native app was driven. One disposable-profile session
with A requires normal Gatekeeper observation, tester Copilot device login and
the accepted chat/web/Luna/consolidation/writing/reconnect journeys, plus actual
update/reopen/data/backup checks. A second Copilot account is needed to observe
live account switching; quota/policy/no-Luna remain simulated where unavailable.

Real unchanged stock HTTPS A→B requires owner-approved, actually published,
command-enabled A/B through the existing manifest/immutable-asset process.
Legacy published previews lack updater. No release/tag/announcement or security
setting change occurred. Headless receipts are not native or owner acceptance.

B released the heavy slot at21:56:38Z, before the22:30Z cap. D/E have the free
window immediately through23:00Z. B holds no standing reservation afterward and
will coordinate a new short native window only when the owner is ready.

## Independent proposal review

Read-only review ofac5deeef againsta921e325 covered all51 changed paths,
compressed receipts/synthetic captures, approval/provenance/privacy boundaries,
and local/Shared metadata.43 evidence checksums matched. Runtime/CI/locks and
unrelated task objects/authority are preserved. No actionable approval,
provenance or privacy findings. One valid nonblocking P3: the archival verifier's
relative package path is invalid from its committed location. Disposition:
document its original execution placement above; keep the raw verifier/result
unchanged. No build, provider, native or publication authority was delegated.
