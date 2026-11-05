//! Shell layout state: the primary view, its supporting pane, the sidebar and
//! the width-based pane policy (spec §8). Pure and independent of rendering.
//!
//! [`Layout`] holds the owner's preferences (sidebar, assistant visibility and
//! widths) next to the current route, in one value so the lead can persist it.
//! [`Layout::arrange`] turns it into what is drawn at a given usable width.
//! A responsive fallback is only ever an [`Arrangement`]; it never writes back
//! into the stored preferences.

use brn_writing_ui::theme::metrics;

/// The content kinds the shell routes to. Each keeps its existing body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum TileKind {
    /// Recent chats, shown in the sidebar.
    Threads,
    Note,
    Chat,
    Linked,
    Notes,
    AllThreads,
    Overview,
    Actions,
    /// The Inbox's Imports view (capture, file intake, manual folder inbox).
    Imports,
    History,
    Settings,
    Compare,
    Connections,
    Projects,
}
impl TileKind {
    pub(super) const ALL: [Self; 14] = [
        Self::Threads,
        Self::Note,
        Self::Chat,
        Self::Linked,
        Self::Notes,
        Self::AllThreads,
        Self::Overview,
        Self::Actions,
        Self::Imports,
        Self::History,
        Self::Settings,
        Self::Compare,
        Self::Connections,
        Self::Projects,
    ];
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Threads => "threads",
            Self::Note => "note",
            Self::Chat => "chat",
            Self::Linked => "linked",
            Self::Notes => "notes",
            Self::AllThreads => "all-threads",
            Self::Overview => "overview",
            Self::Actions => "actions",
            Self::Imports => "imports",
            Self::History => "history",
            Self::Settings => "settings",
            Self::Compare => "compare",
            Self::Connections => "connections",
            Self::Projects => "projects",
        }
    }
    /// The sentence-case destination name shown in headers and menus.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Threads => "Recent chats",
            Self::Note => "Note",
            Self::Chat => "Chat",
            Self::Linked => "Linked work",
            Self::Notes => "Library",
            Self::AllThreads => "All chats",
            Self::Overview => "Inbox",
            Self::Actions => "Tasks",
            Self::Imports => "Imports",
            Self::History => "Recent changes",
            Self::Settings => "Settings",
            Self::Compare => "Compare",
            Self::Connections => "Connections",
            Self::Projects => "Projects",
        }
    }
    /// Kinds that can sit beside a primary view in the supporting pane.
    fn supporting(self) -> bool {
        matches!(
            self,
            Self::Chat | Self::Linked | Self::Compare | Self::Connections
        )
    }
}

pub(super) use brn_writing_ui::layout::{MarginMode, NoteFit, NoteNeeds, ReviewFit};

/// The Note's own minimum before a comments margin is added beside it.
pub(super) const NOTE_MIN: f32 = 360.;

/// The widest comments placement that fits a document region `width` wide.
pub(super) fn margin_mode(width: f32) -> MarginMode {
    [MarginMode::Wide, MarginMode::Narrow]
        .into_iter()
        .find(|mode| NOTE_MIN + mode.width() <= width)
        .unwrap_or(MarginMode::Stacked)
}

/// The three keyboard regions, in visual order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pane {
    Sidebar,
    Primary,
    Assistant,
}

/// How the supporting pane is drawn at the current width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Support {
    /// Nothing beside the primary view.
    Hidden,
    /// Beside the document at this width, after a 1 pt divider.
    Docked(f32),
    /// Too narrow to dock: a temporary sheet over the primary view, shown
    /// only while `open`. The document keeps its full region.
    Sheet { open: bool, width: f32 },
}

/// What the shell draws at one usable width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Arrangement {
    /// Drawn sidebar width: the expanded width or the rail.
    pub(super) sidebar: f32,
    pub(super) sidebar_expanded: bool,
    pub(super) support: Support,
    /// Width left for the primary view.
    pub(super) primary: f32,
}
impl Arrangement {
    pub(super) fn support_visible(&self) -> bool {
        matches!(
            self.support,
            Support::Docked(_) | Support::Sheet { open: true, .. }
        )
    }
    pub(super) fn support_docked(&self) -> bool {
        matches!(self.support, Support::Docked(_))
    }
    /// The supporting pane's drawn width, docked or as a sheet.
    pub(super) fn support_width(&self) -> Option<f32> {
        match self.support {
            Support::Docked(width) => Some(width),
            Support::Sheet { open: true, width } => Some(width),
            _ => None,
        }
    }
}

