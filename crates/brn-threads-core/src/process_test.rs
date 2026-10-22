//! Owned child-process support shared by bounded Unix process witnesses.
use std::{
    io,
    process::{Child, Command, ExitStatus},
};

/// Reaps the child on explicit completion and on every parent unwind path.
pub(crate) struct OwnedChild(Option<Child>);

impl OwnedChild {
    pub(crate) fn spawn(command: &mut Command) -> io::Result<Self> {
        command.spawn().map(|child| Self(Some(child)))
    }

    pub(crate) fn try_wait(&mut self) -> Option<ExitStatus> {
        self.0
            .as_mut()
            .expect("owned child is present")
            .try_wait()
            .expect("child status is readable")
    }

    pub(crate) fn kill_and_reap(&mut self) -> ExitStatus {
        let child = self.0.as_mut().expect("owned child is present");
        let status = match child.try_wait().expect("child status is readable") {
            Some(status) => status,
            None => {
                child.kill().expect("child can be killed");
                child.wait().expect("killed child can be reaped")
            }
        };
        self.0.take();
        status
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}
