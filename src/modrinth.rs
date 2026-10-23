//! Modrinth API 的最小客户端。
//!
//! 只用三件事：按 SHA-1 反查版本、按项目查最新 release、流式下载。
//! 文档：<https://docs.modrinth.com/api/>

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

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
    BadStatus { status: u16, body: String },

    #[error("download failed with HTTP {status}: {url}")]
    Download { url: String, status: u16 },

    #[error("failed to decode Modrinth response: {0}")]
    Decode(#[from] serde_json::Error),

    #[error("a version lookup task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    /// Modrinth 要求带一个能识别调用方的 `User-Agent`，否则可能被限流或拦截。
    /// 需要时可设 `EMCSYNC_USER_AGENT` 覆盖。
    pub fn new() -> Result<Self, ApiError> {
        let user_agent = std::env::var("EMCSYNC_USER_AGENT")
            .unwrap_or_else(|_| format!("eunoia/emcsync-cli/{}", env!("CARGO_PKG_VERSION")));

        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .build()?;

        Ok(Self { http })
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

            let response = self
                .http
                .post(format!("{API_BASE}/version_files"))
                .timeout(API_TIMEOUT)
                .json(&body)
                .send()
                .await?;
            let text = read_body(response).await?;

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
            let response = self
                .http
                .get(format!("{API_BASE}/projects"))
                .timeout(API_TIMEOUT)
                .query(&[("ids", ids)])
                .send()
                .await?;
            let text = read_body(response).await?;

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

        let response = self
            .http
            .get(format!("{API_BASE}/project/{project_id}/version"))
            .timeout(API_TIMEOUT)
            .query(&query[..])
            .send()
            .await?;
        let text = read_body(response).await?;
        let versions: Vec<Version> = serde_json::from_str(&text)?;

        Ok(latest_release(versions))
    }

    /// 发起下载，响应体交给调用方按块读取，不缓存到内存。
    pub async fn download(&self, url: &str) -> Result<reqwest::Response, ApiError> {
        let response = self.http.get(url).send().await?;
        let status = response.status();

        if !status.is_success() {
            return Err(ApiError::Download {
                url: url.to_string(),
                status: status.as_u16(),
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
    let text = response.text().await?;

    if !status.is_success() {
        let body: String = text.chars().take(300).collect();
        return Err(ApiError::BadStatus {
            status: status.as_u16(),
            body,
        });
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Version, latest_release};

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

    fn file(name: &str, primary: bool) -> super::VersionFile {
        super::VersionFile {
            hashes: HashMap::new(),
            url: format!("https://cdn.modrinth.com/{name}"),
            filename: name.to_string(),
            primary,
        }
    }
}
