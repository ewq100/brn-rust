//! Offline measurement driver for the actual checked AI-Off conversion service.
use brn_threads_intake::*;
use serde::Deserialize;
use std::{io::BufRead, path::PathBuf, sync::atomic::AtomicBool, time::Instant};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    path: PathBuf,
    kind: DocumentKind,
    backend: String,
    stock: PathBuf,
    brn: PathBuf,
    scratch: PathBuf,
}

fn main() {
    for line in std::io::stdin().lock().lines() {
        let input: Input = serde_json::from_str(&line.unwrap()).unwrap();
        let started = Instant::now();
        let cancel = AtomicBool::new(false);
        let limits = import_limits(input.kind);
        let result =
            read_input_file(&input.path, limits.max_input_bytes, &cancel).and_then(|bytes| {
                Converter {
                    temp_root: input.scratch,
                    limits,
                }
                .convert_document(
                    ConversionRequest {
                        kind: input.kind,
                        intent: ImportIntent::FullNote,
                        bytes: &bytes,
                        source_reference: "synthetic://intake-score",
                    },
                    &DocumentHelpers {
                        stock: input.stock,
                        brn: input.brn,
                    },
                    &input.backend,
                    &cancel,
                )
            });
        let seconds = started.elapsed().as_secs_f64();
        let value = match result {
            Ok(result) => serde_json::json!({"seconds": seconds, "result": result}),
            Err(error) => serde_json::json!({"seconds": seconds, "error": error.to_string()}),
        };
        println!("{value}");
    }
}
