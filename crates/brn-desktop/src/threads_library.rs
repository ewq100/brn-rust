//! The Library (spec §6.4, wiki spec R7–R8) and the chat lists.
//!
//! Articles (the default) lists current general notes other than full
//! imported references; All files adds those references and retained
//! sources that have no readable import. Topic indexes stay ordinary Notes.
//! Rows open the same Note identity through `open_note`, which the shell
//! opens for reading with the assistant closed. Long libraries render
//! through a virtualized list; searching uses the existing owner search.
//! Knowledge checks and Recent changes are reached from the Library ⋯ menu.
use super::components::AccessibleDropdownMenu;
use super::shell::{item, pill};
use super::tiles::{new_menu, scroll, thread_menu};
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu};
use gpui_kit::component::{Icon, Sizable, Size};
use gpui_kit::{ListAlignment, ListState, SharedString, WeakEntity};
use std::rc::Rc;

/// The Library's local view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum LibraryView {
    /// Consolidated current articles and topics.
    #[default]
    Articles,
    /// Every supported current note, full imported references and retained
    /// sources.
    Files,
    /// The owner-triggered knowledge check, from the Library ⋯ menu.
    Checks,
}

/// How the list is ordered or narrowed (the earlier Recent, Linked,
/// Protected/imported and A–Z sections, one at a time).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum LibraryOrder {
    #[default]
    Recent,
    Alphabetical,
    MostLinked,
    LinkedToOpenChats,
    ProtectedOrImported,
}
impl LibraryOrder {
    pub(super) const ALL: [Self; 5] = [
        Self::Recent,
        Self::Alphabetical,
        Self::MostLinked,
        Self::LinkedToOpenChats,
        Self::ProtectedOrImported,
    ];
    /// The menu entry.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recently changed",
            Self::Alphabetical => "A–Z",
            Self::MostLinked => "Most linked",
            Self::LinkedToOpenChats => "Linked to open chats",
            Self::ProtectedOrImported => "Protected and imported",
        }
    }
    /// The trigger's short text.
    fn short(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Alphabetical => "A–Z",
            Self::MostLinked => "Most linked",
            Self::LinkedToOpenChats => "Linked to chats",
            Self::ProtectedOrImported => "Protected",
        }
    }
}

/// One item of the virtualized list.
#[derive(Clone, Debug, PartialEq)]
enum Entry {
    /// The Topics block: the Topics note and its linked topic pages.
    Topics,
    /// The list's heading: what is shown and how many.
    Heading,
    /// A row of `note_rows`.
    Note(usize),
    /// A row of `LibraryState::sources`.
    Source(usize),
    Empty,
    Footer,
}

/// A retained Source with no current full imported note.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SourceRow {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) locator: String,
    /// When BRN recorded the observation; not the source's own date.
    pub(super) observed: String,
    /// The source's own version or date, when it was recorded.
    pub(super) external_version: Option<String>,
}

pub(super) struct LibraryState {
    pub(super) view: LibraryView,
    pub(super) order: LibraryOrder,
    /// A topic page: only the articles it links are listed.
    pub(super) topic: Option<String>,
    list: ListState,
    entries: Rc<Vec<Entry>>,
    sources: Vec<SourceRow>,
}
impl Default for LibraryState {
    fn default() -> Self {
        Self {
            view: LibraryView::default(),
            order: LibraryOrder::default(),
            topic: None,
            list: ListState::new(0, ListAlignment::Top, px(600.)),
            entries: Rc::new(vec![]),
            sources: vec![],
        }
    }
}

