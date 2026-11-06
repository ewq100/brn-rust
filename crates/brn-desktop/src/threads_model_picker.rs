//! The composer's combined provider/model/effort picker (spec §6.1.1,
//! CHAT-09). Choices come from each connected provider's catalogue, never
//! from a history of model names. A model switch adjusts an effort the new
//! model does not support in the same Settings change, and says so. Every
//! choice changes the explicit Settings through the existing configure path;
//! earlier runs keep the provider, model and effort they recorded.
use super::shell::{one_line, provider_key};
use super::*;
use brn_ai::{ModelOption, ReasoningEffort};
use components::tokens;
use gpui_kit::component::{Icon, Sizable, Size};
use gpui_kit::{MouseButton, Point};

/// Below this window width the reasoning section moves under the models.
const PICKER_STACKED: f32 = 640.;

/// One provider's model catalogue as the picker last learned it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum Catalogue {
    #[default]
    NotLoaded,
    Loading(u64),
    Loaded(Vec<ModelOption>),
    Failed(String),
}

/// The picker's left column: Favorites or one provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PickerTab {
    Favorites,
    Provider(&'static str),
}

impl PickerTab {
    pub(super) const ALL: [PickerTab; 3] = [
        PickerTab::Favorites,
        PickerTab::Provider("chatgpt"),
        PickerTab::Provider("copilot"),
    ];
    fn id(self) -> &'static str {
        match self {
            PickerTab::Favorites => "model-provider-favorites",
            PickerTab::Provider("copilot") => "model-provider-copilot",
            PickerTab::Provider(_) => "model-provider-chatgpt",
        }
    }
    fn label(self) -> &'static str {
        match self {
            PickerTab::Favorites => "Favorites",
            PickerTab::Provider(provider) => provider_name(provider),
        }
    }
    fn icon(self) -> IconName {
        match self {
            PickerTab::Favorites => IconName::Star,
            PickerTab::Provider(provider) => provider_icon(provider),
        }
    }
}

/// Which picker section the arrow keys and Return act on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum PickerSection {
    Providers,
    #[default]
    Models,
    Efforts,
}

pub(super) struct ModelPicker {
    pub open: bool,
    pub tab: PickerTab,
    pub section: PickerSection,
    pub index: usize,
    pub effort_index: usize,
    /// Where the trigger was activated; the panel opens above it.
    pub anchor: Point<gpui_kit::Pixels>,
    pub query: Entity<InputState>,
    pub chatgpt: Catalogue,
    pub copilot: Catalogue,
    /// Fences catalogue answers that a newer request or a Settings change
    /// replaced.
    pub generation: u64,
    /// The credentials the catalogues were learned through.
    pub route: (Option<String>, Option<String>),
    pub favorites: Vec<(String, String)>,
}

impl ModelPicker {
    pub(super) fn new(query: Entity<InputState>, favorites: Vec<(String, String)>) -> Self {
        Self {
            open: false,
            tab: PickerTab::Provider("chatgpt"),
            section: PickerSection::Models,
            index: 0,
            effort_index: 0,
            anchor: Point::default(),
            query,
            chatgpt: Catalogue::NotLoaded,
            copilot: Catalogue::NotLoaded,
            generation: 0,
            route: (None, None),
            favorites,
        }
    }
    pub(super) fn catalogue(&self, provider: &str) -> &Catalogue {
        if provider == "copilot" {
            &self.copilot
        } else {
            &self.chatgpt
        }
    }
    pub(super) fn catalogue_mut(&mut self, provider: &str) -> &mut Catalogue {
        if provider == "copilot" {
            &mut self.copilot
        } else {
            &mut self.chatgpt
        }
    }
    /// The catalogue entry for a model, when its provider's catalogue is
    /// loaded and offers it.
    pub(super) fn option(&self, provider: &str, model: &str) -> Option<&ModelOption> {
        match self.catalogue(provider_key(provider)) {
            Catalogue::Loaded(models) => models.iter().find(|option| option.id == model),
            _ => None,
        }
    }
    pub(super) fn is_favorite(&self, provider: &str, model: &str) -> bool {
        self.favorites
            .iter()
            .any(|(p, m)| p == provider && m == model)
    }
}

pub(super) fn provider_name(provider: &str) -> &'static str {
    match provider_key(provider) {
        "copilot" => "GitHub Copilot",
        "chatgpt" => "ChatGPT",
        _ => "No provider",
    }
}
fn provider_icon(provider: &str) -> IconName {
    match provider_key(provider) {
        "copilot" => IconName::Github,
        _ => IconName::Bot,
    }
}

