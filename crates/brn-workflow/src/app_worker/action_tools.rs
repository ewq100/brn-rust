//! Fixed read-only callbacks to the owning application lane. No Store attachment.
use super::*;
use brn_ai::{
    AiError, AiErrorKind, AiResult, NotePage, ReadScope, ReadTools, ToolNote, ToolSearch,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Bind every field of the complete checked ActionRecord, including origin and times.
pub(super) fn checked_reference(
    record: &crate::actions::ActionRecord,
) -> AiResult<brn_ai::CheckedActionRef> {
    let bytes = serde_json::to_vec(record).map_err(|_| rejected())?;
    Ok(brn_ai::CheckedActionRef {
        id: record.origin.id.to_string(),
        version: record.version,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    })
}

pub(super) enum ActionReadRequest {
    Conflicts {
        path: String,
        scope: ReadScope,
        limit: usize,
        cursor: Option<String>,
    },
    One(String),
    Page {
        state: Option<String>,
        limit: usize,
        cursor: Option<String>,
    },
}
pub(super) struct ActionRead {
    request: ActionReadRequest,
    reply: mpsc::Sender<AiResult<Value>>,
}
#[derive(Clone)]
pub(super) struct ActionReads {
    tx: mpsc::Sender<Message>,
    admission: Arc<Mutex<()>>,
    stopping: Arc<AtomicBool>,
}
impl ActionReads {
    pub(super) fn new(
        tx: mpsc::Sender<Message>,
        admission: Arc<Mutex<()>>,
        stopping: Arc<AtomicBool>,
    ) -> Self {
        Self {
            tx,
            admission,
            stopping,
        }
    }
    pub(super) fn close(&self) {
        let _admission = self.admission.lock().expect("owned admission fence");
        self.stopping.store(true, Ordering::Release);
    }
    pub(super) fn wrap(&self, notes: Arc<dyn ReadTools>) -> Arc<dyn ReadTools> {
        Arc::new(ApplicationReads {
            notes,
            actions: self.clone(),
        })
    }
    fn read(&self, request: ActionReadRequest) -> AiResult<Value> {
        self.enqueue(request)?
            .recv()
            .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
    fn enqueue(&self, request: ActionReadRequest) -> AiResult<mpsc::Receiver<AiResult<Value>>> {
        let (reply, rx) = mpsc::channel();
        {
            let _admission = self
                .admission
                .lock()
                .map_err(|_| AiError::new(AiErrorKind::Other))?;
            if self.stopping.load(Ordering::Acquire) {
                return Err(rejected());
            }
            self.tx
                .send(Message::ActionRead(ActionRead { request, reply }))
                .map_err(|_| AiError::new(AiErrorKind::Other))?;
        }
        // Never retain the admission fence while waiting for application work.
        Ok(rx)
    }
}
impl ActionRead {
    pub(super) fn refuse(self, kind: AiErrorKind) {
        let _ = self.reply.send(Err(AiError::new(kind)));
    }
    pub(super) fn settle(self, app: &mut App, stopping: bool) {
        let result = if stopping {
            Err(rejected())
        } else {
            self.request.run(app)
        };
        let _ = self.reply.send(result);
    }
}
fn rejected() -> AiError {
    AiError::new(AiErrorKind::ToolRejected)
}
fn safe(error: WorkflowError) -> AiError {
    AiError::new(match error.kind {
        ErrorKind::ToolRejected | ErrorKind::NotFound => AiErrorKind::ToolRejected,
        ErrorKind::SaveUncertain
        | ErrorKind::ContextStale
        | ErrorKind::IndexStale
        | ErrorKind::AiIndexStale => AiErrorKind::IndexStale,
        _ => AiErrorKind::Storage,
    })
}
impl ActionReadRequest {
    fn run(self, app: &mut App) -> AiResult<Value> {
        let result = match self {
            Self::Conflicts {
                path,
                scope,
                limit,
                cursor,
            } => {
                if path.is_empty()
                    || path.len() > 512
                    || !(1..=100).contains(&limit)
                    || cursor.as_ref().is_some_and(|c| c.len() > 8192)
                {
                    return Err(rejected());
                }
                let cursor = cursor
                    .map(|c| serde_json::from_str::<crate::findings::NoteConflictCursor>(&c))
                    .transpose()
                    .map_err(|_| rejected())?;
                let scope = match scope {
                    ReadScope::Current => KnowledgeScope::Current,
                    ReadScope::Source => KnowledgeScope::Source,
                    ReadScope::History => KnowledgeScope::History,
                    ReadScope::All => KnowledgeScope::All,
                };
                let page = app
                    .note_conflicts(&crate::findings::NoteConflictRequest {
                        path,
                        scope,
                        limit,
                        cursor,
                    })
                    .map_err(safe)?;
                let next_cursor = page
                    .next_cursor
                    .map(|c| serde_json::to_string(&c))
                    .transpose()
                    .map_err(|_| rejected())?;
                json!({"path":page.path,"note_id":page.note_id,"scope":page.scope,"source":page.source,"entries":page.entries,"next_cursor":next_cursor,"open_count":page.open_count})
            }
            Self::One(id) => {
                if !(1..=64).contains(&id.len()) {
                    return Err(rejected());
                }
                let id = Uuid::parse_str(&id).map_err(|_| rejected())?;
                let record = app.action(id).map_err(safe)?;
                let checked_ref = checked_reference(&record)?;
                let mut result = serde_json::to_value(record).map_err(|_| rejected())?;
                result.as_object_mut().ok_or_else(rejected)?.insert(
                    "checked_ref".into(),
                    serde_json::to_value(checked_ref).map_err(|_| rejected())?,
                );
                result
            }
            Self::Page {
                state,
                limit,
                cursor,
            } => {
                if !(1..=20).contains(&limit)
                    || state.as_ref().is_some_and(|s| s.len() > 16)
                    || cursor.as_ref().is_some_and(|c| c.len() > 256)
                {
                    return Err(rejected());
                }
                let state = state
                    .map(|s| {
                        serde_json::from_value::<crate::actions::ActionState>(Value::String(s))
                    })
                    .transpose()
                    .map_err(|_| rejected())?;
                let before = cursor
                    .map(|c| serde_json::from_str::<crate::actions::ActionCursor>(&c))
                    .transpose()
                    .map_err(|_| rejected())?;
                let page = app
                    .actions(&crate::actions::ActionListRequest {
                        state,
                        limit,
                        before,
                    })
                    .map_err(safe)?;
                let next_cursor = page
                    .next_before
                    .map(|c| serde_json::to_string(&c))
                    .transpose()
                    .map_err(|_| rejected())?;
                json!({"entries":page.entries,"next_cursor":next_cursor})
            }
        };
        if serde_json::to_vec(&result).map_err(|_| rejected())?.len() > brn_ai::READ_ACTION_BYTES {
            return Err(rejected());
        }
        Ok(result)
    }
}
struct ApplicationReads {
    notes: Arc<dyn ReadTools>,
    actions: ActionReads,
}
impl ReadTools for ApplicationReads {
    fn search_notes(&self, query: &str, limit: usize) -> AiResult<ToolSearch> {
        self.notes.search_notes(query, limit)
    }
    fn read_note(&self, path: &str) -> AiResult<ToolNote> {
        self.notes.read_note(path)
    }
    fn list_notes(&self, folder: Option<&str>, cursor: Option<&str>) -> AiResult<NotePage> {
        self.notes.list_notes(folder, cursor)
    }
    fn search_notes_scoped(
        &self,
        query: &str,
        limit: usize,
        scope: ReadScope,
    ) -> AiResult<ToolSearch> {
        self.notes.search_notes_scoped(query, limit, scope)
    }
    fn read_note_scoped(&self, path: &str, scope: ReadScope) -> AiResult<ToolNote> {
        self.notes.read_note_scoped(path, scope)
    }
    fn list_notes_scoped(
        &self,
        folder: Option<&str>,
        cursor: Option<&str>,
        scope: ReadScope,
    ) -> AiResult<NotePage> {
        self.notes.list_notes_scoped(folder, cursor, scope)
    }
    fn read_conflicts(
        &self,
        path: &str,
        scope: ReadScope,
        limit: usize,
        cursor: Option<&str>,
    ) -> AiResult<Value> {
        self.actions.read(ActionReadRequest::Conflicts {
            path: path.into(),
            scope,
            limit,
            cursor: cursor.map(str::to_owned),
        })
    }
    fn read_action(&self, id: &str) -> AiResult<Value> {
        self.actions.read(ActionReadRequest::One(id.into()))
    }
    fn list_actions(
        &self,
        state: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
    ) -> AiResult<Value> {
        self.actions.read(ActionReadRequest::Page {
            state: state.map(str::to_owned),
            limit,
            cursor: cursor.map(str::to_owned),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, AppConfig) {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(base.path().join("data")).unwrap();
        let config = AppConfig {
            vault_root: None,
            credentials_dir: Some(base.path().join("credentials")),
            model_dir: None,
        };
        (base, config)
    }
    #[test]
    fn checked_reference_binds_every_full_record_field_including_same_version_changes() {
        let data = json!({"title":"Exact õ\r\n","description":"\u{feff}Whole 🦀\r\n","state":"waiting",
            "owner":null,"related_person":null,"related_project":null,"sources":[],"thread":null,
            "due_on":null,"follow_up_on":null,"dependencies":[],"parent":null,"follows_up":null,"priority":null});
        let whole = json!({
            "origin":{"id":"11111111-1111-4111-8111-111111111111",
                "proposal":{"id":"22222222-2222-4222-8222-222222222222","version":1},"data":data,"created_at_ms":100},
            "version":3,"data":data,"updated_at_ms":200,"waiting_since_ms":200,"completed_at_ms":null
        });
        let record: crate::actions::ActionRecord = serde_json::from_value(whole.clone()).unwrap();
        record.validate().unwrap();
        let checked = checked_reference(&record).unwrap();
        checked.validate().unwrap();
        assert_eq!(checked.id, record.origin.id.to_string());
        assert_eq!(checked.version, record.version);
        assert_eq!(checked, checked_reference(&record.clone()).unwrap());
        let uuid = "33333333-3333-4333-8333-333333333333";
        let fields = [
            ("title", json!("changed")),
            ("description", json!("different\r\n")),
            ("state", json!("open")),
            ("owner", json!("someone")),
            ("related_person", json!(uuid)),
            ("related_project", json!(uuid)),
            ("sources", json!([uuid])),
            ("thread", json!(uuid)),
            ("due_on", json!("2028-02-29")),
            ("follow_up_on", json!("2028-03-01")),
            ("dependencies", json!([uuid])),
            ("parent", json!(uuid)),
            ("follows_up", json!(uuid)),
            ("priority", json!("high")),
        ];
        let mut mutations = Vec::new();
        for prefix in ["/data/", "/origin/data/"] {
            for (field, value) in &fields {
                mutations.push((format!("{prefix}{field}"), value.clone()));
            }
        }
        for (path, value) in [
            ("/origin/id", json!(uuid)),
            ("/origin/proposal/id", json!(uuid)),
            ("/origin/proposal/version", json!(2)),
            ("/origin/created_at_ms", json!(99)),
            ("/version", json!(4)),
            ("/updated_at_ms", json!(201)),
            ("/waiting_since_ms", Value::Null),
            ("/completed_at_ms", json!(202)),
        ] {
            mutations.push((path.into(), value));
        }
        for (path, value) in mutations {
            let mut changed = whole.clone();
            *changed.pointer_mut(&path).unwrap() = value;
            let changed: crate::actions::ActionRecord = serde_json::from_value(changed).unwrap();
            if path != "/version" {
                assert_eq!(changed.version, record.version);
            }
            assert_ne!(
                checked_reference(&changed).unwrap().sha256,
                checked.sha256,
                "{path}"
            );
        }
    }

    #[test]
    fn private_action_reads_settle_before_shutdown_without_consuming_public_events() {
        let (base, config) = fixture();
        let mut worker = AppWorker::start(base.path().join("data"), config).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let reads = ActionReads::new(
            worker.tx.clone(),
            worker.admission.clone(),
            worker.stopping.clone(),
        );
        let (entered_tx, entered) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let pause = Uuid::new_v4();
        worker
            .submit(
                pause,
                AppCommand::TestPause {
                    entered: entered_tx,
                    release: wait,
                },
            )
            .unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        let one = reads
            .enqueue(ActionReadRequest::One(Uuid::new_v4().to_string()))
            .unwrap();
        let page = reads
            .enqueue(ActionReadRequest::Page {
                state: None,
                limit: 20,
                cursor: None,
            })
            .unwrap();
        let status = Uuid::new_v4();
        worker.submit(status, AppCommand::Status).unwrap();
        let closing = worker.stopping.clone();
        let (done, finished) = mpsc::channel();
        let join = std::thread::spawn(move || {
            let result = worker.shutdown();
            done.send((worker, result)).unwrap();
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !closing.load(Ordering::Acquire) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            reads
                .read(ActionReadRequest::Page {
                    state: None,
                    limit: 20,
                    cursor: None
                })
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
        release.send(()).unwrap();
        for reply in [one, page] {
            assert_eq!(
                reply
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap()
                    .unwrap_err()
                    .kind,
                AiErrorKind::ToolRejected
            );
        }
        let (worker, result) = finished.recv_timeout(Duration::from_secs(10)).unwrap();
        result.unwrap();
        join.join().unwrap();
        let events = std::iter::from_fn(|| worker.try_event()).collect::<Vec<_>>();
        assert_eq!(
            events.len(),
            2,
            "Private replies must produce no frontend events"
        );
        assert!(
            events
                .iter()
                .any(|(id, e)| *id == pause && matches!(e, AppEvent::EditRecovered))
        );
        assert!(events.iter().any(|(id, e)| *id == status
            && matches!(e,AppEvent::Failed(error)if error.kind==ErrorKind::Cancelled)));
        assert_eq!(
            reads
                .read(ActionReadRequest::One(Uuid::new_v4().to_string()))
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
    }
    #[test]
    fn each_private_action_read_checks_the_fresh_current_evidence_fence() {
        let (base, config) = fixture();
        let mut app = App::open(&base.path().join("data"), config).unwrap();
        let page = || ActionReadRequest::Page {
            state: None,
            limit: 20,
            cursor: None,
        };
        assert_eq!(
            page().run(&mut app).unwrap(),
            json!({"entries":[],"next_cursor":null})
        );
        app.completion_uncertain = true;
        assert_eq!(
            page().run(&mut app).unwrap_err().kind,
            AiErrorKind::IndexStale
        );
        assert_eq!(
            ActionReadRequest::One(Uuid::new_v4().to_string())
                .run(&mut app)
                .unwrap_err()
                .kind,
            AiErrorKind::IndexStale
        );
        app.completion_uncertain = false;
        assert_eq!(
            page().run(&mut app).unwrap(),
            json!({"entries":[],"next_cursor":null})
        );
    }
    #[test]
    fn closed_private_lanes_and_abandoned_reply_channels_refuse_without_waiting_forever() {
        let (tx, rx) = mpsc::channel();
        drop(rx);
        let reads = ActionReads::new(
            tx,
            Arc::new(Mutex::new(())),
            Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(
            reads
                .read(ActionReadRequest::One(Uuid::new_v4().to_string()))
                .unwrap_err()
                .kind,
            AiErrorKind::Other
        );
        let (tx, rx) = mpsc::channel();
        let reads = ActionReads::new(
            tx,
            Arc::new(Mutex::new(())),
            Arc::new(AtomicBool::new(false)),
        );
        let join = std::thread::spawn(move || {
            let message = rx.recv_timeout(Duration::from_secs(10)).unwrap();
            drop(message);
        });
        assert_eq!(
            reads
                .read(ActionReadRequest::One(Uuid::new_v4().to_string()))
                .unwrap_err()
                .kind,
            AiErrorKind::Other
        );
        join.join().unwrap();
        let (base, config) = fixture();
        let mut app = App::open(&base.path().join("data"), config).unwrap();
        let (reply, rx) = mpsc::channel();
        drop(rx);
        ActionRead {
            request: ActionReadRequest::Page {
                state: None,
                limit: 20,
                cursor: None,
            },
            reply,
        }
        .settle(&mut app, false);
    }

    #[test]
    fn retained_application_action_reads_block_detach_and_shutdown_until_the_last_lease_drops() {
        let (base, mut config) = fixture();
        let vault = base.path().join("vault");
        std::fs::create_dir(&vault).unwrap();
        std::fs::write(vault.join("a.md"), "current").unwrap();
        config.vault_root = Some(vault);
        let (lease, received) = mpsc::channel();
        let hook: crate::simple_worker_tests::AnswerHook =
            Arc::new(move |_, _, tools, cancel, _| {
                lease.send(tools.clone()).unwrap();
                Box::pin(async move {
                    cancel.cancelled().await;
                    drop(tools);
                    brn_ai::AiAnswer {
                        text: "retained private read".into(),
                        terminal: brn_ai::AiTerminal::Interrupted,
                    }
                })
            });
        let mut worker = AppWorker::start_owned(
            base.path().join("data"),
            config,
            Hooks {
                chat: chat_worker::Hooks {
                    answer: Some(hook),
                    ..chat_worker::Hooks::default()
                },
                ..Hooks::default()
            },
        )
        .unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let id = Uuid::new_v4();
        worker
            .submit(
                id,
                AppCommand::Ask(AskRequest {
                    id,
                    conversation: None,
                    question: "synthetic reads".into(),
                    selection: Selection {
                        provider: Provider::Chatgpt,
                        model: "gpt-6-luna".into(),
                    },
                    effort: Some(ReasoningEffort::Low),
                    generation: 7,
                }),
            )
            .unwrap();
        let tools = received.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(
            tools.list_actions(None, 20, None).unwrap(),
            json!({"entries":[],"next_cursor":null})
        );
        let chat = worker.controls.lock().unwrap().chat.clone().unwrap();
        assert_eq!(chat.set_tools(None).unwrap_err().kind, ErrorKind::ToolsBusy);
        let closing = worker.stopping.clone();
        let (done, finished) = mpsc::channel();
        let join = std::thread::spawn(move || {
            let result = worker.shutdown();
            done.send((worker, result)).unwrap();
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !closing.load(Ordering::Acquire) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            tools.list_actions(None, 20, None).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
        assert!(
            finished.try_recv().is_err(),
            "Retained Action capabilities must keep the lease active"
        );
        assert!(
            brn_store::WorkStore::open(&base.path().join("data")).is_err(),
            "Owner remains held during drain"
        );
        drop(tools);
        let (worker, result) = finished.recv_timeout(Duration::from_secs(10)).unwrap();
        result.unwrap();
        join.join().unwrap();
        let terminal = std::iter::from_fn(|| worker.try_event())
            .filter_map(|(_, e)| match e {
                AppEvent::Chat(ChatEvent::Finished { turn, .. }) => Some(turn),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(terminal.len(), 1);
        assert_eq!(
            terminal[0].status,
            brn_store::work::WorkTurnStatus::Interrupted
        );
        drop(worker);
        let (store, _) = brn_store::WorkStore::open(&base.path().join("data")).unwrap();
        assert_eq!(
            store.turn(id).unwrap().unwrap().answer,
            "retained private read"
        );
    }

    #[test]
    fn fatal_application_lane_errors_refuse_queued_action_reads_before_joining_retained_chat_leases()
     {
        let (base, mut config) = fixture();
        let vault = base.path().join("vault");
        std::fs::create_dir(&vault).unwrap();
        std::fs::write(vault.join("a.md"), "current").unwrap();
        config.vault_root = Some(vault);
        let (lease, received) = mpsc::channel();
        let hook: crate::simple_worker_tests::AnswerHook =
            Arc::new(move |_, _, tools, cancel, _| {
                lease.send(tools.clone()).unwrap();
                Box::pin(async move {
                    cancel.cancelled().await;
                    drop(tools);
                    brn_ai::AiAnswer {
                        text: "partial before application failure".into(),
                        terminal: brn_ai::AiTerminal::Interrupted,
                    }
                })
            });
        let mut worker = AppWorker::start_owned(
            base.path().join("data"),
            config,
            Hooks {
                chat: chat_worker::Hooks {
                    answer: Some(hook),
                    ..chat_worker::Hooks::default()
                },
                ..Hooks::default()
            },
        )
        .unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let turn_id = Uuid::new_v4();
        worker
            .submit(
                turn_id,
                AppCommand::Ask(AskRequest {
                    id: turn_id,
                    conversation: None,
                    question: "synthetic error".into(),
                    selection: Selection {
                        provider: Provider::Chatgpt,
                        model: "gpt-6-luna".into(),
                    },
                    effort: Some(ReasoningEffort::Low),
                    generation: 7,
                }),
            )
            .unwrap();
        let tools = received.recv_timeout(Duration::from_secs(10)).unwrap();
        let (entered_tx, entered) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let pause = Uuid::new_v4();
        worker
            .submit(
                pause,
                AppCommand::TestPause {
                    entered: entered_tx,
                    release: wait,
                },
            )
            .unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        {
            let _admission = worker.admission.lock().unwrap();
            worker.tx.send(Message::FailLane).unwrap();
        }
        let reads = ActionReads::new(
            worker.tx.clone(),
            worker.admission.clone(),
            worker.stopping.clone(),
        );
        let queued = reads
            .enqueue(ActionReadRequest::Page {
                state: None,
                limit: 20,
                cursor: None,
            })
            .unwrap();
        let status = Uuid::new_v4();
        worker.submit(status, AppCommand::Status).unwrap();
        release.send(()).unwrap();
        let reply = queued.recv_timeout(Duration::from_secs(1));
        // Always release the synthetic lease before asserting, even on RED.
        drop(tools);
        let error = worker.shutdown().unwrap_err();
        assert_eq!(error.kind, ErrorKind::Other);
        assert!(
            matches!(
                reply,
                Ok(Err(AiError {
                    kind: AiErrorKind::Storage,
                    ..
                }))
            ),
            "A fatal lane error must settle its private reply BEFORE joining chat: {reply:?}"
        );
        assert_eq!(
            reads
                .read(ActionReadRequest::Page {
                    state: None,
                    limit: 20,
                    cursor: None
                })
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
        let events = std::iter::from_fn(|| worker.try_event()).collect::<Vec<_>>();
        assert!(events.iter().any(|(id, e)| *id == status
            && matches!(e,AppEvent::Failed(error)if error.kind==ErrorKind::Cancelled)));
        assert_eq!(
            events
                .iter()
                .filter(|(_, e)| matches!(e, AppEvent::Chat(ChatEvent::Finished { .. })))
                .count(),
            1
        );
    }
}
