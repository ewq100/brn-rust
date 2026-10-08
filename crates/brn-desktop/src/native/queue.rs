//! "Needs you": one queue of everything waiting for the owner (D24).
//!
//! Presentation only. Each section reads the page its own view already loads
//! (Actions dashboard, proposals, Inbox, Needs Review); rows open those views,
//! which keep every existing command, guard and approval.
use super::simple::EditorTransition;
use super::theme::color;
use super::ui::{self, Tone};
use super::*;
use crate::ai::Pending;
use brn_workflow::{app_worker::AppCommand, dashboard::DashboardFilter, proposals::ProposalState};
use gpui_kit::{
    AnyElement, TestSupportExt,
    assets::IconName,
    base::Disableable,
    component::{Selectable, WindowExt},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum QueueFilter {
    #[default]
    All,
    Due,
    Decide,
    Sort,
    Check,
}

/// Last counts seen from each section's page. Pages are dropped when their
/// view closes; the sidebar keeps showing the last known values.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Attention {
    pub due: Option<u64>,
    pub overdue: Option<u64>,
    pub sort: Option<usize>,
    pub check: Option<usize>,
}

pub(super) fn awaiting(record: &brn_workflow::proposals::ProposalRecord) -> bool {
    !matches!(
        record.state,
        ProposalState::Applied | ProposalState::Rejected
    )
}

impl Desktop {
    /// Opens the queue and loads every page it summarises.
    pub(super) fn open_queue_pages(&mut self, cx: &mut Context<Self>) {
        let ai = self.ai.as_mut().unwrap();
        ai.dashboard.filter = DashboardFilter::Active;
        ai.finding_queue.state = Some(brn_workflow::findings::FindingState::Open);
        let commands = [ai.open_dashboard(), ai.open_inbox(), ai.open_findings()];
        for command in commands.into_iter().flatten() {
            self.simple_send(command, cx);
        }
    }

    pub(super) fn refresh_attention(&mut self) {
        let ai = self.ai.as_ref().unwrap();
        if let Some(page) = &ai.dashboard.page {
            // An Action can be both overdue and due for follow-up; count it once.
            let due = page
                .entries
                .iter()
                .filter(|entry| entry.overdue || entry.follow_up)
                .count() as u64;
            self.attention.due = Some(due);
            self.attention.overdue = Some(page.counts.overdue);
        }
        if let Some(page) = &ai.inbox_queue.page {
            self.attention.sort = Some(page.total_count);
        }
        if let Some(page) = &ai.finding_queue.page {
            self.attention.check = Some(page.open_count);
        }
    }

    /// Sidebar count: everything known to need the owner, with the most urgent tone.
    pub(super) fn attention_signal(&self) -> Option<(String, Tone)> {
        let ai = self.ai.as_ref().unwrap();
        let decide = ai
            .proposals
            .iter()
            .filter(|record| awaiting(record))
            .count() as u64;
        let a = self.attention;
        let total =
            decide + a.due.unwrap_or(0) + a.sort.unwrap_or(0) as u64 + a.check.unwrap_or(0) as u64;
        let tone = if a.overdue.unwrap_or(0) > 0 {
            Tone::Danger
        } else {
            Tone::Attention
        };
        (total > 0).then(|| (total.to_string(), tone))
    }

