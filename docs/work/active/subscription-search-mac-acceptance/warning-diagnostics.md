# Provider warning diagnosis

## Observation and scope

[Search qualification 002](https://github.com/ewq100/brn-rust/issues/243#issuecomment-6522947555) failed on source `2764b52f73f2269d3ba770516f5314b2056c8522` after 1.406 seconds with `UnexpectedNotification { method: "warning", item_type: None }`. The executable did not retain the message. Actual provider dispatch, token/cache usage and cost remain unknown. Its single-attempt allocation is consumed and closed; the older six-attempt session and pilot evidence remain separate.

The owner subsequently requested investigation and a fix. This local correction addresses **protocol classification and missing diagnostic evidence**. It does not establish what that missing warning meant, fix an unobserved provider problem or qualify successful live search. No additional live request, hosted CI, push, ready transition, merge or native interaction is authorized by this document.

The installed qualified App Server (`codex-cli 0.162.0-alpha.17.2`) generates a `WarningNotification` schema with a required string `message` and optional string/null `threadId`. It has no severity or code. Therefore neither schema validity nor an English keyword proves that a warning is harmless. The original payload cannot be reconstructed from the failed result.

## Local correction

- A recognized warning stops with typed `ProviderWarning`, rather than an unknown-protocol label. One shared validator covers process reads/RPCs, pre-turn draining and result collection, so an earlier warning cannot disappear while waiting for another response.
- BRN requires nonblank UTF-8 warning text of at most 4096 bytes and validates the optional thread target. These bounds are BRN policy, stricter than the generated schema. A mismatched target or malformed warning is rejected without retaining its text. A notification containing a request ID remains unsafe.
- Warning text is untrusted, opaque and redacted in ordinary `Debug` and `Display`. Desktop errors and the qualification example's normal stdout contain no warning payload. The accessor is explicitly for private diagnostics, never instructions or a capability grant.
- The feature-gated qualification example optionally accepts `--private-warning-file ABSOLUTE_NEW_FILE`. Before dispatch it exclusively creates a fresh owner-only `0600` file in an existing canonical directory outside repositories, refusing existing files, symlinks and ambiguous paths. Capture is off by default and unavailable on platforms without this private-file contract.
- Only a bounded warning message is written to that explicit private sink. A write failure leaves the original search failure intact and reports that the diagnostic was not saved. No warning is admitted as search success; capability controls, deadlines, cancellation, source/citation requirements and retry policy remain unchanged.

Under a **future separately authorized** diagnostic allocation, the existing example can retain the evidence:

```text
subscription-search --live-public-query gpt-6.1-sol PUBLIC_QUERY --private-warning-file /absolute/fresh/private-warning.json
```

This is a manual interface reference, not permission to retry 002. Use a newly selected source/binary identity, explicit query and durable allowance. The file may contain private provider text: keep it local and inspect privacy before sharing any derived technical finding. Local process cleanup does not prove upstream cancellation or zero charge.

## Verification and remaining gates

The focused regression first reproduced the old unknown-notification message. The corrected candidate is checked with synthetic protocol messages and fake local subprocesses; no model/account call is needed for these tests. Cases cover bounded/private text, malformed fields, wrong threads, warning-as-server-request refusal, RPC/pre-turn interruption, cited-output refusal, private-file permissions/collision/symlink/repository refusal and write failure. Existing account, unsafe-tool, citation and deadline/cancellation tests remain applicable.

One read-only independent reviewer examines the complete provider/example candidate; the lead owns implementation and evidence, with one serialized build slot. This development review is separate from BRN's closed live-test allocation. Results are recorded in the task report and local gate receipts. Hosted results at `2764b52f` do not cover this later local correction. Successful live search and actual Mac interaction remain outstanding.

### Local results

- Focused regression: the original unknown-notification message was observed before correction; the corrected case passed.
- `cargo +1.98.1 test --locked --offline -p brn-ai subscription_search`: **22 passed**, none failed/ignored.
- `cargo +1.98.1 test --locked --offline -p brn-ai --features capability-spike --example subscription-search`: **3 passed**, none failed/ignored.
- Strict affected all-target Clippy passed for AI/application with `capability-spike` and desktop with `native-test-support`; the qualification example built successfully.
- Independent review found no actionable findings. Reviewed provider SHA256: `a392328d349c0ef4c4194c158e6f6faf34771f3360054ceeaa5fbf1919291ef5`; example SHA256: `2400b15a0951df44e4795ef5edd9bc036330f3f2c717e74706522ec5e4637a9c`.

These results cover the local diagnostic correction only. No new live request or hosted check ran; no push, merge or release occurred. The next technical evidence needed is an actual warning message under a new expressly bounded diagnostic allocation, not a retry of the closed allocation.

The later [fresh allocation003](diagnostic-allocation-003.md) captures a startup advisory on this correction and records the separately diagnosed per-child setting. This does not recover002's missing message or rewrite its failed result.