/// Shell layout state. Preferences and route in one value; see the module docs.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Layout {
    /// The view filling the workspace.
    primary: TileKind,
    /// Where Settings returns to.
    before_settings: TileKind,
    /// The kind the supporting pane shows; the assistant (Chat) by default.
    support: TileKind,
    /// Owner preference: the supporting pane is open beside a document.
    support_open: bool,
    /// Owner preference, within the assistant's useful range.
    support_width: f32,
    /// Owner preference: sidebar expanded (otherwise the rail).
    sidebar_expanded: bool,
    /// The owner expanded the sidebar on purpose; the pane policy then
    /// keeps it expanded instead of collapsing it to make room.
    sidebar_deliberate: bool,
    sidebar_width: f32,
    /// Session only: the compact sheet is showing.
    sheet_open: bool,
    /// Session only: the keyboard region.
    focus: Pane,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            primary: TileKind::Chat,
            before_settings: TileKind::Chat,
            support: TileKind::Chat,
            support_open: true,
            support_width: metrics::ASSISTANT,
            sidebar_expanded: true,
            sidebar_deliberate: false,
            sidebar_width: metrics::SIDEBAR,
            sheet_open: false,
            focus: Pane::Primary,
        }
    }
}
impl Layout {
    pub(super) fn primary(&self) -> TileKind {
        self.primary
    }
    /// The supporting pane's kind while the primary view can host one.
    pub(super) fn support(&self) -> Option<TileKind> {
        self.hosts_support().then_some(self.support)
    }
    #[cfg(test)]
    pub(super) fn support_open(&self) -> bool {
        self.support_open
    }
    pub(super) fn support_width(&self) -> f32 {
        self.support_width
    }
    #[cfg(test)]
    pub(super) fn sidebar_expanded(&self) -> bool {
        self.sidebar_expanded
    }
    #[cfg(test)]
    pub(super) fn sidebar_width(&self) -> f32 {
        self.sidebar_width
    }
    pub(super) fn pane(&self) -> Pane {
        self.focus
    }
    /// Only a document or a conversation has a supporting pane; a
    /// conversation never hosts a second conversation.
    fn hosts_support(&self) -> bool {
        match self.primary {
            TileKind::Note => true,
            TileKind::Chat => self.support != TileKind::Chat,
            _ => false,
        }
    }
    /// The supporting pane is wanted now (before the width policy).
    fn support_wanted(&self) -> bool {
        self.hosts_support() && self.support_open
    }
    /// The focused content kind: the sidebar reads as Recent chats.
    pub(super) fn focus(&self) -> Option<TileKind> {
        Some(match self.focus {
            Pane::Sidebar => TileKind::Threads,
            Pane::Assistant if self.support_wanted() => self.support,
            _ => self.primary,
        })
    }
    pub(super) fn settings_open(&self) -> bool {
        self.primary == TileKind::Settings
    }
    /// Kinds shown under the stored preferences: the primary view and an
    /// open supporting pane. The width policy may still hide the latter.
    pub(super) fn visible(&self) -> Vec<TileKind> {
        let mut kinds = vec![self.primary];
        if self.support_wanted() {
            kinds.push(self.support);
        }
        kinds
    }
    pub(super) fn contains(&self, kind: TileKind) -> bool {
        if kind == TileKind::Threads {
            self.sidebar_expanded
        } else {
            self.visible().contains(&kind)
        }
    }

