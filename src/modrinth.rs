//! Modrinth API 的最小客户端。
//!
//! 只用三件事：按 SHA-1 反查版本、按项目查最新 release、流式下载。
//! 文档：<https://docs.modrinth.com/api/>

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::RETRY_AFTER;
use serde::Deserialize;
use thiserror::Error;

use crate::retry;

const API_BASE: &str = "https://api.modrinth.com/v2";

// Modrinth 没写明单次上限，取保守值分块；哈希会先去重。
const HASH_CHUNK: usize = 500;
const PROJECT_CHUNK: usize = 500;

// 单次 API 调用可以整体限时；下载体积不可控，只能限制连接和每次读取的等待。
const API_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub version_number: String,
    pub version_type: String,
    /// RFC 3339 时间戳，同一格式下可以直接按字符串比大小。
    pub date_published: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
}

impl Version {
    /// 主文件优先，没有主文件就退回第一个。
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files
            .iter()
            .find(|file| file.primary)
            .or_else(|| self.files.first())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionFile {
    pub hashes: HashMap<String, String>,
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub project_type: String,
    pub title: String,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Modrinth returned HTTP {status}: {body}")]
    BadStatus {
        status: u16,
        body: String,
        retry_after: Option<Duration>,
    },

    #[error("download failed with HTTP {status}: {url}")]
    Download {
        url: String,
        status: u16,
        retry_after: Option<Duration>,
    },

    #[error("failed to decode Modrinth response: {0}")]
    Decode(#[from] serde_json::Error),

    #[error("a version lookup task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

impl ApiError {
    /// 重连前的等待时间；`None` 表示这个错误不值得重试。
    pub fn retry_delay(&self, attempt: usize) -> Option<Duration> {
        match self {
            Self::Request(err) if transient_request(err) => Some(retry::backoff(attempt)),
            Self::BadStatus {
                status,
                retry_after,
                ..
            }
            | Self::Download {
                status,
                retry_after,
                ..
            } if retry::transient_status(*status) => {
                Some(retry_after.unwrap_or_else(|| retry::backoff(attempt)))
            }
            _ => None,
        }
    }
}

fn transient_request(err: &reqwest::Error) -> bool {
    // reqwest 把流式读取响应体时的中断也归到 decode，这类错误同样值得重连。
    err.is_connect() || err.is_timeout() || err.is_body() || err.is_decode() || err.is_request()
}

/// 重连前回调，用来向用户提示；库本身不直接打印。
pub type RetryNotifier = Arc<dyn Fn(usize, Duration) + Send + Sync>;

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    on_retry: Option<RetryNotifier>,
}

impl Client {
    /// Modrinth 要求带一个能识别调用方的 `User-Agent`，否则可能被限流或拦截。
    /// 需要时可设 `EMCSYNC_USER_AGENT` 覆盖。
    pub fn new() -> Result<Self, ApiError> {
        Self::build(None)
    }

    pub fn with_retry_notifier(notifier: RetryNotifier) -> Result<Self, ApiError> {
        Self::build(Some(notifier))
    }

    fn build(on_retry: Option<RetryNotifier>) -> Result<Self, ApiError> {
        let user_agent = std::env::var("EMCSYNC_USER_AGENT")
            .unwrap_or_else(|_| format!("eunoia/emcsync/{}", env!("CARGO_PKG_VERSION")));

        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .build()?;

        Ok(Self { http, on_retry })
    }

    /// 发送已构建好的请求；瞬时故障按退避重连。
    async fn send(&self, mut request: reqwest::Request) -> Result<String, ApiError> {
        let mut attempt = 1;

        loop {
            let retry_request = request.try_clone();

            match self.execute(request).await {
                Ok(text) => return Ok(text),
                Err(err) => {
                    if attempt >= retry::MAX_ATTEMPTS {
                        return Err(err);
                    }

                    let Some(wait) = err.retry_delay(attempt) else {
                        return Err(err);
                    };
                    let Some(next) = retry_request else {
                        return Err(err);
                    };

                    if let Some(notify) = &self.on_retry {
                        notify(attempt, wait);
                    }

                    tokio::time::sleep(wait).await;
                    request = next;
                    attempt += 1;
                }
            }
        }
    }

    async fn execute(&self, request: reqwest::Request) -> Result<String, ApiError> {
        let response = self.http.execute(request).await?;
        read_body(response).await
    }

    /// 一次反查多个 SHA-1；未命中的哈希不会出现在结果里。
    pub async fn versions_by_sha1(
        &self,
        hashes: &[String],
    ) -> Result<HashMap<String, Version>, ApiError> {
        let unique: Vec<&String> = hashes.iter().collect::<BTreeSet<_>>().into_iter().collect();

        let mut versions = HashMap::new();
        for chunk in unique.chunks(HASH_CHUNK) {
            let body = serde_json::json!({
                "hashes": chunk,
                "algorithm": "sha1",
            });

            let request = self
                .http
                .post(format!("{API_BASE}/version_files"))
                .timeout(API_TIMEOUT)
                .json(&body)
                .build()?;
            let text = self.send(request).await?;

            let found: HashMap<String, Version> = serde_json::from_str(&text)?;
            versions.extend(found);
        }

        Ok(versions)
    }

    /// 批量拿项目信息；slug 不在版本对象里，只能额外查一次。
    pub async fn projects_by_ids(&self, ids: &[String]) -> Result<Vec<Project>, ApiError> {
        let unique: Vec<&String> = ids.iter().collect::<BTreeSet<_>>().into_iter().collect();

        let mut projects = Vec::new();
        for chunk in unique.chunks(PROJECT_CHUNK) {
            let ids = serde_json::to_string(chunk)?;
            let request = self
                .http
                .get(format!("{API_BASE}/projects"))
                .timeout(API_TIMEOUT)
                .query(&[("ids", ids)])
                .build()?;
            let text = self.send(request).await?;

            let mut found: Vec<Project> = serde_json::from_str(&text)?;
            projects.append(&mut found);
        }

        Ok(projects)
    }

    /// 查某个项目在指定游戏版本（和加载器）下的最新稳定版。
    ///
    /// 资源包、光影包没有加载器概念，`loaders` 传 `None`，否则会查不到东西。
    pub async fn latest_release(
        &self,
        project_id: &str,
        game_version: &str,
        loaders: Option<&[&str]>,
    ) -> Result<Option<Version>, ApiError> {
        let game_versions = serde_json::to_string(&[game_version])?;
        let mut query = vec![("game_versions", game_versions)];

        if let Some(loaders) = loaders {
            query.push(("loaders", serde_json::to_string(loaders)?));
        }

        let request = self
            .http
            .get(format!("{API_BASE}/project/{project_id}/version"))
            .timeout(API_TIMEOUT)
            .query(&query[..])
            .build()?;
        let text = self.send(request).await?;
        let versions: Vec<Version> = serde_json::from_str(&text)?;

        Ok(latest_release(versions))
    }

    /// 发起下载；`offset` 非零时从断点续传，响应体交给调用方按块读取。
    pub async fn download(&self, url: &str, offset: u64) -> Result<reqwest::Response, ApiError> {
        let mut request = self.http.get(url);

        if offset > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            return Err(ApiError::Download {
                url: url.to_string(),
                status: status.as_u16(),
                retry_after: retry_after(&response),
            });
        }

        Ok(response)
    }
}

/// 只要 release，按发布时间取最新。
fn latest_release(versions: Vec<Version>) -> Option<Version> {
    versions
        .into_iter()
        .filter(|version| version.version_type == "release")
        .max_by(|a, b| a.date_published.cmp(&b.date_published))
}

async fn read_body(response: reqwest::Response) -> Result<String, ApiError> {
    let status = response.status();
    let retry_after = retry_after(&response);
    let text = response.text().await?;

    if !status.is_success() {
        let body: String = text.chars().take(300).collect();
        return Err(ApiError::BadStatus {
            status: status.as_u16(),
            body,
            retry_after,
        });
    }

    Ok(text)
}

/// `Retry-After` 只解析秒数形式；HTTP 日期形式交给指数退避。
fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    let value = response.headers().get(RETRY_AFTER)?.to_str().ok()?;
    parse_retry_after(value)
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    let seconds = value.trim().parse::<u64>().ok()?;

