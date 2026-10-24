//! The current worker's channel protocol and invocation-owned continuation payload.
//!
//! This boundary admits one old-worker batch through its first terminal event.
//! The host still owns presentation and consumes that terminal before replacing
//! the worker; the existing checked service owns durable Run and queue effects.
//! A saved prompt belongs only to its Run invocation. The host commits and arms
//! it, then consumes it at the terminal boundary; this module never decides
//! persistence, cancellation, queue policy or replacement dispatch.
use super::{ConversionResult, DocumentConversionSettings, ReferenceImportIntent, WorkerKind};
use std::sync::{Arc, atomic::AtomicBool, mpsc::Receiver};
use tokio_util::sync::CancellationToken;

const MAX_EVENTS_PER_DRAIN: usize = 128;

pub(super) enum WorkerEvent {
    AnswerFinalized(String),
    Progress(String),
    Activity(String),
    Text(String),
    Finished(Result<String, String>),
    RestorePrepared(Result<brn_threads_app::PreparedRestore, String>),
    RunFinished {
        result: Result<String, String>,
        terminal: Option<(String, u64)>,
    },
    CopilotLogin {
        verification_uri: String,
        user_code: String,
    },
    CopilotConnected(String),
    CopilotModels(Vec<String>),
    Converted {
        result: Box<Result<ConversionResult, String>>,
        title: String,
        chat: Option<String>,
        settings: DocumentConversionSettings,
        useful_project: Option<String>,
    },
}

pub(super) struct Worker {
    pub(super) receiver: Receiver<WorkerEvent>,
    pub(super) cancel: CancellationToken,
    pub(super) conversion_cancel: Arc<AtomicBool>,
    operation: WorkerOperation,
}

/// One invocation's owner, independent of the chat currently being viewed.
/// Only a Run worker has this identity; both values exist before it starts.
pub(super) struct RunIdentity {
    pub(super) thread: String,
    pub(super) run_id: String,
}

#[derive(Clone)]
pub(super) struct EditNoteTarget {
    pub(super) note_id: String,
    pub(super) version: u64,
}

enum WorkerOperation {
    Run {
        identity: RunIdentity,
        edit_note_target: Option<EditNoteTarget>,
        continuation: Option<String>,
    },
    Import {
        reference: Option<ReferenceImportIntent>,
    },
    Inbox,
    Export,
    Backup,
    Restore {
        publishing: bool,
    },
    Connect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WorkerBatchEnd {
    Pending,
    Finished,
    Disconnected,
}

pub(super) struct WorkerBatch {
    /// Includes the first terminal event, if any, with its original payload.
    events: Vec<WorkerEvent>,
    end: WorkerBatchEnd,
}

impl WorkerBatch {
    pub(super) fn into_parts(self) -> (Vec<WorkerEvent>, WorkerBatchEnd) {
        (self.events, self.end)
    }
}

impl Worker {
    pub(super) fn run(
        receiver: Receiver<WorkerEvent>,
        cancel: CancellationToken,
        thread: String,
        run_id: String,
        edit_note_target: Option<EditNoteTarget>,
    ) -> Self {
        Self {
            receiver,
            cancel,
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            operation: WorkerOperation::Run {
                identity: RunIdentity { thread, run_id },
                edit_note_target,
                continuation: None,
            },
        }
    }

    pub(super) fn import(
        receiver: Receiver<WorkerEvent>,
        cancel: CancellationToken,
        conversion_cancel: Arc<AtomicBool>,
        reference: Option<ReferenceImportIntent>,
    ) -> Self {
        Self {
            receiver,
            cancel,
            conversion_cancel,
            operation: WorkerOperation::Import { reference },
        }
    }

    pub(super) fn inbox(
        receiver: Receiver<WorkerEvent>,
        cancel: CancellationToken,
        conversion_cancel: Arc<AtomicBool>,
    ) -> Self {
        Self {
            receiver,
            cancel,
            conversion_cancel,
            operation: WorkerOperation::Inbox,
        }
    }

    pub(super) fn output(receiver: Receiver<WorkerEvent>, backup: bool) -> Self {
        Self {
            receiver,
            cancel: CancellationToken::new(),
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            operation: if backup {
                WorkerOperation::Backup
            } else {
                WorkerOperation::Export
            },
        }
    }

    pub(super) fn restore(
        receiver: Receiver<WorkerEvent>,
        cancel: CancellationToken,
        publishing: bool,
    ) -> Self {
        Self {
            receiver,
            cancel,
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            operation: WorkerOperation::Restore { publishing },
        }
    }

