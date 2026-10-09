//! Explicit read-only Person/Project lenses over saved records. No profile type is inferred.
use super::{
    DuplicateIdentity, EdgeOrigin, EvidenceEndpoint, IdentityIssue, IdentityOutcome,
    IdentityResolution, NoteEdge, NoteIdentityInfo, note_identity, rejected,
};
use crate::{
    ErrorKind, Result, WorkflowError,
    actions::{ActionCursor, ActionListRequest, ActionRecord},
    app::App,
    library::{KnowledgeScope, saved_metadata},
    proposals::{ProposalSource, SourceVersion},
    vault::{self, EvidencePath, VaultPath},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileLens {
    Person,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileContextRequest {
    #[serde(with = "strict_source")]
    pub profile: SourceVersion,
    pub note_id: Uuid,
    pub lens: ProfileLens,
    pub action_offset: usize,
    pub relationship_offset: usize,
    pub limit: usize,
}

// FileFingerprint predates strict external requests; preserve the shared public
// type while rejecting unknown fields at every level of this request.
mod strict_source {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        value: &SourceVersion,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<SourceVersion, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fingerprint {
            device: u64,
            inode: u64,
            len: u64,
            sha256: [u8; 32],
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Source {
            path: String,
            fingerprint: Fingerprint,
        }
        let source = Source::deserialize(deserializer)?;
        Ok(SourceVersion {
            path: source.path,
            fingerprint: brn_store::files::FileFingerprint {
                device: source.fingerprint.device,
                inode: source.fingerprint.inode,
                len: source.fingerprint.len,
                sha256: source.fingerprint.sha256,
            },
        })
    }
}

mod strict_profile {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        value: &ProposalSource,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<ProposalSource, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Profile {
            #[serde(with = "strict_source")]
            source: SourceVersion,
            text: String,
        }
        let value = Profile::deserialize(deserializer)?;
        Ok(ProposalSource {
            source: value.source,
            text: value.text,
        })
    }
}

impl ProfileContextRequest {
    /// Pure shape checks; this grants no evidence or workspace authority.
    pub fn validate(&self) -> Result<()> {
        VaultPath::parse(&self.profile.path).map_err(|error| rejected(error.to_string()))?;
        if self.note_id.is_nil()
            || self.profile.fingerprint.len > crate::MAX_NOTE_BYTES as u64
            || !(1..=200).contains(&self.limit)
            || self.action_offset.checked_add(self.limit).is_none()
            || self.relationship_offset.checked_add(self.limit).is_none()
        {
            return Err(rejected(
                "Profile context needs a nonnil UUID, bounded saved Markdown fingerprint, limit 1..200 and nonoverflowing offsets.",
            ));
        }
        Ok(())
    }
    fn matches(&self, action: &ActionRecord) -> bool {
        match self.lens {
            ProfileLens::Person => action.data.related_person == Some(self.note_id),
            ProfileLens::Project => action.data.related_project == Some(self.note_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRelationship {
    pub edge: NoteEdge,
    pub source_scope: KnowledgeScope,
    pub target_scope: KnowledgeScope,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileNote {
    pub note: NoteIdentityInfo,
    pub scope: Option<KnowledgeScope>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileReference {
    pub resolution: IdentityResolution,
    pub matches: Vec<ProfileNote>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileContext {
    pub request: ProfileContextRequest,
    #[serde(with = "strict_profile")]
    pub profile: ProposalSource,
    pub action_total: usize,
    pub actions: Vec<ActionRecord>,
    pub relationship_total: usize,
    pub relationships: Vec<ProfileRelationship>,
    pub references: Vec<ProfileReference>,
    pub issues: Vec<IdentityIssue>,
    pub duplicates: Vec<DuplicateIdentity>,
    pub complete: bool,
}

fn classify(text: &str, path: &str) -> std::result::Result<KnowledgeScope, String> {
    let metadata = saved_metadata(text, path);
    if let Some(issue) = metadata.issue {
        return Err(issue);
    }
    // History remains historical even when its retained content is a Source.
    Ok(if metadata.history {
        KnowledgeScope::History
    } else if metadata.source {
        KnowledgeScope::Source
    } else {
        KnowledgeScope::Current
    })
}
fn exact_profile(profile: &ProposalSource, request: &ProfileContextRequest) -> Result<()> {
    profile.validate()?;
    if profile.source != request.profile
        || note_identity::read(&profile.text).ok().flatten() != Some(request.note_id)
        || classify(&profile.text, &profile.source.path).ok() != Some(KnowledgeScope::Current)
    {
        return Err(rejected(
            "Profile context requires the exact saved Current managed note.",
        ));
    }
    Ok(())
}
fn reference_ids(actions: &[ActionRecord]) -> BTreeSet<Uuid> {
    actions
        .iter()
        .flat_map(|action| {
            action
                .data
                .sources
                .iter()
                .copied()
                .chain(action.data.thread)
        })
        .collect()
}
fn page_length(total: usize, offset: usize, limit: usize) -> usize {
    total.saturating_sub(offset).min(limit)
}
fn known(scope: KnowledgeScope) -> bool {
    scope != KnowledgeScope::All
}
fn validate_issue(issue: &IdentityIssue) -> Result<()> {
    // Scanner diagnostics may identify folders or an unsupported filename.
    if issue.path.is_empty() || issue.reason.trim().is_empty() {
        return Err(rejected("Invalid profile coverage diagnostic."));
    }
    Ok(())
}

impl ProfileContext {
    /// Validate correlation and complete returned proof shapes without filesystem access.
    pub fn validate_for(&self, request: &ProfileContextRequest) -> Result<()> {
        request.validate()?;
        if self.request != *request {
            return Err(rejected("Profile reply changed the captured request."));
        }
        exact_profile(&self.profile, request)?;
        if self.actions.len()
            != page_length(self.action_total, request.action_offset, request.limit)
            || self.relationships.len()
                != page_length(
                    self.relationship_total,
                    request.relationship_offset,
                    request.limit,
                )
            || self.complete && (!self.issues.is_empty() || !self.duplicates.is_empty())
        {
            return Err(rejected(
                "Profile reply has inconsistent paging or coverage.",
            ));
        }
        for issue in &self.issues {
            validate_issue(issue)?;
        }
        let mut duplicate_ids = BTreeSet::new();
        for duplicate in &self.duplicates {
            if duplicate.note_id == request.note_id
                || duplicate.note_id.is_nil()
                || !duplicate_ids.insert(duplicate.note_id)
                || duplicate.paths.len() < 2
                || duplicate.paths.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(rejected("Invalid duplicate identity coverage."));
            }
            for path in &duplicate.paths {
                EvidencePath::parse(path).map_err(|error| rejected(error.to_string()))?;
            }
        }
        let mut previous = None;
        for action in &self.actions {
            action
                .validate()
                .map_err(|error| rejected(error.to_string()))?;
            let key = (action.origin.created_at_ms, action.origin.id);
            if !request.matches(action) || previous.is_some_and(|before| before <= key) {
                return Err(rejected("Profile Actions changed role, order or identity."));
            }
            previous = Some(key);
        }
        let mut previous = None;
        let mut endpoints = BTreeMap::new();
        for relationship in &self.relationships {
            let edge = &relationship.edge;
            let key = (&edge.source.path, &edge.target.path, edge.origin);
            if previous.is_some_and(|before| before >= key)
                || !known(relationship.source_scope)
                || !known(relationship.target_scope)
                || edge.source.note_id == edge.target.note_id
                || edge.source.path == edge.target.path
                || edge.source.note_id != request.note_id && edge.target.note_id != request.note_id
                || edge.evidence.is_empty()
                || edge.evidence.len() > 8192
            {
                return Err(rejected("Invalid direct profile relationship."));
            }
            previous = Some(key);
            for (endpoint, scope) in [
                (&edge.source, relationship.source_scope),
                (&edge.target, relationship.target_scope),
            ] {
                EvidencePath::parse(&endpoint.path).map_err(|error| rejected(error.to_string()))?;
                let observation = (endpoint.note_id, endpoint.sha256, scope);
                if endpoints
                    .insert(endpoint.path.as_str(), observation)
                    .is_some_and(|before| before != observation)
                {
                    return Err(rejected("Relationship endpoint observations disagree."));
                }
                if endpoint.note_id.is_nil() {
                    return Err(rejected("Relationship endpoint UUID is nil."));
                }
                if endpoint.note_id == request.note_id
                    && (endpoint.path != request.profile.path
                        || endpoint.sha256 != request.profile.fingerprint.sha256
                        || scope != KnowledgeScope::Current)
                {
                    return Err(rejected(
                        "Relationship profile endpoint differs from captured profile.",
                    ));
                }
                if endpoint.path == request.profile.path && endpoint.note_id != request.note_id {
                    return Err(rejected(
                        "Relationship reused the profile path with a different identity.",
                    ));
                }
            }
            let expected = match edge.origin {
                EdgeOrigin::ExplicitLink => EvidenceEndpoint::Source,
                EdgeOrigin::InferredProvenance => EvidenceEndpoint::Target,
            };
            let mut proofs = BTreeSet::new();
            let mut bytes = 0usize;
            for proof in &edge.evidence {
                bytes = bytes
                    .checked_add(proof.quote.len())
                    .filter(|sum| *sum <= 4 * 1024 * 1024)
                    .ok_or_else(|| {
                        rejected("Relationship proofs exceed the exact evidence bound.")
                    })?;
                if proof.endpoint != expected
                    || proof.start_byte >= proof.end_byte
                    || proof.end_byte > crate::MAX_NOTE_BYTES
                    || proof.end_byte - proof.start_byte != proof.quote.len()
                    || !proofs.insert((proof.start_byte, proof.end_byte, &proof.quote))
                {
                    return Err(rejected("Invalid exact relationship proof shape."));
                }
                let endpoint = match proof.endpoint {
                    EvidenceEndpoint::Source => &edge.source,
                    EvidenceEndpoint::Target => &edge.target,
                };
                if endpoint.note_id == request.note_id
                    && self.profile.text.get(proof.start_byte..proof.end_byte)
                        != Some(proof.quote.as_str())
                {
                    return Err(rejected(
                        "Relationship proof differs from saved profile bytes.",
                    ));
                }
            }
        }
        let ids = reference_ids(&self.actions).into_iter().collect::<Vec<_>>();
        if self
            .references
            .iter()
            .map(|reference| reference.resolution.note_id)
            .collect::<Vec<_>>()
            != ids
        {
            return Err(rejected(
                "Profile reference union differs from displayed Action sources/thread.",
            ));
        }
        for reference in &self.references {
            let resolution = &reference.resolution;
            if reference
                .matches
                .iter()
                .map(|item| item.note.clone())
                .collect::<Vec<_>>()
                != resolution.matches
                || resolution
                    .matches
                    .windows(2)
                    .any(|pair| pair[0].path >= pair[1].path)
            {
                return Err(rejected(
                    "Profile reference matches differ from identity resolution.",
                ));
            }
            let outcome = if resolution.matches.len() > 1 {
                IdentityOutcome::Ambiguous
            } else if !resolution.issues.is_empty() {
                IdentityOutcome::Incomplete
            } else if resolution.matches.is_empty() {
                IdentityOutcome::Absent
            } else {
                IdentityOutcome::Unique
            };
            if resolution.outcome != outcome
                || resolution.outcome == IdentityOutcome::Ambiguous
                    && !self.duplicates.iter().any(|duplicate| {
                        duplicate.note_id == resolution.note_id
                            && duplicate.paths
                                == resolution
                                    .matches
                                    .iter()
                                    .map(|note| note.path.clone())
                                    .collect::<Vec<_>>()
                    })
            {
                return Err(rejected("Inconsistent profile identity outcome."));
            }
            for issue in &resolution.issues {
                validate_issue(issue)?;
                if !self.issues.contains(issue) {
                    return Err(rejected(
                        "Reference diagnostic omitted from profile coverage.",
                    ));
                }
            }
            for item in &reference.matches {
                EvidencePath::parse(&item.note.path)
                    .map_err(|error| rejected(error.to_string()))?;
                if item.note.note_id != Some(resolution.note_id)
                    || item.scope.is_some_and(|scope| !known(scope))
                    || resolution.outcome == IdentityOutcome::Unique && item.scope.is_none()
                    || item.scope.is_none()
                        && !self.issues.iter().any(|issue| issue.path == item.note.path)
                {
                    return Err(rejected(
                        "Invalid or unreported profile reference classification.",
                    ));
                }
                if endpoints
                    .get(item.note.path.as_str())
                    .is_some_and(|(id, sha256, scope)| {
                        item.note.note_id != Some(*id)
                            || item.note.sha256 != *sha256
                            || item.scope != Some(*scope)
                    })
                {
                    return Err(rejected(
                        "Reference and relationship endpoint observations disagree.",
                    ));
                }
                if item.note.note_id == Some(request.note_id)
                    && (item.note.path != request.profile.path
                        || item.note.sha256 != request.profile.fingerprint.sha256
                        || item.scope != Some(KnowledgeScope::Current))
                {
                    return Err(rejected(
                        "Reference profile proof differs from captured profile.",
                    ));
                }
            }
        }
        Ok(())
    }
}

impl App {
    fn read_profile_source(&self, path: &str) -> Result<ProposalSource> {
        let path = EvidencePath::parse(path).map_err(|error| rejected(error.to_string()))?;
        vault::read_evidence_source(self.require_vault()?, &path)
            .map_err(|error| WorkflowError::typed(ErrorKind::ContextStale, error.to_string()))
    }

    /// Fresh saved observations, not an atomic vault snapshot or semantic authority.
    pub fn profile_context(&mut self, request: &ProfileContextRequest) -> Result<ProfileContext> {
        request.validate()?;
        self.require_current_evidence()?;
        let (tools, epoch, vault) = self.citation_review_boundary()?;
        let profile = self.read_profile_source(&request.profile.path)?;
        exact_profile(&profile, request)
            .map_err(|error| WorkflowError::typed(ErrorKind::ContextStale, error.message))?;
        let observation = self.collect_relationships()?;
        let resolution = observation.inventory.resolution(request.note_id);
        if resolution.outcome != IdentityOutcome::Unique
            || resolution.matches[0].path != request.profile.path
            || resolution.matches[0].sha256 != request.profile.fingerprint.sha256
        {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                format!(
                    "Profile identity is {:?}; context needs one exact saved Current identity with complete inspection.",
                    resolution.outcome
                ),
            ));
        }
        let mut issues = observation.issues;
        let mut scopes = BTreeMap::new();
        // Classify the same All-scope inventory before selecting direct edges.
        // A changed/unreadable observation stays unknown, never guessed Current.
        for identity in &observation.inventory.notes {
            let scope = match self.evidence_note(&identity.path) {
                Ok(note)
                    if note.sha256 == identity.sha256
                        && note_identity::read(&note.text).ok().flatten() == identity.note_id =>
                {
                    classify(&note.text, &identity.path)
                }
                Ok(_) => Err("Saved note changed during profile inspection.".into()),
                Err(error) => Err(error.to_string()),
            };
            match scope {
                Ok(scope) => {
                    scopes.insert(identity.path.clone(), Some(scope));
                }
                Err(reason) => {
                    scopes.insert(identity.path.clone(), None);
                    issues.push(IdentityIssue {
                        path: identity.path.clone(),
                        reason,
                    });
                }
            }
        }
        let mut matching = Vec::new();
        let mut before = None;
        loop {
            let page = self.actions(&ActionListRequest {
                state: None,
                limit: 200,
                before,
            })?;
            for action in page.entries {
                action
                    .validate()
                    .map_err(|error| rejected(error.to_string()))?;
                if request.matches(&action) {
                    matching.push(action);
                }
            }
            let Some(next) = page.next_before else {
                break;
            };
            if before.is_some_and(|cursor: ActionCursor| {
                (next.created_at_ms, next.id) >= (cursor.created_at_ms, cursor.id)
            }) {
                return Err(rejected(
                    "Action cursor did not advance during profile inspection.",
                ));
            }
            before = Some(next);
        }
        let action_total = matching.len();
        let actions = matching
            .into_iter()
            .skip(request.action_offset)
            .take(request.limit)
            .collect::<Vec<_>>();
        let relationships = observation
            .edges
            .into_iter()
            .filter(|edge| {
                edge.source.note_id == request.note_id || edge.target.note_id == request.note_id
            })
            .filter_map(|edge| {
                Some(ProfileRelationship {
                    source_scope: scopes.get(&edge.source.path).copied().flatten()?,
                    target_scope: scopes.get(&edge.target.path).copied().flatten()?,
                    edge,
                })
            })
            .collect::<Vec<_>>();
        let relationship_total = relationships.len();
        let relationships = relationships
            .into_iter()
            .skip(request.relationship_offset)
            .take(request.limit)
            .collect();
        issues.sort_by(|a, b| a.path.cmp(&b.path).then(a.reason.cmp(&b.reason)));
        issues.dedup();
        let references = reference_ids(&actions)
            .into_iter()
            .map(|id| {
                let mut resolution = observation.inventory.resolution(id);
                // Changed/unreadable reference bytes cannot retain Unique navigation.
                for note in &resolution.matches {
                    if scopes.get(&note.path).copied().flatten().is_none() {
                        resolution.issues.extend(
                            issues
                                .iter()
                                .filter(|issue| issue.path == note.path)
                                .cloned(),
                        );
                    }
                }
                resolution
                    .issues
                    .sort_by(|a, b| a.path.cmp(&b.path).then(a.reason.cmp(&b.reason)));
                resolution.issues.dedup();
                if resolution.matches.len() <= 1 && !resolution.issues.is_empty() {
                    resolution.outcome = IdentityOutcome::Incomplete;
                }
                let matches = resolution
                    .matches
                    .iter()
                    .map(|note| ProfileNote {
                        note: note.clone(),
                        scope: scopes.get(&note.path).copied().flatten(),
                    })
                    .collect();
                ProfileReference {
                    resolution,
                    matches,
                }
            })
            .collect();
        let final_profile = self.read_profile_source(&request.profile.path)?;
        exact_profile(&final_profile, request)
            .map_err(|error| WorkflowError::typed(ErrorKind::ContextStale, error.message))?;
        self.require_current_evidence()?;
        self.citation_review_end(&tools, epoch, &vault)?;
        let duplicates = observation.inventory.duplicates;
        let context = ProfileContext {
            request: request.clone(),
            profile,
            action_total,
            actions,
            relationship_total,
            relationships,
            references,
            complete: issues.is_empty() && duplicates.is_empty(),
            issues,
            duplicates,
        };
        context.validate_for(request)?;
        Ok(context)
    }
}
