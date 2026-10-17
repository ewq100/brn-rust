//! Per-command help: `brn [--json] COMMAND --help`, generated from the
//! command table plus one summary and example per command.
use crate::spec::{self, Owner, Spec};
use serde_json::{json, Value};

/// `(command, summary, example)`; every command in the table has one entry.
pub(crate) static TEXT: &[(&str, &str, &str)] = &[
    ("init", "Create or open the workspace and print its settings. --purpose test creates a test workspace in a new folder only.", "brn --json --data-dir /abs/fresh-test init --purpose test"),
    ("records", "List every record (or one page with --limit).", "brn --json --data-dir DIR records --kind Note --limit 20"),
    ("notes", "List current general notes (or one page with --limit).", "brn --json --data-dir DIR notes --limit 20"),
    ("threads", "List chats that are not archived (or one page with --limit).", "brn --json --data-dir DIR threads --limit 20"),
    ("needs-you", "List chats waiting for the owner (or one page with --limit).", "brn --json --data-dir DIR needs-you --limit 20"),
    ("history", "List receipts (or one page with --limit, optionally for one record).", "brn --json --data-dir DIR history --record NOTE --limit 10"),
    ("candidates", "List unapplied candidates (or one page with --limit).", "brn --json --data-dir DIR candidates --limit 5"),
    ("skills", "List built-ins and inactive installed packages; local configuration is excluded from BRN backups/exports.", "brn --json --data-dir DIR skills"),
    ("skill-inspect", "Inspect exact instruction files, origin, installation scope and digest. No enablement or execution.", "brn --json --data-dir DIR skill-inspect --name document-review"),
    ("skill-install", "Owner-only: copy bounded instructions. Preserve originals: installed bytes are excluded from current backups/exports. Different bytes conflict; remove by exact digest before reinstall.", "brn --json --data-dir DIR skill-install --source /abs/package"),
    ("skill-remove", "Owner-only: remove an installed package only at its exact digest. Built-ins are preserved.", "brn --json --data-dir DIR skill-remove --name document-review --digest SHA256"),
    ("settings", "Print the workspace settings.", "brn --json --data-dir DIR settings"),
    ("pause", "Pause maintenance; stops Working runs.", "brn --json --data-dir DIR pause"),
    ("resume", "Resume the chosen maintenance mode.", "brn --json --data-dir DIR resume"),
    ("recovery", "List open edit sessions with unsaved text (or one page with --limit).", "brn --json --data-dir DIR recovery --limit 20"),
    ("topics", "Print the Topics note and its entries.", "brn --json --data-dir DIR topics"),
    ("topics-create", "Create a Topics note and point the workspace at it.", "brn --json --data-dir DIR topics-create --key topics-1"),
    ("projects", "List projects (with --trash yes, also deleted ones; or one page with --limit).", "brn --json --data-dir DIR projects --limit 20"),
    ("project-create", "Create a project.", "brn --json --data-dir DIR project-create --title-file /abs/title.txt --key p1"),
    ("project-chat", "Create a chat inside a project.", "brn --json --data-dir DIR project-chat --project P --title Planning"),
    ("project-document", "Create a document inside a project.", "brn --json --data-dir DIR project-document --project P --title Plan --text-file /abs/plan.md"),
    ("project-settings", "Change a project's title, instructions and knowledge use at its current version.", "brn --json --data-dir DIR project-settings --project P --base 3 --title T --instructions-file /abs/i.md --knowledge yes"),
    ("chat-move", "Move a chat into a project, or out of it with --project none.", "brn --json --data-dir DIR chat-move --thread CHAT --project P"),
    ("project-link", "Link a note to a project.", "brn --json --data-dir DIR project-link --project P --note N"),
    ("project-unlink", "Unlink a note from a project.", "brn --json --data-dir DIR project-unlink --project P --note N"),
    ("project-delete-preview", "Print what deleting a project removes; save it for project-delete.", "brn --data-dir DIR project-delete-preview --project P > /abs/preview.json"),
    ("project-delete", "Delete a project exactly as a saved preview describes.", "brn --json --data-dir DIR project-delete --preview-file /abs/preview.json --actions '{}' --key d1"),
    ("project-restore", "Restore a deleted project at its current version.", "brn --json --data-dir DIR project-restore --project P --base 4"),
    ("topics-choose", "Point the workspace at an existing Topics note.", "brn --json --data-dir DIR topics-choose --note N"),
    ("connections", "Show a note's linked neighbours.", "brn --json --data-dir DIR connections --note N --hops 2"),
    ("knowledge-check", "Check note structure; optionally seed a review chat.", "brn --json --data-dir DIR knowledge-check --seed-chat yes --key check-1"),
    ("model-install", "Download the pinned embedding model (native builds).", "brn --json --data-dir DIR model-install --destination /abs/fresh-model-dir"),
    ("note-create", "Create a note.", "brn --json --data-dir DIR note-create --title Plan --text-file /abs/plan.md --key plan-1"),
    ("thread-start", "Start a chat.", "brn --json --data-dir DIR thread-start --title Handoff --key chat-1"),
    ("delegation", "Choose how maintenance runs: maintain or review-first.", "brn --json --data-dir DIR delegation --choice review-first"),
    ("read", "Print one record, or a bounded slice of a note with --limit.", "brn --json --data-dir DIR read --id N --limit 65536"),
    ("revisions", "List a note's accepted revisions.", "brn --json --data-dir DIR revisions --note N --limit 10"),
    ("revision", "Print a slice of one accepted note revision.", "brn --json --data-dir DIR revision --note N --version 2"),
    ("runs", "List runs, optionally for one chat or state.", "brn --json --data-dir DIR runs --state working"),
    ("messages", "List one chat's messages in creation order, one page at a time.", "brn --json --data-dir DIR messages --thread CHAT --limit 20"),
    ("operation", "Show where an operation stands, by id or by a keyed command's key.", "brn --json --data-dir DIR operation --key plan-1 --command note-create"),
    ("search", "Search current notes and saved chat messages; page either lane with --cursor (result.cursor or result.message_cursor).", "brn --json --data-dir DIR search --query 'loading depth' --limit 5"),
    ("begin-edit", "Open an edit session on a note at a base version; its token is returned once (result.token, or the new owner-only --token-out file).", "brn --json --data-dir DIR begin-edit --note N --base 2 --token-out /abs/edit.token"),
    ("update-buffer", "Replace your session's unsaved text, with your session's token.", "brn --json --data-dir DIR update-buffer --session S --generation 1 --text-file /abs/draft.md --token-file /abs/edit.token"),
    ("save", "Save your session's text as the next note version, with your session's token.", "brn --json --data-dir DIR save --session S --generation 1 --token-file /abs/edit.token"),
    ("discard", "Discard your session's unsaved text and close it, with your session's token.", "brn --json --data-dir DIR discard --session S --generation 1 --token-file /abs/edit.token"),
    ("reclaim-edit", "Test workspaces only: a new token for an open edit session whose token was lost; the earlier token stops working.", "brn --json --data-dir DIR reclaim-edit --session S --token-out /abs/new.token"),
    ("apply", "Apply one exact reviewed candidate.", "brn --json --data-dir DIR apply --operation OP"),
    ("dismiss", "Dismiss one exact candidate.", "brn --json --data-dir DIR dismiss --operation OP"),
    ("undo", "Undo an applied operation with a compensating change.", "brn --json --data-dir DIR undo --operation OP --key undo-1"),
    ("archive", "Archive a chat; stops its runs and pauses queued work.", "brn --json --data-dir DIR archive --thread CHAT --key a1"),
    ("resolve", "Alias of archive.", "brn --json --data-dir DIR resolve --thread CHAT"),
    ("unarchive", "Unarchive a chat.", "brn --json --data-dir DIR unarchive --thread CHAT"),
    ("keep-in-brn", "Keep a private chat attachment as general knowledge.", "brn --json --data-dir DIR keep-in-brn --note N --base 1 --key k1"),
    ("queue", "Queue a message for a chat.", "brn --json --data-dir DIR queue --thread CHAT --text-file /abs/msg.md --key q1"),
    ("ask", "Run one model turn in a chat.", "brn --json --data-dir DIR --timeout 600 ask --thread CHAT --prompt-file /abs/prompt.md"),
    ("intake", "Import a document: useful information, a full note, a chat or a project attachment.", "brn --json --data-dir DIR intake --source /abs/doc.md --intent full --title Doc --key i1"),
    ("configure", "Choose the provider, model, effort and credentials.", "brn --json --data-dir DIR configure --provider Codex --model ID --effort medium --auth-file /abs/auth.json"),
    ("export", "Export readable files to a fresh absolute folder.", "brn --json --data-dir DIR export --destination /abs/fresh-export"),
    ("backup", "Write a complete backup to a fresh absolute path.", "brn --json --data-dir DIR backup --destination /abs/backup.sqlite3"),
    ("restore", "Restore a backup into a fresh data directory.", "brn --json --data-dir /abs/fresh restore --source /abs/backup.sqlite3"),
    ("upgrade", "Upgrade an older workspace after an external backup.", "brn --json --data-dir DIR upgrade --backup-directory /abs/backups"),
];