/// Current general sources that a reader can only reach as a source: those
/// without a current full imported note.
pub(super) fn source_rows(records: &[Record], notes: &[NoteRow]) -> Vec<SourceRow> {
    let imported: BTreeSet<&str> = notes
        .iter()
        .filter(|row| row.is_file())
        .filter_map(|row| row.source.as_deref())
        .collect();
    let mut rows: Vec<SourceRow> = records
        .iter()
        .filter(|record| !record.archived && !imported.contains(record.id.as_str()))
        .filter(|record| record_in_chat(record, records, None))
        .filter_map(|record| match &record.data {
            RecordData::Source(source) if source.chat.is_none() => Some(SourceRow {
                id: record.id.clone(),
                label: if source.label.trim().is_empty() {
                    source.locator.clone()
                } else {
                    source.label.clone()
                },
                locator: source.locator.clone(),
                observed: source.observed_at.clone(),
                external_version: source.external_version.clone(),
            }),
            _ => None,
        })
        .collect();
    rows.sort_by(|a, b| {
        a.label
            .to_lowercase()
            .cmp(&b.label.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    rows
}

/// The notes each topic page links to, by topic page.
pub(super) fn topic_members(records: &[Record], topics: &Topics) -> BTreeMap<String, Vec<String>> {
    let pages: BTreeSet<&str> = topics.entries.iter().map(|e| e.note.as_str()).collect();
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for record in records.iter().filter(|r| !r.archived) {
        if let RecordData::Link(link) = &record.data
            && pages.contains(link.from.as_str())
            && link.from != link.to
        {
            out.entry(link.from.clone())
                .or_default()
                .push(link.to.clone());
        }
    }
    out
}

/// The rows shown for one view, order and topic, as indices into `rows`.
pub(super) fn library_rows(
    rows: &[NoteRow],
    view: LibraryView,
    order: LibraryOrder,
    topic: Option<&BTreeSet<String>>,
    linked_to_open: &BTreeSet<String>,
) -> Vec<usize> {
    let mut shown: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| view == LibraryView::Files || !row.is_file())
        .filter(|(_, row)| topic.is_none_or(|members| members.contains(&row.id)))
        .filter(|(_, row)| match order {
            LibraryOrder::LinkedToOpenChats => linked_to_open.contains(&row.id),
            LibraryOrder::ProtectedOrImported => row.protected || row.imported,
            LibraryOrder::MostLinked => row.inbound > 0,
            _ => true,
        })
        .map(|(index, _)| index)
        .collect();
    match order {
        // `rows` is already A–Z.
        LibraryOrder::Alphabetical => {}
        LibraryOrder::MostLinked => shown.sort_by(|&a, &b| {
            rows[b]
                .inbound
                .cmp(&rows[a].inbound)
                .then_with(|| a.cmp(&b))
        }),
        _ => shown.sort_by_key(|&index| std::cmp::Reverse(rows[index].order)),
    }
    shown
}

/// "today" or "14 Oct" from a commit time; the tooltip keeps the full time.
pub(super) fn day_label(stamp: &str) -> String {
    let when = lists::human_time(stamp);
    if when.starts_with("Today") {
        "today".into()
    } else {
        when.split(',').next().unwrap_or(&when).to_owned()
    }
}

/// The restrained facts under an article or file row: its kind for files,
/// the last change (and when delegated maintenance made it) and inbound
/// links. Saved never claims personal review.
pub(super) fn row_facts(row: &NoteRow, stamp: &str, delegated: bool) -> Vec<String> {
    let mut facts = vec![];
    if row.is_file() {
        facts.push("Full imported reference".to_owned());
    }
    if !stamp.is_empty() {
        let when = day_label(stamp);
        facts.push(if delegated {
            format!("Updated {when} by delegated maintenance")
        } else {
            format!("Saved {when}")
        });
    }
    if row.inbound > 0 {
        facts.push(format!("Linked from {}", row.inbound));
    }
    facts
}

/// Open a retained source: its absolute file or web address when BRN can
/// reach it; otherwise say what was retained.
pub(super) fn open_source_locator(locator: &str, cx: &mut App) -> Result<(), String> {
    let path = std::path::Path::new(locator);
    if path.is_absolute() {
        if path.exists() {
            cx.open_with_system(path);
            return Ok(());
        }
        return Err(format!(
            "The original is no longer at {}. BRN keeps only what it recorded; anything imported from it stays readable.",
            short(locator)
        ));
    }
    if locator.starts_with("https://")
        || locator.starts_with("http://")
        || locator.starts_with("outlook:")
    {
        cx.open_url(locator);
        return Ok(());
    }
    Err(format!(
        "BRN recorded this source as “{}” but did not retain an original it can open.",
        short(locator)
    ))
}

fn row_title(text: impl Into<SharedString>) -> gpui_kit::Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(px(theme::metrics::TEXT_BODY))
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .text_color(rgb(pal().ink))
        .child(text.into())
}

fn row_description(text: impl Into<SharedString>) -> gpui_kit::Div {
    div()
        .min_w_0()
        .line_clamp(2)
        .text_size(px(theme::metrics::TEXT_LABEL))
        .text_color(rgb(pal().muted))
        .child(text.into())
}

fn row_meta(text: impl Into<SharedString>) -> gpui_kit::Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(px(theme::metrics::TEXT_METADATA))
        .text_color(rgb(pal().muted))
        .child(text.into())
}

