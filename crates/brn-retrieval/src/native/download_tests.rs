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
        let digest = if i < 2 {
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
fn loaded_asset_bytes_require_exact_size_and_content_for_both_pin_types() {
    for (asset, bytes) in manifest().iter().zip(BYTES) {
        download::verify_bytes(asset, bytes).unwrap();
        for wrong_size in [&bytes[..bytes.len() - 1], &[bytes, b"!"].concat()] {
            assert!(matches!(
                download::verify_bytes(asset, wrong_size),
                Err(Error::ModelInstall(ModelInstallError::SizeMismatch))
            ));
        }
        let mut changed = bytes.to_vec();
        changed[0] ^= 1;
        assert!(matches!(
            download::verify_bytes(asset, &changed),
            Err(Error::ModelInstall(ModelInstallError::DigestMismatch))
        ));
    }
}

#[test]
fn loaded_git_blob_pin_requires_the_git_header_not_a_raw_content_digest() {
    let bytes = "\u{feff}λ\r\n\0configuration".as_bytes();
    let mut hash = sha1::Sha1::new();
    hash.update(format!("blob {}\0", bytes.len()));
    hash.update(bytes);
    let asset = Asset {
        source: "synthetic",
        name: "config.json",
        bytes: bytes.len() as u64,
        digest: DigestPin::GitBlobSha1(hash.finalize().into()),
    };
    download::verify_bytes(&asset, bytes).unwrap();
    let wrong = Asset {
        digest: DigestPin::GitBlobSha1(sha1::Sha1::digest(bytes).into()),
        ..asset
    };
    assert!(matches!(
        download::verify_bytes(&wrong, bytes),
        Err(Error::ModelInstall(ModelInstallError::DigestMismatch))
    ));
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
        "2c4055b12046f11709e9df2c122e59ffbdc2f900"
    );
    assert_eq!(download::ASSETS.len(), 5);
    assert_eq!(
        download::ASSETS.iter().map(|a| a.bytes).sum::<u64>(),
        135_392_488
    );
    assert_eq!(download::ASSETS[0].source, "onnx/model_quantized.onnx");
    assert_eq!(download::ASSETS[0].name, "model.onnx");
    assert!(matches!(download::ASSETS[0].digest, DigestPin::Sha256(_)));
    assert!(matches!(download::ASSETS[1].digest, DigestPin::Sha256(_)));
    assert!(
        download::ASSETS[2..]
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
            "onnx/model_quantized.onnx",
            "model.onnx",
            118_308_126,
            "66fc00f5f29afcaff34092e1bdd20008ca3918265a82fb9695a551e510cc4ebc",
        ),
        (
            "tokenizer.json",
            "tokenizer.json",
            17_082_913,
            "b60b6b43406a48bf3638526314f3d232d97058bc93472ff2de930d43686fa441",
        ),
        (
            "config.json",
            "config.json",
            673,
            "ce7fe159b2eee53ef2b9704a72a4efa6c22248b4",
        ),
        (
            "special_tokens_map.json",
            "special_tokens_map.json",
            280,
            "d5698132694f4f1bcff08fa7d937b1701812598e",
        ),
        (
            "tokenizer_config.json",
            "tokenizer_config.json",
            496,
            "9f3bfd538ec86d360dc988dac25e3cfb0c4c14d5",
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