pub(super) fn effort_name(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
    }
}
fn parse_effort(effort: &str) -> Option<ReasoningEffort> {
    match effort {
        "low" => Some(ReasoningEffort::Low),
        "medium" => Some(ReasoningEffort::Medium),
        "high" => Some(ReasoningEffort::High),
        _ => None,
    }
}
const ALL_EFFORTS: [ReasoningEffort; 3] = [
    ReasoningEffort::Low,
    ReasoningEffort::Medium,
    ReasoningEffort::High,
];

/// The reasoning levels a model offers that BRN can request. `reported` is
/// false when the provider's catalogue says nothing about reasoning (GitHub
/// Copilot, or a model not in a loaded catalogue): BRN then offers its own
/// levels as requests and says the provider did not confirm them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Efforts {
    pub levels: Vec<ReasoningEffort>,
    pub reported: bool,
}
pub(super) fn supported_efforts(option: Option<&ModelOption>) -> Efforts {
    match option.and_then(|option| option.efforts.clone()) {
        Some(levels) => Efforts {
            levels,
            reported: true,
        },
        None => Efforts {
            levels: ALL_EFFORTS.to_vec(),
            reported: false,
        },
    }
}

/// The effort to keep after switching to `option`: the current one when the
/// new model supports it, else the model's default, else medium, else its
/// first supported level. With no level BRN can request, the current value
/// is kept and the model is not selectable (see [`selectable`]).
pub(super) fn effort_after_switch(current: &str, option: Option<&ModelOption>) -> String {
    let efforts = supported_efforts(option);
    if parse_effort(current).is_some_and(|effort| efforts.levels.contains(&effort)) {
        return current.to_owned();
    }
    let default = option
        .and_then(|option| option.default_effort)
        .filter(|effort| efforts.levels.contains(effort));
    default
        .or_else(|| {
            efforts
                .levels
                .contains(&ReasoningEffort::Medium)
                .then_some(ReasoningEffort::Medium)
        })
        .or_else(|| efforts.levels.first().copied())
        .map_or_else(|| current.to_owned(), |effort| effort_name(effort).into())
}

/// A model BRN can run: no reported reasoning, or at least one reported
/// level BRN can request.
pub(super) fn selectable(option: Option<&ModelOption>) -> bool {
    !supported_efforts(option).levels.is_empty()
}

/// One model row in the picker's middle column.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PickerRow {
    pub provider: &'static str,
    pub model: String,
    pub label: String,
    pub option: Option<ModelOption>,
    pub favorite: bool,
    pub current: bool,
}

impl PickerRow {
    /// The accessible name keeps provider and model identity together.
    pub(super) fn accessible_label(&self) -> String {
        let mut label = format!("{} · {}", provider_name(self.provider), self.label);
        if self.label != self.model {
            label.push_str(&format!(" ({})", self.model));
        }
        if self.current {
            label.push_str(", current");
        }
        if self.favorite {
            label.push_str(", favorite");
        }
        label
    }
}

fn matches(query: &str, model: &str, label: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || model.to_lowercase().contains(&query)
        || label.to_lowercase().contains(&query)
}

/// The middle column for `tab`, filtered by `query`. A provider whose
/// catalogue is not loaded still lists the current Settings model, so the
/// owner always sees the actual selection; nothing else is invented.
pub(super) fn picker_rows(
    picker: &ModelPicker,
    query: &str,
    current_provider: &str,
    current_model: &str,
) -> Vec<PickerRow> {
    let current_provider = provider_key(current_provider);
    let row = |provider: &'static str, model: &str, option: Option<&ModelOption>| {
        let label = option
            .and_then(|option| option.display_name.clone())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| model.to_owned());
        PickerRow {
            provider,
            model: model.to_owned(),
            label,
            option: option.cloned(),
            favorite: picker.is_favorite(provider, model),
            current: provider == current_provider && model == current_model,
        }
    };
    let mut rows = Vec::new();
    match picker.tab {
        PickerTab::Favorites => {
            for (provider, model) in &picker.favorites {
                let provider = match provider_key(provider) {
                    "copilot" => "copilot",
                    "chatgpt" => "chatgpt",
                    _ => continue,
                };
                rows.push(row(provider, model, picker.option(provider, model)));
            }
        }
        PickerTab::Provider(provider) => match picker.catalogue(provider) {
            Catalogue::Loaded(models) => {
                for option in models {
                    rows.push(row(provider, &option.id, Some(option)));
                }
            }
            _ if provider == current_provider && !current_model.trim().is_empty() => {
                rows.push(row(provider, current_model, None));
            }
            _ => {}
        },
    }
    rows.retain(|row| matches(query, &row.model, &row.label));
    rows
}

