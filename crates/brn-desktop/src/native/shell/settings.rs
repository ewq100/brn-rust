use super::*;
use gpui_kit::{App, Entity, TestSupportExt, base::Disableable, component::WindowExt};

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
        .child(backup_settings(desktop, cx))
        .child(section_label("Connection", p))
        .child(format!("Data directory: {data_dir}"))
        .child(super::super::simple::account_settings(desktop, cx))
}

/// The same Settings section is also exercised by headless widget tests.
pub(in crate::native) fn backup_settings(desktop: &Entity<Desktop>, cx: &App) -> impl IntoElement {
    let this = desktop.read(cx);
    let p = this.palette();
    let ai = this.ai.as_ref().unwrap();
    let busy = ai.backup_pending();
    let disabled = !ai.ready || busy || this.closing.is_some() || this.closed || this.close_failed;
    let status = ai.backup_status.as_ref();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|value| u64::try_from(value.as_millis()).ok());
    let button = |id: &'static str, label: &'static str, checkpoint: bool| {
        let target = desktop.downgrade();
        Button::new(id)
            .label(label)
            .compact()
            .disabled(disabled)
            .on_click(move |_, _, cx| {
                let _ = target.update(cx, |this, cx| {
                    if this.closing.is_some() || this.closed || this.close_failed {
                        return;
                    }
                    if let Some(command) = this.ai.as_mut().unwrap().request_backup(checkpoint) {
                        this.simple_send(command, cx);
                    }
                });
            })
    };
    let mut body = div()
        .id("backup-settings")
        .test_support()
        .flex()
        .flex_col()
        .gap_2()
        .child(section_label("Internal state backups", p))
        .child(backup_line(
            "backup-path",
            status.map_or_else(
                || "Last usable copy: status not yet available".into(),
                |status| {
                    format!(
                        "Last usable copy: {}",
                        spaced_identifier(&status.latest_path.display().to_string())
                    )
                },
            ),
            p.text,
        ))
        .child(backup_line(
            "backup-copy-time",
            backup_copy_time(status.and_then(|status| status.completed_at_ms), now_ms),
            p.muted,
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(button("checkpoint-backup", "Back up current state", true))
                .child(button(
                    "refresh-backup-status",
                    "Refresh backup status",
                    false,
                )),
        );
    if busy {
        body = body.child(backup_line(
            "backup-pending",
            "Waiting for backup acknowledgement".into(),
            p.muted,
        ));
    }
    if let Some(warning) = status.and_then(|status| status.retention_warning.as_ref()) {
        body = body.child(backup_line(
            "backup-retention-warning",
            format!("Retention warning: {warning}"),
            p.amber,
        ));
    }
    if let Some(error) = status.and_then(|status| status.last_error.as_ref()) {
        body = body.child(backup_line(
            "backup-error",
            format!("Backup failed: {error}"),
            p.amber,
        ));
    }
    if let Some(error) = &ai.backup_request_error {
        body = body.child(backup_line(
            "backup-request-error",
            format!("Backup request failed: {error}"),
            p.amber,
        ));
    }
    body
}

fn backup_line(id: &'static str, text: String, text_color: u32) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .aria_label(text.clone())
        .text_color(color(text_color))
        .child(text)
}

fn backup_copy_time(completed_at_ms: Option<u64>, now_ms: Option<u64>) -> String {
    let Some(completed) = completed_at_ms else {
        return "Copy time unknown".into();
    };
    let Some(elapsed) = now_ms.and_then(|now| now.checked_sub(completed)) else {
        return "Copy time is ahead of this clock or clock unavailable".into();
    };
    let (count, unit) = if elapsed < 60_000 {
        return "Copy completed just now".into();
    } else if elapsed < 3_600_000 {
        (elapsed / 60_000, "minute")
    } else if elapsed < 86_400_000 {
        (elapsed / 3_600_000, "hour")
    } else {
        (elapsed / 86_400_000, "day")
    };
    format!(
        "Copy completed {count} {unit}{} ago",
        if count == 1 { "" } else { "s" }
    )
}

#[cfg(test)]
mod tests {
    use super::backup_copy_time;

    #[test]
    fn copy_time_never_invents_startup_or_future_timestamps() {
        assert_eq!(backup_copy_time(None, Some(100_000)), "Copy time unknown");
        assert_eq!(
            backup_copy_time(Some(100_000), Some(100_000)),
            "Copy completed just now"
        );
        assert_eq!(
            backup_copy_time(Some(100_000), Some(160_000)),
            "Copy completed 1 minute ago"
        );
        assert_eq!(
            backup_copy_time(Some(100_000), Some(7_300_000)),
            "Copy completed 2 hours ago"
        );
        assert_eq!(
            backup_copy_time(Some(100_000), Some(86_500_000)),
            "Copy completed 1 day ago"
        );
        assert!(backup_copy_time(Some(100_001), Some(100_000)).contains("ahead"));
        assert!(backup_copy_time(Some(100_001), None).contains("unavailable"));
    }
}
