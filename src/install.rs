//! 下载待安装资源并落盘。
//!
//! 先写 `.part` 再改名，下载中断不会留下半个 jar；进度条在非 tty 时由 indicatif 自动隐藏。

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use emcsync_cli::modrinth::Client;
use emcsync_cli::plan::{self, InstallItem};
use emcsync_cli::scan::ResourceKind;
use indicatif::{ProgressBar, ProgressStyle};
use rust_i18n::t;
use tokio::io::AsyncWriteExt;

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

    let response = client.download(&item.file.url).await?;
    let total = response.content_length().unwrap_or(0);
    let label = format!("[{index}/{count}] {}", item.file.filename);
    let bar = progress_bar(total, &label);

    let mut file = tokio::fs::File::create(&part).await?;

    let result = async {
        copy(response, &mut file, &bar).await?;
        file.flush().await?;
        Ok::<_, anyhow::Error>(())
    }
    .await;

    // Windows 上文件不关掉无法改名。
    drop(file);
    bar.finish_and_clear();

    match result {
        Ok(()) => {
            std::fs::rename(&part, &dest)?;
            Ok(())
        }
        Err(err) => {
            let _ = std::fs::remove_file(&part);
            Err(err)
        }
    }
}

async fn copy(
    mut response: reqwest::Response,
    file: &mut tokio::fs::File,
    bar: &ProgressBar,
) -> Result<()> {
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
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
