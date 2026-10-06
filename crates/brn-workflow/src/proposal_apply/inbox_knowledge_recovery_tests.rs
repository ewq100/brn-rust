//! Synthetic exact-knowledge eligibility at existing apply/recovery boundaries.
use super::*;
use crate::{
    app::AppConfig,
    inbox_actions::{InboxActionCapture, InboxAnalysisPurpose},
};
use brn_ai::{KnowledgeProposalArgs, KnowledgeQuoteArgs};
use brn_store::{
    note_identity,
    work::{
        WorkTurnStatus,
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
        inbox_processing::InboxConversionFormat,
        inbox_source::InboxSourceBinding,
    },
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

const SOURCE_PATH: &str = "source.md";
const KNOWLEDGE_PATH: &str = "knowledge.md";
const TARGET_PATH: &str = "context.md";
const COLLISION_PATH: &str = "collision.md";

#[derive(Clone, Copy, Debug)]
enum Collision {
    Knowledge,
    Source,
    Target,
}
impl Collision {
    const ALL: [Self; 3] = [Self::Knowledge, Self::Source, Self::Target];
}

struct Fixture {
    base: tempfile::TempDir,
    vault: PathBuf,
    data: PathBuf,
    credentials: PathBuf,
    source_id: Uuid,
    knowledge_id: Uuid,
    target_id: Uuid,
    source_text: String,
    approved_text: String,
    predecessor_text: String,
    approval: ApprovalRequest,
}

impl Fixture {
    fn new() -> (Self, App) {
        Self::with_supersession(false)
    }
    fn with_supersession(supersedes: bool) -> (Self, App) {
        Self::with_input(supersedes, "Synthetic saved analysis")
    }
    fn with_input(supersedes: bool, question: &str) -> (Self, App) {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let vault = base.path().join("vault");
        let data = base.path().join("data");
        let credentials = base.path().join("credentials");
        fs::create_dir(&vault).unwrap();
        fs::create_dir(&data).unwrap();
        let mut app = App::open(
            &data,
            AppConfig {
                vault_root: Some(vault.clone()),
                credentials_dir: Some(credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap();
        let raw = "Original exact Blue õ 🦀\r\n";
        let raw_hash: [u8; 32] = Sha256::digest(raw.as_bytes()).into();
        let source_id = Uuid::new_v4();
        // Portable provenance describes synthetic input. Only the saved Source
        // is filesystem authority for this downstream knowledge qualification.
        let binding = InboxSourceBinding {
            visual: None,
            batch_id: Uuid::new_v4(),
            index: 0,
            original: InboxItem {
                capture: InboxCapture {
                    id: Uuid::new_v4(),
                    kind: InboxKind::Markdown,
                    title: "Synthetic recovery evidence õ".into(),
                    original_name: Some("synthetic-original.md".into()),
                    copy: InboxCopy {
                        directory: base.path().join("synthetic-copies"),
                        directory_device: 1,
                        directory_inode: 2,
                        file_device: 1,
                        file_inode: 3,
                        byte_len: raw.len() as u64,
                        sha256: raw_hash,
                    },
                },
                received_at_ms: 1,
            },
            format: InboxConversionFormat::VerbatimMarkdownV1,
            byte_len: raw.len() as u64,
            sha256: raw_hash,
            note_id: source_id,
        };
        let source_text = binding.markdown(raw).unwrap();
        fs::write(vault.join(SOURCE_PATH), &source_text).unwrap();
        let source = app.proposal_source(SOURCE_PATH).unwrap();
        let capture = InboxActionCapture {
            visual_asset: None,
            purpose: InboxAnalysisPurpose::KnowledgeAndActions,
            id: Uuid::new_v4(),
            conversation: None,
            source: source.source,
            source_text: source.text,
            provider: "chatgpt".into(),
            model: "gpt-6-luna".into(),
            effort: "medium".into(),
        };
        let job = app.store.reserve_inbox_action(&capture, question).unwrap();
        let turn = app.store.begin_inbox_action_turn(&job).unwrap();
        app.store
            .finish_turn(
                turn.id,
                WorkTurnStatus::Completed,
                "Synthetic fixture",
                None,
            )
            .unwrap();
        let target_id = Uuid::new_v4();
        let predecessor_text = note_identity::assign("Saved context õ\r\n", target_id).unwrap();
        fs::write(vault.join(TARGET_PATH), &predecessor_text).unwrap();
        let args = KnowledgeProposalArgs {
            supersedes: supersedes.then(|| TARGET_PATH.into()),
            title: "Exact reviewed knowledge".into(),
            path: KNOWLEDGE_PATH.into(),
            source_paths: if supersedes {
                vec![]
            } else {
                vec![TARGET_PATH.into()]
            },
            text: format!(
                "\u{feff}# Reviewed interpretation\r\nBlue õ 🦀 was selected.\r\n\r\n[Context](brn://note/{target_id})\r\n"
            ),
            quotes: vec![KnowledgeQuoteArgs {
                quote: "Blue õ 🦀".into(),
                occurrence: None,
            }],
        };
        let request = app
            .prepare_inbox_knowledge(&job, &args, turn.conversation_id)
            .unwrap();
        let knowledge_id = request.inbox_knowledge.as_ref().unwrap().note_id;
        let review = app.create_proposal(&request).unwrap();
        let approved_text = review.draft.changes[0].text().unwrap().to_owned();
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        };
        (
            Self {
                base,
                vault,
                data,
                credentials,
                source_id,
                knowledge_id,
                target_id,
                source_text,
                approved_text,
                predecessor_text,
                approval,
            },
            app,
        )
    }

    fn app(&self) -> App {
        App::open(
            &self.data,
            AppConfig {
                vault_root: None,
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap()
    }

    fn collision_text(&self, collision: Collision) -> String {
        let id = match collision {
            Collision::Knowledge => self.knowledge_id,
            Collision::Source => self.source_id,
            Collision::Target => self.target_id,
        };
        note_identity::assign("Independent owner bytes õ\r\n", id).unwrap()
    }

    fn insert_collision(&self, collision: Collision) -> String {
        let text = self.collision_text(collision);
        fs::write(self.vault.join(COLLISION_PATH), &text).unwrap();
        text
    }

    fn assert_source_and_credentials(&self) {
        assert_eq!(
            fs::read(self.vault.join(SOURCE_PATH)).unwrap(),
            self.source_text.as_bytes()
        );
        assert_eq!(fs::read_dir(&self.credentials).unwrap().count(), 0);
    }

    fn crash(&self, phase: &str) {
        self.crash_member(phase, 0);
    }
    fn crash_member(&self, phase: &str, member: usize) {
        fs::write(
            self.base.path().join("approval.json"),
            serde_json::to_vec(&self.approval).unwrap(),
        )
        .unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "proposal_apply::inbox_knowledge_recovery_tests::knowledge_recovery_crash_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BRN_KNOWLEDGE_RECOVERY_BASE", self.base.path())
            .env("BRN_KNOWLEDGE_RECOVERY_PHASE", phase)
            .env("BRN_KNOWLEDGE_RECOVERY_MEMBER", member.to_string())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(86),
            "{phase}: stdout={} stderr={}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr),
        );
    }

    fn finish(&self, app: &mut App) -> RepairRequest {
        let preview = app
            .preview_proposal_repair(self.approval.operation_id)
            .unwrap();
        assert_eq!(preview.phases, vec![ApplyMemberPhase::Applied]);
        RepairRequest {
            id: Uuid::new_v4(),
            operation_id: self.approval.operation_id,
            expected: preview.expected,
            direction: RepairDirection::Finish,
        }
    }

    fn assert_exact_installed_knowledge(&self) {
        assert_eq!(
            fs::read(self.vault.join(KNOWLEDGE_PATH)).unwrap(),
            self.approved_text.as_bytes(),
        );
        self.assert_source_and_credentials();
    }
}

#[test]
fn prepared_knowledge_target_content_change_retains_owner_bytes_without_install() {
    let (f, mut app) = Fixture::new();
    let path = f.vault.join(TARGET_PATH);
    let changed = fs::read_to_string(&path).unwrap() + "Later owner context\r\n";
    let injected = changed.clone();
    APPLY_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |step, member| {
            if step == "prepared" && member == 0 {
                fs::write(&path, &injected).unwrap();
            }
        }));
    });
    let result = app.approve_proposal(&f.approval);
    APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert!(result.is_err());
    assert!(!f.vault.join(KNOWLEDGE_PATH).exists());
    assert_eq!(
        fs::read(f.vault.join(TARGET_PATH)).unwrap(),
        changed.as_bytes()
    );
    assert_eq!(
        app.proposal_apply(f.approval.operation_id)
            .unwrap()
            .unwrap()
            .receipt
            .unwrap()
            .outcome,
        ApplyOutcome::NotApplied
    );
    f.assert_source_and_credentials();
}

