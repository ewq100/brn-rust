# Review-group accessibility preparation

Read-only research at source7e41da52; stopped when allowance reached the5% reserve.
No implementation, build, app, account call or fixture creation occurred.

The selected feedback toggle fixes are qualified in PR176. A separate source-level
finding remains: generic Review group Include/Omit uses selected styling without
boolean accessibility metadata. Do not broaden navigation or one-shot actions.

The existing v2 writing fixture cannot expose this control. The shared Review
renders group controls only for more than one root group (`writing.rs`2017–2020,
2403–2435). The fixture (`agent_runs.rs`706–840) prepares one Note root; writing-
evidence Messages are not separate groups. Group roots are only Note/Action writes
(`review_groups.rs`57–63). Opening the same candidate through Changes does not
remove this condition. No generic prepare or Action-create CLI route exists.

The smallest reusable preparation is the existing synthetic generic-review test
fixture in `crates/brn-desktop/src/threads_review_tests.rs`: seed two Open Actions
through `Workspace::owner_change` with public Action/Put/ChangeRequest types
(174–227), then use the normal operation/Store::prepare path in `review_writes`
(129–159). Existing group tests exercise omit/Prepare selection (1060–1105) and
strict subset/restore (1875–1902). They do not create an on-disk native fixture.

After capacity returns, choose a small offline fixture example that reuses the
fresh absolute output-root refusal and no-credential-discovery pattern in
`crates/brn-threads-app/examples/native_writing_fixture.rs`; seed two Actions and
prepare the public multi-write request. Keep it test/developer-only, source-review
it, retain existing-file/directory refusal and synthetic exact-state assertions,
then qualify the actual group AX on/off transition against its matching native
binary. The current v1 example remains a one-note fixture; do not claim it already
implements the generic scenario or add a production CLI route just for this test.