/// The closed control's accessible name: provider, model and effort.
pub(super) fn control_label(provider: &str, model: &str, effort: &str) -> String {
    match provider_key(provider) {
        "" => format!("Model {model}, effort {effort}"),
        _ => format!(
            "Model {} · {model}, effort {effort}",
            provider_name(provider)
        ),
    }
}

impl ThreadsUi {
    pub(super) fn picker_rows(&self, cx: &App) -> Vec<PickerRow> {
        let query = self.model_picker.query.read(cx).value().to_string();
        picker_rows(
            &self.model_picker,
            &query,
            &self.settings.provider,
            &self.settings.model,
        )
    }
    /// The model the picker's reasoning column describes: the highlighted
    /// row while browsing models, else the current Settings model.
    fn picker_focus_row(&self, cx: &App) -> Option<PickerRow> {
        let rows = self.picker_rows(cx);
        rows.get(self.model_picker.index)
            .cloned()
            .or_else(|| rows.into_iter().find(|row| row.current))
    }
    fn picker_efforts(&self, cx: &App) -> (Efforts, String) {
        let provider = self.settings.provider.clone();
        let model = self.settings.model.clone();
        let focus = self.picker_focus_row(cx);
        match focus {
            Some(row) if !row.current => (
                supported_efforts(row.option.as_ref()),
                effort_after_switch(&self.settings.effort, row.option.as_ref()),
            ),
            _ => (
                supported_efforts(self.model_picker.option(&provider, &model)),
                self.settings.effort.clone(),
            ),
        }
    }

