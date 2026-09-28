use crate::{
    Chunk, Document, Error, Result, check_cancel, hash, inventory, native_identity, sync_dir,
};
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int32Array, RecordBatch, StringArray,
    types::Float32Type,
};
use arrow_schema::{DataType, Field, Schema};
use fastembed::{
    InitOptionsUserDefined, Pooling, TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel,
};
use futures::TryStreamExt;
use lancedb::{
    DistanceType, Table,
    query::{ExecutableQuery, QueryBase},
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

type BuildArtifacts = (String, BTreeMap<String, String>, BTreeMap<String, String>);
const FILES: [&str; 5] = [
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];
const DIM: usize = 384;
const BATCH: usize = 16;
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
fn open_model(dir: &Path) -> Result<TextEmbedding> {
    let load = |name| fs::read(dir.join(name)).map_err(Error::Io);
    let files = TokenizerFiles {
        tokenizer_file: load("tokenizer.json")?,
        config_file: load("config.json")?,
        special_tokens_map_file: load("special_tokens_map.json")?,
        tokenizer_config_file: load("tokenizer_config.json")?,
    };
    let user =
        UserDefinedEmbeddingModel::new(load("model.onnx")?, files).with_pooling(Pooling::Mean);
    TextEmbedding::try_new_from_user_defined(
        user,
        InitOptionsUserDefined::default().with_intra_threads(4),
    )
    .map_err(native)
}
fn persist_model(
    source: &Path,
    dest: &Path,
    cancel: &AtomicBool,
) -> Result<BTreeMap<String, String>> {
    if !source.is_dir() {
        return Err(Error::Unavailable("model directory absent"));
    }
    fs::create_dir(dest)?;
    let mut files = BTreeMap::new();
    for name in FILES {
        check_cancel(cancel)?;
        let src = source.join(name);
        if !src.is_file() || src.symlink_metadata()?.file_type().is_symlink() {
            return Err(Error::Unavailable("verified local model files required"));
        }
        let bytes = fs::read(src)?;
        let mut file = File::create_new(dest.join(name))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        files.insert(name.into(), hash(&bytes));
    }
    sync_dir(dest)?;
    Ok(files)
}
fn schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("source_id", DataType::Utf8, false),
        Field::new("version_id", DataType::Utf8, false),
        Field::new("source_hash", DataType::Utf8, false),
        Field::new("start", DataType::Int32, false),
        Field::new("end", DataType::Int32, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                DIM as i32,
            ),
            false,
        ),
    ]))
}
fn batch(chunks: &[Chunk], docs: &[Document], vectors: &[Vec<f32>]) -> Result<RecordBatch> {
    if vectors.len() != chunks.len() || vectors.iter().any(|v| v.len() != DIM) {
        return Err(Error::Corrupt("embedding count or dimensions"));
    }
    let arr = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        vectors
            .iter()
            .map(|v| Some(v.iter().copied().map(Some).collect::<Vec<_>>())),
        DIM as i32,
    );
    let ids = Int32Array::from_iter_values((0..chunks.len()).map(|v| v as i32));
    let sources = StringArray::from(
        chunks
            .iter()
            .map(|c| docs[c.doc].source_id.as_str())
            .collect::<Vec<_>>(),
    );
    let versions = StringArray::from(
        chunks
            .iter()
            .map(|c| docs[c.doc].version_id.as_str())
            .collect::<Vec<_>>(),
    );
    let hashes = StringArray::from(
        chunks
            .iter()
            .map(|c| docs[c.doc].source_hash.as_str())
            .collect::<Vec<_>>(),
    );
    let starts = Int32Array::from_iter_values(chunks.iter().map(|c| c.start as i32));
    let ends = Int32Array::from_iter_values(chunks.iter().map(|c| c.end as i32));
    RecordBatch::try_new(
        schema(),
        vec![
            Arc::new(ids),
            Arc::new(sources),
            Arc::new(versions),
            Arc::new(hashes),
            Arc::new(starts),
            Arc::new(ends),
            Arc::new(arr),
        ],
    )
    .map_err(native)
}
fn sync_tree(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(Error::Corrupt("symlink in LanceDB"));
        }
        if kind.is_dir() {
            sync_tree(&entry.path())?;
        } else if kind.is_file() {
            File::open(entry.path())?.sync_all()?;
        } else {
            return Err(Error::Corrupt("invalid LanceDB entry"));
        }
    }
    sync_dir(path)
}