/// Longer notes for commands whose retries or recovery need explaining.
pub(crate) static NOTES: &[(&str, &str)] = &[
    ("intake", "A keyed full/chat/project intake replays when bytes, path, scope and title match a committed import, without converting or sending the document again; if that import found the bytes already imported, the retry converts again and records only an observation. --thread names a chat; naming a project there attaches the document to that project, which is an owner decision (use --intent project --project ID)."),
    ("messages", "Lists the chat's messages that are not archived, in creation order, also when the chat is archived (chat_archived says so). Partial replies appear as recorded (role assistant-partial). search's message lane differs: it leaves out Trashed chats and partial replies."),
    ("save", "Stale save (exit 3), only for a session your own begin-edit returned: 1. keep result.session.markdown; 2. discard --session <result.session.id> --generation <result.session.generation> --token-file OLD.token; 3. begin-edit --note N --base <conflict[0].version> --token-out NEW.token; 4. reapply your edit; 5. update-buffer --session NEW --generation 1 --text-file merged.md --token-file NEW.token; 6. save --session NEW --generation 1 --token-file NEW.token."),
    ("begin-edit", "holder caller means a CLI caller holds the token; holder desktop means only the owner, in the BRN desktop app, can continue or close it. A retried begin-edit on a note whose session you opened gets edit_session_open with holder caller: in a test workspace run reclaim-edit; otherwise report the note to the owner."),
    ("reclaim-edit", "In an owner workspace this is refused with test_workspace_required: the owner recovers the edit in the BRN desktop app. The buffer and generation are kept."),
    ("init", "Never create the desktop's own data folder with --purpose test; use a fresh throwaway folder. A folder that already holds a workspace is refused with invalid_request reason workspace_exists. The CLI cannot change a workspace's purpose afterwards."),
];

