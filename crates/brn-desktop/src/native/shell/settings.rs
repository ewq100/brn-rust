use super::*;
use gpui_kit::{App, Entity, TestSupportExt, component::WindowExt};

impl Desktop {
    pub(in crate::native) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let desktop = cx.entity();
        window.open_dialog(cx, move |dialog, _, cx| {
            dialog
                .title("Settings")
                .w(px(460.))
                .child(settings_body(&desktop, cx))
        });
    }
}

/// Rebuilt on every dialog render from the current `Desktop` state.
fn settings_body(desktop: &Entity<Desktop>, cx: &App) -> impl IntoElement {
    let this = desktop.read(cx);
    let p = this.palette();
    let prefs = this.layout.clone();
    let data_dir = spaced_identifier(&this.path.display().to_string());
    let appearance_button = |id: &'static str, label: &'static str, value: Appearance| {
        let target = desktop.downgrade();
        Button::new(id)
            .label(label)
            .compact()
            .selected(prefs.appearance == value)
            .on_click(move |_, window, cx| {
                let _ = target.update(cx, |this, cx| this.set_appearance(value, window, cx));
            })
    };
    let resize_button = |id: &'static str, label: &'static str, rail: Rail, delta: f32| {
        let target = desktop.downgrade();
        Button::new(id)
            .label(label)
            .compact()
            .on_click(move |_, _, cx| {
                let _ = target.update(cx, |this, cx| {
                    this.layout.resize_rail(rail, delta);
                    this.persist_layout(cx);
                });
            })
    };
    let reset_target = desktop.downgrade();
    div()
        .id("settings-body")
        .test_support()
        .flex()
        .flex_col()
        .max_h(px(340.))
        .overflow_y_scroll()
        .gap_3()
        .child(section_label("Workspace", p))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(140.)).child("Appearance"))
                .child(appearance_button(
                    "appearance-system",
                    "System",
                    Appearance::System,
                ))
                .child(appearance_button(
                    "appearance-dark",
                    "Dark",
                    Appearance::Dark,
                ))
                .child(appearance_button(
                    "appearance-light",
                    "Light",
                    Appearance::Light,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(140.))
                        .child(format!("History: {} pt", prefs.history_w.round() as i32)),
                )
                .child(resize_button(
                    "history-narrower",
                    "−",
                    Rail::History,
                    -layout::KEY_STEP,
                ))
                .child(resize_button(
                    "history-wider",
                    "+",
                    Rail::History,
                    layout::KEY_STEP,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(140.))
                        .child(format!("Vault: {} pt", prefs.vault_w.round() as i32)),
                )
                .child(resize_button(
                    "vault-narrower",
                    "−",
                    Rail::Vault,
                    -layout::KEY_STEP,
                ))
                .child(resize_button(
                    "vault-wider",
                    "+",
                    Rail::Vault,
                    layout::KEY_STEP,
                )),
        )
        .child(div().text_color(color(p.muted)).child(format!(
            "Document share: {}% of the centre",
            (prefs.doc_share * 100.0).round() as i32
        )))
        .child(
            Button::new("reset-layout")
                .label("Reset layout")
                .on_click(move |_, _, cx| {
                    let _ = reset_target.update(cx, |this, cx| {
                        this.layout.reset_layout();
                        this.persist_layout(cx);
                    });
                }),
        )
        .child(section_label("Connection", p))
        .child(format!("Data directory: {data_dir}"))
        .child(super::super::simple::account_settings(desktop, cx))
}