/// A Library list row: kind icon, title, description, facts, chevron.
/// Keyboard-reachable with a visible focus border; the full text is its
/// accessible name and, unless replaced, the tooltip.
#[allow(clippy::too_many_arguments)]
pub(super) fn library_row(
    id: impl Into<SharedString>,
    icon: IconName,
    title: String,
    description: String,
    facts: String,
    marks: Vec<gpui_kit::AnyElement>,
    selected: bool,
    tooltip: Option<String>,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let p = pal();
    let id: SharedString = id.into();
    let announced = [title.as_str(), description.as_str(), facts.as_str()]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let tooltip: SharedString = tooltip.unwrap_or_else(|| announced.clone()).into();
    div()
        .id(id.clone())
        .flex()
        .flex_shrink_0()
        .items_start()
        .gap_3()
        .w_full()
        .px_3()
        .py(tokens::SPACE_3)
        .rounded(px(theme::metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(gpui_kit::rgba(0))
        .cursor_pointer()
        .font_family(UI_FONT)
        .when(selected, |row| row.bg(rgb(p.active)))
        .when(!selected, |row| {
            row.hover(|style| style.bg(rgb(pal().hover)))
        })
        .focus_visible(|style| style.border_color(rgb(pal().focus)))
        .role(gpui_kit::Role::Button)
        .tab_index(0)
        .aria_label(announced)
        .aria_selected(selected)
        .tooltip(move |window, cx| {
            gpui_kit::component::tooltip::Tooltip::new(tooltip.clone()).build(window, cx)
        })
        .child(
            Icon::new(icon)
                .with_size(Size::Small)
                .flex_shrink_0()
                .mt(px(2.))
                .text_color(rgb(p.muted)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(2.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .min_w_0()
                        .child(
                            row_title(title)
                                .id(SharedString::from(format!("{id}-title")))
                                .flex_1()
                                .test_support(),
                        )
                        .children(marks),
                )
                .when(!description.is_empty(), |column| {
                    column.child(row_description(description))
                })
                .when(!facts.is_empty(), |column| {
                    column.child(
                        row_meta(facts)
                            .id(SharedString::from(format!("{id}-facts")))
                            .test_support(),
                    )
                }),
        )
        .child(
            Icon::new(IconName::ChevronRight)
                .with_size(Size::Small)
                .flex_shrink_0()
                .mt(px(2.))
                .text_color(rgb(p.muted)),
        )
}

/// A local view tab: an underlined label, announced as a selected tab.
fn view_tab(id: &'static str, label: String, selected: bool) -> gpui_kit::Stateful<gpui_kit::Div> {
    let p = pal();
    div()
        .id(id)
        .flex()
        .items_center()
        .h(px(theme::metrics::CONTROL_HEIGHT + 4.))
        .px_1()
        .border_b_2()
        .border_color(if selected {
            rgb(p.ink)
        } else {
            gpui_kit::rgba(0)
        })
        .cursor_pointer()
        .font_family(UI_FONT)
        .text_size(px(theme::metrics::TEXT_LABEL))
        .text_color(rgb(if selected { p.ink } else { p.muted }))
        .when(selected, |tab| {
            tab.font_weight(gpui_kit::FontWeight::MEDIUM)
        })
        .hover(|style| style.text_color(rgb(pal().ink)))
        .focus_visible(|style| style.bg(rgb(pal().hover)))
        .role(gpui_kit::Role::Tab)
        .tab_index(0)
        .aria_label(label.clone())
        .aria_selected(selected)
        .child(label)
}

/// The Library's ⋯ menu: Knowledge checks, Recent changes and Topics.
fn library_menu(
    menu: PopupMenu,
    this: &WeakEntity<ThreadsUi>,
    topics: Option<String>,
) -> PopupMenu {
    let menu = menu
        .item(item(
            this,
            "Knowledge checks",
            None,
            false,
            false,
            |view, window, cx| view.show_library(LibraryView::Checks, window, cx),
        ))
        .item(item(
            this,
            "Recent changes",
            None,
            false,
            false,
            |view, window, cx| view.show_tile(TileKind::History, window, cx),
        ))
        .separator();
    match topics {
        Some(id) => menu.item(item(
            this,
            "Open Topics note",
            None,
            false,
            false,
            move |view, window, cx| view.open_note(&id, None, window, cx),
        )),
        None => menu.item(item(
            this,
            "Create Topics note",
            None,
            false,
            false,
            |view, window, cx| view.create_topics(window, cx),
        )),
    }
}

/// A note row's menu: Open, Related and sources, New chat about this note.
fn note_menu(menu: PopupMenu, this: &WeakEntity<ThreadsUi>, id: String) -> PopupMenu {
    let open = id.clone();
    let related = id.clone();
    menu.item(item(
        this,
        "Open",
        None,
        false,
        false,
        move |view, window, cx| view.open_note(&open, None, window, cx),
    ))
    .item(item(
        this,
        "Related and sources",
        None,
        false,
        false,
        move |view, window, cx| {
            view.open_note(&related, None, window, cx);
            if view.open_note_id() == Some(related.as_str()) {
                view.open_related(true, window, cx);
            }
        },
    ))
    .item(item(
        this,
        "New chat about this note",
        None,
        false,
        false,
        move |view, window, cx| {
            view.open_note(&id, None, window, cx);
            if view.open_note_id() == Some(id.as_str()) {
                view.new_writing_thread(window, cx);
            }
        },
    ))
}

fn source_menu(menu: PopupMenu, this: &WeakEntity<ThreadsUi>, source: SourceRow) -> PopupMenu {
    let locator = source.locator.clone();
    let copy = source.locator;
    menu.item(item(
        this,
        "Open original",
        None,
        false,
        false,
        move |view, _, cx| {
            if let Err(limit) = open_source_locator(&locator, cx) {
                view.warn(limit);
                cx.notify();
            }
        },
    ))
    .item(item(
        this,
        "Copy reference",
        None,
        false,
        false,
        move |_, _, cx| cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(copy.clone())),
    ))
}

use components::tokens;

impl ThreadsUi {
    /// Show the Library in `view`.
    pub(super) fn show_library(
        &mut self,
        view: LibraryView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.library.view = view;
        self.show_tile(TileKind::Notes, window, cx);
    }
    /// Related articles and sources beside the open document: the bounded
    /// Connections view as its supporting panel, with sources listed when
    /// `sources`. The document ⋯ menu and Library rows call this.
    pub(super) fn open_related(
        &mut self,
        sources: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.open_note_id().is_none() {
            return;
        }
        self.graph.sources = sources;
        self.show_tile(TileKind::Connections, window, cx);
    }
    /// The Library header's local actions: + (new and import) and ⋯.
    pub(super) fn library_controls(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let this = cx.entity().downgrade();
        let busy = self.worker.is_some();
        let menu_this = this.clone();
        let topics = self.topics.as_ref().map(|topics| topics.note.clone());
        vec![
            components::icon_button("notes-new", IconName::Plus, "New chat, note or information")
                .accessible_dropdown_menu(move |menu, _, _| new_menu(menu, &this, busy)),
            components::overflow_menu("library-menu", "Library actions").accessible_dropdown_menu(
                move |menu, _, _| library_menu(menu, &menu_this, topics.clone()),
            ),
        ]
    }

    /// The Library body.
    pub(super) fn notes_tile(&mut self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        if self.library.view == LibraryView::Checks {
            return self.knowledge_checks_view(cx);
        }
        let query = self.search.read(cx).value().to_string();
        let tools = self.library_tools(cx);
        let body = if query.trim().is_empty() {
            self.library_list(cx)
        } else {
            self.library_search(&query, cx)
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(tools)
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }

    fn library_counts(&self) -> (usize, usize) {
        let files = self.note_rows.iter().filter(|row| row.is_file()).count();
        (
            self.note_rows.len() - files,
            files + self.library.sources.len(),
        )
    }

    /// The heading, then the Articles / All files tabs, the Show menu and
    /// the Library search, above the list.
    fn library_tools(&mut self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        self.library.sources = source_rows(&self.records, &self.note_rows);
        let p = pal();
        let (articles, files) = self.library_counts();
        let view = self.library.view;
        let this = cx.entity().downgrade();
        let order = self.library.order;
        let topic = self.library.topic.clone();
        let topic_title = topic.as_ref().and_then(|id| {
            self.topics
                .as_ref()?
                .entries
                .iter()
                .find(|entry| &entry.note == id)
                .map(|entry| entry.title.clone())
        });
        let entries: Vec<(String, String)> = self
            .topics
            .as_ref()
            .map(|topics| {
                topics
                    .entries
                    .iter()
                    .map(|entry| (entry.note.clone(), entry.title.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let show_label = match &topic_title {
            Some(title) => format!("{} · {}", order.short(), short(title)),
            None => order.short().to_owned(),
        };
        let show = button("library-show")
            .label(show_label.clone())
            .icon(IconName::ChevronDown)
            .ghost()
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .accessibility_label(format!("Show: {show_label}"))
            .tooltip("Order the list, or show one topic's articles")
            .accessible_dropdown_menu(move |mut menu, _, _| {
                menu = menu.label("Order");
                for choice in LibraryOrder::ALL {
                    menu = menu.item(item(
                        &this,
                        choice.label(),
                        None,
                        false,
                        choice == order,
                        move |view, _, cx| {
                            view.library.order = choice;
                            cx.notify();
                        },
                    ));
                }
                if !entries.is_empty() {
                    menu = menu.separator().label("Topic").item(item(
                        &this,
                        "Every article",
                        None,
                        false,
                        topic.is_none(),
                        |view, _, cx| {
                            view.library.topic = None;
                            cx.notify();
                        },
                    ));
                    for (id, title) in &entries {
                        let chosen = id.clone();
                        menu = menu.item(item(
                            &this,
                            short(title),
                            None,
                            false,
                            topic.as_deref() == Some(id.as_str()),
                            move |view, _, cx| {
                                view.library.topic = Some(chosen.clone());
                                cx.notify();
                            },
                        ));
                    }
                }
                menu
            });
        let subtitle = format!(
            "Current knowledge · {articles} article{} · {files} file{}",
            if articles == 1 { "" } else { "s" },
            if files == 1 { "" } else { "s" },
        );
        let heading = div()
            .flex()
            .flex_col()
            .gap_1()
            .pt(tokens::SPACE_6)
            .pb(tokens::SPACE_3)
            .child(
                div()
                    .id("library-heading")
                    .text_size(px(theme::metrics::TEXT_CONVERSATION_HEADING))
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(rgb(p.ink))
                    .child("Your library")
                    .test_support(),
            )
            .child(
                div()
                    .id("library-subtitle")
                    .text_size(px(theme::metrics::TEXT_LABEL))
                    .text_color(rgb(p.muted))
                    .aria_label(subtitle.clone())
                    .child(subtitle)
                    .when(self.results.preparing, |line| {
                        line.child(components::loading("search-preparing", "Preparing search…"))
                    })
                    .test_support(),
            );
        let tabs = div()
            .id("library-views")
            .flex()
            .items_end()
            .gap(tokens::SPACE_4)
            .role(gpui_kit::Role::TabList)
            .aria_label("Library view")
            .child(
                view_tab(
                    "library-articles",
                    format!("Articles · {articles}"),
                    view == LibraryView::Articles,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.library.view = LibraryView::Articles;
                    cx.notify();
                })),
            )
            .child(
                view_tab(
                    "library-files",
                    format!("All files · {files}"),
                    view == LibraryView::Files,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.library.view = LibraryView::Files;
                    cx.notify();
                })),
            )
            .test_support();
        let tools = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(rgb(p.hairline))
            .child(tabs)
            .child(div().flex_1())
            .child(show)
            .child(
                div().w(px(240.)).max_w_full().pb_1().child(
                    Input::new(&self.search)
                        .prefix(
                            Icon::new(IconName::Search)
                                .with_size(Size::Small)
                                .text_color(rgb(p.muted)),
                        )
                        .cleanable(true)
                        .w_full(),
                ),
            );
        div()
            .id("library-tools")
            .flex_shrink_0()
            .w_full()
            .flex()
            .justify_center()
            .px(self.library_inset())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(px(LIBRARY_WIDTH))
                    .child(heading)
                    .child(tools),
            )
            .test_support()
            .into_any_element()
    }

    /// Page inset: 32, reduced in a compact window (spec §8).
    pub(super) fn library_inset(&self) -> gpui_kit::Pixels {
        if self.arrangement.primary < 760. {
            px(theme::metrics::COMPACT_PAGE_INSET)
        } else {
            px(theme::metrics::PAGE_INSET)
        }
    }

    /// The browsing list: topics, the heading, then rows, virtualized.
    fn library_list(&mut self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let view = self.library.view;
        let members = self
            .topics
            .as_ref()
            .map(|topics| topic_members(&self.records, topics))
            .unwrap_or_default();
        // A topic that no longer exists falls back to every article.
        if self.library.topic.as_ref().is_some_and(|topic| {
            !self
                .topics
                .as_ref()
                .is_some_and(|t| t.entries.iter().any(|e| &e.note == topic))
        }) {
            self.library.topic = None;
        }
        let topic: Option<BTreeSet<String>> = self.library.topic.as_ref().map(|topic| {
            members
                .get(topic)
                .map(|ids| ids.iter().cloned().collect())
                .unwrap_or_default()
        });
        let linked = lists::notes_linked_to_open(&self.records, &self.thread_rows);
        let rows = library_rows(
            &self.note_rows,
            view,
            self.library.order,
            topic.as_ref(),
            &linked,
        );
        let mut entries = vec![];
        if view == LibraryView::Articles && self.library.topic.is_none() {
            entries.push(Entry::Topics);
        }
        entries.push(Entry::Heading);
        entries.extend(rows.into_iter().map(Entry::Note));
        // Sources join All files in their own order (A–Z), after notes.
        if view == LibraryView::Files
            && matches!(
                self.library.order,
                LibraryOrder::Recent | LibraryOrder::Alphabetical
            )
            && self.library.topic.is_none()
        {
            entries.extend((0..self.library.sources.len()).map(Entry::Source));
        }
        if !entries
            .iter()
            .any(|entry| matches!(entry, Entry::Note(_) | Entry::Source(_)))
        {
            entries.push(Entry::Empty);
        }
        entries.push(Entry::Footer);
        if *self.library.entries != entries {
            self.library.list.reset(entries.len());
            self.library.entries = Rc::new(entries);
        }
        let render = cx
            .processor(|this: &mut Self, index: usize, _window, cx| this.library_entry(index, cx));
        div()
            .id("notes-list")
            .size_full()
            .child(gpui_kit::list(self.library.list.clone(), render).size_full())
            .test_support()
            .into_any_element()
    }

    /// One list item inside the centred Library column.
    fn library_entry(&mut self, index: usize, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let entry = self.library.entries.get(index).cloned();
        let inner = match entry {
            Some(Entry::Topics) => self.topics_section(cx),
            Some(Entry::Heading) => self.library_heading().into_any_element(),
            Some(Entry::Note(row)) => self.note_entry(row, cx),
            Some(Entry::Source(row)) => self.source_entry(row, cx),
            Some(Entry::Empty) => self.library_empty().into_any_element(),
            Some(Entry::Footer) => self.library_footer().into_any_element(),
            None => div().into_any_element(),
        };
        div()
            .w_full()
            .flex()
            .justify_center()
            .px(self.library_inset())
            .child(div().w_full().max_w(px(LIBRARY_WIDTH)).child(inner))
            .into_any_element()
    }

    fn library_heading(&self) -> gpui_kit::Div {
        let shown = self
            .library
            .entries
            .iter()
            .filter(|entry| matches!(entry, Entry::Note(_) | Entry::Source(_)))
            .count();
        let what = match self.library.view {
            LibraryView::Files => "All files",
            _ => "Articles",
        };
        let topic = self.library.topic.as_ref().and_then(|id| {
            self.topics
                .as_ref()?
                .entries
                .iter()
                .find(|entry| &entry.note == id)
                .map(|entry| entry.title.clone())
        });
        let label = match topic {
            Some(topic) => format!("{what} in {}", short(&topic)),
            None => what.to_owned(),
        };
        components::section_header(
            label,
            Some(shown),
            Some(
                metadata(self.library.order.label())
                    .id("library-order")
                    .test_support()
                    .into_any_element(),
            ),
        )
        .px_3()
    }

    fn library_empty(&self) -> gpui_kit::AnyElement {
        if self.note_rows.is_empty() && self.library.sources.is_empty() {
            return components::empty_state(
                "notes-empty",
                IconName::FileText,
                "No notes yet",
                "Create one with ⌘N, or add information with +.",
                None,
            )
            .into_any_element();
        }
        let body = match (self.library.view, self.library.order) {
            (_, LibraryOrder::LinkedToOpenChats) => "No article is linked to an open chat.",
            (_, LibraryOrder::ProtectedOrImported) => "Nothing here is protected or imported.",
            (_, LibraryOrder::MostLinked) => "No article has links into it yet.",
            _ if self.library.topic.is_some() => "This topic page links no current article yet.",
            (LibraryView::Articles, _) => {
                "Every current note here is a full imported reference; see All files."
            }
            _ => "Nothing matches this view.",
        };
        components::empty_state("notes-empty", IconName::FileText, "", body, None)
            .into_any_element()
    }

    /// Saved means stored, never personally reviewed (spec §5).
    fn library_footer(&self) -> gpui_kit::AnyElement {
        let delegation = if !self.settings.delegation_chosen {
            "No maintenance delegation chosen yet"
        } else if !self.settings.maintenance {
            "Maintenance paused"
        } else if self.settings.review_first {
            "Changes are reviewed first"
        } else {
            "BRN may update ordinary articles under your delegation"
        };
        let text = format!(
            "Saved library: saved means stored in BRN, not personally reviewed. {delegation}; proposed changes wait in Inbox until applied."
        );
        div()
            .id("library-footer")
            .flex()
            .items_start()
            .gap_2()
            .px_3()
            .pt(tokens::SPACE_4)
            .pb(tokens::SPACE_6)
            .text_size(px(theme::metrics::TEXT_METADATA))
            .text_color(rgb(pal().muted))
            .aria_label(text.clone())
            .child(
                Icon::new(IconName::Info)
                    .with_size(Size::XSmall)
                    .flex_shrink_0()
                    .mt(px(1.))
                    .text_color(rgb(pal().muted)),
            )
            .child(div().flex_1().min_w_0().child(text))
            .test_support()
            .into_any_element()
    }

    /// The marks a row carries for real state: Protected, a proposed change
    /// waiting for review, and a kept unsaved edit.
    fn note_marks(
        &self,
        row_id: &str,
        note: &NoteRow,
    ) -> (Vec<gpui_kit::AnyElement>, Vec<&'static str>) {
        let mut marks = vec![];
        let mut words = vec![];
        if note.protected {
            words.push("Protected");
            marks.push(
                components::short_status_chip(
                    format!("{row_id}-protected"),
                    Some(Tone::Info),
                    Some(IconName::Lock),
                    "",
                    "Protected",
                )
                .into_any_element(),
            );
        }
        let proposed = self.candidates.iter().any(|candidate| {
            candidate
                .request
                .writes
                .iter()
                .any(|write| write.id == note.id && matches!(write.data, RecordData::Note(_)))
        });
        if proposed {
            words.push("Proposed change waiting");
            marks.push(
                components::status_chip(
                    format!("{row_id}-proposed"),
                    Some(Tone::Warning),
                    None,
                    "Proposed change",
                )
                .into_any_element(),
            );
        }
        if self.buffers.iter().any(|buffer| buffer.note == note.id) {
            words.push("Unsaved edit kept");
            marks.push(
                components::status_chip(
                    format!("{row_id}-unsaved"),
                    Some(Tone::Warning),
                    None,
                    "Unsaved edit",
                )
                .into_any_element(),
            );
        }
        (marks, words)
    }

    fn note_entry(&mut self, index: usize, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(note) = self.note_rows.get(index) else {
            return div().into_any_element();
        };
        let id = note.id.clone();
        let row_id = format!("note-{id}");
        let stamp = self.activity.stamp(&id).to_owned();
        let mut facts = row_facts(note, &stamp, self.activity.delegated(&id));
        let (marks, words) = self.note_marks(&row_id, note);
        facts.extend(words.into_iter().map(str::to_owned));
        let icon = if note.is_file() {
            IconName::Paperclip
        } else {
            IconName::FileText
        };
        let selected = self.open_note_id() == Some(id.as_str());
        let this = cx.entity().downgrade();
        let (open, menu_id) = (id.clone(), id.clone());
        let tooltip = (!stamp.is_empty()).then(|| format!("{} · last change {stamp}", note.title));
        library_row(
            row_id,
            icon,
            note.title.clone(),
            note.description.clone(),
            facts.join(" · "),
            marks,
            selected,
            tooltip,
        )
        .on_click(cx.listener(move |this, _, window, cx| this.open_note(&open, None, window, cx)))
        .context_menu(move |menu, _, _| note_menu(menu, &this, menu_id.clone()))
        .into_any_element()
    }

    fn source_entry(&mut self, index: usize, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(source) = self.library.sources.get(index).cloned() else {
            return div().into_any_element();
        };
        let mut facts = vec!["Source".to_owned()];
        if !source.observed.is_empty() {
            facts.push(format!("Recorded by BRN {}", day_label(&source.observed)));
        }
        if let Some(version) = source.external_version.as_deref().filter(|v| !v.is_empty()) {
            facts.push(format!("Source version {}", short(version)));
        }
        let this = cx.entity().downgrade();
        let open = source.locator.clone();
        let menu_source = source.clone();
        library_row(
            format!("source-{}", source.id),
            IconName::Link,
            source.label.clone(),
            short(&source.locator),
            facts.join(" · "),
            vec![],
            false,
            None,
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            if let Err(limit) = open_source_locator(&open, cx) {
                this.warn(limit);
                cx.notify();
            }
        }))
        .context_menu(move |menu, _, _| source_menu(menu, &this, menu_source.clone()))
        .into_any_element()
    }

    /// Typing in the Library search: matching titles (their kind named),
    /// then the owner search's note text and, separately, chat history.
    fn library_search(&mut self, query: &str, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let (titles, total) = lists::filter_notes(&self.note_rows, query, 100);
        let scope = "Searching current knowledge in your saved library: titles and note text. Chat history is listed separately and is not current knowledge.";
        let mut column = div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .max_w(px(LIBRARY_WIDTH))
            .child(
                div()
                    .id("library-search-scope")
                    .pt(tokens::SPACE_3)
                    .px_3()
                    .text_size(px(theme::metrics::TEXT_METADATA))
                    .text_color(rgb(p.muted))
                    .aria_label(scope)
                    .child(scope)
                    .test_support(),
            )
            .child(section(format!("Titles · {total}")).px_3());
        let current = self.open_note_id().map(str::to_owned);
        for data in titles {
            let id = data.id.clone();
            let menu_id = id.clone();
            let this = cx.entity().downgrade();
            let row_id = format!("note-{id}");
            let kind = if data.is_file() {
                "Full imported reference"
            } else {
                "Article"
            };
            let mut entry = components::ListRow::new(row_id.clone(), data.title.clone())
                .meta(kind)
                .selected(current.as_deref() == Some(id.as_str()));
            if data.protected {
                entry = entry.chip(
                    "Protected",
                    components::short_status_chip(
                        format!("{row_id}-protected"),
                        Some(Tone::Info),
                        Some(IconName::Lock),
                        "",
                        "Protected",
                    ),
                );
            }
            column =
                column.child(
                    entry
                        .actionable()
                        .build()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_note(&id, None, window, cx)
                        }))
                        .context_menu(move |menu, _, _| note_menu(menu, &this, menu_id.clone())),
                );
        }
        if total > 100 {
            column = column.child(metadata(format!("Showing 100 of {total} · keep typing")).px_3());
        }
        column = column.child(self.search_results(cx));
        scroll("notes-list")
            .items_center()
            .px(self.library_inset())
            .child(column)
            .test_support()
            .into_any_element()
    }

    /// Knowledge checks: affected records, limits stated, repairs through
    /// review; reached from the Library ⋯ menu.
    fn knowledge_checks_view(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let back = pill("library-back", "← Library")
            .tooltip("Back to articles")
            .on_click(cx.listener(|this, _, _, cx| {
                this.library.view = LibraryView::Articles;
                cx.notify();
            }));
        let changes = pill("library-recent-changes", "Recent changes").on_click(
            cx.listener(|this, _, window, cx| this.show_tile(TileKind::History, window, cx)),
        );
        let column = div()
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .max_w(px(LIBRARY_WIDTH))
            .pt(tokens::SPACE_4)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(back)
                    .child(div().flex_1())
                    .child(changes),
            )
            .child(
                div()
                    .id("library-checks-heading")
                    .text_size(px(theme::metrics::TEXT_CONVERSATION_HEADING))
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(rgb(pal().ink))
                    .child("Knowledge checks")
                    .test_support(),
            )
            .child(self.knowledge_panel(cx));
        scroll("library-checks")
            .items_center()
            .px(self.library_inset())
            .pb_6()
            .child(column)
            .test_support()
            .into_any_element()
    }

    /// The chat list for the Recent chats route: the same searchable list.
    pub(super) fn threads_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        self.all_threads_tile(cx)
    }

    /// All chats and archived chats: a searchable list styled like the
    /// Library, grouped Today / This week / Older, with the earlier filters
    /// in one menu.
    pub(super) fn all_threads_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let query = self.threads_query.read(cx).value().to_string();
        let filter = self.thread_filter;
        let grouped = lists::all_threads(&self.thread_rows, filter, &query, lists::today());
        let counts = lists::sidebar(&self.thread_rows, self.chat_thread());
        let archived_view = filter == ThreadFilter::Resolved;
        let filter_this = this.clone();
        let filter_label = if matches!(filter, ThreadFilter::All | ThreadFilter::Resolved) {
            "Any chat".to_owned()
        } else {
            filter.label().to_owned()
        };
        let filters = button("threads-filter")
            .label(filter_label.clone())
            .icon(IconName::ChevronDown)
            .ghost()
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .accessibility_label(format!("Filter: {filter_label}"))
            .accessible_dropdown_menu(move |mut menu, _, _| {
                for choice in [
                    ThreadFilter::All,
                    ThreadFilter::Open,
                    ThreadFilter::NeedsYou,
                    ThreadFilter::LinkedNote,
                    ThreadFilter::HasAction,
                ] {
                    let label = if choice == ThreadFilter::All {
                        "Any chat"
                    } else {
                        choice.label()
                    };
                    menu = menu.item(item(
                        &filter_this,
                        label,
                        None,
                        false,
                        filter == choice,
                        move |view, _, cx| {
                            view.thread_filter = choice;
                            cx.notify();
                        },
                    ));
                }
                menu
            });
        let tabs = div()
            .id("chat-views")
            .flex()
            .items_end()
            .gap(tokens::SPACE_4)
            .role(gpui_kit::Role::TabList)
            .aria_label("Chats view")
            .child(
                view_tab(
                    "all-threads",
                    format!("All · {}", counts.total),
                    !archived_view,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.thread_filter = ThreadFilter::All;
                    cx.notify();
                })),
            )
            .child(
                view_tab(
                    "resolved-count",
                    format!("Archived · {}", counts.resolved),
                    archived_view,
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.thread_filter = ThreadFilter::Resolved;
                    this.show_tile(TileKind::AllThreads, window, cx);
                })),
            );
        let status = if grouped.matched > lists::LIST_LIMIT {
            format!(
                "Showing {} of {} · keep typing",
                lists::LIST_LIMIT,
                grouped.matched
            )
        } else {
            format!(
                "{} chat{}",
                grouped.matched,
                if grouped.matched == 1 { "" } else { "s" }
            )
        };
        let tools = div()
            .flex()
            .flex_col()
            .w_full()
            .max_w(px(LIBRARY_WIDTH))
            .child(
                div()
                    .pt(tokens::SPACE_6)
                    .pb(tokens::SPACE_3)
                    .text_size(px(theme::metrics::TEXT_CONVERSATION_HEADING))
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(rgb(p.ink))
                    .child(if archived_view {
                        "Archived chats"
                    } else {
                        "All chats"
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgb(p.hairline))
                    .child(tabs)
                    .child(div().flex_1())
                    .child(filters)
                    .child(
                        div().w(px(240.)).max_w_full().pb_1().child(
                            Input::new(&self.threads_query)
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .with_size(Size::Small)
                                        .text_color(rgb(p.muted)),
                                )
                                .cleanable(true)
                                .w_full(),
                        ),
                    ),
            )
            .child(
                metadata(status)
                    .id("all-threads-status")
                    .px_3()
                    .pt_2()
                    .test_support(),
            );
        let mut column = div().flex().flex_col().w_full().max_w(px(LIBRARY_WIDTH));
        if grouped.groups.is_empty() {
            column = column.child(components::empty_state(
                "all-threads-empty",
                IconName::Search,
                "No matching chats",
                "Try another word or filter.",
                None,
            ));
        }
        for (name, rows) in grouped.groups {
            column = column.child(section(name).px_3());
            for row_data in rows {
                let id = row_data.id.clone();
                let menu_id = id.clone();
                let (resolved, note) = (row_data.resolved, row_data.note.clone());
                let this = this.clone();
                let mut facts = vec![];
                if row_data.needs_you() {
                    facts.push("Needs you".to_owned());
                }
                if resolved {
                    facts.push("Archived".to_owned());
                }
                if row_data.has_action {
                    facts.push("Task".to_owned());
                }
                if let Some(note) = &note {
                    let title = self
                        .note_rows
                        .iter()
                        .find(|row| &row.id == note)
                        .map_or_else(|| "a note".to_owned(), |row| short(&row.title));
                    facts.push(format!("About {title}"));
                }
                let stamp = self.activity.stamp(&id);
                if !stamp.is_empty() {
                    facts.push(format!("Last activity {}", day_label(stamp)));
                }
                let reason = row_data.reasons.first().cloned().unwrap_or_default();
                column = column.child(
                    library_row(
                        format!("all-thread-{id}"),
                        if resolved {
                            IconName::Archive
                        } else {
                            IconName::MessageSquare
                        },
                        row_data.title.clone(),
                        reason,
                        facts.join(" · "),
                        vec![],
                        self.chat_thread() == Some(id.as_str()),
                        None,
                    )
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)),
                    )
                    .context_menu(move |menu, _, _| {
                        thread_menu(menu, &this, menu_id.clone(), resolved, note.clone())
                    }),
                );
            }
        }
        let inset = self.library_inset();
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(
                div()
                    .flex_shrink_0()
                    .w_full()
                    .flex()
                    .justify_center()
                    .px(inset)
                    .child(tools),
            )
            .child(
                div().flex_1().min_h_0().child(
                    scroll("all-threads-list")
                        .items_center()
                        .px(inset)
                        .pb_6()
                        .child(column)
                        .test_support(),
                ),
            )
            .into_any_element()
    }
}

