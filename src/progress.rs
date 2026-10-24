//! 下载进度：总文件数一条，正在下载的文件各一条。
//!
//! 完成的行不立即清除，保留最近几条供回看；总进度条始终置顶。
//! 非 tty 时由 indicatif 自动隐藏，重试提示改走 stderr。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rust_i18n::t;

// 已完成行保留的条数，太多会占满屏幕。
const KEEP_FINISHED: usize = 6;

#[derive(Clone)]
pub struct DownloadProgress {
    multi: MultiProgress,
    overall: ProgressBar,
    finished: Arc<Mutex<VecDeque<ProgressBar>>>,
}

/// 单个文件的进度行；长度要等响应到达后才能确定。
pub struct FileProgress {
    bar: ProgressBar,
    label: String,
}

impl DownloadProgress {
    pub fn new(total: usize) -> Self {
        let multi = MultiProgress::new();
        let overall = multi.add(
            ProgressBar::new(total as u64)
                .with_style(overall_style())
                .with_message(t!("sync.downloading").to_string()),
        );

        Self {
            multi,
            overall,
            finished: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn start(&self, index: usize, count: usize, filename: &str) -> FileProgress {
        let label = format!("[{index}/{count}] {filename}");
        let bar = ProgressBar::new_spinner()
            .with_style(spinner_style())
            .with_message(label.clone());
        bar.enable_steady_tick(Duration::from_millis(120));

        FileProgress {
            bar: self.multi.add(bar),
            label,
        }
    }

    pub fn succeed(&self, file: FileProgress) {
        file.bar.set_style(message_style());
        file.bar
            .finish_with_message(format!("{} {}", file.label, t!("sync.file_done")));
        self.keep(file.bar);
    }

    pub fn fail(&self, file: FileProgress) {
        file.bar.set_style(message_style());
        file.bar
            .abandon_with_message(format!("{} {}", file.label, t!("sync.file_failed")));
        self.keep(file.bar);
    }

    pub fn finish(&self) {
        self.overall
            .finish_with_message(t!("sync.download_finished").to_string());
    }

    fn keep(&self, bar: ProgressBar) {
        self.overall.inc(1);
        let mut finished = self.finished.lock().expect("完成队列的锁不会中毒");

        finished.push_back(bar);

        if finished.len() > KEEP_FINISHED {
            if let Some(old) = finished.pop_front() {
                self.multi.remove(&old);
            }
        }
    }
}

impl FileProgress {
    pub fn bar(&self) -> &ProgressBar {
        &self.bar
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn set_message(&self, message: String) {
        self.bar.set_message(message);
    }

    /// 由转圈切到按字节的进度条；总长未知时保持转圈。
    pub fn set_total(&self, total: u64) {
        if total == 0 {
            return;
        }

        self.bar.disable_steady_tick();
        self.bar.set_style(byte_style());
        self.bar.set_length(total);
    }

    pub fn set_position(&self, position: u64) {
        self.bar.set_position(position);
    }
}

fn overall_style() -> ProgressStyle {
    let template = format!(
        "{{msg}} [{{bar:32}}] {{pos}}/{{len}} {}",
        t!("sync.file_unit")
    );

    ProgressStyle::with_template(&template)
        .expect("固定的进度条模板不会失败")
        .progress_chars("=>-")
}

fn byte_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg} [{bar:32}] {bytes}/{total_bytes} {bytes_per_sec} {eta}")
        .expect("固定的进度条模板不会失败")
        .progress_chars("=>-")
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg} {spinner} {bytes}").expect("固定的进度条模板不会失败")
}

fn message_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg}").expect("固定的进度条模板不会失败")
}
