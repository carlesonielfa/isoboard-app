use super::{DownloadBundle, DownloadStatus, ModelStore};
use crate::application_config::SERVICES;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

type DownloadResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub(crate) type DownloadProgress = dyn Fn(DownloadStatus, u64, u64) + Send + Sync;

async fn hash_prefix(
    path: &std::path::Path,
    cancellation: &CancellationToken,
) -> DownloadResult<Sha256> {
    let mut hasher = Sha256::new();
    if !path.exists() {
        return Ok(hasher);
    }
    let mut input = tokio::fs::File::open(path).await?;
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        if cancellation.is_cancelled() {
            return Err("download cancelled".into());
        }
        let count = input.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher)
}

impl ModelStore {
    pub(crate) async fn download_bundle(
        &self,
        id: &str,
        bundle: &DownloadBundle,
        cancellation: &CancellationToken,
        progress: &DownloadProgress,
    ) -> DownloadResult<()> {
        self.download_bundle_from(
            id,
            bundle,
            cancellation,
            progress,
            SERVICES.hugging_face.base_url.as_str(),
        )
        .await
    }
    pub(crate) async fn download_bundle_from(
        &self,
        id: &str,
        bundle: &DownloadBundle,
        cancellation: &CancellationToken,
        progress: &DownloadProgress,
        origin: &str,
    ) -> DownloadResult<()> {
        let parent = self.root.join(id);
        let stage = self.partial_directory(id, bundle);
        tokio::fs::create_dir_all(&stage).await?;
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(
                SERVICES.hugging_face.connect_timeout_seconds,
            ))
            .read_timeout(std::time::Duration::from_secs(
                SERVICES.hugging_face.read_timeout_seconds,
            ))
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build()?;
        let total = bundle.files.iter().map(|file| file.bytes).sum();
        progress(
            DownloadStatus::Downloading,
            self.partial_bytes_for_bundle(&stage, bundle),
            total,
        );
        for file in &bundle.files {
            if cancellation.is_cancelled() {
                return Err("download cancelled".into());
            }
            let verified = stage.join(&file.name);
            if verified.exists() {
                progress(
                    DownloadStatus::Verifying,
                    self.partial_bytes_for_bundle(&stage, bundle),
                    total,
                );
                let hash = hash_prefix(&verified, cancellation).await?;
                if verified.metadata()?.len() == file.bytes
                    && format!("{:x}", hash.finalize()) == file.sha256
                {
                    continue;
                }
                tokio::fs::remove_file(&verified).await?;
            }
            let partial = stage.join(format!("{}.part", file.name));
            let mut offset = partial.metadata().map_or(0, |metadata| metadata.len());
            if offset > file.bytes {
                tokio::fs::remove_file(&partial).await?;
                offset = 0;
            }
            if offset > 0 {
                progress(
                    DownloadStatus::Verifying,
                    self.partial_bytes_for_bundle(&stage, bundle),
                    total,
                );
            }
            let mut hasher = hash_prefix(&partial, cancellation).await?;
            if offset < file.bytes {
                let url = format!(
                    "{}/{}/resolve/{}/{}",
                    origin, bundle.repo, bundle.revision, file.name
                );
                let mut request = client
                    .get(url)
                    .header(reqwest::header::ACCEPT_ENCODING, "identity");
                if offset > 0 {
                    request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
                }
                let mut response = tokio::select! {
                    _ = cancellation.cancelled() => return Err("download cancelled".into()),
                    response = request.send() => response?.error_for_status()?
                };
                match response.status() {
                    reqwest::StatusCode::PARTIAL_CONTENT => {
                        let expected =
                            format!("bytes {}-{}/{}", offset, file.bytes - 1, file.bytes);
                        if response
                            .headers()
                            .get(reqwest::header::CONTENT_RANGE)
                            .and_then(|header| header.to_str().ok())
                            != Some(expected.as_str())
                        {
                            return Err("invalid model download range".into());
                        }
                    }
                    reqwest::StatusCode::OK => {
                        offset = 0;
                        hasher = Sha256::new();
                    }
                    _ => return Err("unexpected model download response".into()),
                }
                if response
                    .content_length()
                    .is_some_and(|length| length != file.bytes - offset)
                {
                    return Err("model download has an unexpected size".into());
                }
                let mut output = tokio::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(offset == 0)
                    .append(offset > 0)
                    .open(&partial)
                    .await?;
                progress(
                    DownloadStatus::Downloading,
                    self.partial_bytes_for_bundle(&stage, bundle),
                    total,
                );
                let mut last_report = std::time::Instant::now();
                loop {
                    let chunk = tokio::select! {
                        _ = cancellation.cancelled() => { output.sync_all().await?; return Err("download cancelled".into()); },
                        chunk = response.chunk() => match chunk { Ok(chunk) => chunk, Err(error) => { output.sync_all().await?; return Err(error.into()); } }
                    };
                    let Some(chunk) = chunk else { break };
                    if offset + chunk.len() as u64 > file.bytes {
                        return Err("model file exceeds its pinned size".into());
                    }
                    output.write_all(&chunk).await?;
                    hasher.update(&chunk);
                    offset += chunk.len() as u64;
                    if last_report.elapsed() >= std::time::Duration::from_millis(200) {
                        progress(
                            DownloadStatus::Downloading,
                            self.partial_bytes_for_bundle(&stage, bundle),
                            total,
                        );
                        last_report = std::time::Instant::now();
                    }
                }
                output.sync_all().await?;
            }
            progress(
                DownloadStatus::Verifying,
                self.partial_bytes_for_bundle(&stage, bundle),
                total,
            );
            if offset != file.bytes {
                return Err("model download was interrupted".into());
            }
            if format!("{:x}", hasher.finalize()) != file.sha256 {
                tokio::fs::remove_file(&partial).await?;
                return Err("model file failed checksum verification".into());
            }
            tokio::fs::rename(&partial, &verified).await?;
            progress(
                DownloadStatus::Downloading,
                self.partial_bytes_for_bundle(&stage, bundle),
                total,
            );
        }
        if cancellation.is_cancelled() {
            return Err("download cancelled".into());
        }
        let target = parent.join(&bundle.revision);
        let backup = tempfile::Builder::new()
            .prefix(".replace-")
            .tempdir_in(&parent)?;
        let existing = target.exists();
        if existing {
            tokio::fs::rename(&target, backup.path().join("bundle")).await?;
        }
        if let Err(error) = tokio::fs::rename(&stage, &target).await {
            if existing {
                tokio::fs::rename(backup.path().join("bundle"), &target).await?;
            }
            return Err(error.into());
        }
        Ok(())
    }
    fn partial_bytes_for_bundle(&self, stage: &std::path::Path, bundle: &DownloadBundle) -> u64 {
        bundle
            .files
            .iter()
            .map(|file| {
                [
                    stage.join(&file.name),
                    stage.join(format!("{}.part", file.name)),
                ]
                .iter()
                .find_map(|path| {
                    path.metadata()
                        .ok()
                        .map(|metadata| metadata.len().min(file.bytes))
                })
                .unwrap_or(0)
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn bundle() -> DownloadBundle {
        DownloadBundle {
            repo: "fixture".into(),
            revision: "pinned".into(),
            license_file: None,
            files: [
                super::super::DownloadFile {
                    name: "model".into(),
                    bytes: 6,
                    sha256: "bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721"
                        .into(),
                },
                super::super::DownloadFile {
                    name: "projector".into(),
                    bytes: 6,
                    sha256: "bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721"
                        .into(),
                },
            ],
        }
    }
    // Serve exactly one request; the second asset is already verified locally.
    fn server(response: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            loop {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8(request).unwrap()
        });
        (origin, handle)
    }
    fn stage(store: &ModelStore, bundle: &DownloadBundle, prefix: &[u8]) -> std::path::PathBuf {
        let stage = store.partial_directory("fixture", bundle);
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(stage.join("model.part"), prefix).unwrap();
        std::fs::write(stage.join("projector"), b"abcdef").unwrap();
        stage
    }
    #[tokio::test]
    async fn resumes_verified_prefix_and_publishes_complete_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let store = ModelStore::new(temp.path().into());
        let bundle = bundle();
        stage(&store, &bundle, b"abc");
        let (origin, server) = server("HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 3-5/6\r\nConnection: close\r\n\r\ndef");
        store
            .download_bundle_from(
                "fixture",
                &bundle,
                &CancellationToken::new(),
                &|_, _, _| {},
                &origin,
            )
            .await
            .unwrap();
        assert!(server
            .join()
            .unwrap()
            .to_lowercase()
            .contains("range: bytes=3-"));
        assert_eq!(
            std::fs::read(temp.path().join("fixture/pinned/model")).unwrap(),
            b"abcdef"
        );
        assert!(!store.partial_directory("fixture", &bundle).exists());
    }
    #[tokio::test]
    async fn ignored_range_restarts_instead_of_appending() {
        let temp = tempfile::tempdir().unwrap();
        let store = ModelStore::new(temp.path().into());
        let bundle = bundle();
        stage(&store, &bundle, b"bad");
        let (origin, server) =
            server("HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabcdef");
        store
            .download_bundle_from(
                "fixture",
                &bundle,
                &CancellationToken::new(),
                &|_, _, _| {},
                &origin,
            )
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(
            std::fs::read(temp.path().join("fixture/pinned/model")).unwrap(),
            b"abcdef"
        );
    }
    #[tokio::test]
    async fn rejects_wrong_range_without_publishing_or_losing_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let store = ModelStore::new(temp.path().into());
        let bundle = bundle();
        let stage = stage(&store, &bundle, b"abc");
        let (origin, server) = server("HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 2-4/6\r\nConnection: close\r\n\r\ndef");
        assert!(store
            .download_bundle_from(
                "fixture",
                &bundle,
                &CancellationToken::new(),
                &|_, _, _| {},
                &origin
            )
            .await
            .is_err());
        server.join().unwrap();
        assert_eq!(std::fs::read(stage.join("model.part")).unwrap(), b"abc");
        assert!(!temp.path().join("fixture/pinned").exists());
    }
    #[tokio::test]
    async fn checksum_failure_discards_only_bad_asset() {
        let temp = tempfile::tempdir().unwrap();
        let store = ModelStore::new(temp.path().into());
        let bundle = bundle();
        let stage = stage(&store, &bundle, b"badbad");
        assert!(store
            .download_bundle_from(
                "fixture",
                &bundle,
                &CancellationToken::new(),
                &|_, _, _| {},
                "http://127.0.0.1:1"
            )
            .await
            .is_err());
        assert!(!stage.join("model.part").exists());
        assert!(stage.join("projector").exists());
        assert!(!temp.path().join("fixture/pinned").exists());
    }
    #[tokio::test]
    async fn cancellation_keeps_partial_and_complete_files() {
        let temp = tempfile::tempdir().unwrap();
        let store = ModelStore::new(temp.path().into());
        let bundle = bundle();
        let stage = stage(&store, &bundle, b"abc");
        let token = CancellationToken::new();
        token.cancel();
        assert!(store
            .download_bundle_from(
                "fixture",
                &bundle,
                &token,
                &|_, _, _| {},
                "http://127.0.0.1:1"
            )
            .await
            .is_err());
        assert_eq!(std::fs::read(stage.join("model.part")).unwrap(), b"abc");
        assert!(stage.join("projector").exists());
    }
}
