//! Explicit FastEmbed CPU + LanceDB exact cosine trial. Normal queries load only
//! the pinned model files copied into a completed state; no hub path is used.
use crate::{
    eligible, fixtures, fuse, hash, search_keyword, Document, Error, Evidence, Filters, Profile,
    Request, Result, ScoreKind,
};
use arrow_array::{
    types::Float32Type, Array, FixedSizeListArray, Float32Array, Int32Array, RecordBatch,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use fastembed::{
    EmbeddingModel, InitOptionsUserDefined, Pooling, TextEmbedding, TextInitOptions,
    TokenizerFiles, UserDefinedEmbeddingModel,
};
use futures::TryStreamExt;
use lancedb::{
    query::{ExecutableQuery, QueryBase},
    DistanceType, Table,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

const FILES: [&str; 5] = [
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];
const MODEL: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
const DIM: usize = 384;

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: u32,
    model: String,
    dimensions: usize,
    pooling: String,
    runtime: String,
    docs_sha256: String,
    files_sha256: BTreeMap<String, String>,
    db_sha256: BTreeMap<String, String>,
}
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
fn read_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(native)
}
fn model_files_dir(root: &Path) -> Result<PathBuf> {
    fn visit(dir: &Path) -> Option<PathBuf> {
        if FILES.iter().all(|name| dir.join(name).is_file()) {
            return Some(dir.to_path_buf());
        }
        for entry in fs::read_dir(dir).ok()?.flatten() {
            if entry.file_type().ok()?.is_dir() {
                if let Some(found) = visit(&entry.path()) {
                    return Some(found);
                }
            }
        }
        None
    }
    visit(root).ok_or_else(|| native("FastEmbed cache lacks a complete AllMiniLML6V2 snapshot"))
}
fn persist_model(cache: &Path, dest: &Path) -> Result<BTreeMap<String, String>> {
    let source = model_files_dir(cache)?;
    fs::create_dir(dest).map_err(native)?;
    let mut hashes = BTreeMap::new();
    for name in FILES {
        let bytes = read_file(&source.join(name))?;
        fs::write(dest.join(name), &bytes).map_err(native)?;
        hashes.insert(name.into(), hash(&bytes));
    }
    Ok(hashes)
}
fn inventory(root: &Path) -> Result<BTreeMap<String, String>> {
    fn walk(root: &Path, dir: &Path, found: &mut BTreeMap<String, String>) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(native)? {
            let entry = entry.map_err(native)?;
            let kind = entry.file_type().map_err(native)?;
            if kind.is_symlink() {
                return Err(native("symlink in database state"));
            }
            if kind.is_dir() {
                walk(root, &entry.path(), found)?;
            } else if kind.is_file() {
                let rel = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(native)?
                    .to_string_lossy()
                    .into_owned();
                found.insert(rel, hash(&read_file(&entry.path())?));
            } else {
                return Err(native("unsupported database state entry"));
            }
        }
        Ok(())
    }
    if !root.is_dir() {
        return Err(native("missing LanceDB directory"));
    }
    let mut found = BTreeMap::new();
    walk(root, root, &mut found)?;
    if found.is_empty() {
        return Err(native("empty LanceDB directory"));
    }
    Ok(found)
}
fn open_model(path: &Path) -> Result<TextEmbedding> {
    let load = |name| read_file(&path.join(name));
    let files = TokenizerFiles {
        tokenizer_file: load("tokenizer.json")?,
        config_file: load("config.json")?,
        special_tokens_map_file: load("special_tokens_map.json")?,
        tokenizer_config_file: load("tokenizer_config.json")?,
    };
    let model =
        UserDefinedEmbeddingModel::new(load("model.onnx")?, files).with_pooling(Pooling::Mean);
    TextEmbedding::try_new_from_user_defined(
        model,
        InitOptionsUserDefined::default().with_intra_threads(4),
    )
    .map_err(native)
}
fn open_state(path: &Path) -> Result<(Vec<Document>, TextEmbedding)> {
    if !path.join("COMPLETE").is_file() {
        return Err(native("missing or incomplete retrieval state"));
    }
    let manifest: Manifest =
        serde_json::from_slice(&read_file(&path.join("manifest.json"))?).map_err(native)?;
    if manifest.format != 1
        || manifest.model != MODEL
        || manifest.dimensions != DIM
        || manifest.pooling != "mean"
        || manifest.runtime != "fastembed-7.1.0"
    {
        return Err(native("incompatible retrieval state/model"));
    }
    let docs_bytes = read_file(&path.join("documents.json"))?;
    if hash(&docs_bytes) != manifest.docs_sha256 {
        return Err(native("document snapshot hash mismatch"));
    }
    let docs: Vec<Document> = serde_json::from_slice(&docs_bytes).map_err(native)?;
    for doc in &docs {
        doc.verify()?;
    }
    if docs.len() != fixtures().len()
        || docs.iter().zip(fixtures()).any(|(a, b)| {
            a.source_id != b.source_id
                || a.version_id != b.version_id
                || a.source_hash != b.source_hash
                || a.current != b.current
                || a.approval != b.approval
        })
    {
        return Err(native("fixture generation mismatch"));
    }
    if inventory(&path.join("lancedb"))? != manifest.db_sha256 {
        return Err(native("LanceDB file inventory mismatch"));
    }
    for name in FILES {
        let actual = hash(&read_file(&path.join("model").join(name))?);
        if manifest.files_sha256.get(name).map(String::as_str) != Some(actual.as_str()) {
            return Err(native(format!("model file mismatch: {name}")));
        }
    }
    Ok((docs, open_model(&path.join("model"))?))
}

