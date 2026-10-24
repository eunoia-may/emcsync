//! 下载待安装资源并落盘。
//!
//! 先写 `.part` 再改名，下载中断不会留下半个 jar；进度条在非 tty 时由 indicatif 自动隐藏。
//! 网络中断时保留 `.part`，按 Range 续传并有限重连。

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use emcsync::modrinth::{ApiError, Client};
use emcsync::plan::{self, InstallItem};
use emcsync::retry;
use emcsync::scan::ResourceKind;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::StatusCode;
use rust_i18n::t;
use tokio::io::AsyncWriteExt;

use crate::ui;

pub async fn install(
    client: &Client,
    items: &[InstallItem],
    target: &Path,
    create: &[ResourceKind],
) -> Result<()> {
    for kind in create {
        std::fs::create_dir_all(target.join(kind.folder()))
            .with_context(|| t!("sync.create_dir_failed", path = kind.folder()).to_string())?;
    }

    let total = items.len();
    for (index, item) in items.iter().enumerate() {
        download(client, item, target, index + 1, total)
            .await
            .with_context(|| {
                t!(
                    "sync.install_item_failed",
                    name = item.project.slug.as_str()
                )
                .to_string()
            })?;
    }

    Ok(())
}

async fn download(
    client: &Client,
    item: &InstallItem,
    target: &Path,
    index: usize,
    count: usize,
) -> Result<()> {
    let dest = plan::install_path(target, item.kind, &item.file.filename);
    let part = dest.with_file_name(format!("{}.part", item.file.filename));
    let label = format!("[{index}/{count}] {}", item.file.filename);
    let mut bar: Option<ProgressBar> = None;
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

                let message = retry_message(&label, attempt, wait);
                notify_retry(bar.as_ref(), &message);

                tokio::time::sleep(wait).await;
                attempt += 1;
                continue;
            }
        };

        let resumed = offset > 0 && response.status() == StatusCode::PARTIAL_CONTENT;
        let total = response.content_length().unwrap_or(0) + if resumed { offset } else { 0 };
        let bar = bar.get_or_insert_with(|| progress_bar(total, &label));
        bar.set_message(label.clone());

        // 续传时进度条对齐已有字节；服务器不支持 Range 返回 200 时从头写。
        bar.set_position(if resumed { offset } else { 0 });

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
            copy(response, &mut file, bar).await?;
            file.flush().await?;
            Ok::<_, CopyError>(())
        }
        .await;

        // Windows 上文件不关掉无法改名。
        drop(file);

        match result {
            Ok(()) => {
                bar.finish_and_clear();
                std::fs::rename(&part, &dest)?;
                return Ok(());
            }
            Err(err) => {
                if attempt >= retry::MAX_ATTEMPTS {
                    bar.finish_and_clear();
                    let _ = std::fs::remove_file(&part);
                    return Err(err.into());
                }

                let Some(wait) = err.retry_delay(attempt) else {
                    bar.finish_and_clear();
                    let _ = std::fs::remove_file(&part);
                    return Err(err.into());
                };

                notify_retry(Some(bar), &retry_message(&label, attempt, wait));
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

fn retry_message(label: &str, attempt: usize, wait: Duration) -> String {
    format!(
        "{label} {}",
        t!(
            "sync.download_retrying",
            attempt = attempt,
            seconds = wait.as_secs().max(1)
        )
    )
}

/// 有可见进度条就写在条上，否则退回 stderr，避免非 tty 时重连悄无声息。
fn notify_retry(bar: Option<&ProgressBar>, message: &str) {
    if let Some(bar) = bar {
        bar.set_message(message.to_string());

        if !bar.is_hidden() {
            return;
        }
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

/// 知道总长度就给百分比，不知道就给转圈加已下载字节。
fn progress_bar(total: u64, label: &str) -> ProgressBar {
    if total == 0 {
        let bar = ProgressBar::new_spinner()
            .with_style(spinner_style())
            .with_message(label.to_string());
        bar.enable_steady_tick(Duration::from_millis(120));
        return bar;
    }

    ProgressBar::new(total)
        .with_style(bar_style())
        .with_message(label.to_string())
}

fn bar_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg} [{bar:36}] {bytes}/{total_bytes} {bytes_per_sec} {eta}")
        .expect("固定的进度条模板不会失败")
        .progress_chars("=>-")
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg} {spinner} {bytes}").expect("固定的进度条模板不会失败")
}