#[test]
fn prepared_knowledge_approval_refuses_new_note_and_source_identity_collisions_before_install() {
    for collision in Collision::ALL {
        let (f, mut app) = Fixture::new();
        let path = f.vault.join(COLLISION_PATH);
        let text = f.collision_text(collision);
        let injected = text.clone();
        APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, member| {
                if step == "prepared" && member == 0 {
                    fs::write(&path, &injected).unwrap();
                }
            }));
        });
        let result = app.approve_proposal(&f.approval);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(
            result.is_err(),
            "{collision:?}: eligibility changed after preparation"
        );
        assert!(!f.vault.join(KNOWLEDGE_PATH).exists(), "{collision:?}");
        assert_eq!(
            fs::read(f.vault.join(COLLISION_PATH)).unwrap(),
            text.as_bytes()
        );
        let journal = app
            .proposal_apply(f.approval.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            journal.receipt.as_ref().unwrap().outcome,
            ApplyOutcome::NotApplied
        );
        let mirrored = app
            .application_records()
            .unwrap()
            .read(f.approval.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            mirrored.journal.receipt.unwrap().outcome,
            ApplyOutcome::NotApplied
        );
        f.assert_source_and_credentials();
    }
}

#[test]
#[ignore = "subprocess entry point for genuine knowledge approval interruption"]
fn knowledge_recovery_crash_child() {
    let base = PathBuf::from(std::env::var("BRN_KNOWLEDGE_RECOVERY_BASE").unwrap());
    let phase = std::env::var("BRN_KNOWLEDGE_RECOVERY_PHASE").unwrap();
    let approval: ApprovalRequest =
        serde_json::from_slice(&fs::read(base.join("approval.json")).unwrap()).unwrap();
    let mut app = App::open(
        &base.join("data"),
        AppConfig {
            vault_root: None,
            credentials_dir: Some(base.join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    let member = std::env::var("BRN_KNOWLEDGE_RECOVERY_MEMBER")
        .unwrap()
        .parse()
        .unwrap();
    APPLY_CHECKPOINT.with(|point| *point.borrow_mut() = Some((phase, member)));
    let result = app.approve_proposal(&approval);
    panic!("Requested genuine knowledge interruption was not reached: {result:?}");
}

#[test]
fn synced_knowledge_collision_stays_uncertain_until_fresh_finish_then_replays_owner_edits() {
    for collision in Collision::ALL {
        let (f, app) = Fixture::new();
        drop(app);
        f.crash("synced");
        f.assert_exact_installed_knowledge();
        let collided = f.insert_collision(collision);
        // Injection precedes App::open because startup itself reconciles pending
        // journals. No artificial receipt or second change forces uncertainty.
        let mut app = f.app();
        let uncertain = app.reconcile_proposal(f.approval.operation_id).unwrap();
        assert_eq!(uncertain.outcome, ApplyOutcome::Uncertain, "{collision:?}");
        assert!(app.current_evidence_blocked().unwrap());
        let refused = f.finish(&mut app);
        assert!(app.repair_proposal(&refused).is_err(), "{collision:?}");
        assert!(
            app.store.proposal_repair(refused.id).unwrap().is_none(),
            "pre-existing {collision:?} must refuse before repair admission"
        );
        assert_eq!(
            app.proposal_apply(f.approval.operation_id)
                .unwrap()
                .unwrap()
                .receipt
                .unwrap()
                .outcome,
            ApplyOutcome::Uncertain,
        );
        assert_eq!(
            fs::read(f.vault.join(COLLISION_PATH)).unwrap(),
            collided.as_bytes()
        );
        f.assert_exact_installed_knowledge();

        fs::remove_file(f.vault.join(COLLISION_PATH)).unwrap();
        let finish = f.finish(&mut app);
        let receipt = app.repair_proposal(&finish).unwrap();
        assert_eq!(receipt.outcome, Some(ApplyOutcome::Applied));
        let settled = app
            .proposal_apply(f.approval.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            settled.receipt.as_ref().unwrap().outcome,
            ApplyOutcome::Applied
        );
        assert_eq!(
            settled.approved.draft.changes[0].text(),
            Some(f.approved_text.as_str())
        );
        f.assert_exact_installed_knowledge();

        let later_knowledge = f.approved_text.clone() + "\r\nLater owner interpretation λ\r\n";
        let later_source = f.source_text.clone() + "\r\nLater owner Source annotation õ\r\n";
        fs::write(f.vault.join(KNOWLEDGE_PATH), &later_knowledge).unwrap();
        fs::write(f.vault.join(SOURCE_PATH), &later_source).unwrap();
        assert_eq!(app.repair_proposal(&finish).unwrap(), receipt);
        assert_eq!(
            app.approve_proposal(&f.approval).unwrap(),
            settled.receipt.clone().unwrap()
        );
        drop(app);
        let mut reopened = f.app();
        assert_eq!(reopened.repair_proposal(&finish).unwrap(), receipt);
        assert_eq!(
            reopened.approve_proposal(&f.approval).unwrap(),
            settled.receipt.unwrap()
        );
        assert_eq!(
            fs::read(f.vault.join(KNOWLEDGE_PATH)).unwrap(),
            later_knowledge.as_bytes()
        );
        assert_eq!(
            fs::read(f.vault.join(SOURCE_PATH)).unwrap(),
            later_source.as_bytes()
        );
        assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    }
}

#[test]
fn synced_knowledge_reconciliation_accepts_only_its_own_exact_prepared_member() {
    let (f, app) = Fixture::new();
    drop(app);
    f.crash("synced");
    let mut app = f.app();
    let receipt = app.reconcile_proposal(f.approval.operation_id).unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    let journal = app
        .proposal_apply(f.approval.operation_id)
        .unwrap()
        .unwrap();
    let installed = app.proposal_source(KNOWLEDGE_PATH).unwrap();
    assert_eq!(
        installed.source.fingerprint,
        journal.prepared.as_ref().unwrap()[0]
    );
    assert_eq!(
        note_identity::read(&installed.text).unwrap(),
        Some(f.knowledge_id)
    );
    assert_eq!(app.approve_proposal(&f.approval).unwrap(), receipt);
    f.assert_exact_installed_knowledge();
}

#[test]
fn admitted_knowledge_finish_refuses_collisions_at_repair_admission_mirror_and_final_verification()
{
    for checkpoint in ["repair-intent", "repair-mirror", "repair-verified"] {
        for collision in Collision::ALL {
            let (f, app) = Fixture::new();
            drop(app);
            f.crash("synced");
            f.insert_collision(collision);
            let mut app = f.app();
            assert_eq!(
                app.reconcile_proposal(f.approval.operation_id)
                    .unwrap()
                    .outcome,
                ApplyOutcome::Uncertain,
                "{checkpoint}, {collision:?}",
            );
            fs::remove_file(f.vault.join(COLLISION_PATH)).unwrap();
            let finish = f.finish(&mut app);
            let path = f.vault.join(COLLISION_PATH);
            let text = f.collision_text(collision);
            let injected = text.clone();
            APPLY_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |step, member| {
                    if step == checkpoint && member == 0 {
                        fs::write(&path, &injected).unwrap();
                    }
                }));
            });
            let result = app.repair_proposal(&finish);
            APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
            assert!(result.is_err(), "{checkpoint}, {collision:?}");
            assert_eq!(
                app.store
                    .proposal_repair(finish.id)
                    .unwrap()
                    .unwrap()
                    .outcome,
                Some(ApplyOutcome::Uncertain),
            );
            let unresolved = app
                .proposal_apply(f.approval.operation_id)
                .unwrap()
                .unwrap();
            assert_eq!(
                unresolved.receipt.as_ref().unwrap().outcome,
                ApplyOutcome::Uncertain
            );
            let mirror = app
                .application_records()
                .unwrap()
                .read(f.approval.operation_id)
                .unwrap()
                .unwrap();
            assert_eq!(
                mirror.journal.receipt.as_ref().unwrap().outcome,
                ApplyOutcome::Uncertain,
                "{checkpoint}, {collision:?}: refused repair cannot publish Applied authority"
            );
            assert_eq!(
                mirror
                    .journal
                    .repair
                    .as_ref()
                    .unwrap()
                    .attempts
                    .last()
                    .unwrap()
                    .outcome,
                Some(ApplyOutcome::Uncertain),
            );
            assert!(app.current_evidence_blocked().unwrap());
            assert_eq!(
                fs::read(f.vault.join(COLLISION_PATH)).unwrap(),
                text.as_bytes()
            );
            f.assert_exact_installed_knowledge();
        }
    }
}

