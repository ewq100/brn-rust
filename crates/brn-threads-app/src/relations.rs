//! Recommended link relation names (D7). Core does not validate relation
//! names: any other concise name stays allowed and is shown as written.

/// The linking note draws on this evidence.
pub const SOURCE: &str = "source";
/// The linking note replaces the linked one.
pub const SUPERSEDES: &str = "supersedes";
/// The notes make unresolved conflicting claims.
pub const CONTRADICTS: &str = "contradicts";
pub const RELATED: &str = "related";

pub const RECOMMENDED: [&str; 4] = [SOURCE, SUPERSEDES, CONTRADICTS, RELATED];

/// Whether `relation` is one of the recommended names.
pub fn is_recommended(relation: &str) -> bool {
    RECOMMENDED.contains(&relation)
}
