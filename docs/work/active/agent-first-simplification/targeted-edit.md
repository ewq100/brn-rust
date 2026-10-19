# M3 exact local edit — implementation preparation

Selected outcome: [owner plan M3](plan.md#m3-add-one-targeted-editing-capability)
lets an agent replace one genuinely local passage without reading/retransmitting
a long document. This remains preparation, not an implemented tool or authority
grant. Baseline lookup is lead333345fb plus current workstream records; reconcile
M1's owned tool-registration/validation interface before editing the same source.

## Reuse and scope

A bounded Luna medium pass and lead source inspection found no existing local-edit
operation. Existing read_note returns exact versioned UTF-8 byte offsets, core
exact_writing_range checks an exact comment quote/range without relocation, and
Rust string get/replace_range can perform one checked replacement. Reuse that
byte-exact convention and the existing prepare/apply/review/Undo/service path.
No text-edit dependency, patch language/database, editor fork, schema, raw SQL
write or general editing framework is needed.

Use one explicit range: note ID, expected version, start/end byte offsets, exact
expected_text, replacement_text and request_key, with a useful existing-style
reason. The first slice may replace/delete a nonempty range (start < end). Repeated
wording is disambiguated by the supplied exact range; never search or silently
relocate to another occurrence. Zero-width insertion/multiple edits are outside
this first operation unless a real accepted caller requires them.

## Host construction and retained authority

The host checks live scope/current access, active Note/base version, valid UTF-8
boundaries and exact expected bytes. It constructs the complete candidate from
that checked base, changing Markdown only. Preserve all other Note fields/record
identity and untouched bytes, subject to the existing core rule that a changed
content revision clears confirmation without Confirm authority. It must not
change protection, import metadata, visibility, archive, supersession or assets.

An edit covering the complete current Markdown must retain the ordinary whole-read
requirement (or refuse with guidance to use whole replacement); range syntax must
not become a way to choose a whole-document guard bypass.

Only an actual host-constructed exact-edit value may select the local-edit path.
Use a private non-deserializable input/proof variant at the existing shared
preparation owner, not a model-supplied bypass boolean or mutation mode. Preserve
current scope/project inputs, Run operation tracking, immutable request identity,
prepared-output limits, attention and exact review. A host read does not update
the model full-read tracker; ordinary model-authored replacement still reaches
unread_replacement. No helper receives write tools.

Use existing allocate_bound_operation with a Run/tool-specific key and a
deterministic hash of the exact edit arguments, including reason. After live and
current-access checks, resolve/bind the operation first: changed arguments conflict.
If already prepared, recheck current scope and return the immutable preparation
without rebuilding it from later canonical text or comparing that text to the old
base. Otherwise require the active current Note at the supplied version, validate
range/expected bytes, construct the replacement, track the operation and share the
existing checked preparation/output tail. Existing Apply supplies receipt replay.
A fresh Continue has a new Run namespace; recover predecessor operations/receipts
through current continuation rather than implying cross-Run request-key replay.

Apply currently promotes every resulting Note version to complete model-read
coverage because the model authored its whole result. That assumption no longer
holds for a host-constructed local edit. Use existing Receipt.writes.before to
promote only when the complete prior version was actually known (or the model
authored a complete new Note); otherwise preserve incomplete coverage through
Apply and receipt replay. No new persisted provenance/patch log is needed.

Existing Apply decides maintenance, scoped protected edit, review-first, dirty
deferral and stale outcomes. The new operation only prepares; it cannot grant
protected access or bypass review. Whole-document Rewrite remains available and
proposal-only through retry/Continue/replacement. Exclude the new tool from writing_snapshot runs in this first slice, with a
defensive handler refusal. Keep actual registration, offered schemas, validation
and budget definitions consistent. Frozen Rewrite keeps writing_request, captured
inputs/evidence/attention and durable proposal-only checks. Supporting local edits
inside that mode needs a later explicit host adaptation; no model flag selects it.

## Required evidence and readiness

Use current runtime mock/wire/service witnesses, independently expected bytes and
meaningful old-behavior limitation where practical. No giant parallel suite.

- Edit a passage of a Note larger than a bounded read; edited passage changes,
  untouched prefix/suffix and permitted metadata stay exact. Host construction
  does not make a later ordinary whole replacement pass its full-read guard.
- Repeated wording at two offsets changes only the specified occurrence. Wrong
  expected text, non-boundary Unicode offsets, invalid range and stale base refuse
  without fuzzy relocation or canonical mutation. No normalization.
- Outside-scope private target stays unavailable; in-scope private content cannot
  be published to general knowledge. Protected content still needs applicable
  scoped instruction or exact review. Selected text alone grants nothing.
- Dirty owner writing defers Apply with buffer intact; a later edit makes the
  candidate stale. Review binds its exact result; existing Undo restores eligible
  content and preserves subsequent/conflicting work. Partial read → local edit →
  Apply and replayed Apply must still leave an ordinary whole replacement Invalid
  until an actual full read, including fresh Continue/recovered receipts.
- Identical prepare/Apply replay, lost-response continuation and conflicting key
  reuse behave through existing identities/receipts; no duplicate write.
- Explicit Rewrite remains proposal-only and its evidence/continuation contracts
  remain intact. Tool/schema/guidance and actual provider-history DTOs agree.

Independent Sol source/design review of lead69ceec10 identified and resolved
these three construction/replay/Rewrite specifications, including the post-Apply
read-ledger assumption. No source was changed or checks run by that reviewer.
Implementation remains gated on current M1 interface freeze and reconciliation. Existing campaign authority permits
this in-scope next slice; no owner decision is currently missing. Deterministic
checks, real-model usefulness, native interaction, owner acceptance and required
head/resulting-main verification remain separate. Broader live/provider activation
is not authorized.
