# Reference import intent ownership plan

Status: selected engineering candidate; independent plan review pending.
Baseline lead1395c92c, production Rust matching qualified source d98374f4. PR166
remains fixed at73780065 while this later work uses a separate branch.

A bounded source audit found no current reference-import leak or cancellation defect.
The exact setup target, purpose and idempotency key are captured before the file
picker, validated when its path returns, then stored in a separate UI option after
an Import worker starts. Converted completion consumes/clears it; disconnect clears
it, and the writing view combines worker kind with that option to show importing.
This payload belongs to one conversion invocation. It does not span independent
scoped search, approval or successor workers. See threads_ui.rs choose_import_for_reference,
convert_file, Converted completion, worker_disconnected and writing_view.

Adapt the existing private WorkerOperation::Import variant to own an optional
ReferenceImportIntent. Keep the captured payload type at its existing UI boundary;
the worker stores it without performing setup/service decisions. Use one existing
Worker::import constructor with an explicit optional intent; ordinary imports and
fixtures pass None, scoped reference imports pass Some. Expose a read-only accessor
that returns None for non-Import operations. No new operation kind, general owner,
builder, framework, service contract or dependency is needed.

Thread the captured option through the existing private convert_file admission
path and into worker construction. Retain the current file-picker busy check,
target validation and attachment-chat selection exactly. Do not assign metadata
after starting the worker. Unsupported paths/settings failures create no worker or
retained intent. Existing conversion settings, cancellation tokens and service job
remain unchanged. Preserve the distinction between full-note and useful imports.

At Converted delivery, copy the current worker's optional intent before retiring
it. Use that local intent for existing completion and off-context error handling;
retirement/disconnect drops the original structurally. Remove the separate UI field,
initialization, reset, post-start assignment and terminal/disconnect cleanup together.
Derive writing_view reference_importing from the current worker accessor plus the
same exact displayed note/scope comparison. Keep complete_writing_reference_import,
its checked target validation, dirty-setup handling, scoped import/attach rules,
idempotency, cancellation refusal and queue/successor policy unchanged.

Migrate existing fixtures into the supported import constructor; preserve actual
cancelled conversion, disconnected failed reference plus successor, navigation/stale
target, revoked scope, dirty setup and import idempotency witnesses. Ordinary useful
and full-note imports, worker protocol/Stop and scope behavior must remain covered.
Prefer these effect checks and Rust variant/visibility proof over mirrored getter
tests. No broad test deletion or live provider/model call is included. Offline
fixtures are not a successful live conversion/provider or native Mac acceptance.

Independent Sol must approve this source-backed plan before implementation. A Luna
medium worker can use the clean managed notice-state checkout on a new branch at
current source; preserve older helpers/binaries/evidence. One shared build slot.
Run focused reference/conversion/worker checks, then the relevant full native suite,
strict all-target lint, format/whitespace and production-only native-ui build. Retain
exact source, raw logs/commands and a new immutable artifact/hash. Reassess after two
failed approaches or an authority/lifetime ambiguity. Lead owns docs/integration;
independent meaningful candidate review and required head/main checks remain gates.
