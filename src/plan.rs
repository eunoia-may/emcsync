//! 查兼容版本，组装安装计划，并算出要补的子目录。
//!
//! 一个待安装项目对应一个最新的 release；查不到就归入“此版本不存在”，不进入下载。

use std::path::{Path, PathBuf};

use tokio::task::JoinSet;

use crate::matching::Resolved;
use crate::modrinth::{ApiError, Client, Project, VersionFile};
use crate::scan::{FOLDERS, ResourceKind};

// 并发查询；Modrinth 按分钟限流，八个足够快也不会打满。
const LOOKUP_CONCURRENCY: usize = 8;

#[derive(Clone, Debug)]
pub struct InstallItem {
    pub project: Project,
    pub kind: ResourceKind,
    pub file: VersionFile,
}

#[derive(Debug, Default)]
pub struct Plan {
    /// 能查到兼容版本、待安装的资源。
    pub installable: Vec<InstallItem>,
    /// 项目存在，但没有该游戏版本（或该加载器）的 release。
    pub unavailable: Vec<Resolved>,
}

/// 对差集里的每个项目查最新 release。
pub async fn build(
    client: &Client,
    missing: &[Resolved],
    game_version: &str,
    loader: &str,
) -> Result<Plan, ApiError> {
    let mut plan = Plan::default();

    for chunk in missing.chunks(LOOKUP_CONCURRENCY) {
        let mut tasks = JoinSet::new();

        for resolved in chunk {
            let client = client.clone();
            let resolved = resolved.clone();
            let game_version = game_version.to_string();
            let loader = loader.to_string();

            tasks.spawn(async move {
                // 只有模组认加载器；资源包、光影包带上会查不到。
                let loaders: Option<Vec<&str>> = match resolved.file.kind {
                    ResourceKind::Mod => Some(vec![loader.as_str()]),
                    ResourceKind::ResourcePack | ResourceKind::ShaderPack => None,
                };

                let version = client
                    .latest_release(&resolved.project.id, &game_version, loaders.as_deref())
                    .await?;

                Ok::<_, ApiError>((resolved, version))
            });
        }

        while let Some(joined) = tasks.join_next().await {
            let (resolved, version) = joined??;

            match version.and_then(|version| version.primary_file().cloned()) {
                Some(file) => plan.installable.push(InstallItem {
                    project: resolved.project.clone(),
                    kind: resolved.file.kind,
                    file,
                }),
                None => plan.unavailable.push(resolved),
            }
        }
    }

    plan.installable
        .sort_by(|a, b| a.project.slug.cmp(&b.project.slug));
    plan.unavailable
        .sort_by(|a, b| a.project.slug.cmp(&b.project.slug));

    Ok(plan)
}

/// 待安装资源需要、但目标里还不存在的子目录。
pub fn missing_folders(target: &Path, items: &[InstallItem]) -> Vec<ResourceKind> {
    FOLDERS
        .iter()
        .copied()
        .filter(|kind| items.iter().any(|item| item.kind == *kind))
        .filter(|kind| !target.join(kind.folder()).is_dir())
        .collect()
}

/// 安装后的完整路径：按类型落到对应子目录，文件名沿用上游给的。
pub fn install_path(target: &Path, kind: ResourceKind, file_name: &str) -> PathBuf {
    target.join(kind.folder()).join(file_name)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{InstallItem, install_path, missing_folders};
    use crate::modrinth::{Project, VersionFile};
    use crate::scan::ResourceKind;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("emcsync-plan-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn item(kind: ResourceKind, slug: &str) -> InstallItem {
        InstallItem {
            project: Project {
                id: slug.to_string(),
                slug: slug.to_string(),
                project_type: kind.label().to_string(),
                title: slug.to_string(),
            },
            kind,
            file: VersionFile {
                hashes: Default::default(),
                url: format!("https://cdn.modrinth.com/{slug}.jar"),
                filename: format!("{slug}.jar"),
                primary: true,
            },
        }
    }

    #[test]
    fn only_lists_folders_that_are_needed_and_missing() {
        let target = temp_dir("folders");
        fs::create_dir_all(target.join("mods")).unwrap();

        let items = [
            item(ResourceKind::Mod, "sodium"),
            item(ResourceKind::ShaderPack, "complementary"),
        ];

        assert_eq!(
            missing_folders(&target, &items),
            vec![ResourceKind::ShaderPack]
        );

        fs::create_dir_all(target.join("shaderpacks")).unwrap();
        assert!(missing_folders(&target, &items).is_empty());

        fs::remove_dir_all(&target).unwrap();
    }

    #[test]
    fn installs_into_the_folder_of_its_kind() {
        let target = PathBuf::from("/instances/demo");

        assert_eq!(
            install_path(&target, ResourceKind::Mod, "Sodium.jar"),
            target.join("mods/Sodium.jar")
        );
        assert_eq!(
            install_path(&target, ResourceKind::ResourcePack, "Fresh.zip"),
            target.join("resourcepacks/Fresh.zip")
        );
    }
}
