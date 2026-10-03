//! The local embedding model: all-MiniLM-L6-v2 with mean pooling (384
//! dimensions) through FastEmbed, loaded from a folder holding its five files.
use crate::{Error, Result, note_index::Embedder};
use fastembed::{
    InitOptionsUserDefined, Pooling, TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel,
};
use sha2::{Digest, Sha256};
use std::path::Path;

const DIMENSION: usize = 384;

#[path = "native/download.rs"]
pub mod download;

#[cfg(test)]
#[path = "native/download_tests.rs"]
mod download_tests;

pub struct LocalEmbedder {
    model: TextEmbedding,
    identity: String,
}

impl LocalEmbedder {
    /// Loads the model from `dir`, which must contain `model.onnx`,
    /// `tokenizer.json`, `config.json`, `special_tokens_map.json` and
    /// `tokenizer_config.json` as regular files. Never downloads anything.
    /// The identity hashes all five files in that order, prefixed by each
    /// file's name and its byte length encoded as a big-endian u64.
    pub fn open(dir: &Path) -> Result<Self> {
        let load = |name: &str| -> Result<Vec<u8>> {
            let path = dir.join(name);
            match path.symlink_metadata() {
                Ok(meta) if meta.is_file() => Ok(std::fs::read(path)?),
                _ => Err(Error::Unavailable("embedding model files are missing")),
            }
        };
        let onnx = load("model.onnx")?;
        let files = TokenizerFiles {
            tokenizer_file: load("tokenizer.json")?,
            config_file: load("config.json")?,
            special_tokens_map_file: load("special_tokens_map.json")?,
            tokenizer_config_file: load("tokenizer_config.json")?,
        };
        let mut hash = Sha256::new();
        for (name, bytes) in [
            ("model.onnx", &onnx),
            ("tokenizer.json", &files.tokenizer_file),
            ("config.json", &files.config_file),
            ("special_tokens_map.json", &files.special_tokens_map_file),
            ("tokenizer_config.json", &files.tokenizer_config_file),
        ] {
            hash.update(name.as_bytes());
            hash.update((bytes.len() as u64).to_be_bytes());
            hash.update(bytes);
        }
        let digest = hex::encode(hash.finalize());
        let identity = format!(
            "fastembed-7.1.0/all-MiniLM-L6-v2/mean/{DIMENSION}/{}",
            &digest[..16]
        );
        let user = UserDefinedEmbeddingModel::new(onnx, files).with_pooling(Pooling::Mean);
        let model = TextEmbedding::try_new_from_user_defined(
            user,
            InitOptionsUserDefined::default().with_intra_threads(4),
        )
        .map_err(|e| Error::Native(e.to_string()))?;
        Ok(Self { model, identity })
    }
}

impl Embedder for LocalEmbedder {
    fn identity(&self) -> &str {
        &self.identity
    }
    fn dimension(&self) -> usize {
        DIMENSION
    }
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.model
            .embed(texts, None)
            .map_err(|e| Error::Native(e.to_string()))
    }
}
