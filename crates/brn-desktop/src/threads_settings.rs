//! Settings: the local section list and each section's controls. Every
//! control calls the existing checked services.
use super::tiles::column;
use super::*;
use components::unavailable;
use gpui_kit::SharedString;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;
use gpui_kit::component::radio::Radio;
use gpui_kit::component::switch::Switch;

/// Session-only disclosure key for the advanced (legacy) palette list.
const APPEARANCE_ADVANCED: &str = "appearance-advanced";
/// Settings forms are narrower than pages.
const FORM_WIDTH: f32 = 640.;

fn group(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div().id(id).flex().flex_col().gap(px(2.)).min_w_0()
}

/// A labelled form field: the label above its control.
fn field(label: &str, control: impl IntoElement) -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(metadata(label.to_owned()))
        .child(control)
}

/// The saved provider's credential state, as far as BRN can tell without
/// contacting the provider: a saved location that is missing is said so.
pub(super) fn credential_status(settings: &WorkspaceSettings) -> (Tone, String) {
    let copilot = shell::provider_key(&settings.provider) == "copilot";
    let saved = if copilot {
        settings.credentials_dir.as_deref()
    } else {
        settings.auth_file.as_deref()
    }
    .map(str::trim)
    .filter(|path| !path.is_empty());
    let kind = if copilot {
        "Copilot credentials folder"
    } else {
        "Codex auth file"
    };
    let provider = if copilot {
        "GitHub Copilot"
    } else {
        "Codex / ChatGPT"
    };
    match saved {
        Some(path) if std::path::Path::new(path).exists() => (
            Tone::Success,
            format!("Saved {kind} found. Sign-in is checked when a chat starts."),
        ),
        Some(path) => (
            Tone::Warning,
            format!("{kind} not found at {path}. {provider} chats are unavailable until it exists."),
        ),
        None if copilot => (
            Tone::Warning,
            "GitHub Copilot is not connected. Use Connect GitHub Copilot… below.".into(),
        ),
        None => (
            Tone::Info,
            "No Codex auth file saved. BRN uses your Codex sign-in if there is one; a chat says so if not.".into(),
        ),
    }
}

