use super::download::{self, Asset, DigestPin, HttpSource, ModelInstallError, install_with};
use crate::Error;
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
};

const BYTES: [&[u8]; 5] = [b"fake onnx", b"tokenizer", b"{}", b"special", b"config"];

fn manifest() -> Vec<Asset> {
    [
        "model.onnx",
        "tokenizer.json",
        "config.json",
        "special_tokens_map.json",
        "tokenizer_config.json",
    ]
    .into_iter()
    .zip(BYTES)
    .enumerate()
    .map(|(i, (name, bytes))| {
        let digest = if i == 0 {
            DigestPin::Sha256(Sha256::digest(bytes).into())
        } else {
            let mut hash = sha1::Sha1::new();
            hash.update(format!("blob {}\0", bytes.len()));
            hash.update(bytes);
            DigestPin::GitBlobSha1(hash.finalize().into())
        };
        Asset {
            source: name,
            name,
            bytes: bytes.len() as u64,
            digest,
        }
    })
    .collect()
}

fn source(asset: &Asset) -> crate::Result<Box<dyn Read>> {
    let i = manifest()
        .iter()
        .position(|a| a.name == asset.name)
        .unwrap();
    Ok(Box::new(Cursor::new(BYTES[i])))
}

#[test]
fn synthetic_five_assets_install_exclusively_and_verified_reuse_has_no_fetch() {
    let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let target = data.path().join("model");
    let mut progress = Vec::new();
    let report = install_with(
        &target,
        &AtomicBool::new(false),
        |n, total| progress.push((n, total)),
        &manifest(),
        source,
    )
    .unwrap();
    assert_eq!(report.directory, target);
    let total = BYTES.iter().map(|b| b.len() as u64).sum::<u64>();
    assert_eq!(report.downloaded_bytes, total);
    assert_eq!(progress.last(), Some(&(total, total)));
    for (asset, bytes) in manifest().iter().zip(BYTES) {
        assert_eq!(std::fs::read(target.join(asset.name)).unwrap(), bytes);
    }
    let report = install_with(
        &target,
        &AtomicBool::new(false),
        |_, _| panic!("reuse progress"),
        &manifest(),
        |_| panic!("reuse network"),
    )
    .unwrap();
    assert_eq!(report.downloaded_bytes, 0);
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 1);
    std::fs::write(target.join("tokenizer.json"), b"corrupted").unwrap();
    assert!(matches!(
        install_with(
            &target,
            &AtomicBool::new(false),
            |_, _| {},
            &manifest(),
            |_| panic!("corrupt occupied target must not fetch")
        ),
        Err(Error::ModelInstall(ModelInstallError::TargetOccupied))
    ));
    assert_eq!(
        std::fs::read(target.join("tokenizer.json")).unwrap(),
        b"corrupted"
    );
}

#[test]
fn synthetic_corruption_truncation_oversize_http_error_and_cancel_clean_only_owned_stage() {
    for case in 0..5 {
        let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let target = data.path().join("model");
        let unrelated = data.path().join(".brn-model-unrelated");
        std::fs::create_dir(&unrelated).unwrap();
        std::fs::write(unrelated.join("keep"), b"sentinel").unwrap();
        let cancel = AtomicBool::new(false);
        let result = install_with(
            &target,
            &cancel,
            |_, _| {
                if case == 4 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &manifest(),
            |asset| {
                if asset.name != "model.onnx" {
                    return source(asset);
                }
                match case {
                    0 => Ok(Box::new(Cursor::new(b"bad  onnx".as_slice())) as Box<dyn Read>),
                    1 => Ok(Box::new(Cursor::new(b"short".as_slice()))),
                    2 => Ok(Box::new(Cursor::new(b"far too much input".as_slice()))),
                    3 => Err(Error::ModelInstall(ModelInstallError::Network)),
                    _ => source(asset),
                }
            },
        );
        match (case, result) {
            (0, Err(Error::ModelInstall(ModelInstallError::DigestMismatch))) => {}
            (1 | 2, Err(Error::ModelInstall(ModelInstallError::SizeMismatch))) => {}
            (3, Err(Error::ModelInstall(ModelInstallError::Network))) => {}
            (4, Err(Error::Cancelled)) => {}
            (_, other) => panic!("unexpected case {case}: {other:?}"),
        }
        assert!(!target.exists());
        assert_eq!(std::fs::read(unrelated.join("keep")).unwrap(), b"sentinel");
        assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 1);
    }
}

#[test]
fn concurrent_destination_creation_cannot_be_overwritten_by_install() {
    let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let target = data.path().join("model");
    let result = install_with(
        &target,
        &AtomicBool::new(false),
        |_, _| {},
        &manifest(),
        |asset| {
            if asset.name == "model.onnx" {
                std::fs::create_dir(&target).unwrap();
                std::fs::write(target.join("sentinel"), b"concurrent").unwrap();
            }
            source(asset)
        },
    );
    assert!(matches!(
        result,
        Err(Error::ModelInstall(ModelInstallError::TargetOccupied))
    ));
    assert_eq!(
        std::fs::read(target.join("sentinel")).unwrap(),
        b"concurrent"
    );
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 1);
}