fn text_of(name: &str) -> (&'static str, &'static str) {
    TEXT.iter()
        .find(|(command, _, _)| *command == name)
        .map(|(_, summary, example)| (*summary, *example))
        .unwrap_or(("", ""))
}
pub(crate) fn summary(name: &str) -> &'static str {
    text_of(name).0
}
pub(crate) fn example(name: &str) -> &'static str {
    text_of(name).1
}
pub(crate) fn note(name: &str) -> Option<&'static str> {
    NOTES
        .iter()
        .find(|(command, _)| *command == name)
        .map(|(_, note)| *note)
}

/// `brn --json COMMAND --help`: the command's description entry.
pub(crate) fn json(spec: &Spec) -> Value {
    json!({"contract": crate::contract::CONTRACT, "command": spec::describe(spec)})
}

/// `brn COMMAND --help`: plain text from the same entry.
pub(crate) fn text(spec: &Spec) -> String {
    let entry = spec::describe(spec);
    let list = |value: &Value| -> String {
        value
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    };
    let mut out = format!("brn {} — {}\n\n", spec.name, summary(spec.name));
    let required: Vec<String> = spec
        .required
        .iter()
        .map(|name| {
            if spec.options.contains(&format!("{name}-file").as_str()) {
                format!("--{name} VALUE (or --{name}-file ABSOLUTE_FILE|-)")
            } else {
                format!("--{name} VALUE")
            }
        })
        .collect();
    out.push_str(&format!(
        "Usage: brn [--json] --data-dir ABSOLUTE_DIRECTORY [--timeout SECONDS] {}{}{}\n",
        spec.name,
        if required.is_empty() { "" } else { " " },
        required.join(" ")
    ));
    let optional: Vec<String> = spec
        .options
        .iter()
        .filter(|name| !spec.required.contains(name))
        .map(|name| format!("--{name}"))
        .collect();
    if !optional.is_empty() {
        out.push_str(&format!("Options: {}\n", optional.join(" ")));
    }
    for rule in spec.required_when {
        out.push_str(&format!(
            "With --{} {}: also {}\n",
            rule.option,
            rule.equals,
            rule.required
                .iter()
                .map(|r| format!("--{r}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    for value in entry["values"].as_array().into_iter().flatten() {
        let grammar = match value["grammar"].as_str() {
            Some("enum") => list(&value["values"]),
            Some("integer") => format!("integer {}..{}", value["min"], value["max"]),
            Some("cursor") => format!("cursor ({})", value["form"].as_str().unwrap_or("")),
            Some("absolute_path") => "absolute path".to_owned(),
            other => other.unwrap_or("").to_owned(),
        };
        out.push_str(&format!(
            "  --{}: {grammar}\n",
            value["option"].as_str().unwrap_or("")
        ));
    }
    let retry = &entry["retry"];
    out.push_str(&format!(
        "Retry: {}{}\n",
        retry["default"].as_str().unwrap_or(""),
        retry["keyed"]
            .as_str()
            .map(|keyed| format!("; with --key: {keyed}"))
            .unwrap_or_default()
    ));
    for variant in retry["variants"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "  with --{} {}: {}{}\n",
            variant["when"]["option"].as_str().unwrap_or(""),
            variant["when"]["equals"].as_str().unwrap_or(""),
            variant["retry"].as_str().unwrap_or(""),
            variant["keyed"]
                .as_str()
                .map(|keyed| format!("; with --key: {keyed}"))
                .unwrap_or_default()
        ));
    }
    match spec.owner {
        Owner::Yes => {
            out.push_str("Owner decision: yes. Run it only on the owner's explicit instruction.\n")
        }
        Owner::WithOptions(options) => out.push_str(&format!(
            "Owner decision: with {}.\n",
            options
                .iter()
                .map(|o| format!("--{o}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Owner::No => {}
    }
    for when in entry["owner_decision_when"]
        .as_array()
        .into_iter()
        .flatten()
    {
        out.push_str(&format!(
            "Owner decision when --{} names a {}: {}.\n",
            when["option"].as_str().unwrap_or(""),
            when["names"].as_str().unwrap_or(""),
            when["note"].as_str().unwrap_or("")
        ));
    }
    if !spec.outcomes.is_empty() {
        out.push_str(&format!("Outcomes: {}\n", spec.outcomes.join(", ")));
    }
    out.push_str(&format!("Example: {}\n", example(spec.name)));
    if let Some(note) = note(spec.name) {
        out.push_str(&format!("Notes: {note}\n"));
    }
    out.push_str("Machine contract: brn --json --help; this command: brn --json COMMAND --help.");
    out
}
