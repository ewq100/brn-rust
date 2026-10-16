# B3 Opus round1 response and corrected native qualification

Round1 was **CHANGES_REQUIRED** at reviewed source ba7161cc/docs HEAD ed8574b5. [Full report copied verbatim](opus-review-round1.md), [actual new command/source/artifact associations](round1-checks.json). Fresh principal-owned Opus2 and acceptance remain open. No prior execution is relabelled to this correction.

| Finding | Disposition |
|---|---|
| **B3-1 Medium** | Corrected. A dirty bound brief/reference form is retained. Actual Converted completion saves the legitimate import, skips attachment/reload when that form is dirty, and gives explicit recheck/Save/Find instructions. Ordinary OpenSetup also preserves dirty edits instead of replacing them. |
| **R1 Low: query/cursor identity** | Corrected. Each request stores its query; Next uses that query, while Find starts a new search from the edited box. UI states which query the results belong to. Scope/generation/token fences and cancellation remain. |
| **R2 Low: store-wide invalidation** | Bounded correction only. Existing token privacy guard remains. Recheck and keep edits freshly loads the exact displayed context, rechecks the current saved Note version, refreshes only service-checked labels and retains pending edits. Pending additions stay unnamed/unchecked until checked Save. Saved-version conflicts remain conflicts; no automatic overwrite. No narrow-token/cache redesign. |
| **R3 Low: quotation title** | Corrected. Provenance uses the same already checked evidence source's title and exact version, with a generic fallback; no unscoped lookup. Existing fixture now asserts title and absence of raw ID. |
| **R4 Low: per-review-render setup read** | Retained, explicitly nonblocking. No new cache framework or performance claim. |
| **R5 Low: notices** | Corrected. Unresolved observation-only/replayed import says the file import was recorded and no eligible canonical Note was resolved, without asserting a Note exists. Core self-reference refusal has specific honest wording. Existing re-import fixture includes both notices, preserving all earlier assertions. |
| **R6 Low: test limits** | Retained honestly. The existing delayed reply test does not independently isolate every target/token predicate from generation. The configured fixture executes the actual request/preparer and scripted runtime but does not start the desktop shared worker; B1b carries that dispatch evidence. Freshness selection still uses its existing direct setup in that fixture. No broad duplicate campaign or extra worker claim. |

The retained draft never rebinds to another Note/scope. While context or access is invalid, no brief, source names, reference controls or Save render. It stays internally bound to its original context until the owner returns/rechecks or explicitly chooses **Reload saved (discard edits)**. That label and discard explanation are visible before the click. A failed recheck does not discard the form. Same-context recheck retains the original expected setup version, including when newer saved setup is discovered; Save refuses that conflict. No silent merge, version override, new membership rule or raw Store write.

## Genuine before and actual corrected owner path

The initial test-only before patch was prepared against exact ba7161cc before production correction: `3c8b1f29dc4187924ebb20a37c732009cb658f2cc70473c12c88bcbe32b61127`. Its first compile found two test-only FocusHandle calls missing the App argument. That compile defect is retained separately; only those two calls were repaired. The repaired fixture **compiled and failed behaviorally**, 0 passed / 1 failed / 285 filtered, at **Converted completion must not discard owner-entered brief text**: actual `Saved before conversion`, expected `Owner typed this brief during conversion`.

The before root `/private/tmp/brn-b3-opus1-before-20261017-source` has synthetic Git, no remotes and base tree3754ad17 exactly equal to ba7161cc. Its target `/private/tmp/brn-b3-opus1-before-20261017-target` never aliases candidate artifacts. Original and repaired fixture patches/logs/hashes remain distinct. The candidate additionally checks a concurrent saved-setup edit through real Recheck/Save; that later assertion is not fabricated baseline evidence.

The new native fixture uses real text input, purpose/remove/attach clicks, an actual Converted channel and existing poll_worker completion adapter. It checks brief and pending reference edits retained, legitimate import retained, no automatic setup write, unrelated canonical write then owner Recheck/Save, different scope, access loss/restoration, explicit discard and actual Save refusing a newer setup without a receipt. It does **not** claim a real converter/picker process ran. The new async paging fixture uses real scoped search and actual Find/Next controls after the owner edits the query.

