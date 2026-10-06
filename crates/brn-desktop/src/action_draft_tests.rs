use super::*;
use crate::review::action_fields::ActionFields;
use brn_workflow::actions::ActionState;
use brn_workflow::proposals::ActionChange;

fn action_review() -> (DraftRequest, ProposalRecord) {
    let mut record = crate::review::action_tests::fixture();
    record.draft.changes.clear();
    record.draft.sources.clear();
    record.draft.vault = None;
    let request = DraftRequest {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: record.draft.id,
        group_id: record.draft.group_id,
        session_id: record.draft.session_id,
        title: record.draft.title.clone(),
        sources: vec![],
        changes: vec![],
        action_changes: record.draft.action_changes.clone(),
    };
    request.validate().unwrap();
    (request, record)
}

#[test]
fn action_creation_ack_binds_every_ordered_kind_identity_and_full_before() {
    let (request, record) = action_review();
    assert!(creation_matches(&request, &record));
    let mut cases = vec![];
    let mut wrong = record.clone();
    wrong.draft.action_changes.pop();
    cases.push(wrong);
    let mut wrong = record.clone();
    wrong
        .draft
        .action_changes
        .push(record.draft.action_changes[0].clone());
    cases.push(wrong);
    let mut wrong = record.clone();
    wrong.draft.action_changes.reverse();
    cases.push(wrong);
    let mut wrong = record.clone();
    let ActionChange::Create { id, .. } = &mut wrong.draft.action_changes[0] else {
        panic!()
    };
    *id = Uuid::new_v4();
    cases.push(wrong);
    let mut wrong = record.clone();
    let ActionChange::Replace { before, data } = &wrong.draft.action_changes[1] else {
        panic!()
    };
    wrong.draft.action_changes[1] = ActionChange::Create {
        id: before.origin.id,
        data: data.clone(),
    };
    cases.push(wrong);
    let mut wrong = record.clone();
    let ActionChange::Replace { before, .. } = &mut wrong.draft.action_changes[1] else {
        panic!()
    };
    before.data.description.push_str(" drift");
    cases.push(wrong);
    for wrong in cases {
        assert!(
            !creation_matches(&request, &wrong),
            "misbound Action creation acknowledged"
        );
    }
    let mut later_review = record;
    later_review.version += 1;
    later_review.draft.title.push_str(" edited review");
    for change in &mut later_review.draft.action_changes {
        change.data_mut().description.push_str(" later review text");
    }
    assert!(creation_matches(&request, &later_review));
}

#[test]
fn action_only_invalid_typing_and_follow_up_guard_leave_and_separate_exactly() {
    assert!(DraftForm::new_action(Some(Uuid::nil())).is_none());
    let mut form = DraftForm::new_action(None).unwrap();
    assert!(form.can_leave());
    let action_id = form.action.as_ref().unwrap().id;
    let mut fields = form.action.as_ref().unwrap().fields.clone();
    fields.values[1] = "\u{feff} Full Eesti õ 日本語 🧭\r\n".into();
    fields.values[3] = "half-a-uuid".into();
    fields.values[7] = "2028-02-30".into();
    form.edit_action_fields(fields.clone());
    assert!(!form.can_leave());
    assert!(form.prepare().is_none());
    assert_eq!(form.action.as_ref().unwrap().fields, fields);
    let separate = form.separate().unwrap();
    assert_ne!(separate.id, form.id);
    assert_ne!(separate.action.as_ref().unwrap().id, action_id);
    assert_eq!(separate.action.as_ref().unwrap().fields, fields);
    assert!(!separate.can_leave());
    let followed = Uuid::new_v4();
    let follow_up = DraftForm::new_action(Some(followed)).unwrap();
    assert_eq!(
        follow_up.action.as_ref().unwrap().fields.values[11],
        followed.to_string()
    );
    assert_eq!(
        follow_up.action.as_ref().unwrap().fields.state,
        ActionState::Open
    );
    assert!(!follow_up.can_leave());
}

#[test]
fn action_submission_freezes_ids_and_full_payload_preserving_later_raw_fields() {
    let mut form = DraftForm::new_action(None).unwrap();
    let mut fields = ActionFields::from(action_review().0.action_changes[0].data());
    form.edit_action_fields(fields.clone());
    let id = form.action.as_ref().unwrap().id;
    let first = form.prepare().unwrap();
    assert!(first.request.changes.is_empty());
    assert_eq!(
        first.request.action_changes,
        vec![ActionChange::Create {
            id,
            data: fields.data().unwrap()
        }]
    );
    assert_eq!(first.request.title, fields.values[0]);
    assert!(form.separate().is_none());
    assert!(!form.can_leave());
    fields.values[1].push_str(" later raw typing\r\n");
    fields.values[5] = "half-source-uuid".into();
    form.edit_action_fields(fields.clone());
    assert_eq!(form.submitted.as_ref().unwrap().request, first.request);
    let mut returned = action_review().1;
    returned.draft.id = first.request.id;
    returned.draft.session_id = first.request.session_id;
    returned.draft.action_changes = first.request.action_changes.clone();
    assert!(form.created(first.operation, returned));
    assert_eq!(form.action.as_ref().unwrap().fields, fields);
    assert!(!form.can_leave());
    let separate = form.separate().unwrap();
    assert_eq!(separate.action.as_ref().unwrap().fields, fields);
    assert_ne!(separate.id, form.id);
    assert_ne!(separate.action.as_ref().unwrap().id, id);

    let mut retry = DraftForm::new_action(None).unwrap();
    retry.edit_action_fields(ActionFields::from(first.request.action_changes[0].data()));
    let first = retry.prepare().unwrap();
    retry.failed(first.operation, "response uncertain".into());
    let next = retry.prepare().unwrap();
    assert_ne!(first.operation, next.operation);
    assert_eq!(first.request, next.request);
    retry.failed(next.operation, "response uncertain again".into());
    let mut changed = retry.action.as_ref().unwrap().fields.clone();
    changed.values[0].push_str(" changed");
    retry.edit_action_fields(changed);
    assert!(retry.prepare().is_none());
    assert_eq!(retry.submitted.as_ref().unwrap().request, first.request);
}
