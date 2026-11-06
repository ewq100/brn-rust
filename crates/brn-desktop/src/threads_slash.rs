//! The composer's / command list (spec §6.1.1, CHAT-08). Typing / at the
//! start of a word lists the actual skill catalogue; choosing a command only
//! inserts `/command` into the unsent message. The request is read from the
//! sent text, so it belongs to that one message and run: deleting the text
//! removes it, and nothing persists for the chat, project or later messages.
//! Built-in skills stay automatically available either way.
use super::*;
use gpui_kit::{EntityInputHandler, Focusable};
use std::ops::Range;

/// One choosable command, as the skill catalogue reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SlashCommand {
    /// The skill id a run receives in its requested skills.
    pub id: String,
    /// What the owner types after `/`.
    pub command: String,
    pub description: String,
    /// Why the skill cannot be loaded now, when it cannot.
    pub unavailable: Option<String>,
}

/// The open list: the `/word` under the caret and the commands it matches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SlashMenu {
    /// The `/word` being completed, in UTF-16 offsets of the composer text.
    pub range: Range<usize>,
    pub query: String,
    /// Indices into the catalogue, best match first.
    pub matches: Vec<usize>,
    pub index: usize,
}

/// The composer's command state.
#[derive(Clone, Debug, Default)]
pub(super) struct Slash {
    /// The catalogue the list was built from, for the current scope.
    pub commands: Vec<SlashCommand>,
    pub menu: Option<SlashMenu>,
    /// The start of a `/word` the owner dismissed with Escape; it stays
    /// closed until the caret leaves that word.
    pub dismissed: Option<usize>,
    /// Keeps the keyboard-selected row in view when the list scrolls.
    pub scroll: gpui_kit::ScrollHandle,
}

fn command_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_'
}

fn byte_at_utf16(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units >= utf16 {
            return byte;
        }
        units += c.len_utf16();
    }
    text.len()
}

fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// The `/word` the caret is in, when it starts a word: its UTF-16 range
/// (the whole word, also past the caret) and the typed name. Slashes inside
/// prose (`and/or`), URLs and paths (`/usr/bin`) never qualify.
pub(super) fn slash_token(text: &str, caret_utf16: usize) -> Option<(Range<usize>, String)> {
    let caret = byte_at_utf16(text, caret_utf16);
    let before = &text[..caret];
    let start = before
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map_or(0, |(i, c)| i + c.len_utf8());
    let word = &before[start..];
    let name = word.strip_prefix('/')?;
    if !name.chars().all(command_char) {
        return None;
    }
    let rest = &text[caret..];
    let tail = rest.find(|c: char| !command_char(c)).unwrap_or(rest.len());
    if rest[tail..].starts_with('/') {
        return None;
    }
    let from = utf16_len(&text[..start]);
    let to = utf16_len(&text[..caret + tail]);
    Some((from..to, format!("{name}{}", &rest[..tail])))
}

/// Matching commands, best first: name prefix, then name, then description.
pub(super) fn slash_matches(commands: &[SlashCommand], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    let mut ranked = commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            let name = command.command.to_lowercase();
            let rank = if name.starts_with(&query) {
                0
            } else if name.contains(&query) {
                1
            } else if command.description.to_lowercase().contains(&query) {
                2
            } else {
                return None;
            };
            Some((rank, command.unavailable.is_some(), index))
        })
        .collect::<Vec<_>>();
    ranked.sort();
    ranked.into_iter().map(|(_, _, index)| index).collect()
}

/// The skills a message explicitly requests: each word `/command` naming an
/// available skill, in order and once. Words inside prose, URLs and paths,
/// and unknown or unavailable names, request nothing.
pub(super) fn requested_skills(text: &str, commands: &[SlashCommand]) -> Vec<String> {
    let mut ids = Vec::new();
    for word in text.split_whitespace() {
        let Some(name) = word.strip_prefix('/') else {
            continue;
        };
        let name = name.trim_end_matches(['.', ',', ';', ':', '!', '?']);
        if name.is_empty() || !name.chars().all(command_char) {
            continue;
        }
        if let Some(command) = commands
            .iter()
            .find(|c| c.command == name && c.unavailable.is_none())
            && !ids.contains(&command.id)
        {
            ids.push(command.id.clone());
        }
    }
    ids
}

