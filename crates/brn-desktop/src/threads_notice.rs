//! Session-only workspace notices. Text, severity and work provenance change together.
use super::Tone;

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
}

impl NoticeState {
    fn current(&self) -> Option<&Notice> {
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
        matches!(self.current, Presentation::RememberedWork)
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
}