pub async fn build(path: &Path) -> Result<()> {
    fs::create_dir(path).map_err(|e| native(format!("new state directory required: {e}")))?;
    let start = Instant::now();
    let cache = path.join("hf-cache");
    fs::create_dir(&cache).map_err(native)?;
    let mut download_model = TextEmbedding::try_new(
        TextInitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_cache_dir(cache.clone())
            .with_intra_threads(4),
    )
    .map_err(native)?;
    let model_hashes = persist_model(&cache, &path.join("model"))?;
    let docs = fixtures();
    eligible(&docs, &Filters::default())?;
    let texts: Vec<&str> = docs.iter().map(|d| d.text.as_str()).collect();
    let vectors = download_model.embed(&texts, None).map_err(native)?;
    if vectors.len() != docs.len() || vectors.iter().any(|v| v.len() != DIM) {
        return Err(native("unexpected embedding dimensions/count"));
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("source_id", DataType::Utf8, false),
        Field::new("version_id", DataType::Utf8, false),
        Field::new("source_hash", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                DIM as i32,
            ),
            false,
        ),
    ]));
    let ids = Int32Array::from_iter_values((0..docs.len()).map(|id| id as i32));
    let arr = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        vectors
            .iter()
            .map(|v| Some(v.iter().copied().map(Some).collect::<Vec<_>>())),
        DIM as i32,
    );
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(StringArray::from(
                docs.iter()
                    .map(|d| d.source_id.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                docs.iter()
                    .map(|d| d.version_id.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                docs.iter()
                    .map(|d| d.source_hash.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(arr),
        ],
    )
    .map_err(native)?;
    let db = lancedb::connect(path.join("lancedb").to_string_lossy().as_ref())
        .execute()
        .await
        .map_err(native)?;
    db.create_table("passages", batch)
        .execute()
        .await
        .map_err(native)?;
    let docs_bytes = serde_json::to_vec_pretty(&docs).map_err(native)?;
    fs::write(path.join("documents.json"), &docs_bytes).map_err(native)?;
    let manifest = Manifest {
        format: 1,
        model: MODEL.into(),
        dimensions: DIM,
        pooling: "mean".into(),
        runtime: "fastembed-7.1.0".into(),
        docs_sha256: hash(&docs_bytes),
        files_sha256: model_hashes,
        db_sha256: inventory(&path.join("lancedb"))?,
    };
    fs::write(
        path.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(native)?,
    )
    .map_err(native)?;
    fs::write(path.join("COMPLETE"), b"brn retrieval trial v1\n").map_err(native)?;
    println!(
        "build complete: {} versioned passages, {} ms, state={}",
        docs.len(),
        start.elapsed().as_millis(),
        path.display()
    );
    Ok(())
}

async fn open_table(path: &Path) -> Result<Table> {
    let db = lancedb::connect(path.join("lancedb").to_string_lossy().as_ref())
        .execute()
        .await
        .map_err(native)?;
    db.open_table("passages").execute().await.map_err(native)
}
async fn verify_table(table: &Table, docs: &[Document]) -> Result<()> {
    let batches: Vec<RecordBatch> = table
        .query()
        .execute()
        .await
        .map_err(native)?
        .try_collect()
        .await
        .map_err(native)?;
    let mut seen = vec![false; docs.len()];
    for batch in batches {
        let ids = batch
            .column_by_name("id")
            .ok_or_else(|| native("LanceDB missing id"))?
            .as_any()
            .downcast_ref::<Int32Array>()
            .ok_or_else(|| native("LanceDB id type mismatch"))?;
        let get = |name| -> Result<&StringArray> {
            batch
                .column_by_name(name)
                .ok_or_else(|| native("LanceDB missing provenance column"))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| native("LanceDB provenance type mismatch"))
        };
        let sources = get("source_id")?;
        let versions = get("version_id")?;
        let hashes = get("source_hash")?;
        for row in 0..batch.num_rows() {
            let id = usize::try_from(ids.value(row)).map_err(native)?;
            let doc = docs
                .get(id)
                .ok_or_else(|| native("LanceDB row id outside snapshot"))?;
            if seen[id]
                || sources.value(row) != doc.source_id
                || versions.value(row) != doc.version_id
                || hashes.value(row) != doc.source_hash
            {
                return Err(native("LanceDB row provenance mismatch"));
            }
            seen[id] = true;
        }
    }
    if seen.iter().any(|v| !v) {
        return Err(native("LanceDB row missing"));
    }
    Ok(())
}
async fn semantic(
    docs: &[Document],
    model: &mut TextEmbedding,
    table: &Table,
    request: &Request,
) -> Result<Vec<Evidence>> {
    let allowed = eligible(docs, &request.filters)?;
    if allowed.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<usize> = allowed
        .iter()
        .map(|d| docs.iter().position(|x| std::ptr::eq(x, *d)).unwrap())
        .collect();
    request.validate()?;
    let vector = model
        .embed([request.query.as_str()], None)
        .map_err(native)?
        .remove(0);
    if vector.len() != DIM {
        return Err(native("query embedding dimensions mismatch"));
    }
    let filter = format!(
        "id IN ({})",
        ids.iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    let batches: Vec<RecordBatch> = table
        .query()
        .nearest_to(vector.as_slice())
        .map_err(native)?
        .distance_type(DistanceType::Cosine)
        .only_if(filter)
        .limit(request.limit)
        .execute()
        .await
        .map_err(native)?
        .try_collect()
        .await
        .map_err(native)?;
    let mut hits = Vec::new();
    for batch in batches {
        let id_array = batch
            .column_by_name("id")
            .ok_or_else(|| native("LanceDB result missing id"))?
            .as_any()
            .downcast_ref::<Int32Array>()
            .ok_or_else(|| native("LanceDB id type mismatch"))?;
        let distance = batch
            .column_by_name("_distance")
            .ok_or_else(|| native("LanceDB result missing distance"))?
            .as_any()
            .downcast_ref::<Float32Array>()
            .ok_or_else(|| native("LanceDB distance type mismatch"))?;
        for row in 0..batch.num_rows() {
            let id = id_array.value(row) as usize;
            if !ids.contains(&id) {
                return Err(native("LanceDB returned a row outside request filter"));
            }
            let doc = docs
                .get(id)
                .ok_or_else(|| native("LanceDB id outside document snapshot"))?;
            let hit = Evidence::from_document(
                doc,
                1.0 - distance.value(row),
                ScoreKind::CosineSimilarity,
            );
            hit.verify(docs)?;
            hits.push(hit);
        }
    }
    Ok(hits)
}
async fn run_query(
    docs: &[Document],
    model: &mut TextEmbedding,
    table: &Table,
    request: &Request,
) -> Result<Vec<Evidence>> {
    request.validate()?;
    let hits = match request.profile {
        Profile::Keyword => search_keyword(docs, request)?,
        Profile::Semantic => semantic(docs, model, table, request).await?,
        Profile::Hybrid => {
            let mut candidates = request.clone();
            candidates.limit = 100;
            candidates.profile = Profile::Keyword;
            let lexical = search_keyword(docs, &candidates)?;
            candidates.profile = Profile::Semantic;
            let vector = semantic(docs, model, table, &candidates).await?;
            fuse(&lexical, &vector, request.limit)
        }
    };
    for hit in &hits {
        hit.verify(docs)?;
    }
    Ok(hits)
}
/// Shared caller entrypoint. The requested profile selects only ranking;
/// request shape and evidence provenance remain unchanged.
pub async fn search(path: &Path, request: &Request) -> Result<Vec<Evidence>> {
    request.validate()?;
    let (docs, mut model) = open_state(path)?;
    let table = open_table(path).await?;
    verify_table(&table, &docs).await?;
    run_query(&docs, &mut model, &table, request).await
}
pub async fn evaluate(path: &Path) -> Result<()> {
    let (docs, mut model) = open_state(path)?;
    let table = open_table(path).await?;
    verify_table(&table, &docs).await?;
    let queries = [
        ("launch window", "launch-current"),
        ("café", "unicode-note"),
        ("BRN-482", "identifier-note"),
        ("when does the release begin", "launch-current"),
    ];
    for profile in [Profile::Keyword, Profile::Semantic, Profile::Hybrid] {
        for (query, expected) in queries {
            let request = Request::new(query, profile, 3, Filters::default())?;
            let start = Instant::now();
            let hits = run_query(&docs, &mut model, &table, &request).await?;
            let names: Vec<_> = hits.iter().map(|h| h.source_id.as_str()).collect();
            println!("profile={profile:?} query={query:?} expected={expected} hit@3={} latency_ms={} hits={names:?}", names.contains(&expected), start.elapsed().as_millis());
        }
    }
    for profile in [Profile::Keyword, Profile::Semantic, Profile::Hybrid] {
        let empty = Request::new(
            "launch",
            profile,
            3,
            Filters {
                source_ids: Some(vec!["absent".into()]),
                ..Filters::default()
            },
        )?;
        if !run_query(&docs, &mut model, &table, &empty)
            .await?
            .is_empty()
        {
            return Err(native("empty eligibility returned results"));
        }
        let old = Request::new(
            "March",
            profile,
            3,
            Filters {
                source_ids: Some(vec!["launch-current".into()]),
                version_ids: Some(vec!["v1".into()]),
                current_only: false,
                ..Filters::default()
            },
        )?;
        let old_hits = run_query(&docs, &mut model, &table, &old).await?;
        if old_hits.len() != 1 || old_hits[0].version_id != "v1" {
            return Err(native("historical version filter invariant failed"));
        }
        for (source, approval) in [
            ("draft-brief", crate::Approval::Draft),
            ("withdrawn-brief", crate::Approval::Withdrawn),
        ] {
            let req = Request::new(
                "launch",
                profile,
                3,
                Filters {
                    source_ids: Some(vec![source.into()]),
                    approvals: vec![approval],
                    ..Filters::default()
                },
            )?;
            let hits = run_query(&docs, &mut model, &table, &req).await?;
            if hits.len() != 1 || hits[0].source_id != source {
                return Err(native("approval filter invariant failed"));
            }
        }
    }
    println!("native filter invariants passed");
    Ok(())
}
pub async fn reopen(path: &Path) -> Result<()> {
    let (docs, mut model) = open_state(path)?;
    let table = open_table(path).await?;
    verify_table(&table, &docs).await?;
    let req = Request::new("launch window", Profile::Semantic, 3, Filters::default())?;
    let start = Instant::now();
    let hits = run_query(&docs, &mut model, &table, &req).await?;
    println!(
        "reopen semantic latency_ms={} hits={:?}",
        start.elapsed().as_millis(),
        hits.iter()
            .map(|h| h.source_id.as_str())
            .collect::<Vec<_>>()
    );
    Ok(())
}
