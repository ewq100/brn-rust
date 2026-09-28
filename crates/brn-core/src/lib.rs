//! A UI-independent, in-memory shell model and one owned sample worker.
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Generation(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationId(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Progress {
        operation: OperationId,
        generation: Generation,
        completed: u32,
        total: u32,
    },
    Completed {
        operation: OperationId,
        generation: Generation,
        result: String,
    },
    Cancelled {
        operation: OperationId,
        generation: Generation,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartError {
    Busy(OperationId),
    Closed,
    InputTooLarge,
    Spawn(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    InputTooLarge,
}
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
#[derive(Clone, Copy, Debug)]
pub struct WorkConfig {
    pub steps: u32,
    pub step_delay: Duration,
}
impl Default for WorkConfig {
    fn default() -> Self {
        Self {
            steps: 240,
            step_delay: Duration::from_millis(12),
        }
    }
}
#[derive(Default)]
struct Mailbox {
    cancelled: bool,
    waiting: bool,
    progress: Option<Event>,
    terminal: Option<Event>,
}
struct Worker {
    shared: Arc<(Mutex<Mailbox>, Condvar)>,
    handle: JoinHandle<()>,
}
/// Owns one worker. Dropping the shell cancels and joins it before returning.
pub struct Shell {
    input: String,
    input_valid: bool,
    generation: Generation,
    next_operation: u64,
    active: Option<OperationId>,
    worker: Option<Worker>,
    progress: Option<(u32, u32)>,
    result: Option<String>,
    cancelled: bool,
    closed: bool,
}
impl Shell {
    pub fn new(input: impl Into<String>) -> Self {
        let input = input.into();
        Self {
            input_valid: input.len() <= MAX_INPUT_BYTES,
            input,
            generation: Generation(0),
            next_operation: 1,
            active: None,
            worker: None,
            progress: None,
            result: None,
            cancelled: false,
            closed: false,
        }
    }
    pub fn input(&self) -> &str {
        &self.input
    }
    pub fn input_is_valid(&self) -> bool {
        self.input_valid
    }
    pub fn generation(&self) -> Generation {
        self.generation
    }
    pub fn active(&self) -> Option<OperationId> {
        self.active
    }
    pub fn progress(&self) -> Option<(u32, u32)> {
        self.progress
    }
    pub fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }
    pub fn was_cancelled(&self) -> bool {
        self.cancelled
    }
    #[doc(hidden)]
    pub fn worker_waiting(&self) -> bool {
        self.worker.as_ref().is_some_and(|worker| {
            let mailbox = worker.shared.0.lock().unwrap_or_else(|e| e.into_inner());
            mailbox.waiting
        })
    }
    pub fn terminal_ready(&self) -> bool {
        self.worker.as_ref().is_some_and(|worker| {
            let mailbox = worker.shared.0.lock().unwrap_or_else(|e| e.into_inner());
            mailbox.terminal.is_some()
        })
    }
    pub fn pending_events(&self) -> usize {
        self.worker.as_ref().map_or(0, |worker| {
            let mailbox = worker.shared.0.lock().unwrap_or_else(|e| e.into_inner());
            usize::from(mailbox.progress.is_some()) + usize::from(mailbox.terminal.is_some())
        })
    }
    pub fn edit(&mut self, input: impl Into<String>) -> Result<(), EditError> {
        let input = input.into();
        self.generation.0 = self
            .generation
            .0
            .checked_add(1)
            .expect("working-copy generation exhausted");
        self.result = None;
        self.progress = None;
        self.cancelled = false;
        if input.len() > MAX_INPUT_BYTES {
            self.input_valid = false;
            return Err(EditError::InputTooLarge);
        }
        self.input = input;
        self.input_valid = true;
        Ok(())
    }
    pub fn start(&mut self, config: WorkConfig) -> Result<OperationId, StartError> {
        if self.closed {
            return Err(StartError::Closed);
        }
        if !self.input_valid {
            return Err(StartError::InputTooLarge);
        }
        self.poll();
        if let Some(id) = self.active {
            return Err(StartError::Busy(id));
        }
        if let Some(previous) = self.worker.take() {
            let _ = previous.handle.join();
        }
        let id = OperationId(self.next_operation);
        let generation = self.generation;
        let input = self.input.clone();
        let shared = Arc::new((Mutex::new(Mailbox::default()), Condvar::new()));
        let for_worker = Arc::clone(&shared);
        let handle = thread::Builder::new()
            .name("brn-sample-worker".into())
            .spawn(move || {
                let total = config.steps.max(1);
                for completed in 1..=total {
                    let (lock, wake) = &*for_worker;
                    let mut mailbox = lock.lock().unwrap_or_else(|e| e.into_inner());
                    if mailbox.cancelled {
                        mailbox.terminal = Some(Event::Cancelled {
                            operation: id,
                            generation,
                        });
                        mailbox.progress = None;
                        return;
                    }
                    mailbox.waiting = true;
                    let (next, _) = wake
                        .wait_timeout(mailbox, config.step_delay)
                        .unwrap_or_else(|e| e.into_inner());
                    mailbox = next;
                    mailbox.waiting = false;
                    if mailbox.cancelled {
                        mailbox.terminal = Some(Event::Cancelled {
                            operation: id,
                            generation,
                        });
                        mailbox.progress = None;
                        return;
                    }
                    mailbox.progress = Some(Event::Progress {
                        operation: id,
                        generation,
                        completed,
                        total,
                    });
                }
                let mut mailbox = for_worker.0.lock().unwrap_or_else(|e| e.into_inner());
                if mailbox.cancelled {
                    mailbox.terminal = Some(Event::Cancelled {
                        operation: id,
                        generation,
                    });
                } else {
                    mailbox.terminal = Some(Event::Completed {
                        operation: id,
                        generation,
                        result: sample_result(&input),
                    });
                }
                mailbox.progress = None;
            })
            .map_err(|e| StartError::Spawn(e.to_string()))?;
        self.worker = Some(Worker { shared, handle });
        self.active = Some(id);
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .expect("operation identity exhausted");
        self.result = None;
        self.progress = None;
        self.cancelled = false;
        Ok(id)
    }
    pub fn cancel(&mut self) -> bool {
        let Some(worker) = &self.worker else {
            return false;
        };
        let mut mailbox = worker.shared.0.lock().unwrap_or_else(|e| e.into_inner());
        if self.active.is_none() || mailbox.terminal.is_some() {
            return false;
        }
        mailbox.cancelled = true;
        worker.shared.1.notify_one();
        true
    }
    pub fn poll(&mut self) {
        let Some(worker) = &self.worker else {
            return;
        };
        let (progress, terminal) = {
            let mut mailbox = worker.shared.0.lock().unwrap_or_else(|e| e.into_inner());
            (mailbox.progress.take(), mailbox.terminal.take())
        };
        if let Some(event) = progress {
            self.apply_event(event);
        }
        if let Some(event) = terminal {
            self.apply_event(event);
            self.active = None;
            if let Some(worker) = self.worker.take() {
                let _ = worker.handle.join();
            }
        }
    }
    /// Returns whether an event matched the live operation and current working copy.
    pub fn apply_event(&mut self, event: Event) -> bool {
        let (operation, generation) = match &event {
            Event::Progress {
                operation,
                generation,
                ..
            }
            | Event::Completed {
                operation,
                generation,
                ..
            }
            | Event::Cancelled {
                operation,
                generation,
            } => (*operation, *generation),
        };
        if self.active != Some(operation) || self.generation != generation {
            return false;
        }
        match event {
            Event::Progress {
                completed, total, ..
            } => self.progress = Some((completed, total)),
            Event::Completed { result, .. } => {
                self.result = Some(result);
                self.progress = None;
            }
            Event::Cancelled { .. } => {
                self.cancelled = true;
                self.progress = None;
            }
        }
        true
    }
    pub fn close(&mut self) {
        self.closed = true;
        if let Some(worker) = self.worker.take() {
            {
                let mut mailbox = worker.shared.0.lock().unwrap_or_else(|e| e.into_inner());
                mailbox.cancelled = true;
                worker.shared.1.notify_one();
            }
            let _ = worker.handle.join();
        }
        self.active = None;
    }
}
impl Drop for Shell {
    fn drop(&mut self) {
        self.close();
    }
}
fn sample_result(input: &str) -> String {
    format!(
        "Sample result: {} words, {} characters. Preview: {}",
        input.split_whitespace().count(),
        input.chars().count(),
        input.chars().take(80).collect::<String>()
    )
}
