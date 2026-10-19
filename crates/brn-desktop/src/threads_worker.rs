//! The current worker's channel protocol, not the full Run lifecycle.
//!
//! This boundary admits one old-worker batch through its first terminal event.
//! The host still owns presentation and consumes that terminal before replacing
//! the worker; the existing checked service owns durable Run and queue effects.
use super::{ConversionResult, DocumentConversionSettings, WorkerKind};
use std::sync::{Arc, atomic::AtomicBool, mpsc::Receiver};
use tokio_util::sync::CancellationToken;

pub(super) enum WorkerEvent {
    AnswerFinalized(String),
    Progress(String),
    Activity(String),
    Text(String),
    Finished(Result<String, String>),
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
    pub(super) kind: WorkerKind,
    /// The native stream and cancellation belong to this invocation's thread,
    /// independently of whichever thread the owner is currently viewing.
    pub(super) thread: Option<String>,
    /// Created synchronously before the provider worker can start.
    pub(super) run_id: Option<String>,
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
    /// Drain currently available events from this receiver only. The first
    /// terminal wins even if the producer then sends more events or disconnects.
    /// The host must retire this worker after consuming a terminal batch, before
    /// dispatching another invocation; no old events can reach its successor.
    pub(super) fn drain(&self) -> WorkerBatch {
        let mut batch = WorkerBatch {
            events: Vec::new(),
            end: WorkerBatchEnd::Pending,
        };
        loop {
            match self.receiver.try_recv() {
                Ok(event) => {
                    if batch.end == WorkerBatchEnd::Finished {
                        continue;
                    }
                    if matches!(
                        event,
                        WorkerEvent::Finished(_)
                            | WorkerEvent::RunFinished { .. }
                            | WorkerEvent::Converted { .. }
                    ) {
                        batch.end = WorkerBatchEnd::Finished;
                    }
                    batch.events.push(event);
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
            Worker {
                receiver,
                cancel: CancellationToken::new(),
                conversion_cancel: Arc::new(AtomicBool::new(false)),
                kind: WorkerKind::Run,
                thread: Some("original-chat".into()),
                run_id: Some("original-run".into()),
            },
        )
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
}