    Some(Duration::from_secs(seconds.min(30)))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use super::{ApiError, Version, latest_release, parse_retry_after};

    fn version(number: &str, version_type: &str, date: &str) -> Version {
        Version {
            id: number.to_string(),
            project_id: "P1".to_string(),
            version_number: number.to_string(),
            version_type: version_type.to_string(),
            date_published: date.to_string(),
            game_versions: vec!["1.21.1".to_string()],
            loaders: vec!["fabric".to_string()],
            files: Vec::new(),
        }
    }

    #[test]
    fn picks_newest_release() {
        let versions = vec![
            version("2.0.0", "beta", "2026-08-01T00:00:00Z"),
            version("1.0.0", "release", "2026-01-01T00:00:00Z"),
            version("1.1.0", "release", "2026-02-01T00:00:00Z"),
        ];

        assert_eq!(latest_release(versions).unwrap().version_number, "1.1.0");
    }

    #[test]
    fn ignores_non_release() {
        assert!(latest_release(vec![version("2.0.0", "beta", "2026-08-01T00:00:00Z")]).is_none());
        assert!(latest_release(Vec::new()).is_none());
    }

    #[test]
    fn primary_file_prefers_the_primary_flag() {
        let mut version = version("1.0.0", "release", "2026-01-01T00:00:00Z");
        version.files = vec![file("extra.jar", false), file("main.jar", true)];

        assert_eq!(version.primary_file().unwrap().filename, "main.jar");
    }