pub(super) fn build(
    root: &Path,
    chunks: &[Chunk],
    docs: &[Document],
    model_dir: &Path,
    cancel: &AtomicBool,
    on_progress: &mut impl FnMut(&str),
) -> Result<BuildArtifacts> {
    let model_files = persist_model(model_dir, &root.join("model"), cancel)?;
    check_cancel(cancel)?;
    let mut model = open_model(&root.join("model"))?;
    check_cancel(cancel)?; // ORT initialization itself is not interruptible.
    let mut vectors = Vec::with_capacity(chunks.len());
    for group in chunks.chunks(BATCH) {
        check_cancel(cancel)?;
        on_progress("embedding batch");
        check_cancel(cancel)?;
        let texts: Vec<&str> = group
            .iter()
            .map(|c| &docs[c.doc].text[c.start..c.end])
            .collect();
        let embedded = model.embed(&texts, None).map_err(native)?;
        if embedded.len() != group.len() {
            return Err(Error::Corrupt("embedding count"));
        }
        vectors.extend(embedded);
        check_cancel(cancel)?; // Each ORT batch is bounded to at most 16 passages.
    }
    check_cancel(cancel)?;
    on_progress("writing vector index");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(native)?;
    let batch = batch(chunks, docs, &vectors)?;
    runtime.block_on(async {
        let db = lancedb::connect(root.join("lancedb").to_string_lossy().as_ref())
            .execute()
            .await
            .map_err(native)?;
        db.create_table("passages", batch)
            .execute()
            .await
            .map_err(native)?;
        Ok::<_, Error>(())
    })?;
    check_cancel(cancel)?;
    sync_tree(&root.join("lancedb"))?;
    let db_files = inventory(&root.join("lancedb"))?;
    Ok((native_identity().into(), model_files, db_files))
}

pub(super) struct NativeState {
    runtime: tokio::runtime::Runtime,
    model: TextEmbedding,
    table: Table,
    empty: bool,
}
impl NativeState {
    pub(super) fn open(root: &Path, docs: &[Document], chunks: &[Chunk]) -> Result<Self> {
        let model = open_model(&root.join("model"))?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(native)?;
        let table = runtime.block_on(async {
            let db = lancedb::connect(root.join("lancedb").to_string_lossy().as_ref())
                .execute()
                .await
                .map_err(native)?;
            db.open_table("passages").execute().await.map_err(native)
        })?;
        runtime.block_on(verify_table(&table, docs, chunks))?;
        Ok(Self {
            runtime,
            model,
            table,
            empty: chunks.is_empty(),
        })
    }
    pub(super) fn search(&mut self, query: &str, limit: usize) -> Result<Vec<(usize, f32)>> {
        if self.empty {
            return Ok(Vec::new());
        }
        let vector = self.model.embed([query], None).map_err(native)?.remove(0);
        if vector.len() != DIM {
            return Err(Error::Corrupt("query embedding dimensions"));
        }
        self.runtime.block_on(async {
            let batches: Vec<RecordBatch> = self
                .table
                .query()
                .nearest_to(vector.as_slice())
                .map_err(native)?
                .distance_type(DistanceType::Cosine)
                .limit(limit)
                .execute()
                .await
                .map_err(native)?
                .try_collect()
                .await
                .map_err(native)?;
            let mut result = Vec::new();
            for batch in batches {
                let ids = batch
                    .column_by_name("id")
                    .ok_or(Error::Corrupt("missing vector id"))?
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .ok_or(Error::Corrupt("vector id type"))?;
                let distance = batch
                    .column_by_name("_distance")
                    .ok_or(Error::Corrupt("missing vector distance"))?
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .ok_or(Error::Corrupt("vector distance type"))?;
                for row in 0..batch.num_rows() {
                    let id = usize::try_from(ids.value(row)).map_err(native)?;
                    result.push((id, 1.0 - distance.value(row)));
                }
            }
            Ok(result)
        })
    }
}
async fn verify_table(table: &Table, docs: &[Document], chunks: &[Chunk]) -> Result<()> {
    let batches: Vec<RecordBatch> = table
        .query()
        .execute()
        .await
        .map_err(native)?
        .try_collect()
        .await
        .map_err(native)?;
    let mut seen = vec![false; chunks.len()];
    for batch in batches {
        let int = |name| -> Result<&Int32Array> {
            batch
                .column_by_name(name)
                .ok_or(Error::Corrupt("missing vector provenance"))?
                .as_any()
                .downcast_ref::<Int32Array>()
                .ok_or(Error::Corrupt("vector provenance type"))
        };
        let string = |name| -> Result<&StringArray> {
            batch
                .column_by_name(name)
                .ok_or(Error::Corrupt("missing vector provenance"))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(Error::Corrupt("vector provenance type"))
        };
        let ids = int("id")?;
        let starts = int("start")?;
        let ends = int("end")?;
        let sources = string("source_id")?;
        let versions = string("version_id")?;
        let hashes = string("source_hash")?;
        for row in 0..batch.num_rows() {
            let id = usize::try_from(ids.value(row)).map_err(native)?;
            let chunk = chunks
                .get(id)
                .ok_or(Error::Corrupt("vector id outside passage map"))?;
            let doc = &docs[chunk.doc];
            if seen[id]
                || sources.value(row) != doc.source_id
                || versions.value(row) != doc.version_id
                || hashes.value(row) != doc.source_hash
                || starts.value(row) != chunk.start as i32
                || ends.value(row) != chunk.end as i32
            {
                return Err(Error::Corrupt("vector row provenance"));
            }
            seen[id] = true;
        }
    }
    if seen.iter().any(|seen| !*seen) {
        return Err(Error::Corrupt("vector rows missing"));
    }
    Ok(())
}
