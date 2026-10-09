//! Bounded approved activity through the shared application worker.
use super::{
    error::CliError, expect_positionals, scan, sub_word, usage, Globals, Output, Scanned, Tokens,
};
use brn_workflow::activity::{
    ActivityActionChangeKind, ActivityChangeKind, ActivityPage, ActivityRequest,
};
use std::fmt::Write as _;

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    if sub_word(tokens, "activity", "list")? != "list" {
        return Err(usage("unknown activity subcommand"));
    }
    *name = Some("activity.list");
    scan(tokens, globals, &[("limit", true), ("before", true)])
}

pub(super) fn parse_command(scanned: &Scanned) -> Result<ActivityRequest, CliError> {
    expect_positionals(scanned, 0)?;
    let request = ActivityRequest {
        limit: scanned
            .value("limit")
            .map(|value| {
                value
                    .parse::<usize>()
                    .map_err(|_| usage("invalid --limit integer"))
            })
            .transpose()?
            .unwrap_or_else(|| ActivityRequest::default().limit),
        before: scanned.uuid("before")?,
    };
    request.validate().map_err(|error| usage(error.message))?;
    Ok(request)
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
}

pub(super) fn output(page: ActivityPage) -> Output {
    let mut text = String::new();
    if page.entries.is_empty() {
        text.push_str("No approved durable changes.\n");
    }
    for (index, entry) in page.entries.iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        let when = entry
            .approved_at_utc
            .as_deref()
            .map(escape)
            .unwrap_or_else(|| format!("{} ms since Unix epoch", entry.approved_at_ms));
        writeln!(text, "Approved at admission: {when}").expect("String write");
        writeln!(text, "{}", escape(&entry.title)).expect("String write");
        writeln!(text, "{}", escape(&entry.summary)).expect("String write");
        for change in &entry.changes {
            let kind = match change.kind {
                ActivityChangeKind::Created => "Created",
                ActivityChangeKind::Replaced => "Replaced",
                ActivityChangeKind::Trashed => "Trashed",
            };
            writeln!(text, "  {kind}: {}", escape(&change.path)).expect("String write");
        }
        for change in &entry.action_changes {
            let kind = match change.kind {
                ActivityActionChangeKind::Created => "Created",
                ActivityActionChangeKind::Replaced => "Replaced",
            };
            writeln!(
                text,
                "  {kind} Action {} · historical approved title: {}",
                change.action_id,
                escape(&change.title)
            )
            .expect("String write");
        }
        writeln!(text, "Operation: {}", entry.operation_id).expect("String write");
        writeln!(text, "Proposal: {}", entry.proposal_id).expect("String write");
        if let Some(undo) = &entry.undo {
            if let Some(member) = undo.trash_member {
                writeln!(
                    text,
                    "Trash restore of: {} (zero-based member {member})",
                    undo.operation_id
                )
                .expect("String write");
            } else {
                writeln!(text, "Undo of: {}", undo.operation_id).expect("String write");
            }
        }
        if let Some(group) = entry.group_id {
            writeln!(text, "Group: {group}").expect("String write");
        }
        if let Some(session) = entry.session_id {
            writeln!(text, "Session: {session}").expect("String write");
        }
    }
    if let Some(cursor) = page.next_before {
        writeln!(text, "Next older page cursor: {cursor}").expect("String write");
    }
    Output {
        text,
        data: serde_json::json!(page),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::activity::{ActivityChange, ActivityEntry, ActivityUndo};
    use uuid::Uuid;

    #[test]
    fn human_activity_escapes_controls_and_preserves_unicode_and_identities() {
        let operation = Uuid::new_v4();
        let proposal = Uuid::new_v4();
        let group = Uuid::new_v4();
        let session = Uuid::new_v4();
        let entry = ActivityEntry {
            operation_id: operation,
            proposal_id: proposal,
            group_id: Some(group),
            session_id: Some(session),
            title: "Review 日本語\n\u{1b}[31m".into(),
            approved_at_ms: 123,
            approved_at_utc: None,
            summary: "3 notes\r\t".into(),
            undo: None,
            action_changes: vec![],
            changes: vec![
                ActivityChange {
                    kind: ActivityChangeKind::Created,
                    path: "new\n日本語.md".into(),
                },
                ActivityChange {
                    kind: ActivityChangeKind::Replaced,
                    path: "old\u{2028}.md".into(),
                },
                ActivityChange {
                    kind: ActivityChangeKind::Trashed,
                    path: "trash.md".into(),
                },
            ],
        };
        let result = output(ActivityPage {
            entries: vec![entry],
            next_before: Some(operation),
        });
        assert!(result
            .text
            .contains("Approved at admission: 123 ms since Unix epoch"));
        assert!(result.text.contains("Review 日本語\\n\\u{1b}[31m"));
        assert!(result.text.contains("3 notes\\r\\t"));
        assert!(result.text.contains("Created: new\\n日本語.md"));
        assert!(result.text.contains("Replaced: old\\u{2028}.md"));
        assert!(result.text.contains("Trashed: trash.md"));
        for (label, id) in [
            ("Operation", operation),
            ("Proposal", proposal),
            ("Group", group),
            ("Session", session),
        ] {
            assert!(result.text.contains(&format!("{label}: {id}")));
        }
        assert!(!result.text.contains('\u{1b}'));
        assert!(!result.text.contains('\r'));
        assert!(!result.text.contains('\t'));
        assert!(!result.text.contains('\u{2028}'));
        assert_eq!(
            result.data["entries"][0]["title"],
            "Review 日本語\n\u{1b}[31m"
        );
        assert_eq!(result.data["next_before"], operation.to_string());
    }

    #[test]
    fn activity_action_inventory_human_and_json_keep_all_64_historical_identities_and_exact_titles()
    {
        use brn_workflow::activity::{ActivityActionChange, ActivityActionChangeKind};
        let changes: Vec<_> = (0..64)
            .map(|index| ActivityActionChange {
                kind: if index % 2 == 0 {
                    ActivityActionChangeKind::Created
                } else {
                    ActivityActionChangeKind::Replaced
                },
                action_id: Uuid::from_u128(index + 1),
                title: format!(
                    "{index} õ 日本語\r\n\t\u{1b}[31m\u{2028}{}TAIL",
                    "λ".repeat(200)
                ),
            })
            .collect();
        let output = output(ActivityPage {
            entries: vec![ActivityEntry {
                operation_id: Uuid::new_v4(),
                proposal_id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Action history".into(),
                approved_at_ms: 1,
                approved_at_utc: None,
                summary: "Created 32 actions; updated 32 actions.".into(),
                changes: vec![],
                undo: None,
                action_changes: changes.clone(),
            }],
            next_before: None,
        });
        assert_eq!(
            output.data["entries"][0]["action_changes"],
            serde_json::to_value(&changes).unwrap()
        );
        for item in changes {
            assert!(output.text.contains(&format!(
                "Action {} · historical approved title: {}",
                item.action_id,
                escape(&item.title)
            )));
        }
        for control in ['\r', '\t', '\u{1b}', '\u{2028}'] {
            assert!(!output.text.contains(control));
        }
        assert_eq!(
            output.text.matches("historical approved title:").count(),
            64
        );
    }

    #[test]
    fn empty_activity_explains_that_no_approved_changes_exist() {
        let result = output(ActivityPage {
            entries: Vec::new(),
            next_before: None,
        });
        assert_eq!(result.text, "No approved durable changes.\n");
        assert_eq!(
            result.data,
            serde_json::json!({"entries":[],"next_before":null})
        );
    }

    #[test]
    fn human_activity_identifies_whole_undo_and_scoped_trash_source() {
        let source = Uuid::new_v4();
        let entries = [None, Some(2)]
            .into_iter()
            .map(|trash_member| ActivityEntry {
                operation_id: Uuid::new_v4(),
                proposal_id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Restored 日本語".into(),
                approved_at_ms: 123,
                approved_at_utc: Some("2026-10-03T00:00:00Z".into()),
                summary: "1 note restored".into(),
                action_changes: vec![],
                changes: vec![ActivityChange {
                    kind: ActivityChangeKind::Created,
                    path: "restored.md".into(),
                }],
                undo: Some(ActivityUndo {
                    operation_id: source,
                    trash_member,
                }),
            })
            .collect();
        let result = output(ActivityPage {
            entries,
            next_before: None,
        });
        assert!(result.text.contains(&format!("Undo of: {source}")));
        assert!(result
            .text
            .contains(&format!("Trash restore of: {source} (zero-based member 2)")));
        assert_eq!(
            result.data["entries"][0]["undo"]["operation_id"],
            source.to_string()
        );
        assert!(result.data["entries"][0]["undo"]["trash_member"].is_null());
        assert_eq!(result.data["entries"][1]["undo"]["trash_member"], 2);
    }
}
