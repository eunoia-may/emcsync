//! Modrinth API 的最小客户端。
//!
//! 只用两件事：按 SHA-1 反查版本，按项目 ID 批量拿 slug。CurseForge 接入前，这里就是全部网络逻辑。
//! 文档：<https://docs.modrinth.com/api/>

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

const API_BASE: &str = "https://api.modrinth.com/v2";

// Modrinth 没写明单次上限，取保守值分块；哈希会先去重。
const HASH_CHUNK: usize = 500;
const PROJECT_CHUNK: usize = 500;

#[derive(Clone, Debug, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub version_number: String,
    pub version_type: String,
    pub files: Vec<VersionFile>,
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

    #[error("failed to decode Modrinth response: {0}")]
    Decode(#[from] serde_json::Error),
}

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
            .timeout(Duration::from_secs(30))
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
                .query(&[("ids", ids)])
                .send()
                .await?;
            let text = read_body(response).await?;

            let mut found: Vec<Project> = serde_json::from_str(&text)?;
            projects.append(&mut found);
        }

        Ok(projects)
    }
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
