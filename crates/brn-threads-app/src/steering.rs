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
        let binding = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                "stop-and-post",
                run_id,
                thread,
                text
            ))?)
        );
        let operation = self.store.allocate_bound_operation(key, &binding)?;
        applied_receipt(self.store.stop_and_post(
            &operation,
            run_id,
            thread,
            text,
            &HostAuthority::owner("owner stopped and sent a new instruction"),
        )?)
    }
}
