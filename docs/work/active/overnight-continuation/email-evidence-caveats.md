# Accurate MIME caveats and decoded thread identifiers

Baseline merged main `bd6947511ac9f5e445078e831d61ac3f5533fe06` (PR95).
The new Linden fixture has Message-ID, In-Reply-To and References and only a
plain-text body. Its maintained extraction retains those decoded strings, but
the helper unconditionally appends "authentication and absent message/thread
identifiers remain unknown; HTML is inert quoted source, remote resources
unavailable". Sol's retained answer interpreted the gap as absent identifiers
despite decoded strings. This demonstrates misleading scope wording, not a
decoder defect or verified identity. Root partial status is a separate contract.

A meaningful new test with two References values exposed another concrete adapter
loss: pinned mail-parser 0.11.8 HeaderValue::as_text returns only the final element
of a TextList. BRN tries it before as_text_list, dropping earlier decoded IDs.
Select the smallest adapter correction using the existing list accessor (which
also handles scalar Text), preserving every decoded In-Reply-To/References value
in source order. Restore the two-value regression and qualify unrelated ordered
multi-ID cases; a single-value fixture must not hide this valid-input defect.

Replace only the unconditional gap with: "decoded email headers are source
claims; sender authenticity and thread relationships have not been independently
verified". Preserve complete ordered decoded identifier values, unknown placeholders, inert HTML labels
only for actual HTML, conditional remote/non-CID and missing/ambiguous CID gaps,
and the partial status. Do not infer sender authenticity from headers or perform
network retrieval. Existing metadata already identifies unavailable fields; a
new missing-header diagnostic mechanism is unnecessary for this fix.

Adapt the existing mail-parser/MIME traversal/html5ever boundary, tests and crate
contract. No dependency, replacement parser, schema, snapshot migration, reconversion of old
evidence, runtime or approval change. Old schema-1 snapshots retain their original
wording and remain readable; new extractions get the clearer caveat.

Acceptance: unrelated complete plain-text headers retain all identifiers without
false absence/HTML/remote gaps; missing identifiers individually/together remain
unknown without invented threading; actual HTML retains inert source labels;
actual remote/CID issues retain precise gaps with no fetch; Authentication-Results
does not establish verified identity. Preserve old-gap partial snapshot validation
and reading without byte changes. Multi-value In-Reply-To/References preserve every decoded ID in order without claiming verified threading. Run focused maintained-helper tests, applicable
helper/workflow/shipping checks, one independent complete read-only review and
required hosted CI before normal integration. No new live call is needed to
qualify deterministic wording/delivery; preserve existing campaign outputs.

Lead owns integration/docs and selected model/effort. A bounded helper may own
only crates/brn-intake implementation/tests/README, with no recursive spawning,
GUI, live calls, private data, credentials, downloads or Git integration. At most
two active helpers and one serial Cargo process across overnight checkouts.
Put expected new-extraction UI wording in the single morning acceptance task;
never relabel retained old snapshots or claim interactive acceptance.
