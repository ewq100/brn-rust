//! Bounded presentation state for Run progress activity.

#[derive(Debug, Default)]
pub(super) struct RunActivity {
    state: State,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Live {
        chat: String,
        steps: Vec<String>,
    },
    Ended {
        chat: String,
        steps: Vec<String>,
        status: String,
    },
}

#[derive(Debug)]
pub(super) enum Event<'a> {
    Started(&'a str),
    Observed { chat: &'a str, text: &'a str },
    Finished { chat: &'a str, status: &'a str },
    Disconnected(&'a str),
}

impl RunActivity {
    /// Apply an event and report whether it appended a visible step.
    pub(super) fn transition(&mut self, event: Event<'_>) -> bool {
        match event {
            Event::Started(chat) => {
                self.state = State::Live {
                    chat: chat.to_owned(),
                    steps: Vec::new(),
                };
                false
            }
            Event::Observed { chat, text } => {
                if text.trim().is_empty() {
                    return false;
                }
                let steps = match &mut self.state {
                    State::Idle => {
                        self.state = State::Live {
                            chat: chat.to_owned(),
                            steps: Vec::new(),
                        };
                        let State::Live { steps, .. } = &mut self.state else {
                            unreachable!()
                        };
                        steps
                    }
                    State::Live { chat: owner, steps } if owner == chat => steps,
                    State::Live { .. } | State::Ended { .. } => return false,
                };
                if steps.last().is_some_and(|last| last == text) {
                    return false;
                }
                steps.push(text.chars().take(8192).collect());
                if steps.len() > 32 {
                    steps.remove(0);
                }
                true
            }
            Event::Finished { chat, status } => {
                let previous = std::mem::replace(&mut self.state, State::Idle);
                self.state = match previous {
                    State::Live { chat: owner, steps } if owner == chat => State::Ended {
                        chat: owner,
                        steps,
                        status: status.to_owned(),
                    },
                    previous => previous,
                };
                false
            }
            Event::Disconnected(chat) => {
                let previous = std::mem::replace(&mut self.state, State::Idle);
                self.state = match previous {
                    State::Live { chat: owner, steps } if owner == chat => State::Ended {
                        chat: owner,
                        steps,
                        status: "Disconnected".into(),
                    },
                    previous => previous,
                };
                false
            }
        }
    }

    pub(super) fn steps(&self) -> &[String] {
        match &self.state {
            State::Live { steps, .. } | State::Ended { steps, .. } => steps,
            State::Idle => &[],
        }
    }

    pub(super) fn status(&self) -> &str {
        match &self.state {
            State::Ended { status, .. } => status,
            State::Idle | State::Live { .. } => "",
        }
    }

    pub(super) fn has_steps_for(&self, chat: &str) -> bool {
        match &self.state {
            State::Live { chat: owner, steps }
            | State::Ended {
                chat: owner, steps, ..
            } => owner == chat && !steps.is_empty(),
            State::Idle => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, RunActivity};

    #[test]
    fn start_resets_steps_and_terminal_status_retains_them() {
        let mut activity = RunActivity::default();
        activity.transition(Event::Started("chat-a"));
        activity.transition(Event::Observed {
            chat: "chat-a",
            text: "reading",
        });
        activity.transition(Event::Finished {
            chat: "chat-a",
            status: "Succeeded",
        });
        assert_eq!(activity.steps(), &["reading"]);
        assert_eq!(activity.status(), "Succeeded");
        activity.transition(Event::Started("chat-b"));
        assert!(activity.steps().is_empty());
        assert_eq!(activity.status(), "");
        assert!(!activity.has_steps_for("chat-b"));
    }

    #[test]
    fn context_and_terminal_events_reject_stale_observations() {
        let mut activity = RunActivity::default();
        assert!(activity.transition(Event::Observed {
            chat: "fixture-chat",
            text: "first",
        }));
        assert!(!activity.transition(Event::Observed {
            chat: "other-chat",
            text: "wrong context",
        }));
        activity.transition(Event::Disconnected("fixture-chat"));
        assert_eq!(activity.status(), "Disconnected");
        assert!(!activity.transition(Event::Observed {
            chat: "fixture-chat",
            text: "late",
        }));
        assert_eq!(activity.steps(), &["first"]);
    }

    #[test]
    fn observations_keep_existing_empty_duplicate_and_unicode_bounds() {
        let mut activity = RunActivity::default();
        activity.transition(Event::Started("chat"));
        assert!(!activity.transition(Event::Observed {
            chat: "chat",
            text: "  \n",
        }));
        assert!(activity.transition(Event::Observed {
            chat: "chat",
            text: "same",
        }));
        assert!(!activity.transition(Event::Observed {
            chat: "chat",
            text: "same",
        }));
        for index in 0..33 {
            activity.transition(Event::Observed {
                chat: "chat",
                text: &format!("step {index}"),
            });
        }
        activity.transition(Event::Observed {
            chat: "chat",
            text: &"é".repeat(8193),
        });
        assert_eq!(activity.steps().len(), 32);
        assert_eq!(activity.steps()[0], "step 2");
        assert_eq!(activity.steps()[31].chars().count(), 8192);
        assert!(
            activity.steps()[31]
                .chars()
                .all(|character| character == 'é')
        );
    }
}
