//! The last workspace location (§3, AC-01): saved as host presentation state
//! whenever the primary destination changes, and restored on launch only when
//! its chat, document or project is still available. Restoring opens a view;
//! it never starts, resumes or scans anything.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum SavedLocation {
    NewChat,
    Chat {
        thread: String,
    },
    Document {
        note: String,
        project: Option<String>,
    },
    Project {
        id: String,
    },
    Page {
        page: SavedPage,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SavedPage {
    Library,
    Inbox,
    Projects,
    AllChats,
}

impl SavedPage {
    fn tile(self) -> TileKind {
        match self {
            Self::Library => TileKind::Notes,
            Self::Inbox => TileKind::Overview,
            Self::Projects => TileKind::Projects,
            Self::AllChats => TileKind::AllThreads,
        }
    }
}

impl ThreadsUi {
    /// The destination worth restoring, or `None` for transient surfaces
    /// (Settings, history, review) that keep the previous saved location.
    pub(super) fn current_location(&self) -> Option<SavedLocation> {
        if self.layout.settings_open() {
            return None;
        }
        Some(match self.layout.primary() {
            TileKind::Note | TileKind::Connections | TileKind::Compare => SavedLocation::Document {
                note: self.open_note_id()?.to_owned(),
                project: self.note_writing_scope.clone(),
            },
            TileKind::Notes => SavedLocation::Page {
                page: SavedPage::Library,
            },
            TileKind::Overview | TileKind::Actions | TileKind::Imports => SavedLocation::Page {
                page: SavedPage::Inbox,
            },
            TileKind::Projects => match &self.selected_project {
                Some(id) if !self.projects_trash => SavedLocation::Project { id: id.clone() },
                _ => SavedLocation::Page {
                    page: SavedPage::Projects,
                },
            },
            TileKind::AllThreads => SavedLocation::Page {
                page: SavedPage::AllChats,
            },
            TileKind::Chat | TileKind::Linked => match self.chat_thread() {
                Some(thread) => SavedLocation::Chat {
                    thread: thread.to_owned(),
                },
                None => SavedLocation::NewChat,
            },
            _ => return None,
        })
    }

    /// Save the location when it changed. A failed save is presentation-only
    /// and never interrupts the owner; the next change tries again.
    pub(super) fn persist_location(&mut self) {
        let Some(location) = self.current_location() else {
            return;
        };
        if self.saved_location.as_ref() == Some(&location) {
            return;
        }
        if let Ok(key) = serde_json::to_string(&location)
            && self.workspace.save_last_location(&key).is_ok()
        {
            self.saved_location = Some(location);
        }
    }

    fn live_record(&self, id: &str, kind: fn(&RecordData) -> bool) -> bool {
        self.records
            .iter()
            .any(|record| record.id == id && !record.archived && kind(&record.data))
    }

    /// Reopen the saved location when its identity is still available;
    /// otherwise stay on the new-chat start.
    pub(super) fn restore_location(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(location) = self
            .workspace
            .last_location()
            .ok()
            .flatten()
            .and_then(|key| serde_json::from_str::<SavedLocation>(&key).ok())
        else {
            return;
        };
        let is_note = |data: &RecordData| matches!(data, RecordData::Note(_));
        let is_project = |data: &RecordData| matches!(data, RecordData::Project(_));
        match &location {
            SavedLocation::NewChat => {}
            SavedLocation::Chat { thread } => {
                if self.thread_rows.iter().any(|row| row.id == *thread) {
                    self.open_thread(thread, window, cx);
                }
            }
            SavedLocation::Document { note, project } => {
                if !self.live_record(note, is_note) {
                    return;
                }
                match project {
                    Some(project) if self.live_record(project, is_project) => {
                        self.open_project_note(project, note, window, cx);
                    }
                    Some(_) => return,
                    None => self.open_note(note, None, window, cx),
                }
                if self.open_note_id() != Some(note.as_str()) {
                    self.notice.clear();
                    self.layout = Layout::default();
                    return;
                }
            }
            SavedLocation::Project { id } => {
                if self.live_record(id, is_project) {
                    self.open_project(id, window, cx);
                }
            }
            SavedLocation::Page { page } => self.show_tile(page.tile(), window, cx),
        }
        self.saved_location = self.current_location();
    }
}
