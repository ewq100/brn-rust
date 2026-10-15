//! Projects navigation and owner dialogs over shared checked operations.
use super::shell::pill;
use super::tiles::{row, scroll};
use super::*;
impl ThreadsUi {
    pub(super) fn open_project_note(
        &mut self,
        project: &str,
        note: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_note(note, None, window, cx);
        if self.open_note_id() != Some(note) {
            return;
        }
        self.note_project = Some(project.into());
        if self.writing_threads.get(note).is_some_and(|chat| {
            self.workspace
                .store
                .project_context_record(chat)
                .ok()
                .flatten()
                .is_none_or(|p| p.id != project)
        }) {
            self.writing_threads.remove(note);
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
            .filter(|w| w.kind == WorkerKind::Run)
            .and_then(|w| w.run_id.as_ref())
            .filter(|_| !self.stream.trim().is_empty())
        {
            ProjectCommand::WithProgress {
                command: Box::new(command),
                run: run.clone(),
                partial: self.stream.clone(),
            }
        } else {
            command
        };
        let receipt = self
            .workspace
            .project_command(&format!("ui:{}", uuid::Uuid::new_v4()), command)?;
        if let Some(worker) = &self.worker
            && receipt.writes.iter().any(|w| {
                Some(w.after.id.as_str()) == worker.run_id.as_deref()
                    && matches!(&w.after.data, RecordData::Run(r) if r.state != RunState::Working)
            })
        {
            worker.cancel.cancel();
            self.pending_send = None;
        }
        // A returned receipt is an applied change, like ApplyOutcome::Applied.
        self.confirm("Project saved.");
        self.refresh();
        cx.notify();
        Ok(receipt)
    }
    pub(super) fn projects_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut body = scroll("projects-tile").child(
            div()
                .flex()
                .gap_2()
                .child(pill("project-new", "New project").on_click(cx.listener(
                    |this, _, window, cx| this.project_name_dialog(None, false, window, cx),
                )))
                .child(
                    pill(
                        "project-trash",
                        if self.projects_trash {
                            "Active projects"
                        } else {
                            "Trash"
                        },
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.projects_trash = !this.projects_trash;
                        this.selected_project = None;
                        cx.notify();
                    })),
                ),
        );
        for record in self.records.iter().filter(|r| {
            r.archived == self.projects_trash && matches!(r.data, RecordData::Project(_))
        }) {
            let id = record.id.clone();
            body = body.child(
                row(
                    format!("project-row-{id}"),
                    title(record),
                    if record.archived { "Trash" } else { "" },
                    self.selected_project.as_ref() == Some(&id),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected_project = Some(id.clone());
                    cx.notify();
                })),
            );
        }
        let Some(record) = self.selected_project.as_ref().and_then(|id| {
            self.records
                .iter()
                .find(|r| r.id == *id && r.archived == self.projects_trash)
        }) else {
            return body
                .child(metadata(if self.projects_trash {
                    "Deleted projects stay recoverable here."
                } else {
                    "Group related chats and documents with shared context."
                }))
                .into_any_element();
        };
        let RecordData::Project(p) = &record.data else {
            return body.into_any_element();
        };
        let id = record.id.clone();
        let version = record.version;
        if record.archived {
            return body.child(metadata("Restoring returns project contents and settings. Cancelled tasks stay cancelled; tasks kept outside stay outside. Work remains paused."))
                .child(button("project-restore").label("Restore project").on_click(cx.listener(move |this, _, _, cx| { if let Err(e) = this.apply_project(ProjectCommand::Restore { project: id.clone(), version }, cx) { this.fail(e.to_string()); cx.notify(); } else { this.projects_trash = false; } }))).into_any_element();
        }
        let settings_id = id.clone();
        let chat_id = id.clone();
        let note_id = id.clone();
        let import_id = id.clone();
        let link_id = id.clone();
        let delete_id = id.clone();
        body = body.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    pill("project-settings", "Instructions & settings").on_click(cx.listener(
                        move |this, _, window, cx| {
                            this.project_settings_dialog(&settings_id, window, cx)
                        },
                    )),
                )
                .child(pill("project-chat", "New chat").on_click(cx.listener(
                    move |this, _, window, cx| {
                        this.project_name_dialog(Some(chat_id.clone()), true, window, cx)
                    },
                )))
                .child(pill("project-note", "New document").on_click(cx.listener(
                    move |this, _, window, cx| {
                        this.project_name_dialog(Some(note_id.clone()), false, window, cx)
                    },
                )))
                .child(
                    pill("project-import", "Import file")
                        .disabled(self.worker.is_some())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.attaching_chat = Some(import_id.clone());
                            this.choose_import(ImportIntent::FullNote, window, cx);
                        })),
                )
                .child(pill("project-link", "Link document").on_click(cx.listener(
                    move |this, _, window, cx| this.project_link_dialog(&link_id, window, cx),
                )))
                .child(pill("project-delete", "Delete…").on_click(cx.listener(
                    move |this, _, window, cx| this.project_delete_dialog(&delete_id, window, cx),
                ))),
        );
        body = body.child(metadata(format!(
            "BRN knowledge: {} · {}",
            if p.use_brn_knowledge { "on" } else { "off" },
            if p.instructions.is_empty() {
                "No project instructions"
            } else {
                "Project instructions apply to every chat"
            }
        )));
        body = body.child(metadata("Documents"));
        let docs = project_document_ids(&self.records, record);
        for note in self
            .records
            .iter()
            .filter(|r| !r.archived && docs.contains(&r.id))
        {
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
        body = body.child(metadata("Chats"));
        for chat in self.records.iter().filter(|r| p.chats.contains(&r.id)) {
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
        body.into_any_element()
    }
    pub(super) fn project_name_dialog(
        &mut self,
        project: Option<String>,
        chat: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Title"));
        input.update(cx, |input, cx| input.focus(window, cx));
        let target = cx.entity().downgrade();
        let error = cx.new(|_| String::new());
        window.open_dialog(cx, move |dialog, _, cx| {
            let (input, target, project, error) = (
                input.clone(),
                target.clone(),
                project.clone(),
                error.clone(),
            );
            let submit_error = error.clone();
            let submit = input.clone();
            dialog
                .title(if project.is_none() {
                    "New project"
                } else if chat {
                    "New project chat"
                } else {
                    "New project document"
                })
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Input::new(&input).id("project-name-input"))
                        .child(metadata(error.read(cx).clone()))
                        .child(button("project-name-submit").label("Create").on_click(
                            move |_, window, cx| {
                                let name = submit.read(cx).value().to_string();
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
                                let mut result = Err("Window unavailable".to_owned());
                                let _ = target.update(cx, |this, cx| {
                                    result = this
                                        .apply_project(command, cx)
                                        .map(|r| {
                                            for w in r.writes {
                                                match &w.after.data {
                                                    RecordData::Project(_) => {
                                                        this.selected_project =
                                                            Some(w.after.id.clone())
                                                    }
                                                    RecordData::Thread(_) if w.before.is_none() => {
                                                        this.open_thread(&w.after.id, window, cx)
                                                    }
                                                    RecordData::Note(_) if w.before.is_none() => {
                                                        if let Some(p) = &project {
                                                            this.open_project_note(
                                                                p,
                                                                &w.after.id,
                                                                window,
                                                                cx,
                                                            );
                                                        } else {
                                                            this.open_note(
                                                                &w.after.id,
                                                                None,
                                                                window,
                                                                cx,
                                                            );
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
                                    Err(e) => submit_error.update(cx, |s, cx| {
                                        *s = e;
                                        cx.notify();
                                    }),
                                }
                            },
                        )),
                )
        });
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
        window.open_dialog(cx, move |dialog, _, cx| {
            let (name, instructions, knowledge, error, target, id) = (name.clone(), instructions.clone(), knowledge.clone(), error.clone(), target.clone(), r.id.clone());
            let toggle = knowledge.clone(); let submit_error = error.clone();
            dialog.title("Project instructions & settings").width(px(620.)).child(div().flex().flex_col().gap_3()
                .child(Input::new(&name))
                .child(div().id("project-instructions").h(px(180.)).child(prose_editor(&instructions).h_full().aria_label("Project instructions")).test_support())
                .child(pill("project-knowledge-toggle", if *knowledge.read(cx) { "Use BRN knowledge: on" } else { "Use BRN knowledge: off" }).on_click(move |_, _, cx| toggle.update(cx, |v, cx| { *v = !*v; cx.notify(); })))
                .child(metadata("Off stops new general-BRN lookups. Linked documents and saved project discussion remain usable. Instructions grant no additional write authority."))
                .child(metadata(error.read(cx).clone()))
                .child(button("project-settings-save").label("Save").on_click(move |_, window, cx| {
                    let command = ProjectCommand::Configure { project: id.clone(), version: r.version, title: name.read(cx).value().to_string(), instructions: instructions.read(cx).value().to_string(), use_brn_knowledge: *knowledge.read(cx) };
                    let mut result = Err("Window unavailable".to_owned()); let _ = target.update(cx, |this, cx| result = this.apply_project(command, cx).map(|_| ()).map_err(|e| e.to_string()));
                    match result { Ok(()) => window.close_dialog(cx), Err(e) => submit_error.update(cx, |s, cx| { *s = e; cx.notify(); }) }
                })))
        });
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
        window.open_dialog(cx, move |dialog, _, cx| {
            let mut body = div().flex().flex_col().gap_2().child(metadata("History and attachments stay saved. Subsequent work uses the destination context; current work stops."));
            for (i, (project, label)) in choices.iter().enumerate() {
                let (project, chat, target, error) = (project.clone(), chat.clone(), target.clone(), error.clone());
                body = body.child(button(format!("move-chat-{i}")).label(label.clone()).on_click(move |_, window, cx| { let mut result = Err("Window unavailable".to_owned()); let _ = target.update(cx, |this, cx| result = this.apply_project(ProjectCommand::MoveChat { chat: chat.clone(), project: project.clone() }, cx).map(|_| ()).map_err(|e| e.to_string())); match result { Ok(()) => window.close_dialog(cx), Err(e) => error.update(cx, |s, cx| { *s = e; cx.notify(); }) } }));
            }
            dialog.title("Move chat").child(body.child(metadata(error.read(cx).clone())))
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
        window.open_dialog(cx, move |dialog, _, cx| {
            let mut body = scroll("project-link-list").max_h(px(420.)).child(metadata("Link the same saved document. Its content becomes context for this project; its other chats and instructions remain private."));
            for r in &docs {
                let (project, note, target, error) = (project.clone(), r.id.clone(), target.clone(), error.clone());
                body = body.child(row(format!("project-link-doc-{}", r.id), title(r), "", false).on_click(move |_, window, cx| { let mut result = Err("Window unavailable".to_owned()); let _ = target.update(cx, |this, cx| result = this.apply_project(ProjectCommand::LinkDocument { project: project.clone(), note: note.clone() }, cx).map(|_| ()).map_err(|e| e.to_string())); match result { Ok(()) => window.close_dialog(cx), Err(e) => error.update(cx, |s, cx| { *s = e; cx.notify(); }) } }));
            }
            dialog.title("Link document").child(body.child(metadata(error.read(cx).clone())))
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
        let choices = cx.new(|_| BTreeMap::<String, ProjectActionChoice>::new());
        let error = cx.new(|_| String::new());
        let target = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let mut body = scroll("project-delete-preview").max_h(px(480.)).child(metadata("Move this project and its exclusive contents to Trash. Shared and independently retained documents stay active. Review this current removal set."));
            body = body.child(metadata("Move to Trash"));
            for r in &preview.removed { if matches!(r.data, RecordData::Project(_) | RecordData::Thread(_) | RecordData::Note(_) | RecordData::Action(_)) { body = body.child(metadata(title(r))); } }
            body = body.child(metadata(format!("{} dependent records and relationships also move to Trash", preview.removed.len()))).child(metadata("Keep active"));
            for r in &preview.retained { body = body.child(metadata(title(r))); }
            for r in &preview.unfinished_actions {
                let keep = choices.clone(); let cancel = choices.clone(); let id = r.id.clone(); let cancel_id = id.clone();
                body = body.child(div().flex().flex_col().gap_1().child(metadata(title(r))).child(div().flex().gap_2()
                    .child(pill(format!("project-action-keep-{id}"), "Keep outside project").selected(choices.read(cx).get(&id) == Some(&ProjectActionChoice::Keep)).on_click(move |_, _, cx| keep.update(cx, |v, cx| { v.insert(id.clone(), ProjectActionChoice::Keep); cx.notify(); })))
                    .child(pill(format!("project-action-cancel-{cancel_id}"), "Cancel task").selected(choices.read(cx).get(&cancel_id) == Some(&ProjectActionChoice::Cancel)).on_click(move |_, _, cx| cancel.update(cx, |v, cx| { v.insert(cancel_id.clone(), ProjectActionChoice::Cancel); cx.notify(); })))));
            }
            let (choices, target, submit_error, approved) = (choices.clone(), target.clone(), error.clone(), preview.clone());
            dialog.title("Delete project — review removal").width(px(640.)).child(div().flex().flex_col().gap_2().child(body)
                .child(metadata(error.read(cx).clone()))
                .child(button("project-delete-confirm").label("Move project to Trash").disabled(choices.read(cx).len() != preview.unfinished_actions.len()).on_click(move |_, window, cx| {
                    let command = ProjectCommand::Delete { preview: approved.clone(), actions: choices.read(cx).clone() };
                    let mut result = Err("Window unavailable".to_owned()); let _ = target.update(cx, |this, cx| result = this.apply_project(command, cx).map(|_| { this.selected_project = None; }).map_err(|e| e.to_string()));
                    match result { Ok(()) => window.close_dialog(cx), Err(e) => submit_error.update(cx, |s, cx| { *s = format!("{e}. Close and reopen Delete to refresh the preview."); cx.notify(); }) }
                })))
        });
    }
}
