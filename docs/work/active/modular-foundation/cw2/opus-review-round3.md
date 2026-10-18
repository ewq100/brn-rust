**Verdict: APPROVED** for source `12b0d3296ce1`. I found no blocking defects. This was a read-only review: no edits, builds, tests or delegation.

**The round-2 typing/request-batch blocker is resolved.**
- **Typing no longer triggers per-keystroke reloads.** Each keystroke calls `update_buffer`, then `cx.notify()`, then a render that calls `refresh_writing_review_cache`. Edit-buffer writes no longer change the token, so typing hits the cache.
- **The typing test can catch a regression.** If the token went back to `total_changes`, or if the review errored on every render, the load count would rise and the new `writing_review_loads == before_typing` assertion would fail.
- **The routing filter only removes unrelated requests.** Its checks match `from_message` exactly, and it parses the real nested `snapshot.note.id` field. Every request that passes the full check also passes the filter, so legitimate requests are not hidden. Every retained request still goes through the same full check. The forged matching hint is still refused, and verification work drops from 32 to 2.

**Token, scope and candidate safety are preserved.**
- **Local writes:** every write to `records` is paired with a new `revisions` row, and revisions are never deleted, so the watermark only moves forward. Prepare, retire, replace and apply all change the exact pending list, without a count or max-ID shortcut. Only buffer, draft and profile writes are excluded, and the review reads none of them.
- **External changes:** commits from other connections still change `data_version`.
- **Cache safety:** the token is checked before and after each load, the exact `Prepared` content and key are kept, and a failed load clears the cache. Apply and read still recheck fresh.
- **Aliasing:** `ThreadsUi` never swaps its workspace or store, so cross-store aliasing isn't reachable.

**Receipts:** the recorded `diff_sha256` matches `git diff 144848..12b0 -- crates` exactly, and the pinned hashes match the committed files for all six changed files.

**Non-blocking (LOW):** the per-render token query scans every row in `operations`, not just pending ones, because there's no partial index. This is the same as `candidates()` and was already disclosed. No change is needed.

Still open: the final mutation run, wider evidence, qualification and acceptance, which the parent owns. The privileged `review_replace`/Undo ancestry item stays deferred.
