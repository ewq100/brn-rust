//! Owner skill controls over the checked app service. File I/O happens on
//! refresh/actions, never in render; a stale scope snapshot is not displayed.
use super::*;
#[derive(Default)]
pub(super) struct SkillView {
    pub project: Option<String>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    stamp: Option<SkillStamp>,
    pub loaded: Option<(String, LoadedSkillDetails)>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub rows: Vec<SkillStatus>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    inventory: Vec<PackageInfo>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub inspection: Option<String>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub error: Option<String>,
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(PartialEq, Eq)]
struct SkillStamp {
    policy: Vec<SkillBinding>,
    project: Option<(String, u64, bool)>,
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Clone, Copy)]
pub(super) enum SkillAction {
    Enable,
    AdoptCurrent,
    Disable,
    Select,
    Clear,
    Remove,
    Inspect,
}
impl ThreadsUi {
    pub(super) fn skill_project(&self) -> Option<String> {
        match &self.chat {
            ChatTarget::Thread(chat) => project_for_chat(&self.records, chat).map(|p| p.id.clone()),
            _ => self.selected_project.clone(),
        }
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn refresh_skills(&mut self) {
        let project = self.skill_project();
        let context = project
            .as_ref()
            .and_then(|id| self.records.iter().find(|r| &r.id == id))
            .map(|r| (r.id.clone(), r.version, r.archived));
        let stamp = SkillStamp {
            policy: self.settings.skills.clone(),
            project: context,
        };
        let inspection = if self.skill_view.project == project {
            self.skill_view.inspection.take()
        } else {
            None
        };
        let loaded = self.current_loaded_skills();
        if self.skill_view.project == project && self.skill_view.stamp.as_ref() == Some(&stamp) {
            self.skill_view.loaded = loaded;
            self.skill_view.inspection = inspection;
            return;
        }
        let result = self.workspace.scoped_skills(project.as_deref());
        self.skill_view = match result {
            Ok(mut rows) => {
                let (inventory, error) = match self.workspace.skills() {
                    Ok(i) => (i, None),
                    Err(e) => (vec![], Some(e.to_string())),
                };
                for info in &inventory {
                    if !rows.iter().any(|s| s.id == info.id) {
                        rows.push(SkillStatus {
                            id: info.id.clone(),
                            description: info.description.clone(),
                            origin: "owner_installed".into(),
                            project: project.clone(),
                            digest: info.digest.clone(),
                            enabled: false,
                            selected: false,
                            available: true,
                            unavailable: None,
                        });
                    }
                }
                rows.sort_by(|a, b| a.id.cmp(&b.id));
                SkillView {
                    project,
                    rows,
                    inventory,
                    inspection,
                    loaded,
                    error,
                    stamp: Some(stamp),
                }
            }
            Err(error) => SkillView {
                project,
                error: Some(error.to_string()),
                ..Default::default()
            },
        };
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn force_refresh_skills(&mut self) {
        self.skill_view.stamp = None;
        self.refresh_skills();
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn skill_action(
        &mut self,
        action: SkillAction,
        id: &str,
        digest: &str,
        project: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let current = match &self.chat {
            ChatTarget::Thread(chat) => self
                .workspace
                .store
                .project_context_record(chat)
                .map(|p| p.map(|p| p.id)),
            _ => Ok(self.selected_project.clone()),
        };
        if current.as_ref().ok() != Some(&project) {
            self.warn("Skill scope changed. Review the current scope before choosing again.");
            cx.notify();
            return;
        }
        let result = match action {
            SkillAction::Enable | SkillAction::AdoptCurrent | SkillAction::Disable => self
                .workspace
                .set_skill_enabled(
                    id,
                    digest,
                    project.as_deref(),
                    matches!(action, SkillAction::Enable | SkillAction::AdoptCurrent),
                )
                .map(|_| ()),
            SkillAction::Select => self.workspace.select_skill(Some(id),project.as_deref()).map(|_|()),
            SkillAction::Clear => self.workspace.select_skill(None,project.as_deref()).map(|_|()),
            SkillAction::Remove => self.workspace.remove_skill(id.strip_prefix("installed:").unwrap_or(id),digest).map(|_|()),
            SkillAction::Inspect => self.workspace.inspect_skill(id.strip_prefix("installed:").unwrap_or(id)).and_then(|p| {
                if p.info.digest != digest {
                    return Err(crate::AppError::Invalid(
                        "Current skill package changed since this view was rendered. Refresh and inspect again.".into(),
                    ));
                }
                // Plain text only; bounded preview explicitly labels omission.
                let text=serde_json::to_string_pretty(&p).unwrap_or_default();
                self.skill_view.inspection=Some(if text.len() <= 8192 {text} else {format!("{}\n[Inspection preview truncated at 8192 UTF-8 bytes; exact package remains unchanged.]",truncate_skill(&text,8192))});
                Ok(())
            }),
        };
        match result {
            Ok(()) => {
                if matches!(action, SkillAction::Remove | SkillAction::AdoptCurrent) {
                    self.skill_view.stamp = None;
                }
                self.refresh();
            }
            Err(error) => self.fail(error.to_string()),
        };
        cx.notify();
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn skill_folder_chosen(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match self.workspace.install_skill(&path) {
            Ok(_) => {
                self.skill_view.stamp = None;
                self.refresh();
                self.warn("Skill installed locally. Enable it explicitly in the intended scope before selecting or loading it.");
            }
            Err(error) => self.fail(error.to_string()),
        }
        cx.notify();
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn choose_skill_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose an instruction-only skill package".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let paths = receiver.await;
            let _ = this.update_in(cx, |this, _, cx| match paths {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        this.skill_folder_chosen(path, cx)
                    }
                }
                Ok(Ok(None)) => {}
                _ => {
                    this.fail("Could not choose a skill folder.");
                    cx.notify();
                }
            });
        })
        .detach();
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn skill_selector(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let project = self.skill_project();
        let mut row = div()
            .id("composer-skill-selector")
            .flex()
            .flex_wrap()
            .gap_1()
            .child(metadata("Skill:"));
        if self.skill_view.project == project {
            for skill in self.skill_view.rows.iter().filter(|s| s.enabled) {
                let (id, digest, scope) = (skill.id.clone(), skill.digest.clone(), project.clone());
                let label = if skill.available {
                    skill.id.clone()
                } else {
                    format!("{} · unavailable", skill.id)
                };
                row = row.child(
                    button(format!("composer-skill-{}", skill.id))
                        .label(label)
                        .selected(skill.selected)
                        .disabled(!skill.available)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.skill_action(SkillAction::Select, &id, &digest, scope.clone(), cx)
                        })),
                );
            }
            let scope = project.clone();
            row = row.child(
                button("composer-skill-none")
                    .label("None")
                    .selected(!self.skill_view.rows.iter().any(|s| s.selected))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.skill_action(SkillAction::Clear, "", "", scope.clone(), cx)
                    })),
            );
        }
        row = row.child(
            button("composer-skills-refresh")
                .label("Refresh skills")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.force_refresh_skills();
                    cx.notify();
                })),
        );
        row.test_support().into_any_element()
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(super) fn skills_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let project = self.skill_project();
        let mut panel=div().id("skills-settings").flex().flex_col().gap_3()
            .child(section("Instruction skills — current scope"))
            .child(metadata(project.as_ref().map_or("Scope: general chats".into(),|p|format!("Scope: Project {p}"))))
            .child(metadata(SKILL_RETENTION_NOTICE))
            .child(metadata("Scoped enablement, selection and loaded identities are canonical and backed up. Missing local bytes remain unavailable after restore; reinstall the exact approved original. Policy changes interrupt active runs; nothing restarts automatically."))
            .child(button("skill-install-folder").label("Install workspace-local package…").on_click(cx.listener(|this,_,window,cx|this.choose_skill_folder(window,cx))))
            .child(button("skills-refresh").label("Refresh packages").on_click(cx.listener(|this,_,_,cx|{this.force_refresh_skills();cx.notify();})));
        if let Some(error) = &self.skill_view.error {
            panel = panel.child(metadata(error.clone()));
        }
        if self.skill_view.project == project {
            for skill in &self.skill_view.rows {
                let current_package = self
                    .skill_view
                    .inventory
                    .iter()
                    .find(|package| package.id == skill.id);
                let stale_digest = current_package.filter(|package| package.digest != skill.digest);
                let mut item = div()
                    .id(format!("skill-row-{}", skill.id))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(section(skill.id.clone()))
                    .child(metadata(format!(
                        "{} · {} · Approved digest: {}{}",
                        skill.origin,
                        if skill.enabled { "enabled" } else { "disabled" },
                        skill.digest,
                        if skill.selected { " · selected" } else { "" }
                    )))
                    .child(metadata(skill.description.clone()));
                if let Some(package) = current_package {
                    item = item.child(metadata(format!(
                        "Current package digest: {}{}",
                        package.digest,
                        if stale_digest.is_some() {
                            " · differs from approved digest"
                        } else {
                            ""
                        }
                    )));
                }
                if let Some(reason) = &skill.unavailable {
                    item = item.child(metadata(reason.clone()));
                }
                let mut actions = div().flex().flex_wrap().gap_1();
                let mut choices = vec![
                    (SkillAction::Inspect, "inspect", "Inspect"),
                    (
                        if skill.enabled {
                            SkillAction::Disable
                        } else {
                            SkillAction::Enable
                        },
                        "enable",
                        if skill.enabled { "Disable" } else { "Enable" },
                    ),
                ];
                if stale_digest.is_some() {
                    choices.push((
                        SkillAction::AdoptCurrent,
                        "adopt-current",
                        "Enable current version",
                    ));
                }
                if skill.id.starts_with("installed:") {
                    choices.push((
                        SkillAction::Remove,
                        "remove",
                        "Remove workspace-wide package",
                    ));
                }
                for (action, key, label) in choices {
                    let digest =
                        if matches!(action, SkillAction::AdoptCurrent | SkillAction::Inspect) {
                            current_package.map_or_else(
                                || skill.digest.clone(),
                                |package| package.digest.clone(),
                            )
                        } else {
                            skill.digest.clone()
                        };
                    let (id, scope) = (skill.id.clone(), project.clone());
                    actions = actions.child(
                        button(format!("skill-{key}-{}", skill.id))
                            .label(label)
                            .disabled(match action {
                                SkillAction::Disable | SkillAction::Inspect => false,
                                SkillAction::AdoptCurrent => current_package.is_none(),
                                _ => !skill.available,
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.skill_action(action, &id, &digest, scope.clone(), cx)
                            })),
                    );
                }
                panel = panel.child(item.child(actions));
            }
        }
        if self.skill_view.project == project
            && let Some(text) = &self.skill_view.inspection
        {
            panel = panel.child(
                div()
                    .id("skill-inspection")
                    .text_size(SMALL_TEXT)
                    .child(text.clone())
                    .test_support(),
            );
        }
        panel.test_support().into_any_element()
    }
    fn current_loaded_skills(&self) -> Option<(String, LoadedSkillDetails)> {
        self.chat_thread()
            .and_then(|chat| latest_run(&self.records, &self.history, chat))
            .and_then(|r| {
                self.workspace
                    .loaded_skill_details(&r.id)
                    .ok()
                    .map(|d| (r.id.clone(), d))
            })
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(super) fn refresh_skills(&mut self) {
        self.skill_view.project = self.skill_project();
        self.skill_view.loaded = self.current_loaded_skills();
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(super) fn skill_selector(&self, _cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        div()
            .id("composer-skill-selector")
            .child(metadata(
                "Local instruction skill management is unavailable on this platform.",
            ))
            .test_support()
            .into_any_element()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub(super) fn skills_settings(&self, _cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        div().id("skills-settings").flex().flex_col()
            .child(section("Instruction skills"))
            .child(metadata("Local instruction skill management is unavailable on this platform. No package action was performed."))
            .test_support().into_any_element()
    }
    pub(super) fn loaded_skill_details(&self, chat: &str) -> gpui_kit::AnyElement {
        let mut panel = div().id("loaded-skill-details").flex().flex_col().gap_1();
        if let Some(record) = latest_run(&self.records, &self.history, chat)
            && let RecordData::Run(run) = &record.data
        {
            panel = panel.child(metadata(format!(
                "Guidance: {}",
                truncate_skill(&run.guide_identity, 256)
            )));
            if self.skill_view.project == self.skill_project()
                && let Some((id, visible)) = &self.skill_view.loaded
                && id == &record.id
            {
                for loaded in &visible.details {
                    panel = panel.child(metadata(format!(
                        "Admitted instruction {} · {} · {} · {} · {}",
                        loaded.id,
                        loaded.origin,
                        loaded.package_digest,
                        loaded.file,
                        loaded.file_digest
                    )));
                }
                if visible.omitted > 0 {
                    panel = panel.child(metadata(format!(
                        "{} additional permitted loaded file identities in saved run details",
                        visible.omitted
                    )));
                }
            }
        }
        panel.test_support().into_any_element()
    }
}
fn truncate_skill(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1
    }
    &text[..end]
}