Candidate first focused execution: 13 passed / 1 new notice-fixture failed. A new import key legitimately recreated an archived canonical Note, so that case did not represent an observation-only receipt with no eligible Note. The fixture now replays the actual existing observation-only receipt after archive; no assertion or importer behavior was weakened. All 14 focused fixtures then passed.

## Proportional normal checks

Pinned Rust 1.98.1, locked/offline, CARGO_INCREMENTAL=0, fresh synthetic TMPDIR; candidate target `/private/tmp/brn-b3-20261017-target` remains bound only to the same literal checkout. **14 new command/source/log receipts** include the before compile defect and red result, focused correction and ten successful normal gates:

- Native library strict all-target lint and **44 tests** passed.
- Native desktop build/strict all-target lint and complete suite **286 passed / 0 failed / 1 existing ignored**, 4.99 seconds.
- Whole-workspace fmt; default normal/feature graphs (**zero GPUI**); default workspace build/strict all-target lint passed.

The full native run preserves original A/B reduced motion, CW2 private disclosure/Review/Compare/cache/real typing/access-cycle, F15 editor Undo, queue/Stop/recovery, v1 and work-bound witnesses. Pre-B3 test prefixes remain exact (72,479 and 5,919 bytes); every pre-correction native test line remains in order. Correction changes five of the eight owned Rust paths: four production files and the native fixture file. App resolver/import tests, importer, AI/core/CLI, manifests/lock/CI/scripts are unchanged. App/retrieval/combined/full-default/E2E/headless broad lanes are not repeated solely for native form changes; their earlier counts retain earlier pins, including the carried PDF Unavailable failure. No current all-green/hosted/native-human/full F21/owner claim.

## Review and guard boundary

After this normal pin, only the two separately approved receipt-payload and final displayed-target omissions may run, with one cumulative 600-second cap including setup, copied passing baselines, variants, restore/reap/source/process receipts. B3-1 has its own required ordinary red/corrected proof above and is not attributed to those mutants. No B1b/D1B budget is reused. Guard outcome and heavy release will be appended as evidence only; source stays frozen for Opus2.


## Frozen corrected pin and completed bounded guard recheck

Corrected compiled source/test pin: `def881852d1f3aa24dca03f0f2bb52145b37faf6`; tree `45549edf4df9faab399537ff67c3d1243a18040f`; crates tree `181659aac2c095a9b38995abf9086739e1c20344`. Source is frozen for principal-owned Opus2. Material fingerprint: `72ac0f6e8bcc751d23886550406599789d8c17591442a46007f1de38e03aaf25` (328 inputs). The preceding preparation snapshots inside round1-checks.json retain their original `executed:false`; the separate actual command records document the subsequently executed compile defect, genuine behavioral before failure, corrected candidate and normal gates. They are not renamed historical executions.

[Guard recheck](guard-recheck.md) and [raw receipts](guard-recheck.json) preserve two copied passing baselines and exactly two compiled omission failures. Both omissions are CAUGHT. The displayed-target variant failed at the existing earlier no-attachment notice assertion; later state assertions were not reached. The controller's too-narrow equality-message matcher reported NO VERDICT/error, retained verbatim. A separate raw-log audit classified the successful compile and expected behavioral failure without any rerun or extra variant.

Cumulative B3 guard allocation closed at **217.265250/600 seconds**, including setup, baselines, two variants, restoration, reaping, source/process proof and completed receipt cleanup. Controller execution ended at 170.403376; the subsequent source/process audit was 202.378745. All 328 copied and candidate material hashes match the immutable pin, synthetic copy Git is clean/no remotes, own children are reaped and precise heavy-process receipt is empty. Heavy slot is explicitly released; all eight source/test hunks remain retained and frozen for Opus2. No more heavy execution is authorized by spare budget.