#[test]
fn supersession_process_interruption_finishes_or_restores_the_whole_pair() {
    for member in [0, 1] {
        for direction in [RepairDirection::Finish, RepairDirection::Restore] {
            let (f, app) = Fixture::with_supersession(true);
            drop(app);
            f.crash_member("synced", member);
            let history = brn_store::note_metadata::to_history(&f.predecessor_text).unwrap();
            assert_eq!(
                fs::read(f.vault.join(TARGET_PATH)).unwrap(),
                if member == 0 {
                    f.predecessor_text.as_bytes()
                } else {
                    history.as_bytes()
                }
            );
            // A fresh duplicate predecessor must never be certified by recovery,
            // even when every own member was installed before process death.
            let collision = f.insert_collision(Collision::Target);
            let mut app = f.app();
            assert_eq!(
                app.reconcile_proposal(f.approval.operation_id)
                    .unwrap()
                    .outcome,
                ApplyOutcome::Uncertain
            );
            assert!(app.current_evidence_blocked().unwrap());
            let preview = app
                .preview_proposal_repair(f.approval.operation_id)
                .unwrap();
            assert_eq!(
                preview.phases,
                vec![
                    ApplyMemberPhase::Applied,
                    if member == 0 {
                        ApplyMemberPhase::Before
                    } else {
                        ApplyMemberPhase::Applied
                    }
                ]
            );
            let attempt = RepairRequest {
                id: Uuid::new_v4(),
                operation_id: f.approval.operation_id,
                expected: preview.expected,
                direction: RepairDirection::Finish,
            };
            assert!(app.repair_proposal(&attempt).is_err());
            assert_eq!(
                fs::read(f.vault.join(COLLISION_PATH)).unwrap(),
                collision.as_bytes()
            );
            fs::remove_file(f.vault.join(COLLISION_PATH)).unwrap();
            let preview = app
                .preview_proposal_repair(f.approval.operation_id)
                .unwrap();
            let attempt = RepairRequest {
                id: Uuid::new_v4(),
                operation_id: f.approval.operation_id,
                expected: preview.expected,
                direction,
            };
            let receipt = app.repair_proposal(&attempt).unwrap();
            assert_eq!(
                receipt.outcome,
                Some(if direction == RepairDirection::Finish {
                    ApplyOutcome::Applied
                } else {
                    ApplyOutcome::NotApplied
                })
            );
            assert_eq!(
                fs::read(f.vault.join(TARGET_PATH)).unwrap(),
                if direction == RepairDirection::Finish {
                    history.as_bytes()
                } else {
                    f.predecessor_text.as_bytes()
                }
            );
            if direction == RepairDirection::Finish {
                f.assert_exact_installed_knowledge();
            } else {
                assert!(!f.vault.join(KNOWLEDGE_PATH).exists());
            }
            f.assert_source_and_credentials();
            let later =
                fs::read_to_string(f.vault.join(TARGET_PATH)).unwrap() + "Later owner context\r\n";
            fs::write(f.vault.join(TARGET_PATH), &later).unwrap();
            assert_eq!(app.repair_proposal(&attempt).unwrap(), receipt);
            drop(app);
            let mut app = f.app();
            assert_eq!(app.repair_proposal(&attempt).unwrap(), receipt);
            assert_eq!(
                fs::read(f.vault.join(TARGET_PATH)).unwrap(),
                later.as_bytes()
            );
        }
    }
}

