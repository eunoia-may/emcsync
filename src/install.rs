//! 下载待安装资源并落盘。
//!
//! 多个文件并发下载，先写 `.part` 再改名，下载中断不会留下半个 jar；
//! 进度条在非 tty 时由 indicatif 自动隐藏。网络中断时保留 `.part`，按 Range 续传并有限重连。

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use emcsync::modrinth::{ApiError, Client};
use emcsync::plan::{self, InstallItem};
use emcsync::retry;
use emcsync::scan::ResourceKind;
use indicatif::ProgressBar;
use reqwest::StatusCode;
use rust_i18n::t;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::progress::{DownloadProgress, FileProgress};
use crate::ui;

pub async fn install(
    client: &Client,
    items: &[InstallItem],
    target: &Path,
    create: &[ResourceKind],
    jobs: usize,
) -> Result<()> {
    for kind in create {
        std::fs::create_dir_all(target.join(kind.folder()))
            .with_context(|| t!("sync.create_dir_failed", path = kind.folder()).to_string())?;
    }

    let (items, duplicates) = dedupe(items);

    if !duplicates.is_empty() {
        ui::warn(&t!(
            "sync.duplicate_files",
            names = duplicates.join(&t!("error.separator"))
        ));
    }

    let count = items.len();

    if count == 0 {
        return Ok(());
    }

    let progress = DownloadProgress::new(count);
    let semaphore = Arc::new(Semaphore::new(jobs.min(count).max(1)));
    let mut tasks = JoinSet::new();

    for (index, item) in items.iter().enumerate() {
        let client = client.clone();
        let item = item.clone();
        let target = target.to_path_buf();
        let progress = progress.clone();
        let semaphore = Arc::clone(&semaphore);

        tasks.spawn(async move {
            let permit = semaphore.acquire_owned().await.expect("信号量不会关闭");
            let file = progress.start(index + 1, count, &item.file.filename);
            let result = download(&client, &item, &target, &file).await;

            // 先让出并发额度，再处理收尾。
            drop(permit);

            match result {
                Ok(()) => {
                    progress.succeed(file);
                    Ok(())
                }
                Err(err) => {
                    progress.fail(file);
                    Err(err.context(
                        t!(
                            "sync.install_item_failed",
                            name = item.project.slug.as_str()
                        )
                        .to_string(),
                    ))
                }
            }
        });
    }

    let mut failures = Vec::new();

    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(err)) => failures.push(err),
            Err(err) => failures.push(err.into()),
        }
    }

    progress.finish();

    if failures.is_empty() {
        return Ok(());
    }

    Err(failure_report(failures))
}

/// 并发写同名 `.part` 会互相踩踏，只保留第一个。
fn dedupe(items: &[InstallItem]) -> (Vec<InstallItem>, Vec<String>) {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    let mut duplicates = Vec::new();

    for item in items {
        let key = format!("{}/{}", item.kind.folder(), item.file.filename);

        if seen.insert(key) {
            kept.push(item.clone());
        } else {
            duplicates.push(item.file.filename.clone());
        }
    }

    (kept, duplicates)
}

/// 把所有失败拼成一条错误，避免只看到第一个。
fn failure_report(failures: Vec<anyhow::Error>) -> anyhow::Error {
    let mut text = t!("sync.install_failed_list").to_string();

    for err in failures {
        text.push_str(&format!("\n  {err:#}"));
    }

    anyhow::anyhow!(text)
}

async fn download(
    client: &Client,
    item: &InstallItem,
    target: &Path,
    progress: &FileProgress,
) -> Result<()> {
    let dest = plan::install_path(target, item.kind, &item.file.filename);
    let part = dest.with_file_name(format!("{}.part", item.file.filename));
    let mut attempt = 1;

    loop {
        let offset = std::fs::metadata(&part).map(|meta| meta.len()).unwrap_or(0);

        let response = match client.download(&item.file.url, offset).await {
            Ok(response) => response,
            Err(err) => {
                if attempt >= retry::MAX_ATTEMPTS {
                    let _ = std::fs::remove_file(&part);
                    return Err(err.into());
                }

                // 断点比服务器文件还长，只能丢弃重下。
                if discard_part(&err, &part) {
                    attempt += 1;
                    continue;
                }

                let Some(wait) = err.retry_delay(attempt) else {
                    let _ = std::fs::remove_file(&part);
                    return Err(err.into());
                };

                notify_retry(progress, &retry_message(progress, attempt, wait));
                tokio::time::sleep(wait).await;
                attempt += 1;
                continue;
            }
        };

        let resumed = offset > 0 && response.status() == StatusCode::PARTIAL_CONTENT;
        let total = response.content_length().unwrap_or(0) + if resumed { offset } else { 0 };
        progress.set_total(total);

        // 续传时进度条对齐已有字节；服务器不支持 Range 返回 200 时从头写。
        progress.set_position(if resumed { offset } else { 0 });
        progress.set_message(progress.label().to_string());

        let mut file = if resumed {
            tokio::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&part)
                .await?
        } else {
            tokio::fs::File::create(&part).await?
        };

        let result = async {
            copy(response, &mut file, progress.bar()).await?;
            file.flush().await?;
            Ok::<_, CopyError>(())
        }
        .await;

        // Windows 上文件不关掉无法改名。
        drop(file);

        match result {
            Ok(()) => {
                std::fs::rename(&part, &dest)?;
                return Ok(());
            }
            Err(err) => {
                if attempt >= retry::MAX_ATTEMPTS {
                    let _ = std::fs::remove_file(&part);
                    return Err(err.into());
                }

                let Some(wait) = err.retry_delay(attempt) else {
                    let _ = std::fs::remove_file(&part);
                    return Err(err.into());
                };

                notify_retry(progress, &retry_message(progress, attempt, wait));
                tokio::time::sleep(wait).await;
                attempt += 1;
            }
        }
    }
}

/// 断点失效的错误；同时清掉旧的 `.part`。
fn discard_part(err: &ApiError, part: &Path) -> bool {
    let stale = matches!(err, ApiError::Download { status: 416, .. });

    if stale {
        let _ = std::fs::remove_file(part);
    }

    stale
}

fn retry_message(progress: &FileProgress, attempt: usize, wait: Duration) -> String {
    format!(
        "{} {}",
        progress.label(),
        t!(
            "sync.download_retrying",
            attempt = attempt,
            seconds = wait.as_secs().max(1)
        )
    )
}

/// 有可见进度条就写在条上，否则退回 stderr，避免非 tty 时重连悄无声息。
fn notify_retry(progress: &FileProgress, message: &str) {
    progress.set_message(message.to_string());

    if !progress.bar().is_hidden() {
        return;
    }

    ui::warn(message);
}

#[derive(Debug)]
enum CopyError {
    Network(ApiError),
    Io(std::io::Error),
}

impl From<std::io::Error> for CopyError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl CopyError {
    fn retry_delay(&self, attempt: usize) -> Option<Duration> {
        match self {
            Self::Network(err) => err.retry_delay(attempt),
            // 磁盘满、权限不足之类重试也不会变好。
            Self::Io(_) => None,
        }
    }
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(err) => err.fmt(f),
            Self::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CopyError {}

async fn copy(
    mut response: reqwest::Response,
    file: &mut tokio::fs::File,
    bar: &ProgressBar,
) -> Result<(), CopyError> {
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| CopyError::Network(ApiError::from(err)))?
    {
        file.write_all(&chunk).await.map_err(CopyError::Io)?;
        bar.inc(chunk.len() as u64);
    }

    Ok(())
}