#[test]
fn private_http_seam_uses_synthetic_manifest_and_local_server_only() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for _ in 0..5 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0; 1];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let request = String::from_utf8(request).unwrap();
            let name = request
                .split_whitespace()
                .nth(1)
                .unwrap()
                .trim_start_matches('/');
            let i = manifest().iter().position(|a| a.source == name).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                BYTES[i].len()
            )
            .unwrap();
            stream.write_all(BYTES[i]).unwrap();
        }
    });
    let source = HttpSource::new(&base).unwrap();
    let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    install_with(
        &data.path().join("model"),
        &AtomicBool::new(false),
        |_, _| {},
        &manifest(),
        |asset| source.fetch(asset),
    )
    .unwrap();
    server.join().unwrap();
}

#[test]
fn production_manifest_is_immutable_exactly_five_assets_with_exact_bounds() {
    assert_eq!(
        download::REVISION,
        "751bff37182d3f1213fa05d7196b954e230abad9"
    );
    assert_eq!(download::ASSETS.len(), 5);
    assert_eq!(
        download::ASSETS.iter().map(|a| a.bytes).sum::<u64>(),
        91_100_408
    );
    assert_eq!(download::ASSETS[0].source, "onnx/model.onnx");
    assert_eq!(download::ASSETS[0].name, "model.onnx");
    assert!(matches!(download::ASSETS[0].digest, DigestPin::Sha256(_)));
    assert!(
        download::ASSETS[1..]
            .iter()
            .all(|a| matches!(a.digest, DigestPin::GitBlobSha1(_)))
    );
    let actual: Vec<_> = download::ASSETS
        .iter()
        .map(|a| {
            let digest = match a.digest {
                DigestPin::Sha256(digest) => hex::encode(digest),
                DigestPin::GitBlobSha1(digest) => hex::encode(digest),
            };
            (a.source, a.name, a.bytes, digest)
        })
        .collect();
    let expected = [
        (
            "onnx/model.onnx",
            "model.onnx",
            90_387_606,
            "759c3cd2b7fe7e93933ad23c4c9181b7396442a2ed746ec7c1d46192c469c46e",
        ),
        (
            "tokenizer.json",
            "tokenizer.json",
            711_661,
            "c17ed520ed8438736732a54957a69306b8822215",
        ),
        (
            "config.json",
            "config.json",
            650,
            "72147e4ff4426ebedbfa2146c4a0999def51a313",
        ),
        (
            "special_tokens_map.json",
            "special_tokens_map.json",
            125,
            "a8b3208c2884c4efb86e49300fdd3dc877220cdf",
        ),
        (
            "tokenizer_config.json",
            "tokenizer_config.json",
            366,
            "37fca74771bc76a8e01178ce3a6055a0995f8093",
        ),
    ]
    .map(|(source, name, bytes, digest)| (source, name, bytes, digest.to_owned()));
    assert_eq!(actual, expected);
}

#[test]
fn http_failure_has_safe_typed_error_and_cleans_the_exclusive_stage() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = [0; 4096];
        assert!(stream.read(&mut request).unwrap() > 0);
        stream.write_all(b"HTTP/1.1 503 Unavailable\r\nContent-Length: 19\r\nConnection: close\r\n\r\nsynthetic raw error").unwrap();
    });
    let source = HttpSource::new(&base).unwrap();
    let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let error = install_with(
        &data.path().join("model"),
        &AtomicBool::new(false),
        |_, _| {},
        &manifest(),
        |asset| source.fetch(asset),
    )
    .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        Error::ModelInstall(ModelInstallError::Network)
    ));
    assert!(!error.to_string().contains("synthetic raw error"));
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 0);
}

#[test]
fn cancellation_between_chunks_cleans_a_partial_file_without_installing() {
    let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let bytes = vec![42; 131_072];
    let asset = Asset {
        source: "synthetic",
        name: "model.onnx",
        bytes: bytes.len() as u64,
        digest: DigestPin::Sha256(Sha256::digest(&bytes).into()),
    };
    let cancel = AtomicBool::new(false);
    let mut received = 0;
    let error = install_with(
        &data.path().join("model"),
        &cancel,
        |n, total| {
            received = n;
            assert_eq!(total, 131_072);
            cancel.store(true, Ordering::Relaxed);
        },
        &[asset],
        |_| Ok(Box::new(Cursor::new(bytes.clone()))),
    )
    .unwrap_err();
    assert!(matches!(error, Error::Cancelled));
    assert_eq!(received, 65_536);
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 0);
}
