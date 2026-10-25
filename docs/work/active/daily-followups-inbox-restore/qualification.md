# Follow-ups, inbox and Restore qualification

The three approved slices are implemented in [draft PR227](https://github.com/ewq100/brn-rust/pull/227).
Corrected production source is through `09e26825`; `1e4c91df` strengthens the
existing menu-to-Owner test without changing production. Local deterministic gates
passed. Actual Linux observation established the bounded outcomes below, with
explicit accessibility limitations. No new merge, release, live-model
usefulness or actual Mac acceptance is claimed.

The [record](record.json) owns current activity. The PR records the final published
head and its terminal CI result; earlier-head CI is not substituted for that gate.

## Baseline and implemented outcomes

Main remains `f60c5a2f2102f4925b1b182f9f1bab2739fde76d`, re-fetched on October25.
Its [resulting-main run](https://github.com/ewq100/brn-rust/actions/runs/40058075212)
passed all seven Mac/Linux/docs jobs; three optional Windows jobs failed. The gated
preview workflow skipped publication. Original worktrees and unpublished work were
preserved through the allowance interruption.

| Outcome | Integration commits | Delivered boundary |
| --- | --- | --- |
| Action follow-ups | `ff207ecc`, `13bff3e0`, `512e9eac` | Optional owner/waiting-on and strict ISO follow-up date, separate from due. Checked editing, complete review facts, migration, replay/Undo and unchanged real-world completion semantics. |
| Manual inbox | `4e58e77a`, `2e68d9db`, `504bab8e`, `b2adf4c1` | Explicit General-only folder consent and separate Scan; descriptor-bound bounded reads, durable status/retry, exact source/assets/provenance and canonical receipt recovery. No source writes, watcher, private-context inheritance or automatic scan. |
| Desktop Restore | `6c6b1ff4`, `36ca3779`, `11386576` | Existing staged engine with select/validate/confirm/publish/open flow. Fresh recovery copy, readable current records, original workspace untouched, copied inbox authorization reset. |
| Native corrections | `8ee6b0c0`, `09e26825`, `1e4c91df` | Bidirectional real-scroll witness, review navigation before expanded facts, exact task menus and real menu-to-input focus coverage. |

The inbox is nonrecursive: 256 entries,16MiB/file,64MiB source-input budget (plus a separate bounded recheck) and
64MiB converted Markdown/asset output per scan. Aggregate overflow leaves honest
pending outcomes; individual oversize fails. Prepared recovery obeys the output
budget too. Settings/generation checks share the existing write transaction.
Unchanged canonical receipts do not resurrect an owner-undone or deleted import.
Schema11→12 uses existing backup-first adoption; absent Action fields preserve old
serialized records, requests and revision bytes.

## Local deterministic evidence

Logs remain in `/workspace/brn-next-evidence/`. Checks used pinned Rust1.98.1,
saved native dependencies, unchanged locked document helpers and serialized Cargo
builds. Python process tests used the existing process-local subreaper.

| Check | Outcome | Log |
| --- | --- | --- |
| Full core targets | 228 passed | `core-all.log` |
| Integrated Action/inbox/project app targets | 35 passed: 4+22+9 | `integrated-app-corrected.log` |
| Python tooling | 205 run:204 passed, one Apple Silicon packaging skip | `python-final.log` |
| Strict default/scripted-document all-target Clippy | Passed | `clippy-default.log` |
| Full corrected native desktop and writing targets | 363+44+12 passed; one existing shared-host fixture ignored | `native-corrected-helpers.log` |
| Strict corrected native all-target Clippy | Passed | `clippy-native-final.log` |
| Stronger actual task-menu→Owner test at `1e4c91df` | Passed; types without clicking Owner, retains invalid-date/stale-version assertions | `action-menu-focus-final.log` |
| Rebuilt CLI journey | 108 steps,1,266 checks passed | `cli-journey-final.log` |
| Runtime CLI and native desktop builds | Passed | `build-cli.log`, `build-desktop-final.log` |
| Offline Save/recovery/search/export/backup/startup/refusal | Passed | `e2e-fixtures-final.log` |

The ignored desktop case needs separately seeded `BRN_DWR_SHARED_*` CLI intake
state. It is not counted as passed. Focused author totals overlap integrated
checks and are not added as distinct coverage.

The actual Stock DOCX/PNG inbox witness uses AI off and checks exact asset bytes,
source hash/profile, protected General content, untouched original and reopened
canonical replay without reconversion. Private-session regressions verify no scan
before consent, no import on confirmation, no private sentinel in General and no
queue dispatch on finish/cancel/disconnect. Restore also reopened a synthetic
schema10 backup produced by actual baseline code, preserving its original bytes.

## Actual Linux observation and remaining limits

On October25 the built runtime was exercised in a fresh isolated Xvfb/Openbox
session with software Vulkan and synthetic records only. The successful bounded
run is `/workspace/brn-next-evidence/native/run.v0QYrE`; binary hashes, record delta
and control states are retained in [the readback receipt](evidence/linux-readback.json).

- [Data screen](evidence/linux-data.png): conspicuous General disclosure, manual
  bounds, disabled unconfigured Scan, and separate-copy Restore explanation.
  AT-SPI reports Scan neither enabled nor sensitive and Choose backup enabled.
- [Saved Action](evidence/linux-action-saved.png): physical Owner click, ASCII
  entry and physical Save produce the visible owner and Saved notice. The checked
  record advances v1→v2 with `owner: Synthetic owner`, preserving Suggested state,
  description, due and evidence. A prior corrected run's independent CLI reopen
  also confirmed persistence after app cleanup.

A later controlled run with the compiler idle (`run.wyfobb`) opened the same task
menu and typed without an Owner-field click. The [entry capture](evidence/linux-action-entry.png)
visibly shows Synthetic owner before Save, followed by the saved row and v2 readback.
This establishes initial keyboard focus and visible text for that bounded Linux
observation. Earlier stale placeholder captures are retained; their precise timing
cause is not proved. It is not a broad rendering/performance guarantee.

This is not blanket native acceptance. Owner's AT focus remained on the window,
and its Text interface was unavailable.
Screen coordinates omitted the window origin; the successful physical probe used
explicit Window coordinates. Pinned GPUI has a bounds-update method with no found
caller, and input/frame focus mapping is a plausible contributor to AT focus;
these source findings are not a proved universal diagnosis. Track these limitations
in the native acceptance gate, not as a reason to rewrite the editor here.

No file-picker action was attempted after the historical portal admission denial.
No real folder was configured/scanned, no provider was contacted, and the original
workspace was not replaced. Actual inbox picker/import and Restore picker/confirm/
recovered-window interaction remain pending. The [Mac checklist](native-acceptance.md)
is entirely unperformed; Linux/CLI results do not satisfy Mac or owner acceptance.

## Failures retained and corrections

The first full native run at `c74f3033` had360 pass,3 fail,1 ignored:

| Failure | Diagnosed correction |
| --- | --- |
| Exact task menu omitted new Edit details | `09e26825` updates exact menu expectations, explicitly retaining terminal-task absence of state controls. |
| Review group toggle offscreen | `09e26825` moves the same expanded Action facts after group/member navigation. Original click/footer assertions remain and visibility checks strengthen them. |
| Backup control unreachable after visiting Export | `8ee6b0c0` fixes the downward-only test helper to scroll toward the target; full visibility and finite attempt bounds remain. Inbox/Restore roundtrip coverage added. |

Corrected full parallel execution passed; no flaky-test label or unexplained rerun
replaced diagnosis. `1e4c91df` additionally covers the real menu-to-input route.
Independent source review cleared these changes without test weakening.

Other retained attempts distinguish environment/probe errors from product fixes:

- The first Python invocation lacked saved Cargo configuration and encountered the
  known PID1 reaping limitation. Saved environment/subreaper passed without weakened assertions.
- An Actions fixture used Mac select-all on Linux; existing platform-aware keys fixed it.
- The first native build omitted saved fontconfig configuration. Later a shared
  target reused an older core artifact lacking Inbox exports; cleaning only core
  artifacts recompiled current source. Neither required a product workaround.
- The Stock test initially compared engine identity to a label including `ai=off`;
  corrected oracle checks identity and AI setting separately.
- The first corrected native invocation omitted helper paths (358 pass,5 conversion
  failures,1 ignored). Diagnostics showed Unavailable; normal paths/sibling discovery
  restored the same verified helpers and the complete suite passed.
- Review found prepared inbox recovery omitted output accounting; `504bab8e`
  adds the same bound before Apply with a retained failing-then-passing witness.
- Restore initially accepted readable SQLite with missing/malformed current records;
  staged typed-decoder validation now rejects them, without changing original backups.
- Early Linux probes used unsupported registry/role methods or an unconfigured
  session service, then confused Screen/Window coordinates and raced changing
  nodes. Original captures/logs remain. Corrected physical readback proves only the
  observed saved outcome; it does not erase the accessibility limits and earlier capture caveat above.

## CI and release boundary

[Run40066087095](https://github.com/ewq100/brn-rust/actions/runs/40066087095)
on old `c74f3033` is terminal: Mac/Linux core, Mac combined build and docs passed;
both native widget jobs failed; optional Windows build failed. Normal Mac log
download returned HTTP403 at storage redirect; no bypass was attempted. Local logs
supplied the diagnosed failures. Final corrected-head CI is pending at publication; its terminal receipt will be recorded on PR227.

No new BRN live-model calls occurred. Four authored meeting-note cases extend the
existing evaluator but remain unrun live; scripted integrity is not semantic quality.
Read-only admission/credential restrictions were not bypassed. Calculator remains
deferred. New merge, release and deployment require separate authorization.
