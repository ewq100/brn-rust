//! Shared exact Action facts for desktop review surfaces.
use crate::Action;

/// Display absent facts explicitly: clearing a field is part of the reviewed
/// result, just as assigning a new owner or follow-up date is.
pub fn action_review_text(action: &Action) -> String {
    let shown = |value: &Option<String>| value.clone().unwrap_or_else(|| "Unspecified".into());
    format!(
        "Task: {} · {:?}\nOwner: {}\nWaiting on: {}\nDue: {}\nFollow up: {}\nCompletion evidence: {}\nWork: {}",
        action.description,
        action.state,
        shown(&action.owner),
        shown(&action.waiting_on),
        shown(&action.due),
        shown(&action.follow_up),
        shown(&action.evidence),
        if action.internal {
            "Internal BRN work"
        } else {
            "Human task"
        },
    )
}
