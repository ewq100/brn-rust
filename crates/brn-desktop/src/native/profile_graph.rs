//! A fixed one-hop projection of one validated relationship page, never a graph store.
use super::*;
use brn_workflow::{
    knowledge::{
        EdgeEndpoint, EdgeOrigin, ProfileContext, ProfileContextRequest, ProfileRelationship,
    },
    library::KnowledgeScope,
    proposals::ProposalSource,
};
use gpui_kit::{AnyElement, PathBuilder, TestSupportExt, base::Disableable, canvas, rgb};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Node {
    pub(super) endpoint: EdgeEndpoint,
    pub(super) scope: KnowledgeScope,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rect {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}
impl Rect {
    fn middle_right(self) -> (f32, f32) {
        (self.x + self.width, self.y + self.height / 2.)
    }
    fn middle_left(self) -> (f32, f32) {
        (self.x, self.y + self.height / 2.)
    }
}
pub(super) struct Projection {
    pub(super) nodes: Vec<Node>,
    /// Source and target node indexes, in the original exact relationship order.
    pub(super) edges: Vec<(usize, usize)>,
    pub(super) bounds: Vec<Rect>,
    pub(super) width: f32,
    pub(super) height: f32,
}
impl Projection {
    pub(super) fn new(context: &ProfileContext) -> Self {
        let mut nodes = vec![Node {
            endpoint: EdgeEndpoint {
                path: context.profile.source.path.clone(),
                note_id: context.request.note_id,
                sha256: context.profile.source.fingerprint.sha256,
            },
            scope: KnowledgeScope::Current,
        }];
        let mut edges = Vec::with_capacity(context.relationships.len());
        for relationship in &context.relationships {
            let mut endpoints = [0; 2];
            for (index, (endpoint, scope)) in [
                (&relationship.edge.source, relationship.source_scope),
                (&relationship.edge.target, relationship.target_scope),
            ]
            .into_iter()
            .enumerate()
            {
                let node = Node {
                    endpoint: endpoint.clone(),
                    scope,
                };
                endpoints[index] = nodes
                    .iter()
                    .position(|existing| *existing == node)
                    .unwrap_or_else(|| {
                        nodes.push(node);
                        nodes.len() - 1
                    });
            }
            edges.push((endpoints[0], endpoints[1]));
        }
        // Two columns leave the arrows unobscured; every control owns a disjoint rectangle.
        // This extent grows with the page and is scrollable in both axes.
        let height = (nodes.len().saturating_sub(1).max(1) as f32 * 88.) + 8.;
        let bounds = (0..nodes.len())
            .map(|index| Rect {
                x: if index == 0 { 16. } else { 472. },
                y: if index == 0 {
                    (height - 64.) / 2.
                } else {
                    16. + (index - 1) as f32 * 88.
                },
                width: 280.,
                height: 64.,
            })
            .collect();
        Self {
            nodes,
            edges,
            bounds,
            width: 768.,
            height,
        }
    }
}

/// Full profile, page and generation binding; indexes are only locators after exact comparison.
#[derive(Clone)]
pub(super) struct Binding {
    generation: u64,
    intent: Option<Uuid>,
    request: ProfileContextRequest,
    profile: ProposalSource,
}
impl Binding {
    pub(super) fn capture(desktop: &Desktop, context: &ProfileContext) -> Self {
        Self {
            generation: desktop.ai.as_ref().unwrap().profile_context.generation(),
            intent: desktop.ai.as_ref().unwrap().profile_context.intent,
            request: context.request.clone(),
            profile: context.profile.clone(),
        }
    }
    fn matches(&self, desktop: &Desktop) -> bool {
        let ai = desktop.ai.as_ref().unwrap();
        desktop.profile_context.open
            && !desktop.inspection_blocked()
            && !ai.profile_context_loading()
            && ai.profile_context.generation() == self.generation
            && ai.profile_context.intent == self.intent
            && ai.profile_context.context.as_ref().is_some_and(|context| {
                context.request == self.request && context.profile == self.profile
            })
    }
}
fn origin(origin: EdgeOrigin) -> &'static str {
    match origin {
        EdgeOrigin::ExplicitLink => "Explicit Markdown link",
        EdgeOrigin::InferredProvenance => "Inferred provenance candidate",
    }
}
fn identity(node: &Node) -> String {
    let hash = node
        .endpoint
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "{} · {} · UUID {} · SHA-256 {hash}",
        crate::ai::scope_name(node.scope),
        node.endpoint.path,
        node.endpoint.note_id
    )
}
impl Desktop {
    pub(super) fn select_graph_edge(
        &mut self,
        binding: &Binding,
        index: usize,
        relationship: &ProfileRelationship,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !binding.matches(self)
            || self
                .ai
                .as_ref()
                .unwrap()
                .profile_context
                .context
                .as_ref()
                .and_then(|context| context.relationships.get(index))
                != Some(relationship)
        {
            return;
        }
        self.profile_context.relationship = index;
        self.profile_context.proof = 0;
        self.profile_context.graph_selection = Some(format!(
            "Selected edge {}: {} → {} · {}. Complete exact proof below.",
            index + 1,
            relationship.edge.source.path,
            relationship.edge.target.path,
            origin(relationship.edge.origin)
        ));
        self.sync_profile_context_widgets(window, cx);
        cx.notify();
    }
    pub(super) fn select_graph_node(
        &mut self,
        binding: &Binding,
        node: &Node,
        cx: &mut Context<Self>,
    ) {
        if !binding.matches(self) {
            return;
        }
        let context = self
            .ai
            .as_ref()
            .unwrap()
            .profile_context
            .context
            .as_ref()
            .unwrap();
        let projection = Projection::new(context);
        let Some(index) = projection
            .nodes
            .iter()
            .position(|candidate| candidate == node)
        else {
            return;
        };
        self.profile_context.graph_selection = Some(identity(node));
        if index != 0 {
            self.open_profile_support(node.endpoint.path.clone(), node.scope, cx);
        }
        cx.notify();
    }
    pub(super) fn render_profile_graph(
        &self,
        context: &ProfileContext,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let projection = Projection::new(context);
        let node_count = projection.nodes.len();
        let binding = std::rc::Rc::new(Binding::capture(self, context));
        let blocked =
            self.inspection_blocked() || self.ai.as_ref().unwrap().profile_context_loading();
        let lines = projection
            .edges
            .iter()
            .enumerate()
            .map(|(index, &(source, target))| {
                let (start, end) = if source == target {
                    let anchor = projection.bounds[source].middle_right();
                    (anchor, anchor)
                } else if source == 0 {
                    (
                        projection.bounds[source].middle_right(),
                        projection.bounds[target].middle_left(),
                    )
                } else {
                    (
                        projection.bounds[source].middle_left(),
                        projection.bounds[target].middle_right(),
                    )
                };
                (start, end, context.relationships[index].edge.origin)
            })
            .collect::<Vec<_>>();
        let mut extent = div()
            .relative()
            .w(px(projection.width))
            .h(px(projection.height))
            .flex_shrink_0()
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        for (start, end, edge_origin) in lines {
                            let to_point = |(x, y)| bounds.origin + point(px(x), px(y));
                            let mut path = PathBuilder::stroke(px(1.5));
                            if edge_origin == EdgeOrigin::InferredProvenance {
                                path = path.dash_array(&[px(5.), px(4.)]);
                            }
                            path.move_to(to_point(start));
                            if start == end {
                                // A self relationship keeps an explicit visible loop and its own edge control.
                                path.line_to(to_point((start.0 + 36., start.1 - 48.)));
                                path.line_to(to_point((end.0, end.1 - 32.)));
                            } else {
                                path.line_to(to_point(end));
                            }
                            let direction = if end.0 >= start.0 { -1. } else { 1. };
                            path.move_to(to_point((end.0 + direction * 9., end.1 - 5.)));
                            path.line_to(to_point(end));
                            path.line_to(to_point((end.0 + direction * 9., end.1 + 5.)));
                            if let Ok(path) = path.build() {
                                window.paint_path(path, rgb(0x708090));
                            }
                        }
                    },
                )
                .absolute()
                .size_full(),
            );
        for (index, (node, bounds)) in projection
            .nodes
            .into_iter()
            .zip(projection.bounds)
            .enumerate()
        {
            let label = format!(
                "{} · {}",
                crate::ai::scope_name(node.scope),
                node.endpoint.path
            );
            let accessible = format!(
                "{}: {}",
                if index == 0 {
                    "Reveal retained profile"
                } else {
                    "Open supporting saved Markdown"
                },
                identity(&node)
            );
            let binding = binding.clone();
            extent = extent.child(
                div()
                    .absolute()
                    .left(px(bounds.x))
                    .top(px(bounds.y))
                    .w(px(bounds.width))
                    .h(px(bounds.height))
                    .child(
                        Button::new(format!("profile-graph-node-{index}"))
                            .label(label)
                            .accessibility_label(accessible)
                            .size_full()
                            .disabled(blocked)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_graph_node(&binding, &node, cx)
                            })),
                    ),
            );
        }
        let summary = format!(
            "Graph: relationship offset {} · {} displayed edges · {} total direct relationships · {} distinct page nodes (including Current center)",
            context.request.relationship_offset,
            context.relationships.len(),
            context.relationship_total,
            node_count
        );
        let mut content = div().w_full().min_w_0().flex().flex_col().gap_2().flex_shrink_0()
            .child(div().id("profile-graph-summary").aria_label(summary.clone()).child(summary).test_support())
            .child("One displayed page only. Arrows point Source → Target. Solid: Explicit Markdown link. Dashed: Inferred provenance candidate. Scope labels describe saved Markdown, not profile membership.")
            .child(div().id("profile-graph-scroll").w_full().min_w_0().h(px(320.)).flex_shrink_0()
                .track_scroll(&self.profile_context.graph_scroll).overflow_scroll()
                .vertical_scrollbar(&self.profile_context.graph_scroll)
                .horizontal_scrollbar(&self.profile_context.graph_scroll)
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation()).child(extent).test_support());
        if let Some(selection) = &self.profile_context.graph_selection {
            content = content.child(selection.clone());
        }
        content = content.child("Select any directed edge for its complete exact proof (separate controls preserve overlapping arrows):");
        for (index, relationship) in context.relationships.iter().enumerate() {
            let label = format!(
                "Edge {} · {} {} → {} {} · {} · {} proofs",
                index + 1,
                crate::ai::scope_name(relationship.source_scope),
                relationship.edge.source.path,
                crate::ai::scope_name(relationship.target_scope),
                relationship.edge.target.path,
                origin(relationship.edge.origin),
                relationship.edge.evidence.len()
            );
            let binding = binding.clone();
            let relationship = relationship.clone();
            content = content.child(
                Button::new(format!("profile-graph-edge-{index}"))
                    .label(label.clone())
                    .accessibility_label(label)
                    .compact()
                    .disabled(blocked)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_graph_edge(&binding, index, &relationship, window, cx)
                    })),
            );
        }
        content.into_any_element()
    }
}