    #[inline(never)]
    pub(super) fn render_queue(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.refresh_attention();
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let blocked = !ai.ready
            || self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed;
        let filter = self.queue_filter;
        let show = |section: QueueFilter| filter == QueueFilter::All || filter == section;

        let due_entries: Vec<_> = ai
            .dashboard
            .page
            .iter()
            .flat_map(|page| page.entries.iter())
            .filter(|entry| entry.overdue || entry.follow_up)
            .collect();
        let decide: Vec<_> = ai.proposals.iter().filter(|r| awaiting(r)).collect();
        let sort = ai.inbox_queue.page.as_ref();
        let check = ai.finding_queue.page.as_ref();
        let a = self.attention;
        let counts = [
            (QueueFilter::All, "All", None),
            (QueueFilter::Due, "Due", a.due),
            (QueueFilter::Decide, "Decide", Some(decide.len() as u64)),
            (QueueFilter::Sort, "Sort", a.sort.map(|n| n as u64)),
            (QueueFilter::Check, "Check", a.check.map(|n| n as u64)),
        ];
        let total: u64 = counts.iter().filter_map(|(_, _, n)| *n).sum();

        let mut summary = vec![];
        if a.overdue.unwrap_or(0) > 0 {
            summary.push(format!("{} overdue", a.overdue.unwrap_or(0)));
        }
        summary.push(format!("{} to decide", decide.len()));
        if let Some(n) = a.sort {
            summary.push(format!("{n} to sort"));
        }
        if let Some(n) = a.check {
            summary.push(format!("{n} to check"));
        }

        let mut chips = ui::toolbar().gap(px(tokens::space::XS));
        for (section, label, count) in counts {
            let label = match (section, count) {
                (QueueFilter::All, _) => format!("All {total}"),
                (_, Some(n)) => format!("{label} {n}"),
                (_, None) => label.to_owned(),
            };
            chips = chips.child(
                Button::new(format!("queue-filter-{section:?}").to_lowercase())
                    .label(label)
                    .small()
                    .when(filter == section, |b| b.primary())
                    .when(filter != section, |b| b.ghost())
                    .selected(filter == section)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.queue_filter = section;
                        cx.notify();
                    })),
            );
        }

        let heading = |label: &str, link: Option<Button>| {
            let mut row = div()
                .flex()
                .items_center()
                .pt(px(tokens::space::MD))
                .child(ui::section_label(label.to_owned(), p).px_0().flex_1());
            if let Some(link) = link {
                row = row.child(div().pt(px(tokens::space::SM)).child(link));
            }
            row
        };
        let link =
            |id: &'static str, label: &'static str| Button::new(id).label(label).ghost().xsmall();

        let mut body = div()
            .id("queue-scroll")
            .test_support()
            .track_scroll(&self.queue_scroll)
            .overflow_y_scroll()
            .vertical_scrollbar(&self.queue_scroll)
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .px(px(tokens::space::LG))
            .pb(px(tokens::space::XL))
            .child(chips.pt(px(tokens::space::MD)));

        if show(QueueFilter::Due) {
            body = body.child(heading(
                "Due",
                Some(
                    link("open-dashboard", "All Actions")
                        .disabled(!ai.ready)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if !window.has_active_dialog(cx) {
                                this.simple_leave(EditorTransition::Dashboard, cx);
                            }
                        })),
                ),
            ));
            // The page holds the newest active Actions; the counts cover them all.
            let missing = ai.dashboard.page.as_ref().is_some_and(|page| {
                let overdue = due_entries.iter().filter(|entry| entry.overdue).count() as u64;
                let follow = due_entries.iter().filter(|entry| entry.follow_up).count() as u64;
                page.counts.overdue > overdue || page.counts.follow_up > follow
            });
            if ai.dashboard.page.is_none() {
                body = body.child(ui::hint("Loading Actions…", p).py(px(tokens::space::SM)));
            } else if due_entries.is_empty() && !missing {
                body = body.child(
                    ui::hint("Nothing overdue or due for follow-up.", p).py(px(tokens::space::SM)),
                );
            }
            if missing {
                body = body.child(
                    ui::hint(
                        "More overdue or follow-up Actions are older than this list. Open All Actions to see them.",
                        p,
                    )
                    .py(px(tokens::space::SM)),
                );
            }
            for entry in due_entries {
                let data = &entry.action.data;
                let (badge, detail) = if entry.overdue {
                    (
                        ("overdue", Tone::Danger),
                        format!("Action · due {}", data.due_on.clone().unwrap_or_default()),
                    )
                } else {
                    (
                        ("follow up", Tone::Attention),
                        format!(
                            "Action · follow up {}",
                            data.follow_up_on.clone().unwrap_or_default()
                        ),
                    )
                };
                body = body.child(
                    ui::list_row(
                        format!("queue-action-{}", entry.action.origin.id),
                        data.title.clone(),
                        Some(detail),
                        Some(ui::badge(badge.0, badge.1, p).into_any_element()),
                        p,
                    )
                    .disabled(blocked)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.simple_leave(EditorTransition::Dashboard, cx)
                    })),
                );
            }
        }

        if show(QueueFilter::Decide) {
            body = body.child(
                heading("Decide", None).child(
                    ui::toolbar()
                        .gap(px(2.))
                        .pt(px(tokens::space::SM))
                        .child(
                            Button::new("new-proposal-form")
                                .icon(IconName::Plus)
                                .label("Proposal")
                                .ghost()
                                .xsmall()
                                .tooltip("Draft a new note proposal for review")
                                .disabled(
                                    !ai.ready
                                        || !ai.vault_bound
                                        || ai.application_busy()
                                        || blocked,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.simple_leave(EditorTransition::Draft(None), cx)
                                })),
                        )
                        .child(
                            Button::new("new-action-form")
                                .icon(IconName::Plus)
                                .label("Action")
                                .ghost()
                                .xsmall()
                                .tooltip("Draft a new Action for review")
                                .disabled(!ai.ready || ai.application_busy() || blocked)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if !window.has_active_dialog(cx) {
                                        this.simple_leave(EditorTransition::ActionDraft(None), cx);
                                    }
                                })),
                        )
                        .child(
                            Button::new("refresh-proposal-list")
                                .icon(IconName::RotateCw)
                                .ghost()
                                .xsmall()
                                .tooltip("Refresh proposals")
                                .disabled(!ai.ready)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.simple_command(
                                        Pending::Proposals,
                                        AppCommand::Proposals(None),
                                        cx,
                                    )
                                })),
                        ),
                ),
            );
            if decide.is_empty() {
                body = body.child(
                    ui::hint("No proposals waiting for your decision.", p)
                        .py(px(tokens::space::SM)),
                );
            }
            for record in decide {
                let id = record.draft.id;
                let comments = record.comments.len();
                let mut detail = format!("Proposal · v{}", record.version);
                if comments > 0 {
                    detail.push_str(&format!(
                        " · {comments} comment{}",
                        if comments == 1 { "" } else { "s" }
                    ));
                }
                let badge = (record.state != ProposalState::Draft).then(|| {
                    let (label, tone) = super::simple::proposal_state_badge(record.state);
                    ui::badge(label, tone, p).into_any_element()
                });
                body = body.child(
                    ui::list_row(
                        format!("proposal-{id}"),
                        compact_title(&record.draft.title),
                        Some(detail),
                        badge,
                        p,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.simple_leave(EditorTransition::Review(id), cx)
                    })),
                );
            }
        }

        if show(QueueFilter::Sort) {
            body = body.child(heading(
                "Sort",
                Some(
                    link("open-inbox", "Open Inbox")
                        .disabled(!ai.ready || blocked)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if !window.has_active_dialog(cx) {
                                this.simple_leave(EditorTransition::Inbox, cx);
                            }
                        })),
                ),
            ));
            match sort {
                None => {
                    body = body.child(ui::hint("Loading the Inbox…", p).py(px(tokens::space::SM)))
                }
                Some(page) if page.entries.is_empty() => {
                    body = body.child(ui::hint("The Inbox is empty.", p).py(px(tokens::space::SM)))
                }
                Some(page) => {
                    for item in page.entries.iter().map(|entry| &entry.item) {
                        body = body.child(
                            ui::list_row(
                                format!("queue-inbox-{}", item.capture.id),
                                compact_title(&item.capture.title),
                                Some(format!(
                                    "Inbox · received {}",
                                    ui::utc_time(item.received_at_ms)
                                )),
                                None,
                                p,
                            )
                            .disabled(blocked)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.simple_leave(EditorTransition::Inbox, cx)
                            })),
                        );
                    }
                    if page.total_count > page.entries.len() {
                        body = body.child(ui::hint(
                            format!(
                                "{} more in the Inbox.",
                                page.total_count - page.entries.len()
                            ),
                            p,
                        ));
                    }
                }
            }
        }

        if show(QueueFilter::Check) {
            body = body.child(heading(
                "Check",
                Some(
                    link("open-findings", "Open Needs Review")
                        .disabled(!ai.ready)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.simple_leave(EditorTransition::Findings, cx)
                        })),
                ),
            ));
            let open: Vec<_> = check
                .iter()
                .flat_map(|page| page.entries.iter())
                .filter(|record| record.state == brn_workflow::findings::FindingState::Open)
                .collect();
            if check.is_none() {
                body = body.child(ui::hint("Loading findings…", p).py(px(tokens::space::SM)));
            } else if open.is_empty() {
                body = body.child(ui::hint("No open findings.", p).py(px(tokens::space::SM)));
            }
            for record in open {
                body = body.child(
                    ui::list_row(
                        format!("queue-finding-{}", record.draft.request.id),
                        compact_title(&record.draft.title),
                        Some(record.draft.summary.clone()),
                        Some(ui::badge("check", Tone::Attention, p).into_any_element()),
                        p,
                    )
                    .disabled(blocked)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.simple_leave(EditorTransition::Findings, cx)
                    })),
                );
            }
        }

        body = body.child(
            ui::hint(
                "Decided proposals and approved changes are under History.",
                p,
            )
            .pt(px(tokens::space::LG)),
        );

        div()
            .id("queue-view")
            .size_full()
            .flex()
            .flex_col()
            .bg(color(p.paper))
            .child(
                ui::view_header("Needs you", None, Some(summary.join(" · ")), p).child(
                    Button::new("refresh-queue")
                        .icon(IconName::RotateCw)
                        .ghost()
                        .small()
                        .tooltip("Refresh everything in the queue")
                        .disabled(blocked)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_queue_pages(cx);
                            this.simple_command(
                                Pending::Proposals,
                                AppCommand::Proposals(None),
                                cx,
                            );
                            cx.notify();
                        })),
                ),
            )
            .child(body)
            .into_any_element()
    }
}