    pub(super) fn open_model_picker(
        &mut self,
        anchor: Point<gpui_kit::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = provider_key(&self.settings.provider);
        let route = (
            self.settings.auth_file.clone(),
            self.settings.credentials_dir.clone(),
        );
        if self.model_picker.route != route {
            self.forget_model_catalogues();
            self.model_picker.route = route;
        }
        self.model_picker.open = true;
        self.model_picker.anchor = anchor;
        self.model_picker.section = PickerSection::Models;
        self.model_picker.tab = match current {
            "copilot" => PickerTab::Provider("copilot"),
            _ => PickerTab::Provider("chatgpt"),
        };
        self.model_picker
            .query
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.model_picker.index = self
            .picker_rows(cx)
            .iter()
            .position(|row| row.current)
            .unwrap_or(0);
        self.sync_effort_index(cx);
        self.load_model_catalogues(window, cx);
        self.model_picker
            .query
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }
    pub(super) fn close_model_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.model_picker.open {
            self.model_picker.open = false;
            self.focus_composer(window, cx);
            cx.notify();
        }
    }
    fn sync_effort_index(&mut self, cx: &App) {
        let (efforts, effort) = self.picker_efforts(cx);
        self.model_picker.effort_index = efforts
            .levels
            .iter()
            .position(|level| effort_name(*level) == effort)
            .unwrap_or(0);
    }
    pub(super) fn picker_query_changed(&mut self, cx: &mut Context<Self>) {
        self.model_picker.index = 0;
        self.sync_effort_index(cx);
        cx.notify();
    }
    pub(super) fn set_picker_tab(&mut self, tab: PickerTab, cx: &mut Context<Self>) {
        self.model_picker.tab = tab;
        self.model_picker.index = self
            .picker_rows(cx)
            .iter()
            .position(|row| row.current)
            .unwrap_or(0);
        self.sync_effort_index(cx);
        cx.notify();
    }
    /// Tab moves between providers, models and reasoning; arrows move
    /// within the active section.
    pub(super) fn cycle_picker_section(&mut self, delta: isize, cx: &mut Context<Self>) {
        const ORDER: [PickerSection; 3] = [
            PickerSection::Providers,
            PickerSection::Models,
            PickerSection::Efforts,
        ];
        let at = ORDER
            .iter()
            .position(|section| *section == self.model_picker.section)
            .unwrap_or(1) as isize;
        self.model_picker.section = ORDER[(at + delta).rem_euclid(3) as usize];
        cx.notify();
    }
    pub(super) fn move_picker(&mut self, delta: isize, cx: &mut Context<Self>) {
        match self.model_picker.section {
            PickerSection::Providers => {
                let at = PickerTab::ALL
                    .iter()
                    .position(|tab| *tab == self.model_picker.tab)
                    .unwrap_or(0) as isize;
                let tab = PickerTab::ALL[(at + delta).rem_euclid(3) as usize];
                self.set_picker_tab(tab, cx);
            }
            PickerSection::Models => {
                let count = self.picker_rows(cx).len();
                if count > 0 {
                    self.model_picker.index = (self.model_picker.index as isize + delta)
                        .rem_euclid(count as isize)
                        as usize;
                    self.sync_effort_index(cx);
                }
            }
            PickerSection::Efforts => {
                let count = self.picker_efforts(cx).0.levels.len();
                if count > 0 {
                    self.model_picker.effort_index = (self.model_picker.effort_index as isize
                        + delta)
                        .rem_euclid(count as isize)
                        as usize;
                }
            }
        }
        cx.notify();
    }
    /// Return: open a provider's models, choose a model (and move to its
    /// reasoning) or choose a reasoning level (and close).
    pub(super) fn confirm_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.model_picker.section {
            PickerSection::Providers => {
                self.model_picker.section = PickerSection::Models;
                cx.notify();
            }
            PickerSection::Models => {
                if let Some(row) = self.picker_rows(cx).get(self.model_picker.index).cloned() {
                    self.choose_picker_model(&row, window, cx);
                    if self.model_picker.open {
                        self.model_picker.section = PickerSection::Efforts;
                    }
                }
            }
            PickerSection::Efforts => {
                let levels = self.picker_efforts(cx).0.levels;
                if let Some(effort) = levels.get(self.model_picker.effort_index) {
                    self.choose_picker_effort(*effort, window, cx);
                }
            }
        }
    }
    /// Choose a model and the effort it supports, in one Settings change.
    pub(super) fn choose_picker_model(
        &mut self,
        row: &PickerRow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !selectable(row.option.as_ref()) {
            self.warn(format!(
                "{} offers no reasoning level BRN can request.",
                row.label
            ));
            cx.notify();
            return;
        }
        let previous = self.settings.effort.clone();
        let effort = effort_after_switch(&previous, row.option.as_ref());
        let mut settings = self.settings.clone();
        settings.provider = row.provider.into();
        settings.model = row.model.clone();
        settings.effort = effort.clone();
        self.configure_explicit(settings, window, cx);
        let applied = provider_key(&self.settings.provider) == row.provider
            && self.settings.model == row.model;
        if applied && effort != previous {
            self.inform(format!(
                "Reasoning set to {effort}: {} doesn't offer {previous}.",
                row.label
            ));
        }
        self.sync_effort_index(cx);
        cx.notify();
    }
    pub(super) fn choose_picker_effort(
        &mut self,
        effort: ReasoningEffort,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut settings = self.settings.clone();
        settings.effort = effort_name(effort).into();
        self.configure_explicit(settings, window, cx);
        self.close_model_picker(window, cx);
    }
    pub(super) fn toggle_favorite_model(
        &mut self,
        provider: &'static str,
        model: &str,
        cx: &mut Context<Self>,
    ) {
        let mut favorites = self.model_picker.favorites.clone();
        if let Some(at) = favorites
            .iter()
            .position(|(p, m)| p == provider && m == model)
        {
            favorites.remove(at);
        } else {
            favorites.push((provider.to_owned(), model.to_owned()));
        }
        match self.workspace.save_favorite_models(&favorites) {
            Ok(()) => self.model_picker.favorites = favorites,
            Err(error) => self.fail(format!("Could not save Favorites. {error}")),
        }
        let count = self.picker_rows(cx).len();
        self.model_picker.index = self.model_picker.index.min(count.saturating_sub(1));
        cx.notify();
    }

    /// Learn each connected provider's catalogue off the GUI thread. A
    /// provider without credentials stays NotLoaded and the picker says how
    /// to connect it; nothing is fetched while the panel is closed.
    pub(super) fn load_model_catalogues(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for provider in ["chatgpt", "copilot"] {
            if matches!(
                self.model_picker.catalogue(provider),
                Catalogue::Loading(_) | Catalogue::Loaded(_)
            ) {
                continue;
            }
            let auth_file = self.settings.auth_file.clone().map(PathBuf::from);
            let credentials = self.settings.credentials_dir.clone().map(PathBuf::from);
            let route = match provider {
                "chatgpt" if auth_file.is_some() => auth_file.map(|file| (true, file)),
                _ => credentials.map(|dir| (false, dir)),
            };
            let Some((codex_file, path)) = route else {
                continue;
            };
            self.model_picker.generation += 1;
            let generation = self.model_picker.generation;
            *self.model_picker.catalogue_mut(provider) = Catalogue::Loading(generation);
            let work = cx.background_executor().spawn(async move {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                let cancel = CancellationToken::new();
                runtime.block_on(async move {
                    let result = if codex_file {
                        brn_ai::codex_auth::codex_models(&path, cancel).await
                    } else {
                        let provider = if provider == "copilot" {
                            brn_ai::Provider::Copilot
                        } else {
                            brn_ai::Provider::Chatgpt
                        };
                        match brn_ai::Auth::open(&path) {
                            Ok(auth) => auth.models(provider, cancel).await,
                            Err(error) => Err(error),
                        }
                    };
                    result.map_err(|e| e.to_string())
                })
            });
            cx.spawn_in(window, async move |this, cx| {
                let result = work.await;
                let _ = this.update_in(cx, |this, _, cx| {
                    this.finish_model_catalogue(provider, generation, result, cx)
                });
            })
            .detach();
        }
    }
    pub(super) fn finish_model_catalogue(
        &mut self,
        provider: &'static str,
        generation: u64,
        result: Result<Vec<ModelOption>, String>,
        cx: &mut Context<Self>,
    ) {
        if *self.model_picker.catalogue(provider) != Catalogue::Loading(generation) {
            return;
        }
        *self.model_picker.catalogue_mut(provider) = match result {
            Ok(models) => {
                if provider == "copilot" {
                    // Runs check the selection against what Copilot offered.
                    self.copilot_models = models.iter().map(|m| m.id.clone()).collect();
                }
                Catalogue::Loaded(models)
            }
            Err(error) => Catalogue::Failed(error),
        };
        if self.model_picker.open {
            let rows = self.picker_rows(cx);
            self.model_picker.index = rows.iter().position(|row| row.current).unwrap_or(0);
            self.sync_effort_index(cx);
        }
        cx.notify();
    }
    /// Credentials changed: catalogues learned through old credentials no
    /// longer describe the current route.
    pub(super) fn forget_model_catalogues(&mut self) {
        self.model_picker.chatgpt = Catalogue::NotLoaded;
        self.model_picker.copilot = Catalogue::NotLoaded;
    }

    /// The closed control: provider indicator, short model name, effort and
    /// a caret.
    pub(super) fn model_control(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let provider = self.settings.provider.clone();
        let model = self.settings.model.clone();
        let effort = self.settings.effort.clone();
        let short = self
            .model_picker
            .option(&provider, &model)
            .and_then(|option| option.display_name.clone())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| model.clone());
        let narrow = self.chat_narrow;
        button("model-pill")
            .compact()
            .ghost()
            .h(tokens::COMPACT_CONTROL)
            .flex_shrink(1.)
            .min_w_0()
            .max_w(px(if narrow { 150. } else { 260. }))
            .dropdown_caret(true)
            .when(!provider_key(&provider).is_empty(), |control| {
                control.icon(provider_icon(&provider))
            })
            .accessibility_label(control_label(&provider, &model, &effort))
            .tooltip(format!(
                "{} · {model} · {effort} reasoning. Choose provider, model and reasoning",
                provider_name(&provider)
            ))
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .child(div().min_w_0().truncate().child(short))
                    .child(
                        div()
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_color(rgb(pal().muted))
                            .child(format!("\u{a0}· {effort}")),
                    ),
            )
            .on_click(
                cx.listener(|this, event: &gpui_kit::ClickEvent, window, cx| {
                    if this.model_picker.open {
                        this.close_model_picker(window, cx);
                    } else {
                        this.open_model_picker(event.position(), window, cx);
                    }
                }),
            )
            .into_any_element()
    }

    pub(super) fn model_picker_overlay(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let p = pal();
        let viewport = window.viewport_size();
        let stacked = viewport.width < px(PICKER_STACKED);
        let width = px(if stacked { 360. } else { 600. }).min(viewport.width - px(16.));
        let anchor = self.model_picker.anchor;
        let left = (anchor.x - width / 2.)
            .max(px(8.))
            .min(viewport.width - width - px(8.));
        let bottom = (viewport.height - anchor.y + px(20.)).max(px(8.));
        let max_height = (viewport.height - bottom - px(16.)).max(px(200.));
        let busy = self.worker.is_some();
        let section = self.model_picker.section;
        let rows = self.picker_rows(cx);
        let (efforts, effort) = self.picker_efforts(cx);
        let focus = self.picker_focus_row(cx);

        let highlight = |selected: bool, active: bool| {
            move |row: gpui_kit::Stateful<gpui_kit::Div>| {
                row.when(selected && active, |row| row.bg(rgb(pal().selection)))
                    .when(selected && !active, |row| row.bg(rgb(pal().hover)))
                    .when(!selected, |row| {
                        row.hover(|style| style.bg(rgb(pal().hover)))
                    })
            }
        };
        let heading = |text: &'static str| {
            div()
                .px_2()
                .pb_1()
                .text_size(px(theme::metrics::TEXT_METADATA))
                .text_color(rgb(pal().muted))
                .child(text)
        };

        let mut providers = div()
            .id("model-providers")
            .role(gpui_kit::Role::ListBox)
            .aria_label("Providers")
            .test_support()
            .flex()
            .gap_1()
            .when(stacked, |list| list.flex_row().flex_wrap())
            .when(!stacked, |list| list.flex_col().w(px(160.)).flex_shrink_0());
        for tab in PickerTab::ALL {
            let selected = tab == self.model_picker.tab;
            let status = match tab {
                PickerTab::Favorites => None,
                PickerTab::Provider(provider) => match self.model_picker.catalogue(provider) {
                    Catalogue::Loaded(models) => Some(format!("{}", models.len())),
                    _ => None,
                },
            };
            providers = providers.child(
                highlight(selected, section == PickerSection::Providers)(
                    div()
                        .id(tab.id())
                        .flex()
                        .items_center()
                        .gap_2()
                        .h(px(theme::metrics::SIDEBAR_ROW))
                        .px_2()
                        .rounded(px(theme::metrics::RADIUS_CONTROL))
                        .cursor_pointer()
                        .text_color(rgb(p.ink))
                        .role(gpui_kit::Role::ListBoxOption)
                        .aria_label(tab.label())
                        .aria_selected(selected),
                )
                .child(
                    Icon::new(tab.icon())
                        .with_size(Size::Small)
                        .flex_shrink_0()
                        .text_color(rgb(p.muted)),
                )
                .child(div().flex_1().min_w_0().truncate().child(tab.label()))
                .children(status.map(|count| {
                    div()
                        .flex_shrink_0()
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .child(count)
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.model_picker.section = PickerSection::Models;
                    this.set_picker_tab(tab, cx)
                }))
                .test_support(),
            );
        }

        let mut models = div()
            .id("model-list")
            .role(gpui_kit::Role::ListBox)
            .aria_label("Models")
            .test_support()
            .flex()
            .flex_col()
            .gap_px()
            .flex_1()
            .min_w_0()
            .max_h(px(if stacked { 220. } else { 300. }).min(max_height - px(120.)))
            .overflow_y_scroll();
        let status: Option<String> = match self.model_picker.tab {
            PickerTab::Favorites if self.model_picker.favorites.is_empty() => {
                Some("Star a model to keep it here with its provider.".into())
            }
            PickerTab::Favorites => None,
            PickerTab::Provider(provider) => {
                let connected = match provider {
                    "chatgpt" => {
                        self.settings.auth_file.is_some() || self.settings.credentials_dir.is_some()
                    }
                    _ => self.settings.credentials_dir.is_some(),
                };
                match self.model_picker.catalogue(provider) {
                    Catalogue::Loading(_) => Some("Loading models…".into()),
                    Catalogue::Failed(error) => {
                        Some(format!("Could not load models. {}", one_line(error)))
                    }
                    Catalogue::Loaded(models) if models.is_empty() => {
                        Some("This account offers no models.".into())
                    }
                    Catalogue::NotLoaded if !connected => Some(format!(
                        "{} isn't connected. Connect it in Settings › Models.",
                        provider_name(provider)
                    )),
                    _ => None,
                }
            }
        };
        if let Some(status) = status {
            models = models.child(
                div()
                    .id("model-status")
                    .px_2()
                    .py_1()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .aria_label(status.clone())
                    .test_support()
                    .child(status),
            );
        }
        if busy {
            models = models.child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .child("Available when the current work finishes"),
            );
        }
        for (index, row) in rows.iter().enumerate() {
            let selected = index == self.model_picker.index;
            let disabled = busy || !selectable(row.option.as_ref());
            let show_provider = self.model_picker.tab == PickerTab::Favorites;
            let choose = row.clone();
            let provider = row.provider;
            let model = row.model.clone();
            let star_label = if row.favorite {
                format!("Remove {} from Favorites", row.label)
            } else {
                format!("Add {} to Favorites", row.label)
            };
            models = models.child(
                highlight(selected, section == PickerSection::Models)(
                    div()
                        .id(format!("model-row-{index}"))
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .gap_2()
                        .h(px(theme::metrics::SIDEBAR_ROW))
                        .px_2()
                        .rounded(px(theme::metrics::RADIUS_CONTROL))
                        .text_color(rgb(if disabled { p.muted } else { p.ink }))
                        .when(!disabled, |row| row.cursor_pointer())
                        .role(gpui_kit::Role::ListBoxOption)
                        .aria_label(row.accessible_label())
                        .aria_selected(selected),
                )
                .child(
                    div()
                        .w(px(14.))
                        .flex_shrink_0()
                        .children(row.current.then(|| {
                            Icon::new(IconName::Check)
                                .with_size(Size::Small)
                                .text_color(rgb(p.accent))
                        })),
                )
                .when(show_provider, |line| {
                    line.child(
                        Icon::new(provider_icon(row.provider))
                            .with_size(Size::Small)
                            .flex_shrink_0()
                            .text_color(rgb(p.muted)),
                    )
                })
                .child(div().flex_1().min_w_0().truncate().child(row.label.clone()))
                .when(row.label != row.model, |line| {
                    line.child(
                        div()
                            .flex_shrink(1.)
                            .min_w_0()
                            .max_w(gpui_kit::relative(0.4))
                            .truncate()
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .child(row.model.clone()),
                    )
                })
                .child(
                    components::icon_button(
                        format!("model-favorite-{index}"),
                        if row.favorite {
                            IconName::StarFill
                        } else {
                            IconName::Star
                        },
                        star_label,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_favorite_model(provider, &model, cx)
                    })),
                )
                .when(!disabled, |line| {
                    line.on_click(cx.listener(move |this, _, window, cx| {
                        this.model_picker.index = index;
                        this.choose_picker_model(&choose, window, cx)
                    }))
                })
                .test_support(),
            );
        }
        if rows.is_empty() && !self.model_picker.query.read(cx).value().trim().is_empty() {
            models = models.child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .child("No model matches."),
            );
        }

        let route = focus
            .as_ref()
            .map(|row| format!("{} · {}", provider_name(row.provider), row.label));
        let mut reasoning = div()
            .id("model-efforts")
            .role(gpui_kit::Role::ListBox)
            .aria_label("Reasoning")
            .test_support()
            .flex()
            .gap_1()
            .when(stacked, |list| list.flex_row().flex_wrap().items_center())
            .when(!stacked, |list| list.flex_col().w(px(150.)).flex_shrink_0());
        for (index, level) in efforts.levels.iter().enumerate() {
            let name = effort_name(*level);
            let selected = name == effort;
            let active =
                section == PickerSection::Efforts && index == self.model_picker.effort_index;
            let level = *level;
            reasoning = reasoning.child(
                highlight(
                    active || (selected && section != PickerSection::Efforts),
                    active,
                )(
                    div()
                        .id(format!("model-effort-{name}"))
                        .flex()
                        .items_center()
                        .gap_2()
                        .h(px(theme::metrics::SIDEBAR_ROW))
                        .px_2()
                        .rounded(px(theme::metrics::RADIUS_CONTROL))
                        .text_color(rgb(if busy { p.muted } else { p.ink }))
                        .when(!busy, |row| row.cursor_pointer())
                        .role(gpui_kit::Role::ListBoxOption)
                        .aria_label(if selected {
                            format!("{name}, current")
                        } else {
                            name.to_owned()
                        })
                        .aria_selected(active),
                )
                .child(div().w(px(14.)).flex_shrink_0().children(selected.then(|| {
                    Icon::new(IconName::Check)
                        .with_size(Size::Small)
                        .text_color(rgb(p.accent))
                })))
                .child(name)
                .when(!busy, |row| {
                    row.on_click(cx.listener(move |this, _, window, cx| {
                        let focus = this.picker_focus_row(cx);
                        if let Some(row) = focus.filter(|row| !row.current) {
                            this.choose_picker_model(&row, window, cx);
                        }
                        this.choose_picker_effort(level, window, cx)
                    }))
                })
                .test_support(),
            );
        }
        let note = if efforts.levels.is_empty() {
            Some("No reasoning level BRN can request.".to_owned())
        } else if !efforts.reported {
            Some(match focus.as_ref().map(|row| row.provider) {
                Some("copilot") => "GitHub Copilot doesn't report reasoning levels; BRN requests the one you choose.".to_owned(),
                _ => "Reasoning levels appear once this provider's models load.".to_owned(),
            })
        } else {
            None
        };

        let columns = div()
            .flex()
            .gap_3()
            .min_h_0()
            .when(stacked, |columns| columns.flex_col())
            .child(providers)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap_2()
                    .child(Input::new(&self.model_picker.query).w_full())
                    .child(models),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .when(!stacked, |column| column.w(px(150.)).flex_shrink_0())
                    .child(heading("Reasoning"))
                    .children(route.map(|route| {
                        div()
                            .id("model-route")
                            .px_2()
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .truncate()
                            .aria_label(route.clone())
                            .test_support()
                            .child(route)
                    }))
                    .child(reasoning)
                    .children(note.map(|note| {
                        div()
                            .id("model-effort-note")
                            .px_2()
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .aria_label(note.clone())
                            .test_support()
                            .child(note)
                    })),
            );

        div()
            .id("model-picker-scrim")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_model_picker(window, cx)),
            )
            .child(
                div()
                    .id("model-picker")
                    .key_context("BrnModelPicker")
                    .role(gpui_kit::Role::Dialog)
                    .aria_label("Choose provider, model and reasoning")
                    .absolute()
                    .left(left)
                    .bottom(bottom)
                    .w(width)
                    .max_h(max_height)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .bg(rgb(p.surface_raised))
                    .border_1()
                    .border_color(rgb(p.border))
                    .rounded(px(theme::metrics::RADIUS_CARD))
                    .shadow_lg()
                    .text_color(rgb(p.ink))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    // The search field owns these keys; the picker claims them first.
                    .capture_key_down(cx.listener(
                        |this, event: &gpui_kit::KeyDownEvent, window, cx| {
                            let shift = event.keystroke.modifiers.shift;
                            match event.keystroke.key.as_str() {
                                "escape" => this.close_model_picker(window, cx),
                                "down" => this.move_picker(1, cx),
                                "up" => this.move_picker(-1, cx),
                                "tab" => this.cycle_picker_section(if shift { -1 } else { 1 }, cx),
                                "enter" => this.confirm_picker(window, cx),
                                _ => return,
                            }
                            cx.stop_propagation();
                        },
                    ))
                    .child(columns)
                    .test_support(),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(id: &str, efforts: Option<Vec<ReasoningEffort>>) -> ModelOption {
        let mut option = ModelOption::bare(id);
        option.efforts = efforts;
        option
    }

    #[test]
    fn a_model_switch_keeps_only_a_supported_effort() {
        use ReasoningEffort::*;
        let both = option("a", Some(vec![Low, Medium, High]));
        assert_eq!(effort_after_switch("high", Some(&both)), "high");
        let mut low_only = option("b", Some(vec![Low, Medium]));
        assert_eq!(effort_after_switch("high", Some(&low_only)), "medium");
        low_only.default_effort = Some(Low);
        assert_eq!(effort_after_switch("high", Some(&low_only)), "low");
        let high_only = option("c", Some(vec![High]));
        assert_eq!(effort_after_switch("low", Some(&high_only)), "high");
        // Unreported reasoning keeps the requested level.
        assert_eq!(effort_after_switch("high", None), "high");
        assert_eq!(
            effort_after_switch("high", Some(&option("d", None))),
            "high"
        );
        // Nothing BRN can request: not selectable, the value is untouched.
        let none = option("e", Some(vec![]));
        assert!(!selectable(Some(&none)));
        assert_eq!(effort_after_switch("high", Some(&none)), "high");
    }

    #[test]
    fn the_control_label_keeps_provider_model_and_effort_together() {
        assert_eq!(
            control_label("codex", "gpt-6.1-sol", "low"),
            "Model ChatGPT · gpt-6.1-sol, effort low"
        );
        assert_eq!(
            control_label("", "local-model", "high"),
            "Model local-model, effort high"
        );
    }
}