    pub(super) fn connect(receiver: Receiver<WorkerEvent>, cancel: CancellationToken) -> Self {
        Self {
            receiver,
            cancel,
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            operation: WorkerOperation::Connect,
        }
    }

    pub(super) fn run_identity(&self) -> Option<&RunIdentity> {
        match &self.operation {
            WorkerOperation::Run { identity, .. } => Some(identity),
            _ => None,
        }
    }

    pub(super) fn edit_note_target(&self) -> Option<&EditNoteTarget> {
        match &self.operation {
            WorkerOperation::Run {
                edit_note_target, ..
            } => edit_note_target.as_ref(),
            _ => None,
        }
    }

    /// Attach a committed saved prompt to this Run. The Run identity remains
    /// the sole source of the destination thread.
    pub(super) fn arm_continuation(&mut self, prompt: String) -> Result<(), String> {
        match &mut self.operation {
            WorkerOperation::Run { continuation, .. } if continuation.is_none() => {
                *continuation = Some(prompt);
                Ok(())
            }
            WorkerOperation::Run { .. } => Err("Run already has a continuation".into()),
            _ => Err("Only a Run can own a continuation".into()),
        }
    }

    pub(super) fn continuation(&self) -> Option<&str> {
        match &self.operation {
            WorkerOperation::Run { continuation, .. } => continuation.as_deref(),
            _ => None,
        }
    }

    pub(super) fn discard_continuation(&mut self) -> bool {
        match &mut self.operation {
            WorkerOperation::Run { continuation, .. } => continuation.take().is_some(),
            _ => false,
        }
    }

    /// Consume the saved prompt while its original Run identity is still live.
    pub(super) fn take_terminal_continuation(&mut self) -> Option<(String, String)> {
        match &mut self.operation {
            WorkerOperation::Run {
                identity,
                continuation,
                ..
            } => continuation
                .take()
                .map(|prompt| (prompt, identity.thread.clone())),
            _ => None,
        }
    }

    pub(super) fn reference_import(&self) -> Option<&ReferenceImportIntent> {
        match &self.operation {
            WorkerOperation::Import { reference } => reference.as_ref(),
            _ => None,
        }
    }

    pub(super) fn kind(&self) -> WorkerKind {
        match &self.operation {
            WorkerOperation::Run { .. } => WorkerKind::Run,
            WorkerOperation::Import { .. } => WorkerKind::Import,
            WorkerOperation::Inbox => WorkerKind::Inbox,
            WorkerOperation::Export => WorkerKind::Export,
            WorkerOperation::Backup => WorkerKind::Backup,
            WorkerOperation::Restore { publishing: false } => WorkerKind::RestoreValidation,
            WorkerOperation::Restore { publishing: true } => WorkerKind::Restore,
            WorkerOperation::Connect => WorkerKind::Connect,
        }
    }