impl ThreadsUi {
    /// The delegation choice. In Settings it is two radios and, once chosen,
    /// the maintenance switch; on Home or in a new chat it is the pending
    /// first-use card with two quiet buttons. Both apply immediately.
    pub(super) fn working_knowledge(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let status = delegation_status(&self.settings);
        let chosen = self.settings.delegation_chosen;
        let review_first = self.settings.review_first;
        let options = [
            (
                DelegationChoice::MaintainOrdinaryNotes,
                "delegation-maintain",
                "Maintain ordinary notes",
                "BRN updates ordinary notes and source references.",
                chosen && !review_first,
            ),
            (
                DelegationChoice::ReviewFirst,
                "delegation-review-first",
                "Review changes first",
                "BRN proposes; you approve.",
                chosen && review_first,
            ),
        ];
        let status_line = metadata(status)
            .id("delegation-status")
            .aria_label(status)
            .test_support();
        let protected_line =
            metadata("Protected notes change only with your instruction or review.");
        if self.delegation_host() != Some(TileKind::Settings) {
            // GPUI may wrap before "?", so the last word is its own unit: a
            // narrow chat wraps "notes?" whole onto the next line.
            let (lead, last) = DELEGATION_QUESTION
                .rsplit_once(' ')
                .unwrap_or(("", DELEGATION_QUESTION));
            let mut card = components::card("delegation-pending", None)
                .child(
                    div()
                        .id("delegation-question")
                        .flex()
                        .flex_wrap()
                        .gap_x(px(4.))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .aria_label(DELEGATION_QUESTION)
                        .child(div().min_w_0().child(lead))
                        .child(
                            div()
                                .id("delegation-question-end")
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .child(last)
                                .test_support(),
                        )
                        .test_support(),
                )
                .child(status_line);
            for (choice, id, label, explanation, _) in options {
                card = card.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_x_2()
                        .gap_y_1()
                        .child(
                            button(id)
                                .label(label)
                                .outline()
                                .compact()
                                .h(components::tokens::COMPACT_CONTROL)
                                .accessibility_label(format!("{label}: {explanation}"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.choose_delegation(choice, cx)
                                })),
                        )
                        // Beside the button when there is room, otherwise on
                        // its own line, wrapping inside the card.
                        .child(
                            metadata(explanation)
                                .id(format!("{id}-why"))
                                .flex_grow(1.)
                                .flex_basis(px(160.))
                                .min_w_0()
                                .test_support(),
                        ),
                );
            }
            return card.child(protected_line).test_support().into_any_element();
        }
        let mut radios = div()
            .id("delegation-choice")
            .flex()
            .flex_col()
            .gap_3()
            .aria_label(DELEGATION_QUESTION);
        for (choice, id, label, explanation, checked) in options {
            radios =
                radios.child(
                    Radio::new(id)
                        .label(label)
                        .checked(checked)
                        .accessibility_label(format!("{label}: {explanation}"))
                        .child(metadata(explanation))
                        .on_click(cx.listener(move |this, _: &bool, _, cx| {
                            this.choose_delegation(choice, cx)
                        })),
                );
        }
        let mut form = column()
            .id(if chosen {
                "delegation-settings"
            } else {
                "delegation-pending"
            })
            .gap_3()
            .child(section(DELEGATION_QUESTION))
            .child(radios.test_support())
            .child(protected_line)
            .child(status_line);
        if chosen {
            let running = self.settings.maintenance;
            form = form.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .pt_2()
                    .child(subheading("Maintenance"))
                    .child(
                        Switch::new("maintenance-setting")
                            .checked(running)
                            .label(if running { "Running" } else { "Paused" })
                            .accessibility_label(if running {
                                "Maintenance: Running"
                            } else {
                                "Maintenance: Paused"
                            })
                            .tooltip(if running {
                                "Pause maintenance"
                            } else {
                                "Resume maintenance"
                            })
                            .on_click(cx.listener(|this, _: &bool, _, cx| {
                                match this.workspace.pause(this.settings.maintenance) {
                                    Ok(outcome) => this.report_outcome(&outcome),
                                    Err(error) => this.fail(error.to_string()),
                                }
                                this.refresh();
                                cx.notify();
                            })),
                    ),
            );
        }
        form.test_support().into_any_element()
    }
    /// A Settings segmented choice: each segment keeps its own ID and
    /// handler.
    fn choice<T: Copy + 'static>(
        id: &str,
        options: &[(T, &str, &str)],
        selected: impl Fn(T) -> bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Self, T) + Copy + 'static,
    ) -> gpui_kit::component::button::ButtonGroup {
        components::segmented(
            SharedString::from(id.to_owned()),
            options
                .iter()
                .map(|&(value, key, label)| {
                    button(SharedString::from(key.to_owned()))
                        .label(label.to_owned())
                        .selected(selected(value))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            set(this, value);
                            cx.notify();
                        }))
                })
                .collect(),
        )
    }
    fn models_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let provider = &self.provider_form.provider;
        let efforts = [
            ("low", "effort-low", "Low"),
            ("medium", "effort-medium", "Medium"),
            ("high", "effort-high", "High"),
        ];
        let model = group("settings-provider")
            .gap_3()
            .child(section("Provider and model"))
            .child(field(
                "Provider",
                Self::choice(
                    "settings-provider-choice",
                    &[
                        ("chatgpt", "provider-chatgpt", "Codex / ChatGPT"),
                        ("copilot", "provider-copilot", "GitHub Copilot"),
                    ],
                    // The default "Codex" provider is the Codex / ChatGPT segment.
                    |value| shell::provider_key(provider) == value,
                    cx,
                    |this, value| this.provider_form.provider = value.into(),
                ),
            ))
            .child(field(
                "Model",
                Input::new(&self.model).w_full().aria_label("Model"),
            ))
            .child(field(
                "Reasoning effort",
                Self::choice(
                    "settings-effort-choice",
                    &efforts,
                    |value| self.provider_form.effort == value,
                    cx,
                    |this, value| this.provider_form.effort = value.into(),
                ),
            ))
            .child(field(
                "Codex auth file",
                Input::new(&self.auth_path)
                    .w_full()
                    .aria_label("Codex auth file"),
            ))
            .child(field(
                "Copilot credentials folder",
                Input::new(&self.credentials_path)
                    .w_full()
                    .aria_label("Copilot credentials folder"),
            ))
            .child({
                let (tone, line) = credential_status(&self.settings);
                div()
                    .id("settings-credential-status")
                    .flex()
                    .items_start()
                    .gap_2()
                    .aria_label(format!("{}: {line}", tone.word()))
                    .child(components::tone_mark("settings-credential-tone", tone))
                    .child(div().flex_1().min_w_0().text_size(SMALL_TEXT).child(line))
                    .test_support()
            })
            .child(metadata(
                "Credentials stay outside notes, exports and Changes. The Codex auth file is used only with Codex / ChatGPT.",
            ))
            .child(div().child({
                let connect = button("connect-copilot")
                    .label("Connect GitHub Copilot…")
                    .outline()
                    .disabled(self.worker.is_some())
                    .on_click(
                        cx.listener(|this, _, window, cx| this.connect_copilot(window, cx)),
                    );
                if self.worker.is_some() {
                    let (announced, tooltip) =
                        unavailable("Connect GitHub Copilot", Some("BRN is working"));
                    connect.accessibility_label(announced).tooltip(tooltip)
                } else {
                    connect.tooltip(
                        "Sign in with GitHub in your browser; uses the folder above or ~/.config/brn/copilot",
                    )
                }
            }));
        let helper_on = |value: &str| match &self.provider_form.helper_provider {
            None => value == "off",
            Some(provider) => provider.eq_ignore_ascii_case(value),
        };
        let mut helper = group("settings-helper")
            .gap_3()
            .child(section("Helper sub-agents"))
            .child(
                "Read-only helpers can research your notes for the chat model. They can't change notes or stand in for an unavailable model.",
            )
            .child(
                metadata(brn_threads_app::HELPER_TESTED_NOTE)
                    .id("settings-helper-tested")
                    .aria_label(brn_threads_app::HELPER_TESTED_NOTE)
                    .test_support(),
            )
            .child(field(
                "Helper provider",
                Self::choice(
                    "settings-helper-choice",
                    &[
                        ("off", "helper-provider-off", "Off"),
                        ("Codex", "helper-provider-Codex", "Codex / ChatGPT"),
                        ("Copilot", "helper-provider-Copilot", "GitHub Copilot"),
                    ],
                    helper_on,
                    cx,
                    |this, value| {
                        if value == "off" {
                            this.provider_form.helper_provider = None;
                            this.provider_form.helper_effort = "high".into();
                        } else {
                            this.provider_form.helper_provider = Some(value.into());
                        }
                    },
                ),
            ));
        if self.provider_form.helper_provider.is_some() {
            helper = helper
                .child(field(
                    "Helper model",
                    Input::new(&self.helper_model)
                        .w_full()
                        .aria_label("Helper model"),
                ))
                .child(field(
                    "Helper reasoning effort",
                    Self::choice(
                        "settings-helper-effort-choice",
                        &[
                            ("low", "helper-effort-low", "Low"),
                            ("medium", "helper-effort-medium", "Medium"),
                            ("high", "helper-effort-high", "High"),
                        ],
                        |value| self.provider_form.helper_effort == value,
                        cx,
                        |this, value| this.provider_form.helper_effort = value.into(),
                    ),
                ))
                .child(metadata(
                    "Uses the credential location above for its provider.",
                ));
        }
        // One Save covers exactly what `configure` saves, helper included.
        let why = self.worker.is_some().then_some("BRN is working");
        let (announced, tooltip) = unavailable("Save settings", why);
        let save = div()
            .flex()
            .flex_col()
            .gap_1()
            .pt_4()
            .mt_4()
            .border_t_1()
            .border_color(rgb(pal().hairline))
            .child(
                div().child(
                    button("save-settings")
                        .label("Save settings")
                        .primary()
                        .disabled(why.is_some())
                        .accessibility_label(announced)
                        .tooltip(tooltip)
                        .on_click(cx.listener(|this, _, _, cx| this.configure(cx))),
                ),
            )
            .child(metadata(
                "Saves provider, model, credentials and helper settings.",
            ))
            .children(why.map(|_| {
                metadata("Available when the current work finishes.")
                    .id("save-settings-reason")
                    .test_support()
            }));
        column()
            .gap_6()
            .child(model)
            .child(helper)
            .child(save)
            .into_any_element()
    }
    fn data_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let folder = self
            .settings
            .backup_directory
            .clone()
            .unwrap_or_else(|| "Backup folder not chosen".into());
        let location = self.data_dir.display().to_string();
        let data_location = group("settings-data-location")
            .gap_2()
            .child(section("Data location"))
            .child("This workspace's notes, chats, history and settings live here.")
            .child(
                div()
                    .id("data-location")
                    .aria_label(location.clone())
                    .font_family(MONO)
                    .text_size(SMALL_TEXT)
                    .child(location)
                    .test_support(),
            )
            .child(
                div().child(
                    button("data-location-reveal")
                        .label("Show in Finder")
                        .outline()
                        .compact()
                        .h(components::tokens::COMPACT_CONTROL)
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.reveal_path(&this.data_dir);
                        })),
                ),
            );
        let backup = group("settings-backup")
            .gap_2()
            .child(section("Full backup"))
            .child("A complete copy you can restore: notes, discussions, history, figures and recovery state.")
            .child(
                div()
                    .id("backup-directory")
                    .aria_label(folder.clone())
                    .font_family(MONO)
                    .text_size(SMALL_TEXT)
                    .child(folder)
                    .test_support(),
            )
            .child(metadata(
                "Choose a folder covered by your separate backup system.",
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        button("backup-now")
                            .label("Create full backup")
                            .primary()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_output(true, window, cx)
                            })),
                    )
                    .child(
                        button("backup-change-folder")
                            .label("Change folder…")
                            .outline()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_backup_folder(false, window, cx)
                            })),
                    ),
            )
            .child(metadata(self.backup_status.clone()));
        let export =
            group("settings-export")
                .gap_2()
                .child(section("Markdown export"))
                .child("A readable snapshot of your notes and retained figures for other apps. It is not a backup and cannot be restored.")
                .child(
                    div().child(
                        button("export-now")
                            .label("Export Markdown…")
                            .outline()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_output(false, window, cx)
                            })),
                    ),
                )
                .child(metadata(self.export_status.clone()));
        let deletion = group("settings-deletion")
            .gap_2()
            .child(section("Deleted items"))
            .child(
                "Projects you move to Trash stay there, with their notes and chats, until you restore them. Other changes are listed in Recent changes, with Undo where it applies.",
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        button("data-open-trash")
                            .label("Open Projects Trash")
                            .outline()
                            .compact()
                            .h(components::tokens::COMPACT_CONTROL)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.projects_trash = true;
                                this.show_tile(TileKind::Projects, window, cx)
                            })),
                    )
                    .child(
                        button("data-open-history")
                            .label("Recent changes")
                            .outline()
                            .compact()
                            .h(components::tokens::COMPACT_CONTROL)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.show_tile(TileKind::History, window, cx)
                            })),
                    ),
            );
        column()
            .id("settings-copies")
            .gap_6()
            .child(data_location)
            .child(backup)
            .child(self.restore_section(cx))
            .child(export)
            .child(deletion)
            .test_support()
            .into_any_element()
    }
    fn appearance_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        // Primary choice: System / Light / Dark as a keyboard-reachable
        // segmented control. The selected segment is marked by state, not by
        // colour alone, and each segment carries an accessible name (spec §9,
        // AC-18). System is a BRN preference resolved against the OS.
        let primary = components::segmented(
            SharedString::from("settings-appearance"),
            [
                (Appearance::System, "appearance-system", "System"),
                (Appearance::Light, "appearance-light", "Light"),
                (Appearance::Dark, "appearance-dark", "Dark"),
            ]
            .into_iter()
            .map(|(value, id, label)| {
                button(SharedString::from(id))
                    .label(label)
                    .selected(self.appearance == value)
                    .accessibility_label(format!("{label}: {}", value.description()))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.choose_appearance(value, window, cx)
                    }))
            })
            .collect(),
        );

        // Advanced palettes: the five earlier themes, under a disclosure so
        // they stay out of the primary choice. Each row names the palette and
        // shows its swatches; the active legacy choice is marked by state.
        let advanced_open = self.open_sections.contains(APPEARANCE_ADVANCED);
        let mut advanced = group("settings-legacy-palettes");
        for theme in ThemeId::LEGACY {
            let colors = theme.palette();
            let value = Appearance::Legacy(theme);
            let selected = self.appearance == value;
            let swatches = div().flex().flex_shrink_0().gap_1().children(
                [
                    colors.paper,
                    colors.tile,
                    colors.alt,
                    colors.accent,
                    colors.ink,
                ]
                .into_iter()
                .map(|color| {
                    div()
                        .w(px(14.))
                        .h(px(14.))
                        .rounded(px(3.))
                        .border_1()
                        .border_color(rgb(p.hairline))
                        .bg(rgb(color))
                }),
            );
            advanced = advanced.child(
                button(format!("settings-theme-{}", theme.key()))
                    .ghost()
                    .w_full()
                    .h(components::tokens::FORM_CONTROL)
                    .selected(selected)
                    .accessibility_label(format!("{} (legacy palette)", theme.label()))
                    .tooltip(theme.description())
                    .child(div().w(px(110.)).flex_shrink_0().child(theme.label()))
                    .child(swatches)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(if selected { p.ink } else { p.muted }))
                            .child(theme.description()),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.choose_appearance(value, window, cx)
                    })),
            );
        }
        let advanced_count = ThemeId::LEGACY.len();
        let mut advanced_section = column().id("settings-advanced-palettes").gap_2().child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(section("Advanced palettes"))
                .child(self.disclosure(APPEARANCE_ADVANCED, advanced_count, cx)),
        );
        if advanced_open {
            advanced_section = advanced_section.child(advanced.test_support());
        }

        column()
            .id("settings-appearance-page")
            .gap_2()
            .child(components::section_header(
                "Appearance",
                None,
                Some(metadata("Also in ⌘K.").into_any_element()),
            ))
            .child(group("settings-appearance-primary").gap_2().child(primary))
            .child(metadata(
                "System follows your Mac's light or dark setting as it changes.",
            ))
            .child(advanced_section)
            .test_support()
            .into_any_element()
    }
    pub(super) fn report_problem(&self, cx: &mut Context<Self>) {
        cx.open_url(&crate::problem_report::report_url(&self.settings));
    }
    fn general_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        column()
            .id("settings-general")
            .gap_2()
            .child(section("About BRN"))
            .child(metadata(crate::problem_report::AppVersion::current().label())
                .id("app-version").aria_label(crate::problem_report::AppVersion::current().label()).test_support())
            .child(metadata(format!("Active provider: {} · Model: {}", self.settings.provider, self.settings.model))
                .id("app-provider-model").test_support())
            .child(div().child(button("report-problem").label("Report a problem").outline()
                .on_click(cx.listener(|this, _, _, cx| this.report_problem(cx)))))
            .child(metadata("Opens a GitHub form with build and model details. Review and edit it before submitting."))
            .child(section("Keyboard shortcuts"))
            .child(metadata(
                "Every command, its shortcut and the mouse gestures BRN understands.",
            ))
            .child(
                div().child(
                    button("settings-keys")
                        .label("Show keyboard shortcuts")
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| this.show_keys(window, cx))),
                ),
            )
            .test_support()
            .into_any_element()
    }
    pub(super) fn settings_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let sections = SettingsSection::ALL
            .into_iter()
            .zip(self.settings_section_focus.clone())
            .map(|(item, focus)| {
                button(format!("settings-section-{}", item.key()))
                    .track_focus(&focus)
                    .accessibility_label(item.label())
                    .ghost()
                    .w_full()
                    .px_3()
                    .h(components::tokens::FORM_CONTROL)
                    // The kit centres a plain label; a filling child keeps the
                    // section names left-aligned like a list.
                    .child(
                        gpui_kit::div()
                            .id(gpui_kit::SharedString::from(format!(
                                "settings-section-{}-label",
                                item.key()
                            )))
                            .flex_1()
                            .min_w_0()
                            .text_left()
                            .child(item.label())
                            .test_support(),
                    )
                    .selected(self.settings_section == item)
                    .on_click(
                        cx.listener(move |this, event: &gpui_kit::ClickEvent, window, cx| {
                            this.settings_section = item;
                            // Kit buttons don't take focus on a click, so the
                            // ring would stay on the segment focus landed on.
                            // A pointer click moves it to the chosen section;
                            // a key press already focused it.
                            if !event.is_keyboard() {
                                focus.focus(window, cx);
                            }
                            cx.notify();
                        }),
                    )
            })
            .collect::<Vec<_>>();
        let content = match self.settings_section {
            SettingsSection::Appearance => self.appearance_settings(cx),
            SettingsSection::General => self.general_settings(cx),
            SettingsSection::Models => self.models_settings(cx),
            SettingsSection::Delegation => self.working_knowledge(cx),
            SettingsSection::Documents => self.documents_settings(cx),
            SettingsSection::Data => self.data_settings(cx),
            SettingsSection::Skills => self.skills_settings(cx),
            SettingsSection::Mcp => self.mcp_settings(cx),
        };
        // A readable local section list beside the section's form; narrower
        // in a compact window so the form keeps its room.
        let compact = self.arrangement.primary < 720.;
        let inset = if compact {
            theme::metrics::COMPACT_PAGE_INSET
        } else {
            theme::metrics::PAGE_INSET
        };
        let nav = div()
            .id("settings-sections")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(px(2.))
            .w(px(if compact { 132. } else { 176. }))
            .pt(components::tokens::SPACE_4)
            .pl(px(inset))
            .pr_2()
            .role(gpui_kit::Role::Group)
            .aria_label("Settings sections")
            .children(sections)
            .test_support();
        div()
            .size_full()
            .flex()
            .min_h_0()
            .child(nav)
            .child(
                div()
                    .id("settings-content")
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .pt(components::tokens::SPACE_4)
                    .pl_4()
                    .pr(px(inset))
                    .pb_6()
                    .font_family(UI_FONT)
                    .text_size(UI_TEXT)
                    .overflow_y_scroll()
                    .child(div().w_full().max_w(px(FORM_WIDTH)).child(content))
                    .test_support(),
            )
            .into_any_element()
    }
}

/// The delegation question, asked once in Settings and on the first-use card.
const DELEGATION_QUESTION: &str = "How should BRN update your notes?";
