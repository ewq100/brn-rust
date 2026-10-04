//! Fresh relationship observations; the index is a disposable cache only.
use super::{
    DuplicateIdentity, IdentityInventory, IdentityIssue, IdentityOutcome, NoteIdentityInfo,
    NoteLinkOutcome, rejected,
};
use crate::app::App;
use crate::{
    Result,
    library::{KnowledgeScope, saved_metadata},
};
use brn_store::{note_identity, note_provenance};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use brn_retrieval::note_index::{
    EdgeEndpoint, EdgeEvidence, EdgeOrigin, EvidenceEndpoint, NoteEdge,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipRequest {
    pub scope: KnowledgeScope,
    pub offset: usize,
    pub limit: usize,
}

impl RelationshipRequest {
    pub fn validate(&self) -> Result<()> {
        if !(1..=200).contains(&self.limit) {
            return Err(rejected(
                "Relationship page limit must be between 1 and 200.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipPage {
    pub scope: KnowledgeScope,
    pub offset: usize,
    pub total: usize,
    pub edges: Vec<NoteEdge>,
    pub issues: Vec<IdentityIssue>,
    pub duplicates: Vec<DuplicateIdentity>,
}

impl App {
    /// Rebuilds a disposable observation from saved evidence, without AI or writes
    /// to knowledge. Exact endpoint witnesses do not imply a vault transaction.
    pub fn relationships(&mut self, request: &RelationshipRequest) -> Result<RelationshipPage> {
        request.validate()?;
        self.refresh()?;
        let inventory = self.identity_inventory()?;
        let mut issues = inventory.issues.clone();
        let mut edges = BTreeMap::<(String, String, EdgeOrigin), NoteEdge>::new();
        if inventory.issues.is_empty() {
            for source in &inventory.notes {
                let Some(source_endpoint) = endpoint(source) else {
                    continue;
                };
                if inventory.resolution(source_endpoint.note_id).outcome != IdentityOutcome::Unique
                {
                    continue;
                }
                if let Err(error) =
                    self.derive_relationships(source, &inventory, &mut edges, &mut issues)
                {
                    issues.push(IdentityIssue {
                        path: source.path.clone(),
                        reason: error.to_string(),
                    });
                }
            }
        }
        let mut edges = edges.into_values().collect::<Vec<_>>();
        // Each endpoint must still match the exact saved version used above.
        // A later external edit is reported, never certified by cached metadata.
        let endpoints = edges
            .iter()
            .flat_map(|edge| [&edge.source, &edge.target])
            .map(|endpoint| (endpoint.path.clone(), endpoint.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut changed = Vec::new();
        for (path, endpoint) in endpoints {
            match self.evidence_note(&path) {
                Ok(note)
                    if note.sha256 == endpoint.sha256
                        && note_identity::read(&note.text).ok().flatten()
                            == Some(endpoint.note_id)
                        && saved_metadata(&note.text, &path).issue.is_none() => {}
                result => {
                    changed.push(path.clone());
                    issues.push(IdentityIssue {
                        path,
                        reason: match result {
                            Ok(_) => "Relationship endpoint changed during inspection.".into(),
                            Err(error) => error.to_string(),
                        },
                    });
                }
            }
        }
        edges.retain(|edge| {
            !changed.contains(&edge.source.path) && !changed.contains(&edge.target.path)
        });
        let page = self.cache_relationships(&edges, request)?;
        issues.sort_by(|a, b| a.path.cmp(&b.path).then(a.reason.cmp(&b.reason)));
        issues.dedup();
        Ok(RelationshipPage {
            scope: page.scope,
            offset: page.offset,
            total: page.total,
            edges: page.edges,
            issues,
            duplicates: inventory.duplicates,
        })
    }

    fn derive_relationships(
        &self,
        source: &NoteIdentityInfo,
        inventory: &IdentityInventory,
        edges: &mut BTreeMap<(String, String, EdgeOrigin), NoteEdge>,
        issues: &mut Vec<IdentityIssue>,
    ) -> Result<()> {
        let note = self.evidence_note(&source.path)?;
        let source_endpoint = endpoint(source).expect("managed source");
        if note.sha256 != source.sha256 {
            return Err(rejected("Relationship source changed during inspection."));
        }
        let metadata = saved_metadata(&note.text, &source.path);
        if let Some(reason) = metadata.issue {
            issues.push(IdentityIssue {
                path: source.path.clone(),
                reason,
            });
            return Ok(());
        }
        let links = self.links_from_saved(&source.path, &note, inventory)?;
        for link in links.links {
            issues.extend(link.issues);
            if link.outcome != NoteLinkOutcome::Resolved {
                continue;
            }
            let target = &link.matches[0];
            let Some(target_endpoint) = endpoint(target) else {
                continue;
            };
            let target_note = self.evidence_note(&target.path)?;
            if target_note.sha256 != target.sha256
                || saved_metadata(&target_note.text, &target.path)
                    .issue
                    .is_some()
            {
                continue;
            }
            merge(
                edges,
                NoteEdge {
                    source: source_endpoint.clone(),
                    target: target_endpoint,
                    origin: EdgeOrigin::ExplicitLink,
                    evidence: link
                        .evidence
                        .into_iter()
                        .map(|proof| EdgeEvidence {
                            endpoint: EvidenceEndpoint::Source,
                            start_byte: proof.start_byte,
                            end_byte: proof.end_byte,
                            quote: proof.quote,
                        })
                        .collect(),
                },
            );
        }
        for citation in
            note_provenance::read(&note.text).map_err(|error| rejected(error.to_string()))?
        {
            let resolution = inventory.resolution(citation.note_id);
            if resolution.outcome != IdentityOutcome::Unique {
                continue;
            }
            let target = &resolution.matches[0];
            if target.sha256 != citation.sha256 {
                continue;
            }
            let target_note = self.evidence_note(&target.path)?;
            if target_note.sha256 != citation.sha256
                || target_note.text.get(citation.start_byte..citation.end_byte)
                    != Some(citation.quote.as_str())
                || saved_metadata(&target_note.text, &target.path)
                    .issue
                    .is_some()
            {
                continue;
            }
            merge(
                edges,
                NoteEdge {
                    source: source_endpoint.clone(),
                    target: endpoint(target).expect("resolved managed target"),
                    origin: EdgeOrigin::InferredProvenance,
                    evidence: vec![EdgeEvidence {
                        endpoint: EvidenceEndpoint::Target,
                        start_byte: citation.start_byte,
                        end_byte: citation.end_byte,
                        quote: citation.quote,
                    }],
                },
            );
        }
        Ok(())
    }
}

fn endpoint(note: &NoteIdentityInfo) -> Option<EdgeEndpoint> {
    Some(EdgeEndpoint {
        path: note.path.clone(),
        note_id: note.note_id?,
        sha256: note.sha256,
    })
}

fn merge(edges: &mut BTreeMap<(String, String, EdgeOrigin), NoteEdge>, edge: NoteEdge) {
    if edge.source.note_id == edge.target.note_id {
        return;
    }
    let key = (
        edge.source.path.clone(),
        edge.target.path.clone(),
        edge.origin,
    );
    if let Some(previous) = edges.get_mut(&key) {
        for proof in edge.evidence {
            if !previous.evidence.contains(&proof) {
                previous.evidence.push(proof);
            }
        }
    } else {
        edges.insert(key, edge);
    }
}