#[test]
fn supersession_prepared_predecessor_change_refuses_before_installation() {
    let (f, mut app) = Fixture::with_supersession(true);
    let path = f.vault.join(TARGET_PATH);
    let later = f.predecessor_text.clone() + "Owner changed it\r\n";
    let injected = later.clone();
    APPLY_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |step, member| {
            if step == "prepared" && member == 0 {
                fs::write(&path, &injected).unwrap();
            }
        }))
    });
    let result = app.approve_proposal(&f.approval);
    APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert!(result.is_err());
    assert!(!f.vault.join(KNOWLEDGE_PATH).exists());
    assert_eq!(
        fs::read(f.vault.join(TARGET_PATH)).unwrap(),
        later.as_bytes()
    );
    assert_eq!(
        app.proposal_apply(f.approval.operation_id)
            .unwrap()
            .unwrap()
            .receipt
            .unwrap()
            .outcome,
        ApplyOutcome::NotApplied
    );
    f.assert_source_and_credentials();
}

impl Fixture {
    fn copy_approval_family(&self, label: &str) -> PathBuf {
        let fresh = self.base.path().join(label);
        fs::create_dir(&fresh).unwrap();
        for entry in fs::read_dir(&self.data).unwrap() {
            let entry = entry.unwrap();
            if entry
                .file_name()
                .to_str()
                .unwrap()
                .starts_with(".brn-apply-")
            {
                fs::copy(entry.path(), fresh.join(entry.file_name())).unwrap();
            }
        }
        fresh
    }

