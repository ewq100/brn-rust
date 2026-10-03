use super::*;

impl Desktop {
    pub(super) fn render_vault_rail(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.render_simple_vault(cx)
    }
}