/// The Library and chat list column: wider than a form, near the reading
/// measure (spec §8 document column 680–760).
pub(super) const LIBRARY_WIDTH: f32 = 760.;

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, title: &str, order: usize, file: bool, inbound: usize) -> NoteRow {
        NoteRow {
            id: id.into(),
            title: title.into(),
            protected: false,
            imported: file,
            order,
            inbound,
            description: String::new(),
            source: file.then(|| format!("s-{id}")),
            full_import: file,
        }
    }

    #[test]
    fn articles_exclude_full_imports_and_files_include_them() {
        let rows = vec![
            row("a", "Alpha", 3, false, 0),
            row("b", "Beta", 1, true, 2),
            row("c", "Gamma", 2, false, 5),
        ];
        let none = BTreeSet::new();
        let articles = library_rows(
            &rows,
            LibraryView::Articles,
            LibraryOrder::Recent,
            None,
            &none,
        );
        assert_eq!(articles, [0, 2], "recent first; the full import is a file");
        let files = library_rows(
            &rows,
            LibraryView::Files,
            LibraryOrder::Alphabetical,
            None,
            &none,
        );
        assert_eq!(files, [0, 1, 2], "All files keeps every current note, A–Z");
        let linked = library_rows(
            &rows,
            LibraryView::Files,
            LibraryOrder::MostLinked,
            None,
            &none,
        );
        assert_eq!(linked, [2, 1]);
        let kept = library_rows(
            &rows,
            LibraryView::Files,
            LibraryOrder::ProtectedOrImported,
            None,
            &none,
        );
        assert_eq!(kept, [1]);
        let open: BTreeSet<String> = ["c".to_owned()].into();
        assert_eq!(
            library_rows(
                &rows,
                LibraryView::Articles,
                LibraryOrder::LinkedToOpenChats,
                None,
                &open
            ),
            [2]
        );
        // A topic narrows to the articles its page links; one article can
        // sit under several topics without a copy.
        let topic: BTreeSet<String> = ["a".to_owned(), "b".to_owned()].into();
        assert_eq!(
            library_rows(
                &rows,
                LibraryView::Articles,
                LibraryOrder::Recent,
                Some(&topic),
                &none
            ),
            [0]
        );
    }

    #[test]
    fn facts_name_delegated_maintenance_and_never_claim_review() {
        let article = row("a", "Alpha", 1, false, 3);
        let delegated = row_facts(&article, "2020-01-02T10:00:00Z", true).join(" · ");
        assert_eq!(
            delegated,
            "Updated 2 Jan 2020 by delegated maintenance · Linked from 3"
        );
        let saved = row_facts(&article, "2020-01-02T10:00:00Z", false).join(" · ");
        assert_eq!(saved, "Saved 2 Jan 2020 · Linked from 3");
        for text in [&delegated, &saved] {
            assert!(
                !text.contains("Approved") && !text.contains("eviewed"),
                "{text}"
            );
        }
        let file = row("b", "Beta", 1, true, 0);
        assert_eq!(row_facts(&file, "", false), ["Full imported reference"]);
    }

    #[test]
    fn sources_with_a_readable_import_are_not_listed_twice() {
        let source = |id: &str, chat: Option<&str>| Record {
            id: id.into(),
            version: 1,
            archived: false,
            data: RecordData::Source(Source {
                chat: chat.map(str::to_owned),
                locator: format!("/tmp/{id}.pdf"),
                label: format!("{id}.pdf"),
                observed_at: "2026-11-01T09:00:00Z".into(),
                external_version: None,
                outcome: "converted".into(),
                gaps: vec![],
                report: None,
            }),
        };
        let records = vec![
            source("s-b", None),
            source("loose", None),
            source("private", Some("chat")),
        ];
        let notes = vec![row("b", "Beta", 1, true, 0)];
        let rows = source_rows(&records, &notes);
        assert_eq!(
            rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ["loose"],
            "imported and chat-private sources are not separate rows"
        );
        assert_eq!(rows[0].observed, "2026-11-01T09:00:00Z");
    }
}
