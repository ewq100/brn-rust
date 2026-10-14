//! Pinned multilingual-e5-small retrieval (D3), still recognising the
//! previously pinned MiniLM model and explicit legacy files.
use crate::{Error, Result, embedding::Embedder};
use fastembed::{
    InitOptionsUserDefined, Pooling, QuantizationMode, TextEmbedding, TokenizerFiles,
    UserDefinedEmbeddingModel,
};
use sha2::{Digest, Sha256};
use std::path::Path;

const DIMENSION: usize = 384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    Legacy,
    Multilingual,
    E5Small,
}

fn profile_for_digest(digest: &[u8; 32]) -> Profile {
    let pinned = |assets: &[download::Asset]| matches!(assets[0].digest, download::DigestPin::Sha256(expected) if *digest == expected);
    if pinned(&download::ASSETS) {
        Profile::E5Small
    } else if pinned(&download::MINILM_ASSETS) {
        Profile::Multilingual
    } else {
        Profile::Legacy
    }
}
impl Profile {
    fn options(self) -> InitOptionsUserDefined {
        let options = InitOptionsUserDefined::default().with_intra_threads(4);
        match self {
            Self::Legacy => options,
            Self::Multilingual => options.with_max_length(128),
            Self::E5Small => options.with_max_length(512),
        }
    }
    fn quantization(self) -> QuantizationMode {
        match self {
            Self::Legacy => QuantizationMode::None,
            Self::Multilingual | Self::E5Small => QuantizationMode::Static,
        }
    }
    fn assets(self) -> Option<&'static [download::Asset]> {
        match self {
            Self::Legacy => None,
            Self::Multilingual => Some(&download::MINILM_ASSETS),
            Self::E5Small => Some(&download::ASSETS),
        }
    }
    fn prefixes(self) -> (&'static str, &'static str) {
        match self {
            Self::E5Small => ("query: ", "passage: "),
            Self::Legacy | Self::Multilingual => ("", ""),
        }
    }
    /// e5 compresses similarities (unrelated English text scores about
    /// 0.78–0.82). Measured on the synthetic golden set (see the plan's
    /// evidence): 0.82 and a 0.017 margin keep abstention empty.
    fn similarity_floor(self) -> f32 {
        match self {
            Self::E5Small => 0.82,
            Self::Legacy | Self::Multilingual => 0.5,
        }
    }
    fn min_margin(self) -> f32 {
        match self {
            Self::E5Small => 0.017,
            Self::Legacy | Self::Multilingual => 0.0,
        }
    }
}

