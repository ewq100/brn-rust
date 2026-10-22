//! Projects navigation and owner dialogs over shared checked operations.
use super::shell::{item, pill};
use super::tiles::{PAGE_WIDTH, column, page, row};
use super::*;
use gpui_kit::SharedString;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::PopupMenu;

/// A dialog pick-list row: a full-width ghost button, so Tab reaches it and
/// Space chooses it. The title truncates; the full text is its name.
fn pick_row(id: impl Into<gpui_kit::ElementId>, label: impl Into<SharedString>) -> Button {
    let label = label.into();
    button(id)
        .ghost()
        .w_full()
        .h(components::tokens::COMPACT_CONTROL)
        .accessibility_label(label.clone())
        .tooltip(label.clone())
        .child(div().flex_1().min_w_0().truncate().child(label))
}

/// Shown beside Restore project, before the owner chooses it.
const RESTORE_LINE: &str = "Restoring returns contents and settings. Cancelled tasks stay cancelled; tasks kept outside stay outside. Work stays paused.";

/// The Projects page's one filled primary: New project with nothing
/// selected, New chat for an active project, Restore project in Trash, and
/// none for Trash with nothing selected.
pub(super) fn projects_primary(trash: bool, selected: bool) -> Option<&'static str> {
    match (trash, selected) {
        (false, false) => Some("project-new"),
        (false, true) => Some("project-chat"),
        (true, true) => Some("project-restore"),
        (true, false) => None,
    }
}

/// The delete preview's line for removed items not listed by title, naming
/// only the kinds present; `None` when there are none.
pub(super) fn other_items_line(removed: &[Record]) -> Option<String> {
    const KINDS: [&str; 8] = [
        "messages",
        "comments",
        "links",
        "sources",
        "figures",
        "runs",
        "chat controls",
        "settings",
    ];
    let kind = |data: &RecordData| match data {
        RecordData::Project(_)
        | RecordData::Thread(_)
        | RecordData::Note(_)
        | RecordData::Action(_) => None,
        RecordData::Message(_) => Some(0),
        RecordData::Comment(_) => Some(1),
        RecordData::Link(_) => Some(2),
        RecordData::Source(_) => Some(3),
        RecordData::Asset(_) => Some(4),
        RecordData::Run(_) => Some(5),
        RecordData::ChatControl(_) => Some(6),
        RecordData::Settings(_) => Some(7),
        RecordData::WritingSetup(_) => None,
    };
    let found: Vec<usize> = removed.iter().filter_map(|r| kind(&r.data)).collect();
    if found.is_empty() {
        return None;
    }
    let present: Vec<&str> = (0..KINDS.len())
        .filter(|index| found.contains(index))
        .map(|index| KINDS[index])
        .collect();
    let count = found.len();
    Some(format!(
        "Also moves {count} other item{} ({}).",
        if count == 1 { "" } else { "s" },
        present.join(", ")
    ))
}

/// Records listed by title in the delete preview.
fn listed(record: &Record) -> bool {
    matches!(
        record.data,
        RecordData::Project(_)
            | RecordData::Thread(_)
            | RecordData::Note(_)
            | RecordData::Action(_)
    )
}

