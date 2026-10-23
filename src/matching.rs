//! 把扫描到的文件反查到项目，再做源与目标的差集。
//!
//! 只按 `project_id` 比对：目标里已有同一个项目即视为“已存在”，不区分版本。

use std::collections::{BTreeSet, HashMap};

use crate::modrinth::{ApiError, Client, Project, Version};
use crate::platform::Platform;
use crate::scan::ResourceFile;

#[derive(Clone, Debug)]
pub struct Resolved {
    pub file: ResourceFile,
    pub project: Project,
    pub platform: Platform,
}

#[derive(Clone, Debug, Default)]
pub struct MatchResult {
    /// 源里有、目标里没有的项目。
    pub missing: Vec<Resolved>,
    /// 源与目标都有的项目。
    pub existing: Vec<Resolved>,
    /// 两个平台都查不到的文件名。
    pub unmatched: Vec<String>,
}

pub async fn match_resources(
    client: &Client,
    source: &[ResourceFile],
    target: &[ResourceFile],
) -> Result<MatchResult, ApiError> {
    let mut hashes = Vec::with_capacity(source.len() + target.len());
    hashes.extend(source.iter().map(|file| file.sha1.clone()));
    hashes.extend(target.iter().map(|file| file.sha1.clone()));

    let versions = client.versions_by_sha1(&hashes).await?;

    // 版本对象里只有 project_id，slug 还要再查一次项目。
    let project_ids: Vec<String> = versions
        .values()
        .map(|version| version.project_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let projects: HashMap<String, Project> = client
        .projects_by_ids(&project_ids)
        .await?
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();

    let (source_resolved, source_unmatched) = resolve(source, &versions, &projects);
    let (target_resolved, target_unmatched) = resolve(target, &versions, &projects);

    let target_ids: BTreeSet<String> = target_resolved
        .iter()
        .map(|resolved| resolved.project.id.clone())
        .collect();
    let (missing, existing) = split(source_resolved, &target_ids);

    let mut unmatched: Vec<String> = source_unmatched
        .into_iter()
        .chain(target_unmatched)
        .collect();
    unmatched.sort();
    unmatched.dedup();

    Ok(MatchResult {
        missing,
        existing,
        unmatched,
    })
}

/// 一个文件对应一个项目；查不到的只留文件名。
fn resolve(
    files: &[ResourceFile],
    versions: &HashMap<String, Version>,
    projects: &HashMap<String, Project>,
) -> (Vec<Resolved>, Vec<String>) {
    let mut resolved = Vec::new();
    let mut unmatched = Vec::new();

    for file in files {
        let project = versions
            .get(&file.sha1)
            .and_then(|version| projects.get(&version.project_id));

        match project {
            Some(project) => resolved.push(Resolved {
                file: file.clone(),
                project: project.clone(),
                platform: Platform::Modrinth,
            }),
            None => unmatched.push(file.file_name.clone()),
        }
    }

    (resolved, unmatched)
}

/// 按项目 ID 做差集，同一项目的多个文件只保留一个，最后按 slug 排序。
fn split(source: Vec<Resolved>, target_ids: &BTreeSet<String>) -> (Vec<Resolved>, Vec<Resolved>) {
    let mut missing = Vec::new();
    let mut existing = Vec::new();

    for resolved in source {
        if target_ids.contains(&resolved.project.id) {
            existing.push(resolved);
        } else {
            missing.push(resolved);
        }
    }

    (dedup_by_project(missing), dedup_by_project(existing))
}

fn dedup_by_project(mut items: Vec<Resolved>) -> Vec<Resolved> {
    items.sort_by(|a, b| {
        a.project
            .slug
            .cmp(&b.project.slug)
            .then_with(|| a.file.file_name.cmp(&b.file.file_name))
    });
    items.dedup_by(|a, b| a.project.id == b.project.id);
    items
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use super::{Resolved, resolve, split};
    use crate::modrinth::{Project, Version};
    use crate::platform::Platform;
    use crate::scan::{ResourceFile, ResourceKind};

    fn file(name: &str, sha1: &str) -> ResourceFile {
        ResourceFile {
            path: format!("/tmp/{name}").into(),
            file_name: name.to_string(),
            kind: ResourceKind::Mod,
            sha1: sha1.to_string(),
        }
    }

    fn project(id: &str, slug: &str) -> Project {
        Project {
            id: id.to_string(),
            slug: slug.to_string(),
            project_type: "mod".to_string(),
            title: slug.to_string(),
        }
    }

    fn version(project_id: &str) -> Version {
        Version {
            id: format!("{project_id}-version"),
            project_id: project_id.to_string(),
            version_number: "1.0.0".to_string(),
            version_type: "release".to_string(),
            files: Vec::new(),
        }
    }

    #[test]
    fn unknown_hashes_become_unmatched() {
        let versions = HashMap::from([("aaa".to_string(), version("P1"))]);
        let projects = HashMap::from([("P1".to_string(), project("P1", "sodium"))]);

        let (resolved, unmatched) = resolve(
            &[file("known.jar", "aaa"), file("unknown.jar", "bbb")],
            &versions,
            &projects,
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].project.slug, "sodium");
        assert_eq!(unmatched, vec!["unknown.jar"]);
    }

    #[test]
    fn existing_project_is_removed_from_missing() {
        let source = vec![
            Resolved {
                file: file("sodium.jar", "a"),
                project: project("P1", "sodium"),
                platform: Platform::Modrinth,
            },
            Resolved {
                file: file("sodium-extra.jar", "b"),
                project: project("P2", "sodium-extra"),
                platform: Platform::Modrinth,
            },
            Resolved {
                file: file("lib.jar", "c"),
                project: project("P3", "fabric-api"),
                platform: Platform::Modrinth,
            },
        ];
        let target_ids = BTreeSet::from(["P1".to_string(), "P3".to_string()]);

        let (missing, existing) = split(source, &target_ids);

        let missing_slugs: Vec<_> = missing
            .iter()
            .map(|item| item.project.slug.as_str())
            .collect();
        let existing_slugs: Vec<_> = existing
            .iter()
            .map(|item| item.project.slug.as_str())
            .collect();

        assert_eq!(missing_slugs, vec!["sodium-extra"]);
        assert_eq!(existing_slugs, vec!["fabric-api", "sodium"]);
    }

    #[test]
    fn duplicate_project_keeps_one_entry() {
        let source = vec![
            Resolved {
                file: file("z-sodium.jar", "a"),
                project: project("P1", "sodium"),
                platform: Platform::Modrinth,
            },
            Resolved {
                file: file("a-sodium.jar", "a"),
                project: project("P1", "sodium"),
                platform: Platform::Modrinth,
            },
        ];

        let (missing, _) = split(source, &BTreeSet::new());

        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].file.file_name, "a-sodium.jar");
    }
}