    fn open_fresh(&self, data: &std::path::Path) -> Result<App> {
        App::open(
            data,
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
            },
        )
    }

    fn capture_path(&self, data: &std::path::Path) -> PathBuf {
        data.join(format!(
            ".brn-apply-{}.inbox-capture",
            self.approval.operation_id
        ))
    }
}

#[test]
fn knowledge_receipt_restores_genuine_capture_into_fresh_database_without_chat() {
    for supersedes in [false, true] {
        let (f, mut app) = Fixture::with_supersession(supersedes);
        let binding = app
            .store
            .proposal(f.approval.expected.id)
            .unwrap()
            .unwrap()
            .draft
            .inbox_knowledge
            .unwrap();
        let original_job = app
            .store
            .inbox_action(binding.analysis_id)
            .unwrap()
            .unwrap();
        let receipt = app.approve_proposal(&f.approval).unwrap();
        assert_eq!(receipt.outcome, ApplyOutcome::Applied);
        drop(app);
        let fresh = f.copy_approval_family("fresh-receipt-data");
        if supersedes {
            // Recovery uses the historical capture, never today's Source text.
            fs::remove_file(f.vault.join(SOURCE_PATH)).unwrap();
        }
        let reopened = f.open_fresh(&fresh);
        assert!(
            reopened.is_ok(),
            "fresh Knowledge recovery: {:?}",
            reopened.err()
        );
        let mut reopened = reopened.unwrap();
        assert_eq!(
            reopened.store.inbox_action(binding.analysis_id).unwrap(),
            Some(original_job.clone())
        );
        assert!(reopened.store.turn(binding.analysis_id).unwrap().is_none());
        assert!(reopened.store.conversations().unwrap().is_empty());
        assert!(
            reopened
                .store
                .begin_inbox_action_turn(&original_job)
                .is_err()
        );
        assert_eq!(reopened.approve_proposal(&f.approval).unwrap(), receipt);
        drop(reopened);
        assert_eq!(
            f.open_fresh(&fresh)
                .unwrap()
                .store
                .inbox_action(binding.analysis_id)
                .unwrap(),
            Some(original_job)
        );
        assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    }
}