impl ThreadsUi {
    pub(super) fn open_project_note(
        &mut self,
        project: &str,
        note: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_note_in_context(note, None, Some(project), window, cx);
        if self.open_note_id() != Some(note) || self.note_writing_scope.as_deref() != Some(project)
        {
            return;
        }
        let target = self
            .writing_threads
            .get(note)
            .cloned()
            .map_or_else(|| ChatTarget::NewForNote(note.into()), ChatTarget::Thread);
        self.set_chat(target, window, cx);
    }
    pub(super) fn apply_project(
        &mut self,
        command: ProjectCommand,
        cx: &mut Context<Self>,
    ) -> AppResult<Receipt> {
        let command = if let Some(run) = self
            .worker
            .as_ref()
            .and_then(Worker::run_identity)
            .filter(|_| !self.stream.trim().is_empty())
        {
            ProjectCommand::WithProgress {
                command: Box::new(command),
                run: run.run_id.clone(),
                partial: self.stream.clone(),
            }
        } else {
            command
        };
        let receipt = self
            .workspace
            .project_command(&format!("ui:{}", uuid::Uuid::new_v4()), command)?;
        if let Some(worker) = self.worker.as_mut()
            && let Some(run) = worker.run_identity()
            && receipt.writes.iter().any(|w| {
                w.after.id == run.run_id
                    && matches!(&w.after.data, RecordData::Run(r) if r.state != RunState::Working)
            })
        {
            worker.cancel.cancel();
            worker.discard_continuation();
        }
        // A returned receipt is an applied change, like ApplyOutcome::Applied.
        self.confirm("Project saved.");
        self.refresh();
        self.refresh_connections(cx);
        cx.notify();
        Ok(receipt)
    }
    /// The selected project in the current view (Active or Trash).
    fn shown_project(&self) -> Option<&Record> {
        let id = self.selected_project.as_ref()?;
        self.records.iter().find(|r| {
            &r.id == id
                && r.archived == self.projects_trash
                && matches!(r.data, RecordData::Project(_))
        })
    }
    /// The Projects ⋯'s first section: the selected active project's
    /// actions, Move to Trash… last after a separator.
    pub(super) fn projects_menu_section(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<Box<dyn Fn(PopupMenu) -> PopupMenu + 'static>> {
        let id = self.shown_project().filter(|r| !r.archived)?.id.clone();
        let this = cx.entity().downgrade();
        let busy = self.worker.is_some();
        Some(Box::new(move |menu: PopupMenu| {
            let (settings, note, import, link, delete) =
                (id.clone(), id.clone(), id.clone(), id.clone(), id.clone());
            menu.label("Project")
                .item(item(
                    &this,
                    "Instructions & settings",
                    None,
                    false,
                    false,
                    move |view, window, cx| view.project_settings_dialog(&settings, window, cx),
                ))
                .item(item(
                    &this,
                    "New document",
                    None,
                    false,
                    false,
                    move |view, window, cx| {
                        view.project_name_dialog(Some(note.clone()), false, window, cx)
                    },
                ))
                .item(item(
                    &this,
                    if busy {
                        "Import file… · wait for current work"
                    } else {
                        "Import file…"
                    },
                    None,
                    busy,
                    false,
                    move |view, window, cx| {
                        view.choose_import(
                            ImportIntent::FullNote,
                            Some(import.clone()),
                            window,
                            cx,
                        );
                    },
                ))
                .item(item(
                    &this,
                    "Link document…",
                    None,
                    false,
                    false,
                    move |view, window, cx| view.project_link_dialog(&link, window, cx),
                ))
                .separator()
                .item(item(
                    &this,
                    "Move to Trash…",
                    None,
                    false,
                    false,
                    move |view, window, cx| view.project_delete_dialog(&delete, window, cx),
                ))
        }))
    }
    pub(super) fn projects_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let trash = self.projects_trash;
        let shown = self.shown_project();
        let primary = projects_primary(trash, shown.is_some());
        let scope = |id: &'static str, label: &'static str, to_trash: bool| {
            button(id)
                .label(label)
                .selected(trash == to_trash)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.projects_trash = to_trash;
                    this.selected_project = None;
                    cx.notify();
                }))
        };
        let mut bar = components::toolbar("projects-bar")
            .child(components::segmented(
                "project-scope",
                vec![
                    scope("project-active", "Active", false),
                    scope("project-trash", "Trash", true),
                ],
            ))
            .child(div().flex_1());
        if !trash {
            let new = button("project-new")
                .label("New project")
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.project_name_dialog(None, false, window, cx)
                }));
            bar = bar.child(if primary == Some("project-new") {
                new.primary()
            } else {
                new.ghost()
            });
        }
        let projects: Vec<&Record> = self
            .records
            .iter()
            .filter(|r| r.archived == trash && matches!(r.data, RecordData::Project(_)))
            .collect();
        let mut body = column();
        let mut list = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(components::section_header(
                if trash { "In Trash" } else { "Projects" },
                Some(projects.len()),
                None,
            ));
        if projects.is_empty() {
            list = list.child(components::empty_state(
                "projects-empty",
                if trash {
                    IconName::Trash
                } else {
                    IconName::Folder
                },
                if trash {
                    "Trash is empty"
                } else {
                    "No projects yet"
                },
                if trash {
                    "Deleted projects stay recoverable here."
                } else {
                    "Group related chats and documents with shared context."
                },
                None,
            ));
        }
        for record in &projects {
            let id = record.id.clone();
            list = list.child(
                row(
                    format!("project-row-{id}"),
                    title(record),
                    "",
                    self.selected_project.as_ref() == Some(&id),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected_project = Some(id.clone());
                    cx.notify();
                })),
            );
        }
        if trash && shown.is_none() && !projects.is_empty() {
            list = list.child(
                metadata("Deleted projects stay recoverable here.")
                    .id("projects-trash-hint")
                    .px_2()
                    .pt_1()
                    .test_support(),
            );
        }
        body = body.child(list);
        // The page column, as on Home, Tasks and Changes.
        let frame = |body: gpui_kit::Div| {
            div()
                .size_full()
                .flex()
                .flex_col()
                .min_h_0()
                .child(bar)
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .child(page("projects-tile", PAGE_WIDTH, body).test_support()),
                )
                .into_any_element()
        };
        let Some(record) = shown else {
            return frame(body);
        };
        let RecordData::Project(p) = &record.data else {
            return frame(body);
        };
        let id = record.id.clone();
        let version = record.version;
        let heading = div().flex().items_center().gap_2().pt_6().min_w_0().child(
            div()
                .id("project-title")
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(TITLE_TEXT)
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .child(p.title.clone()),
        );
        if record.archived {
            return frame(
                body.child(heading)
                    .child(
                        metadata(RESTORE_LINE)
                            .id("project-restore-line")
                            .aria_label(RESTORE_LINE)
                            .pt_1()
                            .test_support(),
                    )
                    .child(
                        div().pt_3().child(
                            button("project-restore")
                                .label("Restore project")
                                .primary()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Err(e) = this.apply_project(
                                        ProjectCommand::Restore {
                                            project: id.clone(),
                                            version,
                                        },
                                        cx,
                                    ) {
                                        this.fail(e.to_string());
                                        cx.notify();
                                    } else {
                                        this.projects_trash = false;
                                    }
                                })),
                        ),
                    ),
            );
        }
        let chat_id = id.clone();
        body = body.child(
            heading.child(
                button("project-chat")
                    .label("New chat")
                    .primary()
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .tooltip("Start a chat that uses this project's instructions and documents")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.project_name_dialog(Some(chat_id.clone()), true, window, cx)
                    })),
            ),
        );
        let status = format!(
            "BRN knowledge: {} · {}",
            if p.use_brn_knowledge { "on" } else { "off" },
            if p.instructions.is_empty() {
                "No project instructions"
            } else {
                "Project instructions apply to every chat"
            }
        );
        body = body.child(
            metadata(status.clone())
                .id("project-status")
                .aria_label(status)
                .pt_1()
                .test_support(),
        );
        let docs = project_document_ids(&self.records, record);
        let documents: Vec<&Record> = self
            .records
            .iter()
            .filter(|r| !r.archived && docs.contains(&r.id))
            .collect();
        body = body.child(components::section_header(
            "Documents",
            Some(documents.len()),
            None,
        ));
        if documents.is_empty() {
            body = body.child(
                metadata("Add one with New document or Link document… in the Projects ⋯ menu.")
                    .px_2(),
            );
        }
        for note in documents {
            let id = note.id.clone();
            let unlink = id.clone();
            let keep = id.clone();
            let project = record.id.clone();
            let project_for_open = project.clone();
            let version = note.version;
            let mut line = div().flex().items_center().gap_1().child(
                row(format!("project-doc-{id}"), title(note), "", false)
                    .flex_1()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_project_note(&project_for_open, &id, window, cx)
                    })),
            );
            if matches!(&note.data, RecordData::Note(n) if n.chat.is_some()) {
                line = line.child(
                    pill(format!("project-keep-{keep}"), "Keep in BRN").on_click(cx.listener(
                        move |this, _, _, cx| {
                            match this.workspace.keep_in_brn(
                                &format!("ui:{}", uuid::Uuid::new_v4()),
                                &keep,
                                version,
                            ) {
                                Ok(outcome) => this.report_outcome(&outcome),
                                Err(e) => this.fail(e.to_string()),
                            };
                            this.refresh();
                            cx.notify();
                        },
                    )),
                );
            }
            if p.documents.contains(&unlink) {
                line = line.child(pill(format!("project-unlink-{unlink}"), "Unlink").on_click(
                    cx.listener(move |this, _, _, cx| {
                        if let Err(e) = this.apply_project(
                            ProjectCommand::UnlinkDocument {
                                project: project.clone(),
                                note: unlink.clone(),
                            },
                            cx,
                        ) {
                            this.fail(e.to_string());
                            cx.notify();
                        }
                    }),
                ));
            }
            body = body.child(line);
        }
        let chats: Vec<&Record> = self
            .records
            .iter()
            .filter(|r| p.chats.contains(&r.id))
            .collect();
        body = body.child(components::section_header("Chats", Some(chats.len()), None));
        if chats.is_empty() {
            body = body.child(metadata("Start one with New chat.").px_2());
        }
        for chat in chats {
            let id = chat.id.clone();
            body = body.child(
                row(
                    format!("project-chat-{id}"),
                    title(chat),
                    if chat.archived { "Archived" } else { "" },
                    false,
                )
                .on_click(
                    cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)),
                ),
            );
        }
        frame(body)
    }
    pub(super) fn project_name_dialog(
        &mut self,
        project: Option<String>,
        chat: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Title"));
        let target = cx.entity().downgrade();
        let error = cx.new(|_| String::new());
        // One field: ↵ anywhere in the dialog creates, through the same
        // function as Create; a refusal keeps the dialog open with its reason.
        let create: components::DialogAction = {
            let (input, error, project) = (input.clone(), error.clone(), project.clone());
            std::rc::Rc::new(move |window, cx| {
                let name = input.read(cx).value().to_string();
                let command = match &project {
                    None => ProjectCommand::Create { title: name },
                    Some(p) if chat => ProjectCommand::CreateChat {
                        project: p.clone(),
                        title: name,
                    },
                    Some(p) => ProjectCommand::CreateDocument {
                        project: p.clone(),
                        title: name,
                        markdown: String::new(),
                    },
                };
                let mut result = Err("This window is unavailable.".to_owned());
                let _ = target.update(cx, |this, cx| {
                    result = this
                        .apply_project(command, cx)
                        .map(|r| {
                            for w in r.writes {
                                match &w.after.data {
                                    RecordData::Project(_) => {
                                        this.selected_project = Some(w.after.id.clone())
                                    }
                                    RecordData::Thread(_) if w.before.is_none() => {
                                        this.open_thread(&w.after.id, window, cx)
                                    }
                                    RecordData::Note(_) if w.before.is_none() => {
                                        if let Some(p) = &project {
                                            this.open_project_note(p, &w.after.id, window, cx);
                                        } else {
                                            this.open_note(&w.after.id, None, window, cx);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        })
                        .map_err(|e| e.to_string());
                });
                match result {
                    Ok(()) => window.close_dialog(cx),
                    Err(e) => error.update(cx, |s, cx| {
                        *s = e;
                        cx.notify();
                    }),
                }
            })
        };
        let field = input.clone();
        self.open_dialog(window, cx, move |dialog, window, cx| {
            let (ok, click) = (create.clone(), create.clone());
            dialog
                .title(if project.is_none() {
                    "New project"
                } else if chat {
                    "New project chat"
                } else {
                    "New project document"
                })
                .on_ok(move |_, window, cx| {
                    ok(window, cx);
                    false
                })
                .child(
                    components::dialog_body("project-name-body", window)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(Input::new(&field).id("project-name-input"))
                                .child(components::field_error(
                                    "project-name-error",
                                    error.read(cx).clone(),
                                )),
                        )
                        .test_support(),
                )
                .footer(
                    components::dialog_footer()
                        .child(
                            button("project-name-cancel")
                                .label("Cancel")
                                .outline()
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            button("project-name-submit")
                                .label("Create")
                                .primary()
                                .on_click(move |_, window, cx| click(window, cx)),
                        ),
                )
        });
        input.update(cx, |input, cx| input.focus(window, cx));
    }
    pub(super) fn project_settings_dialog(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(r) = self.records.iter().find(|r| r.id == id).cloned() else {
            return;
        };
        let RecordData::Project(p) = r.data else {
            return;
        };
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Project title"));
        name.update(cx, |i, cx| i.set_value(&p.title, window, cx));
        let instructions = cx.new(|cx| {
            components::prose_editor_state(window, cx)
                .placeholder("Instructions for every chat in this project")
        });
        instructions.update(cx, |i, cx| i.set_value(&p.instructions, window, cx));
        let knowledge = cx.new(|_| p.use_brn_knowledge);
        let error = cx.new(|_| String::new());
        let target = cx.entity().downgrade();
        // A multi-line form: ↵ adds a line in the instructions; ⌘↵ saves.
        let save: components::DialogAction = {
            let (name, instructions, knowledge, error, id) = (
                name.clone(),
                instructions.clone(),
                knowledge.clone(),
                error.clone(),
                r.id.clone(),
            );
            std::rc::Rc::new(move |window, cx| {
                let command = ProjectCommand::Configure {
                    project: id.clone(),
                    version: r.version,
                    title: name.read(cx).value().to_string(),
                    instructions: instructions.read(cx).value().to_string(),
                    use_brn_knowledge: *knowledge.read(cx),
                };
                let mut result = Err("This window is unavailable.".to_owned());
                let _ = target.update(cx, |this, cx| {
                    result = this
                        .apply_project(command, cx)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                });
                match result {
                    Ok(()) => window.close_dialog(cx),
                    Err(e) => error.update(cx, |s, cx| {
                        *s = e;
                        cx.notify();
                    }),
                }
            })
        };
        let field = name.clone();
        self.open_dialog(window, cx, move |dialog, window, cx| {
            let toggle = knowledge.clone();
            let on = *knowledge.read(cx);
            dialog
                .title("Project instructions & settings")
                .width(px(620.))
                .on_ok(|_, _, _| false)
                .child(
                    components::dialog_body("project-settings-body", window).child(
                        components::form_keys(
                            div().flex().flex_col().gap_3(),
                            save.clone(),
                        )
                        .child(Input::new(&name).id("project-settings-title"))
                        .child(
                            div()
                                .id("project-instructions")
                                .h(px(180.))
                                .child(
                                    prose_editor(&instructions)
                                        .h_full()
                                        .aria_label("Project instructions"),
                                )
                                .test_support(),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div().child(
                                        pill(
                                            "project-knowledge-toggle",
                                            if on {
                                                "Use BRN knowledge: on"
                                            } else {
                                                "Use BRN knowledge: off"
                                            },
                                        )
                                        .selected(on)
                                        .on_click(move |_, _, cx| {
                                            toggle.update(cx, |v, cx| {
                                                *v = !*v;
                                                cx.notify();
                                            })
                                        }),
                                    ),
                                )
                                .child(metadata(
                                    "Off stops new lookups in the rest of BRN; linked documents and saved project discussion stay usable. Instructions grant no extra write authority.",
                                )),
                        )
                        .child(components::field_error(
                            "project-settings-error",
                            error.read(cx).clone(),
                        )),
                    ).test_support(),
                )
                .footer(
                    components::form_keys(components::dialog_footer(), save.clone())
                        .child(
                            button("project-settings-cancel")
                                .label("Cancel")
                                .outline()
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child({
                            let save = save.clone();
                            button("project-settings-save")
                                .label("Save")
                                .primary()
                                .tooltip("Save (⌘↵)")
                                .on_click(move |_, window, cx| save(window, cx))
                        }),
                )
        });
        field.update(cx, |input, cx| input.focus(window, cx));
    }
    pub(super) fn move_chat_dialog(
        &mut self,
        chat: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let choices: Vec<_> = std::iter::once((None, "Standalone chat".to_owned()))
            .chain(
                self.records
                    .iter()
                    .filter(|r| !r.archived && matches!(r.data, RecordData::Project(_)))
                    .map(|r| (Some(r.id.clone()), title(r))),
            )
            .collect();
        let chat = chat.to_owned();
        let target = cx.entity().downgrade();
        let error = cx.new(|_| String::new());
        // A pick list: each row is a tab stop; click it, or Tab to it and
        // press Space. ↵ chooses nothing.
        self.open_dialog(window, cx, move |dialog, window, cx| {
            let mut rows = div().flex().flex_col();
            for (i, (project, label)) in choices.iter().enumerate() {
                let (project, chat, target, error) =
                    (project.clone(), chat.clone(), target.clone(), error.clone());
                rows = rows.child(pick_row(format!("move-chat-{i}"), label.clone()).on_click(
                    move |_, window, cx| {
                        let mut result = Err("This window is unavailable.".to_owned());
                        let _ = target.update(cx, |this, cx| {
                            result = this
                                .apply_project(
                                    ProjectCommand::MoveChat {
                                        chat: chat.clone(),
                                        project: project.clone(),
                                    },
                                    cx,
                                )
                                .map(|_| ())
                                .map_err(|e| e.to_string())
                        });
                        match result {
                            Ok(()) => window.close_dialog(cx),
                            Err(e) => error.update(cx, |s, cx| {
                                *s = e;
                                cx.notify();
                            }),
                        }
                    },
                ));
            }
            dialog
                .title("Move chat")
                .on_ok(|_, _, _| false)
                .child(
                    components::dialog_body("move-chat-list", window).child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(metadata(
                                "History and attachments stay saved. Later replies use the destination's context; current work stops.",
                            ))
                            .child(rows)
                            .child(components::field_error(
                                "move-chat-error",
                                error.read(cx).clone(),
                            )),
                    ).test_support(),
                )
                .footer(
                    components::dialog_footer().child(
                        button("move-chat-cancel")
                            .label("Cancel")
                            .outline()
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    ),
                )
        });
    }
    pub(super) fn project_link_dialog(
        &mut self,
        project: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let docs: Vec<_> = self
            .records
            .iter()
            .filter(|r| {
                !r.archived && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())
            })
            .cloned()
            .collect();
        let project = project.to_owned();
        let target = cx.entity().downgrade();
        let error = cx.new(|_| String::new());
        // A pick list, like Move chat: rows are tab stops and ↵ chooses
        // nothing.
        self.open_dialog(window, cx, move |dialog, window, cx| {
            let mut rows = div().flex().flex_col();
            for r in &docs {
                let (project, note, target, error) =
                    (project.clone(), r.id.clone(), target.clone(), error.clone());
                rows = rows.child(
                    pick_row(format!("project-link-doc-{}", r.id), title(r)).on_click(
                        move |_, window, cx| {
                            let mut result = Err("This window is unavailable.".to_owned());
                            let _ = target.update(cx, |this, cx| {
                                result = this
                                    .apply_project(
                                        ProjectCommand::LinkDocument {
                                            project: project.clone(),
                                            note: note.clone(),
                                        },
                                        cx,
                                    )
                                    .map(|_| ())
                                    .map_err(|e| e.to_string())
                            });
                            match result {
                                Ok(()) => window.close_dialog(cx),
                                Err(e) => error.update(cx, |s, cx| {
                                    *s = e;
                                    cx.notify();
                                }),
                            }
                        },
                    ),
                );
            }
            if docs.is_empty() {
                rows = rows.child(components::empty_state(
                    "project-link-empty",
                    IconName::FileText,
                    "No documents to link",
                    "Create a note first, then link it here.",
                    None,
                ));
            }
            dialog
                .title("Link document")
                .on_ok(|_, _, _| false)
                .child(
                    components::dialog_body("project-link-list", window).child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(metadata(
                                "Links the same saved document. Its text becomes context for this project; its other chats and instructions stay private.",
                            ))
                            .child(rows)
                            .child(components::field_error(
                                "project-link-error",
                                error.read(cx).clone(),
                            )),
                    ).test_support(),
                )
                .footer(
                    components::dialog_footer().child(
                        button("project-link-cancel")
                            .label("Cancel")
                            .outline()
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    ),
                )
        });
    }
    pub(super) fn project_delete_dialog(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let preview = match self.workspace.project_delete_preview(id) {
            Ok(p) => p,
            Err(e) => {
                self.fail(e.to_string());
                cx.notify();
                return;
            }
        };
        let name = self
            .records
            .iter()
            .find(|r| r.id == id)
            .map(title)
            .unwrap_or_else(|| "this project".into());
        let choices = cx.new(|_| BTreeMap::<String, ProjectActionChoice>::new());
        let error = cx.new(|_| String::new());
        let target = cx.entity().downgrade();
        self.open_dialog(window, cx, move |dialog, window, cx| {
            let line = |id: &str, label: &str, titles: Vec<String>| {
                let text = format!("{label}: {}.", titles.join(", "));
                div()
                    .id(SharedString::from(id.to_owned()))
                    .aria_label(text.clone())
                    .child(text)
                    .test_support()
            };
            let moved: Vec<String> = preview.removed.iter().filter(|r| listed(r)).map(title).collect();
            let kept: Vec<String> = preview.retained.iter().map(title).collect();
            let mut body = div()
                .flex()
                .flex_col()
                .gap_2()
                .text_size(UI_TEXT)
                .child(metadata("You can restore it from Projects → Trash."))
                .child(line("project-delete-moves", "Moves to Trash", moved));
            if !kept.is_empty() {
                body = body.child(line("project-delete-stays", "Stays active", kept));
            }
            if let Some(other) = other_items_line(&preview.removed) {
                body = body.child(
                    metadata(other.clone())
                        .id("project-delete-other")
                        .aria_label(other)
                        .test_support(),
                );
            }
            let pending = choices.read(cx).len() != preview.unfinished_actions.len();
            if !preview.unfinished_actions.is_empty() {
                body = body.child(section("Open tasks")).child(metadata(
                    "Choose for each task: keep it outside the project, or cancel it.",
                ));
            }
            for r in &preview.unfinished_actions {
                let keep = choices.clone(); let cancel = choices.clone(); let id = r.id.clone(); let cancel_id = id.clone();
                let chosen = choices.read(cx).get(&id).copied();
                body = body.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().min_w(px(160.)).child(title(r)))
                        .child(components::segmented(
                            SharedString::from(format!("project-action-choice-{id}")),
                            vec![
                                button(format!("project-action-keep-{id}"))
                                    .label("Keep outside project")
                                    .selected(chosen == Some(ProjectActionChoice::Keep))
                                    .on_click(move |_, _, cx| keep.update(cx, |v, cx| { v.insert(id.clone(), ProjectActionChoice::Keep); cx.notify(); })),
                                button(format!("project-action-cancel-{cancel_id}"))
                                    .label("Cancel task")
                                    .selected(chosen == Some(ProjectActionChoice::Cancel))
                                    .on_click(move |_, _, cx| cancel.update(cx, |v, cx| { v.insert(cancel_id.clone(), ProjectActionChoice::Cancel); cx.notify(); })),
                            ],
                        )),
                );
            }
            let failure = error.read(cx).clone();
            if !failure.is_empty() {
                body = body.child(components::banner("project-delete-error", Tone::Error, failure, vec![], None));
            }
            let (choices, target, submit_error, approved) = (choices.clone(), target.clone(), error.clone(), preview.clone());
            let why = pending.then_some("choose for each open task first");
            let (announced, tooltip) = match why {
                Some(why) => (format!("Move to Trash, unavailable: {why}"), format!("Move to Trash is unavailable: {why}")),
                None => ("Move to Trash".to_owned(), "Move the project and the items above to Trash".to_owned()),
            };
            // ↵ never confirms: only a click, or Space on the focused button.
            dialog
                .title(format!("Move \u{201c}{name}\u{201d} to Trash?"))
                .width(px(640.))
                .on_ok(|_, _, _| false)
                .child(components::dialog_body("project-delete-preview", window).child(body).test_support())
                .footer(
                    components::dialog_footer()
                        .child(
                            button("project-delete-cancel")
                                .label("Cancel")
                                .outline()
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            button("project-delete-confirm")
                                .label("Move to Trash")
                                .danger()
                                .disabled(pending)
                                .accessibility_label(announced)
                                .tooltip(tooltip)
                                .on_click(move |_, window, cx| {
                                    let command = ProjectCommand::Delete { preview: approved.clone(), actions: choices.read(cx).clone() };
                                    let mut result = Err("Window unavailable".to_owned()); let _ = target.update(cx, |this, cx| result = this.apply_project(command, cx).map(|_| { this.selected_project = None; }).map_err(|e| e.to_string()));
                                    match result { Ok(()) => window.close_dialog(cx), Err(e) => submit_error.update(cx, |s, cx| { *s = format!("{e}. Nothing moved. Close and choose Move to Trash… again to refresh the list."); cx.notify(); }) }
                                }),
                        ),
                )
        });
    }
}