    /// Drain at most 128 events from this receiver. The first terminal wins even
    /// if the producer then sends more events or disconnects. The host must retire
    /// this worker after consuming a terminal batch, before dispatching another
    /// invocation; no old events can reach its successor.
    pub(super) fn drain(&self) -> WorkerBatch {
        let mut batch = WorkerBatch {
            events: Vec::new(),
            end: WorkerBatchEnd::Pending,
        };
        while batch.events.len() < MAX_EVENTS_PER_DRAIN {
            match self.receiver.try_recv() {
                Ok(event) => {
                    let terminal = matches!(
                        event,
                        WorkerEvent::Finished(_)
                            | WorkerEvent::RestorePrepared(_)
                            | WorkerEvent::RunFinished { .. }
                            | WorkerEvent::Converted { .. }
                    );
                    batch.events.push(event);
                    if terminal {
                        batch.end = WorkerBatchEnd::Finished;
                        break;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if batch.end != WorkerBatchEnd::Finished {
                        batch.end = WorkerBatchEnd::Disconnected;
                    }
                    break;
                }
            }
        }
        batch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{self, SyncSender};

    fn worker() -> (SyncSender<WorkerEvent>, Worker) {
        let (sender, receiver) = mpsc::sync_channel(16);
        (
            sender,
            Worker::run(
                receiver,
                CancellationToken::new(),
                "original-chat".into(),
                "original-run".into(),
                None,
            ),
        )
    }

    #[test]
    fn continuation_is_run_only_rejects_overwrite_and_is_consumed_once() {
        let (_sender, mut worker) = worker();
        assert_eq!(worker.continuation(), None);
        worker
            .arm_continuation("saved prompt".into())
            .expect("first continuation arms");
        assert_eq!(worker.continuation(), Some("saved prompt"));
        assert!(worker.arm_continuation("replacement".into()).is_err());
        assert_eq!(worker.continuation(), Some("saved prompt"));
        assert_eq!(
            worker.take_terminal_continuation(),
            Some(("saved prompt".into(), "original-chat".into()))
        );
        assert_eq!(worker.take_terminal_continuation(), None);

        let (_sender, receiver) = mpsc::sync_channel(1);
        let mut import = Worker::import(
            receiver,
            CancellationToken::new(),
            Arc::new(AtomicBool::new(false)),
            None,
        );
        assert!(import.arm_continuation("saved prompt".into()).is_err());
        assert_eq!(import.continuation(), None);
        assert_eq!(import.take_terminal_continuation(), None);
    }

    #[test]
    fn an_empty_live_channel_waits_but_an_empty_closed_channel_disconnects() {
        let (sender, worker) = worker();
        let batch = worker.drain();
        assert!(batch.events.is_empty());
        assert_eq!(batch.end, WorkerBatchEnd::Pending);
        sender
            .send(WorkerEvent::Text("Still working".into()))
            .unwrap();
        let batch = worker.drain();
        assert_eq!(batch.end, WorkerBatchEnd::Pending);
        assert!(
            matches!(batch.events.as_slice(), [WorkerEvent::Text(text)] if text == "Still working")
        );
        drop(sender);
        let batch = worker.drain();
        assert!(batch.events.is_empty());
        assert_eq!(batch.end, WorkerBatchEnd::Disconnected);
    }

    #[test]
    fn progress_and_partial_text_are_admitted_before_disconnect() {
        let (sender, worker) = worker();
        sender
            .send(WorkerEvent::Progress("Reading".into()))
            .unwrap();
        sender
            .send(WorkerEvent::Text("Useful partial".into()))
            .unwrap();
        drop(sender);
        let batch = worker.drain();
        assert_eq!(batch.end, WorkerBatchEnd::Disconnected);
        assert!(
            matches!(batch.events.as_slice(), [WorkerEvent::Progress(progress), WorkerEvent::Text(text)] if progress == "Reading" && text == "Useful partial")
        );
    }

    #[test]
    fn the_first_terminal_keeps_its_payload_and_rejects_trailing_old_events() {
        for terminal in [
            WorkerEvent::Finished(Err("Original failure".into())),
            WorkerEvent::RunFinished {
                result: Ok("Original completion".into()),
                terminal: Some(("original-run".into(), 7)),
            },
            WorkerEvent::Converted {
                result: Box::new(Err("Original conversion failure".into())),
                title: "Original attachment".into(),
                chat: Some("original-chat".into()),
                settings: DocumentConversionSettings::default(),
                useful_project: Some("original-project".into()),
            },
        ] {
            let (sender, worker) = worker();
            sender
                .send(WorkerEvent::Text("Before terminal".into()))
                .unwrap();
            sender.send(terminal).unwrap();
            sender
                .send(WorkerEvent::Text("Late old text".into()))
                .unwrap();
            sender
                .send(WorkerEvent::Finished(Ok("Late success".into())))
                .unwrap();
            sender
                .send(WorkerEvent::Converted {
                    result: Box::new(Err("Late conversion".into())),
                    title: "Wrong attachment".into(),
                    chat: Some("wrong-chat".into()),
                    settings: DocumentConversionSettings::default(),
                    useful_project: None,
                })
                .unwrap();
            drop(sender);
            let batch = worker.drain();
            assert_eq!(batch.end, WorkerBatchEnd::Finished);
            assert_eq!(batch.events.len(), 2);
            assert!(
                matches!(&batch.events[0], WorkerEvent::Text(text) if text == "Before terminal")
            );
            match &batch.events[1] {
                WorkerEvent::Finished(result) => {
                    assert_eq!(result, &Err("Original failure".into()));
                }
                WorkerEvent::RunFinished { result, terminal } => {
                    assert_eq!(result, &Ok("Original completion".into()));
                    assert_eq!(terminal, &Some(("original-run".into(), 7)));
                }
                WorkerEvent::Converted {
                    result,
                    title,
                    chat,
                    useful_project,
                    ..
                } => {
                    assert!(
                        matches!(result.as_ref(), Err(error) if error == "Original conversion failure")
                    );
                    assert_eq!(title, "Original attachment");
                    assert_eq!(chat.as_deref(), Some("original-chat"));
                    assert_eq!(useful_project.as_deref(), Some("original-project"));
                }
                _ => panic!("expected original terminal event"),
            }
        }
    }

    #[test]
    fn a_terminal_finishes_without_waiting_for_the_producer_to_disconnect() {
        let (sender, worker) = worker();
        sender
            .send(WorkerEvent::Finished(Ok("Done".into())))
            .unwrap();
        let batch = worker.drain();
        assert_eq!(batch.end, WorkerBatchEnd::Finished);
        assert!(
            matches!(batch.events.as_slice(), [WorkerEvent::Finished(Ok(text))] if text == "Done")
        );
    }

    #[test]
    fn a_large_backlog_is_drained_in_fifo_batches_through_its_terminal() {
        let before_terminal = MAX_EVENTS_PER_DRAIN * 2;
        let (sender, receiver) = mpsc::sync_channel(before_terminal + 2);
        let worker = Worker::run(
            receiver,
            CancellationToken::new(),
            "original-chat".into(),
            "original-run".into(),
            None,
        );
        for index in 0..before_terminal {
            sender
                .send(WorkerEvent::Text(format!("event-{index}")))
                .unwrap();
        }
        sender
            .send(WorkerEvent::Finished(Ok("Original completion".into())))
            .unwrap();
        sender
            .send(WorkerEvent::Text("After terminal".into()))
            .unwrap();

        let mut observed = Vec::new();
        for _ in 0..2 {
            let (events, end) = worker.drain().into_parts();
            assert_eq!(events.len(), MAX_EVENTS_PER_DRAIN);
            assert_eq!(end, WorkerBatchEnd::Pending);
            observed.extend(events.into_iter().map(|event| match event {
                WorkerEvent::Text(text) => text,
                _ => panic!("expected preterminal text"),
            }));
        }
        let (events, end) = worker.drain().into_parts();
        assert_eq!(end, WorkerBatchEnd::Finished);
        assert!(matches!(
            events.as_slice(),
            [WorkerEvent::Finished(Ok(text))] if text == "Original completion"
        ));
        assert_eq!(
            observed,
            (0..before_terminal)
                .map(|index| format!("event-{index}"))
                .collect::<Vec<_>>()
        );
        assert!(matches!(
            worker.receiver.try_recv(),
            Ok(WorkerEvent::Text(text)) if text == "After terminal"
        ));
    }

    #[test]
    fn terminal_at_the_budget_edge_wins_and_leaves_the_tail_unread() {
        let (sender, receiver) = mpsc::sync_channel(MAX_EVENTS_PER_DRAIN + 1);
        let worker = Worker::run(
            receiver,
            CancellationToken::new(),
            "original-chat".into(),
            "original-run".into(),
            None,
        );
        for index in 0..MAX_EVENTS_PER_DRAIN - 1 {
            sender
                .send(WorkerEvent::Text(format!("before-{index}")))
                .unwrap();
        }
        sender
            .send(WorkerEvent::RunFinished {
                result: Err("Original failure".into()),
                terminal: Some(("original-run".into(), 7)),
            })
            .unwrap();
        sender
            .send(WorkerEvent::Text("After terminal".into()))
            .unwrap();

        let (events, end) = worker.drain().into_parts();
        assert_eq!(events.len(), MAX_EVENTS_PER_DRAIN);
        assert_eq!(end, WorkerBatchEnd::Finished);
        assert!(matches!(
            events.last(),
            Some(WorkerEvent::RunFinished {
                result: Err(error),
                terminal: Some((run, 7)),
            }) if error == "Original failure" && run == "original-run"
        ));
        assert!(matches!(
            worker.receiver.try_recv(),
            Ok(WorkerEvent::Text(text)) if text == "After terminal"
        ));
    }

    #[test]
    fn a_closed_channel_reports_disconnect_after_its_remainder_is_drained() {
        let (sender, receiver) = mpsc::sync_channel(MAX_EVENTS_PER_DRAIN + 1);
        let worker = Worker::run(
            receiver,
            CancellationToken::new(),
            "original-chat".into(),
            "original-run".into(),
            None,
        );
        for index in 0..(MAX_EVENTS_PER_DRAIN + 1) {
            sender
                .send(WorkerEvent::Progress(format!("step-{index}")))
                .unwrap();
        }
        drop(sender);

        let (first, end) = worker.drain().into_parts();
        assert_eq!(first.len(), MAX_EVENTS_PER_DRAIN);
        assert_eq!(end, WorkerBatchEnd::Pending);
        let (remainder, end) = worker.drain().into_parts();
        assert_eq!(end, WorkerBatchEnd::Disconnected);
        assert!(matches!(
            remainder.as_slice(),
            [WorkerEvent::Progress(text)] if text == "step-128"
        ));
    }
}