#[test]
fn legacy_knowledge_receipt_gains_capture_without_changing_receipt_bytes_or_inode() {
    use std::os::unix::fs::MetadataExt;
    let (f, mut app) = Fixture::new();
    app.approve_proposal(&f.approval).unwrap();
    let receipt_path = f
        .data
        .join(format!(".brn-apply-{}.receipt", f.approval.operation_id));
    let original = fs::read(&receipt_path).unwrap();
    let inode = fs::metadata(&receipt_path).unwrap().ino();
    let encoded: serde_json::Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(encoded["format"], 1);
    assert_eq!(encoded.as_object().unwrap().len(), 3);
    fs::remove_file(f.capture_path(&f.data)).unwrap();
    drop(app);
    let reopened = f.app();
    assert!(f.capture_path(&f.data).is_file());
    assert_eq!(fs::read(&receipt_path).unwrap(), original);
    assert_eq!(fs::metadata(&receipt_path).unwrap().ino(), inode);
    drop(reopened);
    let fresh = f.copy_approval_family("fresh-legacy-data");
    assert!(f.open_fresh(&fresh).is_ok());
    assert_eq!(fs::read(&receipt_path).unwrap(), original);
    assert_eq!(fs::metadata(&receipt_path).unwrap().ino(), inode);
}

