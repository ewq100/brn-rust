use super::*;

#[test]
fn activity_labels_keep_unknown_and_future_times_distinct_from_inactivity() {
    let mut conversation = WorkConversation {
        id: Uuid::new_v4(),
        title: "Exact old session õ\r\n".into(),
        turns: 2,
        created_at_ms: 10,
        last_activity_at_ms: None,
    };
    let now = 40 * 86_400_000;
    assert_eq!(
        session_activity_label(&conversation, now),
        "Activity time unknown"
    );
    conversation.last_activity_at_ms = Some(now + 1);
    assert_eq!(
        session_activity_label(&conversation, now),
        "Activity time is ahead of this clock"
    );
    conversation.last_activity_at_ms = Some(now);
    assert_eq!(
        session_activity_label(&conversation, now),
        "Last active just now"
    );
    conversation.last_activity_at_ms = Some(now - 60_000);
    assert_eq!(
        session_activity_label(&conversation, now),
        "Last active 1 minute ago"
    );
    conversation.last_activity_at_ms = Some(now - 3_600_000);
    assert_eq!(
        session_activity_label(&conversation, now),
        "Last active 1 hour ago"
    );
    conversation.last_activity_at_ms = Some(now - 30 * 86_400_000);
    assert_eq!(
        session_activity_label(&conversation, now),
        "Last active 30 days ago"
    );
}

#[test]
fn unknown_unpersisted_and_old_json_turn_times_are_never_invented() {
    let mut state = ready();
    let request = state.ask("Synthetic unsaved partial".into()).unwrap();
    let partial = unfinalized_turn(&request.into(), "Exact partial õ\r\n".into());
    assert!(partial.started_at_ms.is_none());
    assert!(partial.finished_at_ms.is_none());
    let mut old_json = serde_json::to_value(&partial).unwrap();
    old_json.as_object_mut().unwrap().remove("started_at_ms");
    old_json.as_object_mut().unwrap().remove("finished_at_ms");
    let old: WorkTurn = serde_json::from_value(old_json).unwrap();
    assert!(old.started_at_ms.is_none());
    assert!(old.finished_at_ms.is_none());
    assert_eq!(old.answer.as_bytes(), partial.answer.as_bytes());
    let serialized = serde_json::to_value(old).unwrap();
    assert!(serialized["started_at_ms"].is_null());
    assert!(serialized["finished_at_ms"].is_null());
}
