//! Navigation panels: Chats and Notes, plus shared page helpers.
//! Every control calls the existing checked services.
use super::components::AccessibleDropdownMenu;
use super::shell::item;
use super::*;
use gpui_kit::WeakEntity;
use gpui_kit::component::menu::PopupMenu;

pub(super) use brn_writing_ui::native::scroll;

/// Pages keep their content in one column of at most 720 px.
pub(super) const PAGE_WIDTH: f32 = 720.;

/// A page's content column, sections stacked without boxes.
pub(super) fn column() -> gpui_kit::Div {
    div().flex().flex_col().w_full().min_w_0()
}

/// A page: its one vertical scroller around a column at most `width` wide.
pub(super) fn page(
    id: impl Into<gpui_kit::ElementId>,
    width: f32,
    column: gpui_kit::Div,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    scroll(id)
        .gap_0()
        .px_4()
        .pt_0()
        .pb_6()
        .child(column.max_w(px(width)))
}

pub(super) fn thread_menu(
    menu: PopupMenu,
    this: &WeakEntity<ThreadsUi>,
    id: String,
    resolved: bool,
    note: Option<String>,
) -> PopupMenu {
    let open = id.clone();
    let lifecycle = id.clone();
    let mut menu = menu
        .item(item(
            this,
            "Open",
            None,
            false,
            false,
            move |view, window, cx| view.open_thread(&open, window, cx),
        ))
        .item(item(
            this,
            if resolved { "Unarchive" } else { "Archive" },
            None,
            false,
            false,
            move |view, _, cx| {
                if resolved {
                    view.reopen_thread(&lifecycle, cx)
                } else {
                    view.resolve_thread(&lifecycle, cx)
                }
            },
        ));
    if let Some(note) = note {
        menu = menu.item(item(
            this,
            "Open note",
            None,
            false,
            false,
            move |view, window, cx| view.open_note(&note, None, window, cx),
        ));
    }
    menu = menu.separator();
    let chat = id.clone();
    menu = menu.item(item(
        this,
        "Move to project…",
        None,
        false,
        false,
        move |view, window, cx| view.move_chat_dialog(&chat, window, cx),
    ));
    menu
}

pub(super) fn new_menu(menu: PopupMenu, this: &WeakEntity<ThreadsUi>, busy: bool) -> PopupMenu {
    menu.item(item(
        this,
        "New chat",
        Some(Box::new(NewThread)),
        false,
        false,
        |view, window, cx| view.new_thread(window, cx),
    ))
    .item(item(
        this,
        "New note",
        Some(Box::new(NewNote)),
        false,
        false,
        |view, window, cx| view.new_note(window, cx),
    ))
    .separator()
    .item(item(
        this,
        "Add useful information…",
        None,
        busy,
        false,
        |view, window, cx| view.choose_import(ImportIntent::UsefulInformation, None, window, cx),
    ))
    .item(item(
        this,
        "Import full note…",
        None,
        busy,
        false,
        |view, window, cx| view.choose_import(ImportIntent::FullNote, None, window, cx),
    ))
}

impl ThreadsUi {
    pub(super) fn tile_controls(
        &self,
        kind: TileKind,
        cx: &mut Context<Self>,
    ) -> Vec<gpui_kit::AnyElement> {
        let this = cx.entity().downgrade();
        let busy = self.worker.is_some();
        match kind {
            TileKind::Threads | TileKind::Notes => vec![
                components::icon_button(
                    format!("{}-new", kind.key()),
                    IconName::Plus,
                    "New chat, note or information",
                )
                .accessible_dropdown_menu(move |menu, _, _| new_menu(menu, &this, busy))
                .into_any_element(),
            ],
            TileKind::Chat => self.chat_controls(cx),
            TileKind::Note => self.note_controls(cx),
            _ => vec![],
        }
    }
    /// What a needs-you row or menu item says: the chat's first recorded
    /// reason, or for a prepared review the notes it changes, and whether a
    /// protected note is among them. The service's reason is unchanged.
    pub(super) fn needs_reason(&self, thread: &str) -> Option<(String, bool)> {
        let RecordData::Thread(data) = &self.records.iter().find(|r| r.id == thread)?.data else {
            return None;
        };
        let attention = data.attention.first()?;
        Some(
            match self
                .candidates
                .iter()
                .find(|c| attention.record.as_deref() == Some(c.operation.as_str()))
            {
                Some(candidate) => {
                    let summary = review_summary(candidate, &self.records);
                    (
                        review_label(&summary, &candidate.request.reason),
                        summary.protected,
                    )
                }
                None => (attention.reason.clone(), false),
            },
        )
    }
}
