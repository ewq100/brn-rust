use brn_core::{Event, Generation, OperationId, Shell, StartError, WorkConfig};
use std::time::{Duration, Instant};

fn slow() -> WorkConfig {
    WorkConfig {
        steps: 100,
        step_delay: Duration::from_millis(5),
    }
}
fn wait_terminal(shell: &mut Shell) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while shell.active().is_some() && Instant::now() < deadline {
        shell.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(shell.active().is_none(), "worker did not finish in time");
}

#[test]
fn edit_always_advances_generation_even_when_text_returns() {
    let mut shell = Shell::new("A");
    shell.edit("B").unwrap();
    shell.edit("A").unwrap();
    assert_eq!(shell.generation(), Generation(2));
}
#[test]
fn stale_completion_cannot_replace_current_result() {
    let mut shell = Shell::new("a");
    let id = shell
        .start(WorkConfig {
            steps: 1,
            step_delay: Duration::ZERO,
        })
        .unwrap();
    shell.edit("b").unwrap();
    assert!(!shell.apply_event(Event::Completed {
        operation: id,
        generation: Generation(0),
        result: "OLD".into()
    }));
    assert!(shell.result().is_none());
    wait_terminal(&mut shell);
    assert!(shell.result().is_none());
}
#[test]
fn concurrent_start_is_refused_and_completed_worker_can_be_reused() {
    let mut shell = Shell::new("words");
    let first = shell.start(slow()).unwrap();
    assert_eq!(shell.start(slow()), Err(StartError::Busy(first)));
    wait_terminal(&mut shell);
    let second = shell
        .start(WorkConfig {
            steps: 1,
            step_delay: Duration::ZERO,
        })
        .unwrap();
    assert_ne!(first, second);
    wait_terminal(&mut shell);
    assert!(shell.result().unwrap().contains("words"));
}
#[test]
fn accepted_cancellation_wins_over_completion() {
    let mut shell = Shell::new("sample");
    shell.start(slow()).unwrap();
    assert!(shell.cancel());
    wait_terminal(&mut shell);
    assert!(shell.was_cancelled());
    assert!(shell.result().is_none());
}
#[test]
fn close_during_work_is_bounded() {
    let before = Instant::now();
    let mut shell = Shell::new("sample");
    shell.start(slow()).unwrap();
    drop(shell);
    assert!(before.elapsed() < Duration::from_secs(1));
}
#[test]
fn progress_is_coalesced_and_terminal_survives_backpressure() {
    let mut shell = Shell::new("sample");
    shell
        .start(WorkConfig {
            steps: 1000,
            step_delay: Duration::ZERO,
        })
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    assert!(shell.pending_events() <= 2);
    wait_terminal(&mut shell);
    assert!(shell.result().is_some());
}
#[test]
fn unrelated_operation_event_is_ignored() {
    let mut shell = Shell::new("sample");
    shell.start(slow()).unwrap();
    assert!(!shell.apply_event(Event::Completed {
        operation: OperationId(999),
        generation: Generation(0),
        result: "wrong".into()
    }));
    assert!(shell.result().is_none());
    shell.cancel();
    wait_terminal(&mut shell);
}

#[test]
fn oversized_edit_invalidates_current_generation_until_valid_input() {
    let mut shell = Shell::new("safe");
    assert!(
        shell
            .edit("x".repeat(brn_core::MAX_INPUT_BYTES + 1))
            .is_err()
    );
    assert_eq!(shell.input(), "safe");
    assert_eq!(shell.generation(), Generation(1));
    assert_eq!(
        shell.start(WorkConfig::default()),
        Err(StartError::InputTooLarge)
    );
    shell.edit("safe again").unwrap();
    assert!(
        shell
            .start(WorkConfig {
                steps: 1,
                step_delay: Duration::ZERO
            })
            .is_ok()
    );
    wait_terminal(&mut shell);
}

#[test]
fn oversized_edit_while_running_discards_old_completion() {
    let mut shell = Shell::new("old");
    shell.start(slow()).unwrap();
    assert!(
        shell
            .edit("x".repeat(brn_core::MAX_INPUT_BYTES + 1))
            .is_err()
    );
    wait_terminal(&mut shell);
    assert!(shell.result().is_none());
}
#[test]
fn completed_terminal_survives_late_cancel_before_poll() {
    let mut shell = Shell::new("finished");
    shell
        .start(WorkConfig {
            steps: 1,
            step_delay: Duration::ZERO,
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while !shell.terminal_ready() && Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(shell.terminal_ready());
    assert!(!shell.cancel());
    shell.poll();
    assert!(shell.result().unwrap().contains("finished"));
    assert!(!shell.was_cancelled());
}
#[test]
fn close_interrupts_long_worker_wait() {
    let mut shell = Shell::new("long");
    shell
        .start(WorkConfig {
            steps: 1,
            step_delay: Duration::from_secs(30),
        })
        .unwrap();
    let entered_deadline = Instant::now() + Duration::from_secs(1);
    while !shell.worker_waiting() && Instant::now() < entered_deadline {
        std::thread::yield_now();
    }
    assert!(shell.worker_waiting(), "worker did not enter timed wait");
    let before = Instant::now();
    shell.close();
    assert!(before.elapsed() < Duration::from_secs(1));
    assert_eq!(shell.start(WorkConfig::default()), Err(StartError::Closed));
}
#[test]
fn stale_progress_cannot_update_current_progress() {
    let mut shell = Shell::new("old");
    let id = shell.start(slow()).unwrap();
    shell.edit("new").unwrap();
    assert!(!shell.apply_event(Event::Progress {
        operation: id,
        generation: Generation(0),
        completed: 9,
        total: 10
    }));
    assert!(shell.progress().is_none());
    shell.cancel();
    wait_terminal(&mut shell);
}