fn model_identity(profile: Profile, files: &[(&str, &[u8])]) -> String {
    let mut hash = Sha256::new();
    for (name, bytes) in files {
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    let digest = hex::encode(hash.finalize());
    match profile {
        Profile::Legacy => format!(
            "fastembed-7.1.0/all-MiniLM-L6-v2/mean/{DIMENSION}/{}",
            &digest[..16]
        ),
        Profile::Multilingual => format!(
            "fastembed-7.1.0/paraphrase-multilingual-MiniLM-L12-v2/mean/{DIMENSION}/max128/static/{digest}"
        ),
        Profile::E5Small => {
            format!("fastembed-7.1.0/multilingual-e5-small/mean/{DIMENSION}/max512/static/{digest}")
        }
    }
}

#[path = "native/download.rs"]
pub mod download;

#[cfg(test)]
#[path = "native/download_tests.rs"]
mod download_tests;

pub struct LocalEmbedder {
    model: TextEmbedding,
    identity: String,
    profile: Profile,
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
        let profile = profile_for_digest(&Sha256::digest(&onnx).into());
        let buffers: [(&str, &[u8]); 5] = [
            ("model.onnx", &onnx),
            ("tokenizer.json", &files.tokenizer_file),
            ("config.json", &files.config_file),
            ("special_tokens_map.json", &files.special_tokens_map_file),
            ("tokenizer_config.json", &files.tokenizer_config_file),
        ];
        if let Some(assets) = profile.assets() {
            for (asset, (name, bytes)) in assets.iter().zip(buffers) {
                if asset.name != name {
                    return Err(Error::Unavailable(
                        "embedding model companion order differs",
                    ));
                }
                download::verify_bytes(asset, bytes)?;
            }
        }
        let identity = model_identity(profile, &buffers);
        let user = UserDefinedEmbeddingModel::new(onnx, files)
            .with_pooling(Pooling::Mean)
            .with_quantization(profile.quantization());
        let model = TextEmbedding::try_new_from_user_defined(user, profile.options())
            .map_err(|e| Error::Native(e.to_string()))?;
        Ok(Self {
            model,
            identity,
            profile,
        })
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
    fn prefixes(&self) -> (&'static str, &'static str) {
        self.profile.prefixes()
    }
    fn similarity_floor(&self) -> f32 {
        self.profile.similarity_floor()
    }
    fn min_margin(&self) -> f32 {
        self.profile.min_margin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_e5_onnx_selects_its_profile_prefixes_and_identity() {
        let digest: [u8; 32] =
            hex::decode("f80102d3f2a1229f387d3c81909990d8945513e347b0eab049f7de3c6f98c193")
                .unwrap()
                .try_into()
                .unwrap();
        let profile = profile_for_digest(&digest);
        assert_eq!(profile, Profile::E5Small);
        assert_eq!(profile.options().max_length, 512);
        assert!(matches!(profile.quantization(), QuantizationMode::Static));
        assert_eq!(profile.prefixes(), ("query: ", "passage: "));
        assert_eq!(
            (profile.similarity_floor(), profile.min_margin()),
            (0.82, 0.017)
        );
        assert_eq!(profile.assets().unwrap()[0].bytes, 118_308_185);
        let identity = model_identity(profile, &[("model.onnx", b"synthetic model")]);
        assert!(
            identity.starts_with("fastembed-7.1.0/multilingual-e5-small/mean/384/max512/static/")
        );
        assert_eq!(Profile::Multilingual.prefixes(), ("", ""));
    }

    #[test]
    fn pinned_multilingual_onnx_selects_its_own_profile() {
        let digest: [u8; 32] =
            hex::decode("66fc00f5f29afcaff34092e1bdd20008ca3918265a82fb9695a551e510cc4ebc")
                .unwrap()
                .try_into()
                .unwrap();
        let profile = profile_for_digest(&digest);
        assert_eq!(profile, Profile::Multilingual);
        assert_eq!(profile.options().max_length, 128);
        assert!(matches!(profile.quantization(), QuantizationMode::Static));
        let identity = model_identity(
            profile,
            &[
                ("model.onnx", b"synthetic model"),
                ("tokenizer.json", b"exact tokenizer"),
            ],
        );
        assert!(identity.starts_with(
            "fastembed-7.1.0/paraphrase-multilingual-MiniLM-L12-v2/mean/384/max128/static/"
        ));
        assert_eq!(identity.rsplit('/').next().unwrap().len(), 64);
        assert_ne!(
            identity,
            model_identity(
                profile,
                &[
                    ("model.onnx", b"synthetic model"),
                    ("tokenizer.json", b"changed tokenizer")
                ]
            )
        );
        assert_ne!(
            identity,
            model_identity(
                Profile::Legacy,
                &[
                    ("model.onnx", b"synthetic model"),
                    ("tokenizer.json", b"exact tokenizer")
                ]
            )
        );
    }

    #[test]
    fn legacy_explicit_model_identity_keeps_the_original_byte_contract() {
        let files: [(&str, &[u8]); 2] = [
            ("model.onnx", b"synthetic model"),
            ("tokenizer.json", b"exact tokenizer"),
        ];
        assert_eq!(profile_for_digest(&[0x17; 32]), Profile::Legacy);
        assert_eq!(Profile::Legacy.options().max_length, 512);
        assert!(matches!(
            Profile::Legacy.quantization(),
            QuantizationMode::None
        ));
        assert_eq!(
            model_identity(Profile::Legacy, &files),
            "fastembed-7.1.0/all-MiniLM-L6-v2/mean/384/60a5b7e9ab5b12e1"
        );
    }
}