    #[test]
    fn primary_file_falls_back_to_the_first() {
        let mut version = version("1.0.0", "release", "2026-01-01T00:00:00Z");
        version.files = vec![file("only.jar", false)];

        assert_eq!(version.primary_file().unwrap().filename, "only.jar");
    }

    #[test]
    fn retry_after_header_is_preferred_for_retryable_statuses() {
        let err = ApiError::BadStatus {
            status: 503,
            body: String::new(),
            retry_after: Some(Duration::from_secs(7)),
        };

        assert_eq!(err.retry_delay(1), Some(Duration::from_secs(7)));
    }

    #[test]
    fn other_client_errors_are_not_retried() {
        let err = ApiError::BadStatus {
            status: 404,
            body: String::new(),
            retry_after: None,
        };

        assert!(err.retry_delay(1).is_none());
    }

    #[test]
    fn retry_after_only_accepts_seconds_and_is_capped() {
        assert_eq!(parse_retry_after("5"), Some(Duration::from_secs(5)));
        assert_eq!(parse_retry_after("120"), Some(Duration::from_secs(30)));
        assert_eq!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"), None);
    }

    #[tokio::test]
    async fn download_resumes_with_a_range_header() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 512];

            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }

            let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
            assert!(request.contains("range: bytes=5-"), "{request}");

            stream
                .write_all(
                    b"HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 5-7/8\r\n\r\nabc",
                )
                .unwrap();
        });

        let client = super::Client::new().unwrap();
        let response = client
            .download(&format!("http://{addr}/file"), 5)
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.text().await.unwrap(), "abc");
        server.join().unwrap();
    }

    fn file(name: &str, primary: bool) -> super::VersionFile {
        super::VersionFile {
            hashes: HashMap::new(),
            url: format!("https://cdn.modrinth.com/{name}"),
            filename: name.to_string(),
            primary,
        }
    }
}
