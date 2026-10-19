# Read-only Mac observation recipe

This supports MAC-01 and MAC-04 in [the native queue](native-queue.md), using only
the explicitly recorded new synthetic workspace. It is prepared from source;
no Mac action or native data store has been observed with it yet.

The CLI's `read` and `recovery` commands use `Workspace::open_existing_no_upgrade`,
which opens a writable Store and runs app bootstrap. No-upgrade is not a read-only
guarantee. `Store::open_reader` validates a read-only canonical snapshot but has no
recovery-session accessor. Use one short SQLite URI `mode=ro` transaction to inspect
both canonical text and its already-durable open recovery session. Do not invoke
BRN open/bootstrap, upgrade, Save, discard or a repair operation from the checker.

Source authority: [CLI entry](../../../../crates/brn/src/main.rs),
[app open/bootstrap](../../../../crates/brn-threads-app/src/lib.rs),
[Store/schema](../../../../crates/brn-threads-core/src/store.rs),
[read-only snapshot](../../../../crates/brn-threads-core/src/reader.rs) and
[edit-session access](../../../../crates/brn-threads-core/src/edit.rs).
The database name is `threads.sqlite3`; records serialize the complete Record in
`body`, with Note Markdown under `data.Note.markdown`. Open edit sessions retain
`id`, `note`, `base`, `generation`, `markdown` and `closed`.

## Observe one Note without changing authority

At execution, replace the explicit synthetic data directory, Note ID and output
path below with the identities recorded by the actor. Keep the evidence output
outside the workspace. The reader closes before the actor closes the app window;
no read transaction spans OS close/relaunch or pins an earlier WAL snapshot.
Token hashes and unrelated records are not selected.

```sh
python3 - /absolute/synthetic/data NOTE_ID > /absolute/evidence/preclose.json <<'PY'
import hashlib
import json
from pathlib import Path
import sqlite3
import sys

root = Path(sys.argv[1])
if not root.is_absolute():
    raise SystemExit("The recorded synthetic data directory must be absolute")
database = (root / "threads.sqlite3").resolve(strict=True)
connection = sqlite3.connect(database.as_uri() + "?mode=ro", uri=True)
try:
    connection.execute("PRAGMA query_only=ON")
    connection.execute("BEGIN")
    row = connection.execute(
        "SELECT id,version,body FROM records WHERE id=?", (sys.argv[2],)
    ).fetchone()
    if row is None:
        raise SystemExit("Recorded synthetic Note was not found")
    markdown = json.loads(row[2])["data"]["Note"]["markdown"]
    canonical = {"id": row[0], "version": row[1], "markdown": markdown}
    canonical["utf8_sha256"] = hashlib.sha256(markdown.encode("utf-8")).hexdigest()
    sessions = []
    for item in connection.execute(
        "SELECT id,note,base,generation,markdown,closed "
        "FROM edit_sessions WHERE note=? AND closed=0 ORDER BY id",
        (row[0],),
    ):
        session = dict(zip(
            ("id", "note", "base", "generation", "markdown", "closed"), item
        ))
        session["utf8_sha256"] = hashlib.sha256(item[4].encode("utf-8")).hexdigest()
        sessions.append(session)
    result = {"canonical": canonical, "open_sessions": sessions}
    connection.rollback()
finally:
    connection.close()
print(json.dumps(result, ensure_ascii=False, indent=2))
PY
```

For MAC-01 after final Save/reopen, compare the exact canonical UTF-8 bytes with
[save-final](native-fixtures/save-final.txt) and require no open session for that
Note. Record its actual version rather than predicting the number of Save events.
For MAC-04 before close, require that same canonical identity/version/text/hash,
plus the exact [unsaved draft](native-fixtures/unsaved-recovery.txt), its saved base,
session ID and generation. Close the reader, then perform actual OS close/relaunch
and re-observe. Compare all canonical and recovery fields exactly; the editor must
also visibly present the draft. A query result alone does not prove visible recovery.
Do not repair a mismatch or rewrite expected fixtures to fit it.

SQLite read-only access can use normal WAL read bookkeeping; this recipe promises
no canonical write or bootstrap/repair, not a filesystem with no lock activity.
It qualifies an already-durable draft, not unprocessed input, crash/power-loss
recovery, IME, accessibility or owner acceptance. Recheck schema/source before using
this recipe after a relevant change.