    /// Ordinary navigation. A supporting kind joins an open document (or
    /// conversation) beside it; anything else becomes the primary view.
    /// A document opened from a conversation keeps that conversation as
    /// its assistant; one opened from the Library starts with it closed.
    /// Otherwise the owner's pane choice is kept, so re-opening the same
    /// document after Apply, Undo or a refresh changes nothing.
    pub(super) fn open(&mut self, kind: TileKind) {
        match kind {
            TileKind::Threads => {
                self.sidebar_expanded = true;
                self.sidebar_deliberate = true;
                self.focus = Pane::Sidebar;
                return;
            }
            TileKind::Settings => {
                if self.primary != TileKind::Settings {
                    self.before_settings = self.primary;
                }
                self.primary = TileKind::Settings;
            }
            kind if kind.supporting() && self.primary == TileKind::Note => {
                self.support = kind;
                self.support_open = true;
                self.sheet_open = true;
                self.focus = Pane::Assistant;
                return;
            }
            kind if kind.supporting()
                && kind != TileKind::Chat
                && self.primary == TileKind::Chat =>
            {
                self.support = kind;
                self.support_open = true;
                self.sheet_open = true;
                self.focus = Pane::Assistant;
                return;
            }
            TileKind::Note => {
                match self.primary {
                    TileKind::Chat => {
                        self.support = TileKind::Chat;
                        self.support_open = true;
                    }
                    TileKind::Notes => self.support_open = false,
                    _ => {}
                }
                if self.primary != TileKind::Note {
                    self.sheet_open = false;
                }
                self.primary = TileKind::Note;
            }
            kind => self.primary = kind,
        }
        self.focus = Pane::Primary;
    }
    /// Make `kind` the primary view even where it could sit beside the
    /// document: New chat and a chat chosen in the sidebar fill the workspace.
    pub(super) fn open_primary(&mut self, kind: TileKind) {
        if kind == TileKind::Chat && self.primary == TileKind::Note {
            self.primary = TileKind::Chat;
            if self.support == TileKind::Chat {
                self.support = TileKind::Linked;
                self.support_open = false;
            }
            self.focus = Pane::Primary;
        } else {
            self.open(kind);
        }
    }
    /// Test arrangement: a document with the assistant showing beside it
    /// (or as the open sheet when the window is compact), focus in the
    /// assistant. Replaces the retired Write preset in behavior tests.
    #[cfg(all(test, feature = "native-test-support"))]
    pub(super) fn document_with_assistant(&mut self) {
        self.primary = TileKind::Note;
        self.support = TileKind::Chat;
        self.support_open = true;
        self.sheet_open = true;
        self.focus = Pane::Assistant;
    }
    /// Test arrangement: a narrow (300 pt) assistant beside a document,
    /// where the chat body folds its controls like the old companion.
    #[cfg(all(test, feature = "native-test-support"))]
    pub(super) fn narrow_assistant(&mut self) {
        self.document_with_assistant();
        self.support_width = 300.;
    }
    /// Test arrangement: `kind` as the only view (the retired Solo).
    #[cfg(all(test, feature = "native-test-support"))]
    pub(super) fn show_alone(&mut self, kind: TileKind) {
        if kind == TileKind::Settings {
            self.open(kind);
            return;
        }
        self.primary = kind;
        self.support_open = false;
        self.sheet_open = false;
        self.focus = Pane::Primary;
    }
    /// A Library article opens for reading with the assistant closed.
    pub(super) fn open_article(&mut self) {
        self.primary = TileKind::Note;
        self.support_open = false;
        self.sheet_open = false;
        self.focus = Pane::Primary;
    }
    pub(super) fn close(&mut self, kind: TileKind) {
        if kind == TileKind::Threads {
            self.sidebar_expanded = false;
            self.sidebar_deliberate = false;
            if self.focus == Pane::Sidebar {
                self.focus = Pane::Primary;
            }
        } else if kind == TileKind::Settings {
            if self.primary == TileKind::Settings {
                self.primary = self.before_settings;
            }
        } else if self.support_wanted() && self.support == kind {
            self.set_support_open(false);
        } else if self.primary == kind {
            self.primary = if kind == TileKind::Chat {
                TileKind::Notes
            } else {
                TileKind::Chat
            };
            self.focus = Pane::Primary;
        }
    }
    /// An explicit owner choice to show or hide the supporting pane.
    pub(super) fn set_support_open(&mut self, open: bool) {
        self.support_open = open;
        self.sheet_open = open;
        if open {
            self.focus = Pane::Assistant;
        } else if self.focus == Pane::Assistant {
            self.focus = Pane::Primary;
        }
    }
    /// Show or hide the supporting pane at `width`: in the compact
    /// arrangement this only toggles the temporary sheet, so the stored
    /// preference survives for wider windows.
    pub(super) fn toggle_support(&mut self, width: f32) {
        match self.arrange(width).support {
            Support::Sheet { open, .. } => {
                self.sheet_open = !open;
                self.focus = if open { Pane::Primary } else { Pane::Assistant };
            }
            Support::Docked(_) => self.set_support_open(false),
            Support::Hidden => self.set_support_open(true),
        }
    }
    /// Close a showing compact sheet (Escape). Returns whether one closed.
    pub(super) fn close_sheet(&mut self, width: f32) -> bool {
        if matches!(
            self.arrange(width).support,
            Support::Sheet { open: true, .. }
        ) {
            self.sheet_open = false;
            if self.focus == Pane::Assistant {
                self.focus = Pane::Primary;
            }
            true
        } else {
            false
        }
    }
    /// Called with the drawn arrangement: while the pane is docked the
    /// compact sheet has nothing to show, so a later narrowing starts with
    /// the document dominant and the sheet closed.
    pub(super) fn settle(&mut self, arranged: &Arrangement) {
        if arranged.support_docked() {
            self.sheet_open = false;
        }
    }
    pub(super) fn set_support_width(&mut self, width: f32) {
        if width.is_finite() {
            self.support_width = width.clamp(metrics::ASSISTANT_MIN, metrics::ASSISTANT_MAX);
        }
    }
    pub(super) fn set_sidebar_width(&mut self, width: f32) {
        if width.is_finite() {
            self.sidebar_width = width.clamp(metrics::SIDEBAR_MIN, metrics::SIDEBAR_MAX);
        }
    }
    /// ⌘\ acts on what is drawn: a rail (chosen or responsive) expands on
    /// purpose; an expanded sidebar collapses.
    pub(super) fn toggle_sidebar(&mut self, width: f32) {
        if self.arrange(width).sidebar_expanded {
            self.close(TileKind::Threads);
        } else {
            self.sidebar_expanded = true;
            self.sidebar_deliberate = true;
        }
    }
    pub(super) fn set_focus(&mut self, kind: TileKind) {
        if kind == TileKind::Threads {
            self.focus = Pane::Sidebar;
        } else if kind == self.primary {
            self.focus = Pane::Primary;
        } else if self.support_wanted() && kind == self.support {
            self.focus = Pane::Assistant;
        }
    }
    pub(super) fn focus_pane(&mut self, pane: Pane) {
        self.focus = pane;
    }
    /// The regions ⌘[ and ⌘] move between at `width`, in visual order.
    pub(super) fn panes(&self, width: f32) -> Vec<Pane> {
        let mut panes = vec![Pane::Sidebar, Pane::Primary];
        if self.arrange(width).support_visible() {
            panes.push(Pane::Assistant);
        }
        panes
    }
    pub(super) fn cycle_focus(&mut self, delta: isize, width: f32) {
        let panes = self.panes(width);
        let index = panes
            .iter()
            .position(|pane| *pane == self.focus)
            .unwrap_or(1) as isize;
        self.focus = panes[(index + delta).rem_euclid(panes.len() as isize) as usize];
    }

