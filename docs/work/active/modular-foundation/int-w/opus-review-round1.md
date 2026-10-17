**Verdict: APPROVED** for deterministic and source readiness. This is not milestone acceptance; the unavailable gates are listed below.

I ran no Cargo commands, tests or GUI. I checked the pins, source, both diffs and the evidence, and found no blocking defect.

**Pins and evidence I verified**
- Both commits use crates tree `9097baf…`. My diff hashes match `review-diffs.json` (`21691050…`, `c809188e…`).
- Both red proofs reproduce: each committed test-only patch applies to `e8e98304` and gives the recorded test-source hash. The logs show the two failures (`cw2_review.rs:1213`, `threads_writing_tests.rs:4002`) at 0 passed, 1 failed.
- From red to final, the only test change is the added inbound-count Link and assertion. The original prefixes are unchanged.

**The two fixes**
- `export.rs:35-55` decides which Notes are exported from one full snapshot before the scoped Sources are dropped. Source metadata exclusion, assets of exported Notes only, staging, manifest and exclusive rename are unchanged. The only behavior change is hiding imported Notes whose Source is scoped or missing; no new policy.
- `threads_lists.rs:537,557` applies the same visibility check to the general ID/count set and to the displayed rows. The launcher and the linked/kept/recent sections take their rows from this list, so they are fixed too.
- I found no other leaking general-delivery path. Topics and knowledge-check already use the scoped record list, the reference picker uses scoped search, and the AI tools already apply the check.
- CLI `notes` (`lib.rs:272`, `inspect.rs:241`) is a documented owner view (`README:212`, alongside `records` and owner search), not scoped delivery.

**Shared contracts:** these match the accepted CW/B slices. Context resolution has no linked-chat fallback (`writing.rs:48-140`), Rewrite has one route, and Apply goes through `review_apply_in_context`. The library builds without GPUI by default and the retired desktop files are gone.

**Non-blocking findings (Low)**
1. `threads_lists.rs:557` evaluates the visibility check a second time. Using `general.contains(record.id.as_str())` would keep one source of truth and halve a cost that grows with imported Notes × records. The 5000-note scale test uses no imported Notes, so it doesn't exercise this.
2. The inbound-count assertion was never run red. Reasoning says it is the only guard on line 537, so a tiny principal recheck (remove 537 only) would give executed proof.
3. The export fixture puts no asset on the hidden reference, and the export scan doesn't look inside `assets/`. Asset exclusion holds by construction (`export.rs:63`) but has no test. An optional additive test would cover it.
4. Out of scope, existed before INT-W: if the owner applies a P1 rewrite that copies a P1-private image into the shared Note, general export stops with an error naming the asset ID (`assets.rs:19`). It is safe but blocks export.

**Gates not available**
- Hosted Ubuntu/macOS candidate runs.
- F20 native human/IME/picker/menu/quit/first-input checks, and the F18 native launch.
- Native `pdftoppm` (the carried PDF Unavailable failure).
- Live provider runs.
- Mutation testing of the new predicates.
- Owner and full-program acceptance.
