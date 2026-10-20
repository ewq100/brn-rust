//! Session-only workspace notices. Text, severity and work provenance change together.
use super::Tone;
use std::collections::BTreeMap;

struct Notice {
    tone: Tone,
    text: String,
}

#[derive(Default)]
enum Presentation {
    #[default]
    Hidden,
    Ordinary(Notice),
    RememberedWork,
}

#[derive(Default)]
pub(super) struct NoticeState {
    current: Presentation,
    last_work: Option<Notice>,
    composer_recovery: BTreeMap<String, Notice>,
}

impl NoticeState {
    fn current(&self) -> Option<&Notice> {
        if let Some((_, notice)) = self.composer_recovery.first_key_value() {
            return Some(notice);
        }
        match &self.current {
            Presentation::Hidden => None,
            Presentation::Ordinary(notice) => Some(notice),
            Presentation::RememberedWork => self.last_work.as_ref(),
        }
    }

    pub(super) fn text(&self) -> &str {
        self.current().map_or("", |notice| notice.text.as_str())
    }

    pub(super) fn tone(&self) -> Tone {
        self.current().map_or(Tone::Info, |notice| notice.tone)
    }

    pub(super) fn is_work_status(&self) -> bool {
        self.composer_recovery.is_empty() && matches!(self.current, Presentation::RememberedWork)
    }

    pub(super) fn can_restore(&self, worker_active: bool) -> bool {
        !worker_active && self.text().is_empty() && self.last_work.is_some()
    }

    pub(super) fn set(&mut self, tone: Tone, text: impl Into<String>) {
        self.current = Presentation::Ordinary(Notice {
            tone,
            text: text.into(),
        });
    }

    pub(super) fn complete_work(&mut self, tone: Tone, text: impl Into<String>) {
        self.last_work = Some(Notice {
            tone,
            text: text.into(),
        });
        self.current = Presentation::RememberedWork;
    }

    pub(super) fn clear(&mut self) {
        self.current = Presentation::Hidden;
    }

    /// Recovery attention belongs to the failed Note handoff, rather than the
    /// latest work status. Only its successful retry or an explicit Dismiss
    /// removes it; ordinary input/startup clearing cannot acknowledge it.
    pub(super) fn composer_recovery_failed(&mut self, note: &str, text: String) {
        self.composer_recovery.insert(
            note.into(),
            Notice {
                tone: Tone::Error,
                text,
            },
        );
    }

    pub(super) fn composer_recovery_saved(&mut self, note: &str) {
        self.composer_recovery.remove(note);
    }

    pub(super) fn dismiss(&mut self) {
        let dismissed_recovery = if let Some(entry) = self.composer_recovery.first_entry() {
            entry.remove();
            true
        } else {
            false
        };
        if dismissed_recovery && matches!(self.current, Presentation::Ordinary(_)) {
            return;
        }
        self.clear();
    }

    pub(super) fn restore(&mut self, worker_active: bool) -> bool {
        if !self.can_restore(worker_active) {
            return false;
        }
        self.current = Presentation::RememberedWork;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_notices_and_clear_preserve_latest_completed_work() {
        let mut notice = NoticeState::default();
        assert_eq!(notice.text(), "");
        assert!(!notice.restore(false));
        notice.complete_work(Tone::Warning, "Work interrupted");
        assert_eq!(notice.text(), "Work interrupted");
        assert_eq!(notice.tone(), Tone::Warning);
        assert!(notice.is_work_status());
        notice.set(Tone::Error, "New error");
        assert!(!notice.restore(false));
        assert_eq!(notice.text(), "New error");
        assert_eq!(notice.tone(), Tone::Error);
        assert!(!notice.is_work_status());
        for _ in 0..2 {
            notice.clear();
            assert_eq!(notice.text(), "");
            assert!(!notice.restore(true));
            assert_eq!(notice.text(), "");
            assert!(notice.restore(false));
            assert_eq!(notice.text(), "Work interrupted");
            assert_eq!(notice.tone(), Tone::Warning);
            assert!(notice.is_work_status());
        }
        notice.complete_work(Tone::Success, "Replacement finished");
        notice.clear();
        assert!(notice.restore(false));
        assert_eq!(notice.text(), "Replacement finished");
        assert_eq!(notice.tone(), Tone::Success);
    }

    #[test]
    fn equal_ordinary_text_does_not_gain_work_provenance_or_severity() {
        let mut notice = NoticeState::default();
        notice.complete_work(Tone::Warning, "Same text");
        notice.set(Tone::Info, "Same text");
        assert_eq!(notice.text(), "Same text");
        assert_eq!(notice.tone(), Tone::Info);
        assert!(!notice.is_work_status());
        assert!(!notice.can_restore(false));
        assert!(!notice.restore(false));
        notice.clear();
        assert!(notice.restore(false));
        assert!(notice.is_work_status());
        assert_eq!(notice.text(), "Same text");
        assert_eq!(notice.tone(), Tone::Warning);
    }

    #[test]
    fn composer_recovery_has_scoped_attention_without_changing_ordinary_notice_lifecycle() {
        let mut notice = NoticeState::default();
        notice.composer_recovery_failed("a", "Copy draft A before closing".into());
        notice.composer_recovery_failed("b", "Copy draft B before closing".into());
        notice.set(Tone::Info, "New ordinary status");
        notice.clear(); // input/startup is not an explicit Dismiss
        notice.complete_work(Tone::Warning, "Work interrupted");
        assert_eq!(notice.text(), "Copy draft A before closing");
        assert_eq!(notice.tone(), Tone::Error);
        assert!(!notice.is_work_status());
        assert!(!notice.restore(false));
        notice.composer_recovery_saved("unrelated note");
        assert_eq!(notice.text(), "Copy draft A before closing");
        notice.dismiss();
        assert_eq!(notice.text(), "Copy draft B before closing");
        notice.composer_recovery_saved("b");
        assert_eq!(notice.text(), "");
        assert!(notice.restore(false));
        assert_eq!(notice.text(), "Work interrupted");
        assert_eq!(notice.tone(), Tone::Warning);
        assert!(notice.is_work_status());
        notice.set(Tone::Error, "Ordinary error");
        notice.complete_work(Tone::Success, "Next work finished");
        assert_eq!(notice.text(), "Next work finished");
        assert!(notice.is_work_status());
    }

    #[test]
    fn dismissing_recovery_preserves_a_distinct_ordinary_error_for_acknowledgement() {
        let mut notice = NoticeState::default();
        notice.composer_recovery_failed("a", "Copy draft A before closing".into());
        notice.set(Tone::Error, "Could not preserve this edit; copy your text.");

        assert_eq!(notice.text(), "Copy draft A before closing");
        assert_eq!(notice.tone(), Tone::Error);
        assert!(!notice.is_work_status());

        notice.dismiss();
        assert_eq!(
            notice.text(),
            "Could not preserve this edit; copy your text."
        );
        assert_eq!(notice.tone(), Tone::Error);
        assert!(!notice.is_work_status());

        notice.dismiss();
        assert_eq!(notice.text(), "");
        assert!(!notice.is_work_status());
    }
}