    /// The pane policy at a usable `width` (spec §8). A document beside the
    /// assistant needs its 560 pt region; when that does not fit, the
    /// sidebar collapses to its rail unless the owner expanded it on
    /// purpose, and then the assistant becomes a temporary sheet rather than
    /// squeezing the document. Preferences are read, never written.
    pub(super) fn arrange(&self, width: f32) -> Arrangement {
        let width = width.max(0.);
        let expanded = self.sidebar_width + metrics::DIVIDER;
        let rail = metrics::SIDEBAR_RAIL + metrics::DIVIDER;
        let may_collapse = self.sidebar_expanded && !self.sidebar_deliberate;
        let sidebars: Vec<(f32, bool)> = match (self.sidebar_expanded, may_collapse) {
            (true, true) => vec![(expanded, true), (rail, false)],
            (true, false) => vec![(expanded, true)],
            (false, _) => vec![(rail, false)],
        };
        let minimum = if self.primary == TileKind::Note {
            metrics::DOCUMENT_REGION_MIN
        } else {
            0.
        };
        let done = |(sidebar, sidebar_expanded): (f32, bool), support: Support| {
            let beside = match support {
                Support::Docked(width) => width + metrics::DIVIDER,
                _ => 0.,
            };
            Arrangement {
                sidebar: sidebar - metrics::DIVIDER,
                sidebar_expanded,
                support,
                primary: (width - sidebar - beside).max(0.),
            }
        };
        if !self.support_wanted() {
            // A document alone still keeps its region before the sidebar does.
            let chosen = sidebars
                .iter()
                .copied()
                .find(|(sidebar, _)| width - sidebar >= minimum)
                .unwrap_or(*sidebars.last().unwrap());
            return done(chosen, Support::Hidden);
        }
        let need =
            minimum.max(metrics::DOCUMENT_REGION_MIN) + metrics::DIVIDER + self.support_width;
        if let Some(chosen) = sidebars
            .iter()
            .copied()
            .find(|(sidebar, _)| width - sidebar >= need)
        {
            return done(chosen, Support::Docked(self.support_width));
        }
        let chosen = *sidebars.last().unwrap();
        let region = (width - chosen.0).max(0.);
        // The sheet leaves a strip of the document visible as the way back.
        let sheet = self
            .support_width
            .min((region - 48.).max(metrics::ASSISTANT_MIN.min(region)));
        done(
            chosen,
            Support::Sheet {
                open: self.sheet_open,
                width: sheet,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDE: f32 = 1280.;
    const SMALL: f32 = 900.;

    fn note_with_assistant() -> Layout {
        let mut layout = Layout::default();
        layout.open(TileKind::Chat);
        layout.open(TileKind::Note);
        layout
    }

    #[test]
    fn navigation_replaces_the_primary_view_and_never_accumulates() {
        let mut layout = Layout::default();
        for _ in 0..3 {
            for kind in TileKind::ALL {
                layout.open(kind);
                assert!(layout.visible().len() <= 2, "{kind:?} accumulated");
            }
        }
        layout.open(TileKind::Notes);
        assert_eq!(layout.visible(), [TileKind::Notes]);
        assert_eq!(layout.focus(), Some(TileKind::Notes));
        layout.open(TileKind::Overview);
        assert_eq!(layout.primary(), TileKind::Overview);
        assert!(!layout.contains(TileKind::Notes));
    }

    #[test]
    fn documents_from_chat_keep_the_assistant_and_library_articles_start_closed() {
        let layout = note_with_assistant();
        assert_eq!(layout.visible(), [TileKind::Note, TileKind::Chat]);
        assert_eq!(layout.focus(), Some(TileKind::Note));

        let mut library = note_with_assistant();
        library.open(TileKind::Notes);
        library.open(TileKind::Note);
        assert_eq!(library.visible(), [TileKind::Note], "Library opens closed");

        let mut search = note_with_assistant();
        search.open_article();
        assert_eq!(search.visible(), [TileKind::Note]);
        // A later explicit choice is kept by re-opening the same document.
        search.set_support_open(true);
        search.open(TileKind::Note);
        assert!(search.contains(TileKind::Chat));
        search.set_support_open(false);
        search.open(TileKind::Note);
        assert!(!search.contains(TileKind::Chat));
    }

    #[test]
    fn supporting_kinds_join_the_document_and_closing_them_keeps_it() {
        let mut layout = note_with_assistant();
        layout.open(TileKind::Compare);
        assert_eq!(layout.visible(), [TileKind::Note, TileKind::Compare]);
        assert_eq!(layout.focus(), Some(TileKind::Compare));
        layout.close(TileKind::Compare);
        assert_eq!(layout.visible(), [TileKind::Note]);
        assert_eq!(layout.focus(), Some(TileKind::Note));
        // Without a document, a supporting kind is a primary view.
        layout.open(TileKind::Overview);
        layout.open(TileKind::Connections);
        assert_eq!(layout.visible(), [TileKind::Connections]);
        // Linked work sits beside a conversation, a chat never beside a chat.
        layout.open(TileKind::Chat);
        layout.open(TileKind::Linked);
        assert_eq!(layout.visible(), [TileKind::Chat, TileKind::Linked]);
        layout.open_primary(TileKind::Chat);
        assert!(layout.visible().starts_with(&[TileKind::Chat]));
    }

    #[test]
    fn new_chat_from_a_document_fills_the_workspace() {
        let mut layout = note_with_assistant();
        layout.open_primary(TileKind::Chat);
        assert_eq!(layout.visible(), [TileKind::Chat]);
        assert_eq!(layout.focus(), Some(TileKind::Chat));
    }

    #[test]
    fn settings_is_a_primary_view_that_returns_to_the_previous_one() {
        let mut layout = note_with_assistant();
        layout.open(TileKind::Settings);
        assert!(layout.settings_open());
        assert_eq!(layout.focus(), Some(TileKind::Settings));
        layout.open(TileKind::Settings);
        layout.close(TileKind::Settings);
        assert_eq!(layout.visible(), [TileKind::Note, TileKind::Chat]);
    }

    #[test]
    fn the_sidebar_toggles_between_expanded_and_rail() {
        let mut layout = Layout::default();
        assert!(layout.arrange(WIDE).sidebar_expanded);
        assert_eq!(layout.arrange(WIDE).sidebar, metrics::SIDEBAR);
        layout.toggle_sidebar(WIDE);
        assert!(!layout.sidebar_expanded());
        assert_eq!(layout.arrange(WIDE).sidebar, metrics::SIDEBAR_RAIL);
        layout.toggle_sidebar(WIDE);
        assert!(layout.arrange(WIDE).sidebar_expanded);
        layout.set_sidebar_width(500.);
        assert_eq!(layout.sidebar_width(), metrics::SIDEBAR_MAX);
        layout.set_sidebar_width(f32::NAN);
        assert_eq!(layout.sidebar_width(), metrics::SIDEBAR_MAX);
    }

    #[test]
    fn at_1280_the_document_assistant_and_sidebar_all_fit() {
        let arranged = note_with_assistant().arrange(WIDE);
        assert!(arranged.sidebar_expanded);
        assert_eq!(arranged.support, Support::Docked(metrics::ASSISTANT));
        assert!(arranged.primary >= metrics::DOCUMENT_REGION_MIN);
        let used = arranged.sidebar + arranged.primary + metrics::ASSISTANT + 2.;
        assert!((used - WIDE).abs() < 0.01, "{used}");
    }

    #[test]
    fn narrowing_collapses_the_sidebar_first_then_uses_a_closed_sheet() {
        let layout = note_with_assistant();
        // 224 + 560 + 320 + dividers = 1106: one point less collapses the sidebar.
        let collapsed = layout.arrange(1105.);
        assert!(!collapsed.sidebar_expanded);
        assert!(collapsed.support_docked());
        assert!(collapsed.primary >= metrics::DOCUMENT_REGION_MIN);
        // 900: no docked assistant fits, so the document stays dominant and
        // the assistant is a sheet that starts closed.
        let small = layout.arrange(SMALL);
        assert!(!small.sidebar_expanded);
        assert!(matches!(small.support, Support::Sheet { open: false, .. }));
        assert_eq!(small.primary, SMALL - metrics::SIDEBAR_RAIL - 1.);
        // The fallback did not touch the stored preferences.
        assert!(layout.sidebar_expanded() && layout.support_open());
        assert!(layout.arrange(WIDE).support_docked());
    }

    #[test]
    fn a_deliberately_expanded_sidebar_stays_and_the_assistant_yields() {
        let mut layout = note_with_assistant();
        layout.toggle_sidebar(SMALL); // rail drawn → expand on purpose
        let arranged = layout.arrange(SMALL);
        assert!(arranged.sidebar_expanded);
        assert!(matches!(arranged.support, Support::Sheet { .. }));
        assert!(arranged.primary >= metrics::DOCUMENT_REGION_MIN);
        layout.toggle_sidebar(SMALL);
        assert!(!layout.arrange(SMALL).sidebar_expanded);
    }

    #[test]
    fn the_compact_sheet_opens_and_closes_without_changing_the_preference() {
        let mut layout = note_with_assistant();
        layout.toggle_support(SMALL);
        let open = layout.arrange(SMALL);
        assert!(open.support_visible());
        assert_eq!(layout.focus(), Some(TileKind::Chat));
        let sheet = open.support_width().unwrap();
        assert!(sheet <= open.primary - 48., "a strip of the document stays");
        assert_eq!(
            open.primary,
            SMALL - metrics::SIDEBAR_RAIL - 1.,
            "not squeezed"
        );
        assert!(layout.close_sheet(SMALL));
        assert!(!layout.arrange(SMALL).support_visible());
        assert!(!layout.close_sheet(SMALL));
        assert_eq!(layout.focus(), Some(TileKind::Note));
        assert!(layout.support_open(), "preference kept for wider windows");
        // Docked, the same toggle is the owner's preference.
        layout.toggle_support(WIDE);
        assert!(!layout.support_open());
        assert!(!layout.arrange(WIDE).support_visible());
        layout.toggle_support(WIDE);
        assert!(layout.arrange(WIDE).support_docked());
    }

    #[test]
    fn narrowing_from_a_docked_assistant_starts_with_the_sheet_closed() {
        let mut layout = note_with_assistant();
        layout.open(TileKind::Chat); // an explicit request also asks for the sheet
        let wide = layout.arrange(WIDE);
        assert!(wide.support_docked());
        layout.settle(&wide);
        let small = layout.arrange(SMALL);
        assert!(matches!(small.support, Support::Sheet { open: false, .. }));
        layout.settle(&small);
        assert!(!layout.arrange(SMALL).support_visible());
        assert!(layout.arrange(WIDE).support_docked(), "preference kept");
    }

    #[test]
    fn assistant_width_stays_in_its_useful_range_and_survives_navigation() {
        let mut layout = note_with_assistant();
        layout.set_support_width(1000.);
        assert_eq!(layout.support_width(), metrics::ASSISTANT_MAX);
        layout.set_support_width(10.);
        assert_eq!(layout.support_width(), metrics::ASSISTANT_MIN);
        layout.set_support_width(350.);
        layout.open(TileKind::Settings);
        layout.close(TileKind::Settings);
        layout.open(TileKind::Note);
        assert_eq!(layout.support_width(), 350.);
        assert_eq!(layout.arrange(1400.).support, Support::Docked(350.));
    }

    #[test]
    fn a_document_alone_keeps_its_region_before_the_sidebar() {
        let mut layout = Layout::default();
        layout.open_article();
        let arranged = layout.arrange(700.);
        assert!(!arranged.sidebar_expanded);
        assert!(arranged.primary >= metrics::DOCUMENT_REGION_MIN);
        // Other views keep the sidebar.
        layout.open(TileKind::Notes);
        assert!(layout.arrange(700.).sidebar_expanded);
    }

    #[test]
    fn focus_moves_between_visible_regions_in_order() {
        let mut layout = note_with_assistant();
        assert_eq!(
            layout.panes(WIDE),
            [Pane::Sidebar, Pane::Primary, Pane::Assistant]
        );
        layout.cycle_focus(1, WIDE);
        assert_eq!(layout.pane(), Pane::Assistant);
        layout.cycle_focus(1, WIDE);
        assert_eq!(layout.pane(), Pane::Sidebar);
        assert_eq!(layout.focus(), Some(TileKind::Threads));
        layout.cycle_focus(-1, WIDE);
        assert_eq!(layout.pane(), Pane::Assistant);
        // A closed compact sheet is not a stop.
        layout.focus_pane(Pane::Primary);
        assert_eq!(layout.panes(SMALL), [Pane::Sidebar, Pane::Primary]);
        layout.cycle_focus(1, SMALL);
        assert_eq!(layout.pane(), Pane::Sidebar);
    }

    #[test]
    fn margin_mode_picks_the_widest_comments_placement_that_fits() {
        assert_eq!(margin_mode(NOTE_MIN + MarginMode::WIDE), MarginMode::Wide);
        assert_eq!(
            margin_mode(NOTE_MIN + MarginMode::WIDE - 1.),
            MarginMode::Narrow
        );
        assert_eq!(
            margin_mode(NOTE_MIN + MarginMode::NARROW),
            MarginMode::Narrow
        );
        assert_eq!(
            margin_mode(NOTE_MIN + MarginMode::NARROW - 1.),
            MarginMode::Stacked
        );
        // A docked document at 1280 has room for the wide margin.
        let arranged = note_with_assistant().arrange(WIDE);
        assert_eq!(margin_mode(arranged.primary), MarginMode::Wide);
    }
}