#[test]
fn missing_corrupt_or_unknown_capture_companions_refuse_without_poisoning_fresh_work() {
    for damage in ["missing", "hash", "filename", "binding", "oversized"] {
        let (f, mut app) = Fixture::new();
        app.approve_proposal(&f.approval).unwrap();
        drop(app);
        let fresh = f.copy_approval_family("damaged-data");
        let capture = f.capture_path(&fresh);
        let original = fs::read(&capture).unwrap();
        match damage {
            "missing" => fs::remove_file(&capture).unwrap(),
            "hash" => {
                let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
                value["sha256"][0] = ((value["sha256"][0].as_u64().unwrap() + 1) % 256).into();
                fs::write(&capture, serde_json::to_vec(&value).unwrap()).unwrap();
            }
            "filename" => {
                fs::rename(
                    &capture,
                    fresh.join(format!(".brn-apply-{}.unknown", f.approval.operation_id)),
                )
                .unwrap();
            }
            "binding" => {
                fs::rename(
                    &capture,
                    fresh.join(format!(".brn-apply-{}.inbox-capture", Uuid::new_v4())),
                )
                .unwrap();
            }
            "oversized" => {
                fs::OpenOptions::new()
                    .write(true)
                    .open(&capture)
                    .unwrap()
                    .set_len(1024 * 1024 + 4096 + 1)
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(f.open_fresh(&fresh).is_err(), "{damage}");
        let db = rusqlite::Connection::open(fresh.join("brn.sqlite")).unwrap();
        for table in ["proposals", "proposal_applies", "inbox_actions", "messages"] {
            let count: i64 = db
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "{damage}/{table}");
        }
        assert_eq!(
            fs::read(f.vault.join(KNOWLEDGE_PATH)).unwrap(),
            f.approved_text.as_bytes()
        );
        f.assert_source_and_credentials();
    }
}

#[test]
fn immutable_capture_replay_keeps_inode_and_rejects_changed_genuine_job() {
    use std::os::unix::fs::MetadataExt;
    let (f, mut app) = Fixture::new();
    app.approve_proposal(&f.approval).unwrap();
    let journal = app
        .proposal_apply(f.approval.operation_id)
        .unwrap()
        .unwrap();
    let binding = journal.approved.draft.inbox_knowledge.as_ref().unwrap();
    let job = app
        .store
        .inbox_action(binding.analysis_id)
        .unwrap()
        .unwrap();
    let path = f.capture_path(&f.data);
    let original = fs::read(&path).unwrap();
    let inode = fs::metadata(&path).unwrap().ino();
    let records = app.application_records().unwrap();
    records.write_inbox_capture(&journal, &job).unwrap();
    assert_eq!(
        records.read_inbox_capture(&journal).unwrap(),
        Some(job.clone())
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    for field in ["question", "time", "model"] {
        let mut changed = job.clone();
        match field {
            "question" => changed.question.push_str(" Another question"),
            "time" => changed.created_at_ms += 1,
            "model" => changed.capture.model = "another-valid-model".into(),
            _ => unreachable!(),
        }
        changed.validate().unwrap();
        assert!(
            records.write_inbox_capture(&journal, &changed).is_err(),
            "{field}"
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    }
}

#[test]
fn near_limit_escaped_genuine_capture_fits_fixed_companion_bound_and_recovers() {
    let question = "\u{1}".repeat(171_000);
    let (f, mut app) = Fixture::with_input(false, &question);
    app.approve_proposal(&f.approval).unwrap();
    let journal = app
        .proposal_apply(f.approval.operation_id)
        .unwrap()
        .unwrap();
    let id = journal
        .approved
        .draft
        .inbox_knowledge
        .as_ref()
        .unwrap()
        .analysis_id;
    let job = app.store.inbox_action(id).unwrap().unwrap();
    let encoded = serde_json::to_vec(&job).unwrap();
    assert!(encoded.len() > 1024 * 1024 - 32 * 1024);
    assert!(encoded.len() <= 1024 * 1024);
    let companion = fs::read(f.capture_path(&f.data)).unwrap();
    assert!(companion.len() <= 1024 * 1024 + 4096);
    drop(app);
    let fresh = f.copy_approval_family("large-capture-data");
    let recovered = f.open_fresh(&fresh).unwrap();
    assert_eq!(recovered.store.inbox_action(id).unwrap(), Some(job));
    assert!(recovered.store.turn(id).unwrap().is_none());
}

#[test]
fn capture_publication_interruptions_recover_without_new_chat_or_repeated_effects() {
    for phase in ["capture-mirrored", "mirror-intent", "prepared", "synced"] {
        let (f, app) = Fixture::new();
        drop(app);
        f.crash(phase);
        assert!(f.capture_path(&f.data).is_file(), "{phase}");
        let fresh = f.copy_approval_family("interrupted-data");
        let mut reopened = f.open_fresh(&fresh).unwrap();
        let imported = reopened.proposal_apply(f.approval.operation_id).unwrap();
        if phase == "capture-mirrored" {
            assert!(imported.is_none());
            assert!(reopened.store.conversations().unwrap().is_empty());
            assert!(!f.vault.join(KNOWLEDGE_PATH).exists());
        } else {
            let journal = imported.unwrap();
            let binding = journal.approved.draft.inbox_knowledge.as_ref().unwrap();
            let job = reopened
                .store
                .inbox_action(binding.analysis_id)
                .unwrap()
                .unwrap();
            assert!(reopened.store.turn(binding.analysis_id).unwrap().is_none());
            assert!(reopened.store.begin_inbox_action_turn(&job).is_err());
            let receipt = reopened
                .reconcile_proposal(f.approval.operation_id)
                .unwrap();
            assert_eq!(
                receipt.outcome,
                if phase == "synced" {
                    ApplyOutcome::Applied
                } else {
                    ApplyOutcome::NotApplied
                }
            );
        }
        f.assert_source_and_credentials();
    }
}

#[test]
fn capture_sync_failures_keep_admitted_work_fenced_until_recovery() {
    for failure in [
        "capture_write",
        "capture_file_sync",
        "capture_prepare_sync",
        "capture_rename",
        "capture_directory_sync",
        "capture_postproof",
    ] {
        let (f, mut app) = Fixture::new();
        crate::files::recovery::FAILURE.with(|selected| selected.set(Some(failure)));
        let result = app.approve_proposal(&f.approval);
        crate::files::recovery::FAILURE.with(|selected| selected.set(None));
        assert!(result.is_err(), "{failure}");
        assert!(!f.vault.join(KNOWLEDGE_PATH).exists(), "{failure}");
        if failure == "capture_postproof" {
            // The occupied exact companion was re-read and synced by the
            // no-effect completion retry. That terminal refusal may clear the
            // fence; no vault effect occurred or will be repeated.
            assert!(app.require_current_evidence().is_ok());
            assert_eq!(
                app.proposal_apply(f.approval.operation_id)
                    .unwrap()
                    .unwrap()
                    .receipt
                    .unwrap()
                    .outcome,
                ApplyOutcome::NotApplied,
            );
        } else {
            assert!(app.require_current_evidence().is_err(), "{failure}");
        }
        drop(app);
        let mut recovered = f.app();
        assert_eq!(
            recovered
                .reconcile_proposal(f.approval.operation_id)
                .unwrap()
                .outcome,
            ApplyOutcome::NotApplied,
            "{failure}"
        );
        f.assert_source_and_credentials();
    }
}
