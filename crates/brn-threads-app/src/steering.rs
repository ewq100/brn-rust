use crate::*;

impl Workspace {
    /// The caller keeps this key and exact intent stable until it receives the
    /// durable receipt. Allocation binds intent even if preparation fails.
    pub fn stop_and_post(
        &mut self,
        key: &str,
        run_id: &str,
        thread: &str,
        text: &str,
    ) -> AppResult<Receipt> {
        self.stop_and_post_with_partial(key, run_id, thread, text, None)
    }
    pub fn stop_and_post_with_partial(
        &mut self,
        key: &str,
        run_id: &str,
        thread: &str,
        text: &str,
        partial: Option<&str>,
    ) -> AppResult<Receipt> {
        let binding = format!(
            "{:x}",
            Sha256::digest(if let Some(partial) = partial {
                serde_json::to_vec(&("stop-and-post-partial", run_id, thread, text, partial))?
            } else {
                serde_json::to_vec(&("stop-and-post", run_id, thread, text))?
            })
        );
        let operation = self.store.allocate_bound_operation(key, &binding)?;
        applied_receipt(self.store.stop_and_post_with_partial(
            &operation,
            run_id,
            thread,
            text,
            partial,
            &HostAuthority::owner("owner stopped and sent a new instruction"),
        )?)
    }
}
