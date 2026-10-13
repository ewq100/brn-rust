//! The embedding model interface shared by the search index and the local
//! model. Implemented by `native::LocalEmbedder` and by test fakes.
use crate::Result;

/// Turns text into vectors.
pub trait Embedder {
    /// Stable identity of the model and its settings; a change rebuilds all embeddings.
    fn identity(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    /// Text prefixes the model expects for queries and passages (for
    /// example `query: ` and `passage: `); none by default.
    fn prefixes(&self) -> (&'static str, &'static str) {
        ("", "")
    }
    /// Cosine similarity below which a passage does not count as a semantic
    /// match. Models differ in their similarity range.
    fn similarity_floor(&self) -> f32 {
        0.5
    }
}
