//! **Allow web access** in the composer's + menu and the web sources under an
//! answer (spec §6.1.1, CHAT-07). The service decides access per chat; this
//! view only shows the state and saves the owner's choice. A new chat keeps
//! the owner's choice until the send that creates it.
use super::shell::one_line;
use super::*;
use brn_threads_app::{WebAccessState, WebSource};
use gpui_kit::WeakEntity;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable, Size};

impl ThreadsUi {
    /// The current destination's web access, as the next run would see it.
    pub(super) fn web_access(&self) -> WebAccessState {
        let chat = self.chat_thread().map(str::to_owned);
        let state = match self.workspace.chat_web_access(chat.as_deref()) {
            Ok(state) => state,
            Err(error) => return WebAccessState::Unavailable(error.to_string()),
        };
        match state {
            WebAccessState::Allowed if chat.is_none() && self.new_chat_web_off => {
                WebAccessState::Off
            }
            state => state,
        }
    }

    /// Flip Allowed and Off. Unavailable has nothing to flip; an archived
    /// chat must be unarchived first. The choice applies to later runs.
    pub(super) fn toggle_web_access(&mut self, cx: &mut Context<Self>) {
        let allowed = match self.web_access() {
            WebAccessState::Allowed => false,
            WebAccessState::Off => true,
            WebAccessState::Unavailable(_) => return,
        };
        match self.chat_thread().map(str::to_owned) {
            Some(chat) if self.thread_resolved(&chat) => {
                self.warn("Unarchive this chat before changing web access.");
            }
            Some(chat) => {
                if let Err(error) = self.workspace.set_chat_web_allowed(&chat, allowed) {
                    self.fail(format!("Web access was not changed: {error}"));
                }
            }
            None => self.new_chat_web_off = !allowed,
        }
        cx.notify();
    }

    /// Save a new chat's pending Off before its first run starts.
    pub(super) fn adopt_new_chat_web(&mut self, chat: &str) {
        if std::mem::take(&mut self.new_chat_web_off)
            && let Err(error) = self.workspace.set_chat_web_allowed(chat, false)
        {
            self.fail(format!(
                "Web access could not be turned off for this chat: {error}"
            ));
        }
    }

    pub(super) fn web_menu(&self, this: &WeakEntity<ThreadsUi>, mut menu: PopupMenu) -> PopupMenu {
        let state = self.web_access();
        menu = menu.separator().item(super::shell::item(
            this,
            "Allow web access",
            None,
            matches!(state, WebAccessState::Unavailable(_)),
            state == WebAccessState::Allowed,
            |view, _, cx| view.toggle_web_access(cx),
        ));
        if let WebAccessState::Unavailable(reason) = state {
            menu = menu.label(format!("Web unavailable · {reason}"));
        }
        menu
    }

    /// The public pages this answer's run searched, as links the owner can
    /// open. Untrusted evidence; nothing here is followed automatically.
    pub(super) fn web_sources(
        &self,
        message: &str,
        run: Option<&str>,
    ) -> Option<gpui_kit::AnyElement> {
        let run = run?;
        let sources = self.records.iter().find_map(|record| match &record.data {
            RecordData::Run(r) if record.id == run && !r.web_sources.is_empty() => {
                Some(r.web_sources.clone())
            }
            _ => None,
        })?;
        let mut list = div()
            .id(format!("answer-web-sources-{message}"))
            .flex()
            .flex_col()
            .gap_0p5()
            .aria_label("Web sources")
            .child(metadata("Web sources"));
        for (index, source) in sources.iter().enumerate() {
            list = list.child(source_link(message, index, source));
        }
        Some(list.test_support().into_any_element())
    }
}

fn source_link(message: &str, index: usize, source: &WebSource) -> gpui_kit::AnyElement {
    let url = source.url.clone();
    let label = source_label(source);
    let openable = url.starts_with("https://") || url.starts_with("http://");
    div()
        .id(format!("answer-web-source-{message}-{index}"))
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        .text_size(SMALL_TEXT)
        .text_color(rgb(if openable { pal().accent } else { pal().muted }))
        .when(openable, |row| {
            row.cursor_pointer()
                .hover(|style| style.underline())
                .on_click(move |_, _, cx| cx.open_url(&url))
        })
        .tooltip({
            let url = source.url.clone();
            move |window, cx| Tooltip::new(url.clone()).build(window, cx)
        })
        .aria_label(format!("{label} · {}", source.url))
        .child(
            Icon::new(IconName::Globe)
                .with_size(Size::XSmall)
                .flex_shrink_0(),
        )
        .child(div().min_w_0().truncate().child(label))
        .test_support()
        .into_any_element()
}

/// The page title, else its host, so a long address never fills the row.
pub(super) fn source_label(source: &WebSource) -> String {
    if let Some(title) = source.title.as_deref().map(str::trim)
        && !title.is_empty()
    {
        return one_line(title);
    }
    let rest = source
        .url
        .split_once("://")
        .map_or(source.url.as_str(), |(_, rest)| rest);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .to_owned()
}