impl ThreadsUi {
    /// A chat's skill catalogue as / commands: built-ins and enabled
    /// installed packages for its scope, with truthful availability.
    pub(super) fn slash_catalogue(&self, chat: Option<&str>) -> Vec<SlashCommand> {
        let project = chat
            .and_then(|chat| project_for_chat(&self.records, chat))
            .map(|record| record.id.clone());
        let Ok(commands) = self.workspace.skill_commands(project.as_deref()) else {
            return Vec::new();
        };
        commands
            .into_iter()
            .map(|command| SlashCommand {
                id: command.id,
                command: command.command,
                description: command.description,
                unavailable: (!command.available).then(|| {
                    command
                        .unavailable
                        .unwrap_or_else(|| "Unavailable".to_owned())
                }),
            })
            .collect()
    }

    /// The composer text or caret changed: open, narrow or close the list.
    pub(super) fn update_slash_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (text, caret) = self.composer.update(cx, |state, cx| {
            let caret = state
                .selected_text_range(false, window, cx)
                .filter(|selection| selection.range.is_empty())
                .map(|selection| selection.range.end);
            (state.value().to_string(), caret)
        });
        let token = caret.and_then(|caret| slash_token(&text, caret));
        let before = self.slash.menu.clone();
        match token {
            None => {
                self.slash.menu = None;
                self.slash.dismissed = None;
            }
            Some((range, _)) if self.slash.dismissed == Some(range.start) => {
                self.slash.menu = None;
            }
            Some((range, query)) => {
                self.slash.dismissed = None;
                if self.slash.menu.is_none() {
                    self.slash.commands = self.slash_catalogue(self.chat_thread());
                }
                let matches = slash_matches(&self.slash.commands, &query);
                if matches.is_empty() {
                    self.slash.menu = None;
                } else {
                    let index = before
                        .as_ref()
                        .filter(|menu| menu.query == query)
                        .map_or(0, |menu| menu.index.min(matches.len() - 1));
                    self.slash.scroll.scroll_to_item(index);
                    self.slash.menu = Some(SlashMenu {
                        range,
                        query,
                        matches,
                        index,
                    });
                }
            }
        }
        if before != self.slash.menu {
            cx.notify();
        }
    }

    pub(super) fn move_slash(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some(menu) = &mut self.slash.menu {
            let len = menu.matches.len() as isize;
            menu.index = (menu.index as isize + delta).rem_euclid(len) as usize;
            self.slash.scroll.scroll_to_item(menu.index);
            cx.notify();
        }
    }

    /// The caret left the word or the field lost focus; the next edit in a
    /// `/word` reopens the list.
    pub(super) fn close_slash(&mut self, cx: &mut Context<Self>) {
        if self.slash.menu.take().is_some() {
            cx.notify();
        }
    }

    pub(super) fn dismiss_slash(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.slash.menu.take() {
            self.slash.dismissed = Some(menu.range.start);
            cx.notify();
        }
    }

    /// Replace the `/word` with the chosen command and a space. Only the
    /// message text changes; an unavailable skill is explained, not inserted.
    pub(super) fn choose_slash(
        &mut self,
        index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = self.slash.menu.clone() else {
            return;
        };
        let Some(command) = menu
            .matches
            .get(index.unwrap_or(menu.index))
            .and_then(|&i| self.slash.commands.get(i))
            .cloned()
        else {
            return;
        };
        if let Some(reason) = &command.unavailable {
            self.warn(format!("/{} is unavailable: {reason}", command.command));
            cx.notify();
            return;
        }
        self.slash.menu = None;
        let inserted = format!("/{} ", command.command);
        self.composer.update(cx, |state, cx| {
            state.replace_text_in_range(Some(menu.range.clone()), &inserted, window, cx);
        });
        let focus = self.composer.focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// The skills a message to `chat` explicitly requests, from that chat's
    /// current catalogue.
    pub(super) fn message_requested_skills(&self, chat: &str, text: &str) -> Vec<String> {
        if !text.contains('/') {
            return Vec::new();
        }
        requested_skills(text, &self.slash_catalogue(Some(chat)))
    }

    /// The list above the composer field, when open.
    pub(super) fn slash_menu(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let menu = self.slash.menu.as_ref()?;
        let p = pal();
        // The New chat start surface centres the composer near the top of
        // a short window, so the list opens below it there; a docked
        // composer opens it above, over the transcript.
        let below = self.chat_thread().is_none() && self.layout.primary() == TileKind::Chat;
        let mut list = div()
            .id("slash-menu")
            .role(gpui_kit::Role::ListBox)
            .aria_label("Skill commands")
            .test_support()
            .absolute()
            .left_0()
            .right_0()
            .when(below, |list| list.top(gpui_kit::relative(1.)).mt_1())
            .when(!below, |list| list.bottom(gpui_kit::relative(1.)).mb_1())
            .track_scroll(&self.slash.scroll)
            .p_1()
            .flex()
            .flex_col()
            .gap_px()
            .max_h(px(260.))
            .overflow_y_scroll()
            .bg(rgb(p.surface_raised))
            .border_1()
            .border_color(rgb(p.border))
            .rounded(px(theme::metrics::RADIUS_CARD))
            .shadow_lg()
            .occlude();
        for (row, &index) in menu.matches.iter().enumerate() {
            let Some(command) = self.slash.commands.get(index) else {
                continue;
            };
            let selected = row == menu.index;
            let disabled = command.unavailable.is_some();
            let detail = command
                .unavailable
                .clone()
                .unwrap_or_else(|| command.description.clone());
            let label = if detail.is_empty() {
                format!("/{}", command.command)
            } else {
                format!("/{}, {detail}", command.command)
            };
            list = list.child(
                div()
                    .id(format!("slash-row-{row}"))
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .min_h(px(theme::metrics::SIDEBAR_ROW))
                    .px_2()
                    .py_1()
                    .rounded(px(theme::metrics::RADIUS_CONTROL))
                    .when(selected, |row| row.bg(rgb(p.selection)))
                    .when(!selected, |row| row.hover(|style| style.bg(rgb(p.hover))))
                    .when(!disabled, |row| row.cursor_pointer())
                    .role(gpui_kit::Role::ListBoxOption)
                    .aria_label(label)
                    .aria_selected(selected)
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .text_color(rgb(if disabled { p.muted } else { p.ink }))
                            .child(format!("/{}", command.command)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            // Descriptions are cut at a word to 100 chars;
                            // two lines show them whole at narrow widths.
                            .line_clamp(2)
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .child(detail),
                    )
                    .on_mouse_down(gpui_kit::MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.choose_slash(Some(row), window, cx)
                    }))
                    .test_support(),
            );
        }
        Some(list.into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(name: &str, description: &str) -> SlashCommand {
        SlashCommand {
            id: name.to_owned(),
            command: name.to_owned(),
            description: description.to_owned(),
            unavailable: None,
        }
    }

    fn token(text: &str) -> Option<String> {
        let caret = text.find('|').expect("caret");
        let text = text.replace('|', "");
        slash_token(&text, utf16_len(&text[..caret])).map(|(_, name)| name)
    }

    #[test]
    fn only_a_slash_that_starts_a_word_opens_the_list() {
        assert_eq!(token("/|").as_deref(), Some(""));
        assert_eq!(token("please /sum|").as_deref(), Some("sum"));
        assert_eq!(token("please /su|m now").as_deref(), Some("sum"));
        assert_eq!(token("🙂 /ré|").as_deref(), Some("ré"));
        assert_eq!(token("and/or|"), None);
        assert_eq!(token("see https://example.com/a|"), None);
        assert_eq!(token("/usr/bin|"), None);
        assert_eq!(token("/us|r/bin"), None);
        assert_eq!(token("/sum |"), None);
    }

    #[test]
    fn the_range_covers_the_whole_word_in_utf16() {
        let text = "🙂 /su now";
        let (range, name) = slash_token(text, 5).expect("token");
        assert_eq!(name, "su");
        assert_eq!(range, 3..6);
    }

    #[test]
    fn matches_rank_name_prefix_before_description() {
        let commands = vec![
            command("review", "Check a summary"),
            command("summarize", "Short version"),
            command("presum", "Other"),
        ];
        assert_eq!(slash_matches(&commands, "sum"), vec![1, 2, 0]);
        assert_eq!(slash_matches(&commands, ""), vec![0, 1, 2]);
        assert!(slash_matches(&commands, "zzz").is_empty());
    }

    #[test]
    fn only_command_words_naming_an_available_skill_are_requested() {
        let mut offline = command("offline", "");
        offline.unavailable = Some("Missing tool".into());
        let commands = vec![command("summarize", ""), command("review", ""), offline];
        assert_eq!(
            requested_skills("/summarize this, then /review. /summarize again", &commands),
            vec!["summarize".to_owned(), "review".to_owned()]
        );
        assert!(requested_skills("and/or /usr/bin http://x/summarize", &commands).is_empty());
        assert!(requested_skills("/offline /unknown", &commands).is_empty());
    }
}
