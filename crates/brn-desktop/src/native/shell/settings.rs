//! Settings window: tabs on the left, one decision per row (D25).
use super::*;
use crate::native::ui::{self, Tone};
use gpui_kit::{App, Entity, TestSupportExt, component::WindowExt};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native) enum SettingsTab {
    #[default]
    General,
    Ai,
    Accounts,
    Search,
    Data,
    About,
}

impl SettingsTab {
    const ALL: [SettingsTab; 6] = [
        SettingsTab::General,
        SettingsTab::Ai,
        SettingsTab::Accounts,
        SettingsTab::Search,
        SettingsTab::Data,
        SettingsTab::About,
    ];
    fn id(self) -> &'static str {
        match self {
            SettingsTab::General => "settings-tab-general",
            SettingsTab::Ai => "settings-tab-ai",
            SettingsTab::Accounts => "settings-tab-accounts",
            SettingsTab::Search => "settings-tab-search",
            SettingsTab::Data => "settings-tab-data",
            SettingsTab::About => "settings-tab-about",
        }
    }
    fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Ai => "AI & models",
            SettingsTab::Accounts => "Accounts",
            SettingsTab::Search => "Search",
            SettingsTab::Data => "Data & safety",
            SettingsTab::About => "About",
        }
    }
}

impl Desktop {
    pub(in crate::native) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let desktop = cx.entity();
        window.open_dialog(cx, move |dialog, _, cx| {
            dialog
                .title("Settings")
                .w(px(760.))
                .child(settings_body(&desktop, cx))
        });
    }
}

/// Rebuilt on every dialog render from the current `Desktop` state.
fn settings_body(desktop: &Entity<Desktop>, cx: &App) -> impl IntoElement {
    let this = desktop.read(cx);
    let p = this.palette();
    let current = this.settings_tab;
    let mut tabs = div()
        .w(px(180.))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .pr(px(tokens::space::MD))
        .border_r_1()
        .border_color(color(p.line));
    for tab in SettingsTab::ALL {
        let target = desktop.downgrade();
        tabs = tabs.child(
            Button::new(tab.id())
                .ghost()
                .w_full()
                .h(px(tokens::size::ROW))
                .selected(tab == current)
                .accessibility_label(tab.label())
                .child(
                    div()
                        .flex_1()
                        .text_size(px(tokens::text::BODY))
                        .child(tab.label()),
                )
                .on_click(move |_, _, cx| {
                    let _ = target.update(cx, |this, cx| {
                        this.settings_tab = tab;
                        cx.notify();
                    });
                }),
        );
    }
    let page = match current {
        SettingsTab::General => general(desktop, cx),
        SettingsTab::Ai => super::super::simple::ai_settings(desktop, cx),
        SettingsTab::Accounts => super::super::simple::accounts_settings(desktop, cx),
        SettingsTab::Search => super::super::simple::search_settings(desktop, cx),
        SettingsTab::Data => data(desktop, cx),
        SettingsTab::About => about(desktop, cx),
    };
    div()
        .id("settings-body")
        .test_support()
        .flex()
        .h(px(480.))
        .child(tabs)
        .child(
            div()
                .id("settings-page")
                .flex_1()
                .min_w(px(0.))
                .overflow_y_scroll()
                .pl(px(tokens::space::XL))
                .pr(px(tokens::space::SM))
                .child(
                    div()
                        .text_size(px(tokens::text::TITLE))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .pb(px(tokens::space::SM))
                        .child(current.label()),
                )
                .child(page),
        )
}

fn general(desktop: &Entity<Desktop>, cx: &App) -> AnyElement {
    let this = desktop.read(cx);
    let p = this.palette();
    let appearance = this.layout.appearance;
    let mut choices = ui::toolbar().gap(px(2.));
    for (id, label, value) in [
        ("appearance-system", "System", Appearance::System),
        ("appearance-dark", "Dark", Appearance::Dark),
        ("appearance-light", "Light", Appearance::Light),
    ] {
        let target = desktop.downgrade();
        choices = choices.child(
            Button::new(id)
                .label(label)
                .small()
                .when(appearance == value, |button| button.primary())
                .when(appearance != value, |button| button.ghost())
                .selected(appearance == value)
                .on_click(move |_, window, cx| {
                    let _ = target.update(cx, |this, cx| this.set_appearance(value, window, cx));
                }),
        );
    }
    let reset_target = desktop.downgrade();
    div()
        .flex()
        .flex_col()
        .child(ui::setting_row("Appearance", None, choices, p))
        .child(ui::setting_row(
            "Layout",
            Some("Drag the edges of the sidebars to resize them.".into()),
            Button::new("reset-layout")
                .label("Reset layout")
                .small()
                .outline()
                .on_click(move |_, _, cx| {
                    let _ = reset_target.update(cx, |this, cx| {
                        this.layout.reset_layout();
                        this.persist_layout(cx);
                    });
                }),
            p,
        ))
        .into_any_element()
}

fn data(desktop: &Entity<Desktop>, cx: &App) -> AnyElement {
    let this = desktop.read(cx);
    let p = this.palette();
    let vault = super::super::simple::vault_location(this);
    div()
        .flex()
        .flex_col()
        .child(ui::setting_row(
            "Vault",
            Some(vault.unwrap_or_else(|| "No vault chosen".into())),
            ui::badge("your notes", Tone::Neutral, p),
            p,
        ))
        .child(ui::setting_row(
            "Data folder",
            Some(spaced_identifier(&this.path.display().to_string())),
            ui::badge("work database", Tone::Neutral, p),
            p,
        ))
        .child(
            ui::hint(
                "Notes stay as Markdown files in your vault. The work database in the data folder holds chats, proposals and Actions; BRN checks it at start and keeps backups.",
                p,
            )
            .pt(px(tokens::space::MD)),
        )
        .into_any_element()
}

fn about(desktop: &Entity<Desktop>, cx: &App) -> AnyElement {
    let p = desktop.read(cx).palette();
    div()
        .flex()
        .flex_col()
        .child(ui::setting_row(
            "BRN",
            Some(format!("Version {}", env!("CARGO_PKG_VERSION"))),
            div(),
            p,
        ))
        .into_any_element()
}
