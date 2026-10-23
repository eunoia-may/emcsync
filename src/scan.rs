//! 扫描实例目录里的资源文件。
//!
//! 只看 `mods`、`resourcepacks`、`shaderpacks` 三个子目录，不递归；资源类型由所在目录决定。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::hash;

/// 扫描顺序也用作展示顺序。
pub const FOLDERS: [ResourceKind; 3] = [
    ResourceKind::Mod,
    ResourceKind::ResourcePack,
    ResourceKind::ShaderPack,
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ResourceKind {
    Mod,
    ResourcePack,
    ShaderPack,
}

impl ResourceKind {
    /// 实例内的子目录名。
    pub fn folder(self) -> &'static str {
        match self {
            Self::Mod => "mods",
            Self::ResourcePack => "resourcepacks",
            Self::ShaderPack => "shaderpacks",
        }
    }

    /// 输出用的类型名，与规范里的括号内容一致。
    pub fn label(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::ResourcePack => "resourcepack",
            Self::ShaderPack => "shaderpack",
        }
    }

    pub fn from_folder(name: &str) -> Option<Self> {
        match name {
            "mods" => Some(Self::Mod),
            "resourcepacks" => Some(Self::ResourcePack),
            "shaderpacks" => Some(Self::ShaderPack),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResourceFile {
    pub path: PathBuf,
    pub file_name: String,
    pub kind: ResourceKind,
    /// 小写十六进制 SHA-1。
    pub sha1: String,
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("failed to read directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to hash {path}: {source}")]
    Hash {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// 扫描一个实例目录；目录不存在或没有资源时返回空列表。
pub fn scan_instance(dir: &Path) -> Result<Vec<ResourceFile>, ScanError> {
    let mut files = Vec::new();

    for kind in FOLDERS {
        let folder = dir.join(kind.folder());
        if !folder.is_dir() {
            continue;
        }

        let entries = fs::read_dir(&folder).map_err(|source| ScanError::ReadDir {
            path: folder.clone(),
            source,
        })?;

        for entry in entries {
            let entry = entry.map_err(|source| ScanError::ReadDir {
                path: folder.clone(),
                source,
            })?;
            let path = entry.path();

            // 跟随符号链接判断；目录与断链直接跳过。
            if !path.is_file() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().into_owned();
            if file_name.starts_with('.') {
                continue;
            }

            if !is_resource_file(&path) {
                continue;
            }

            let sha1 = hash::sha1_hex(&path).map_err(|source| ScanError::Hash {
                path: path.clone(),
                source,
            })?;

            files.push(ResourceFile {
                path,
                file_name,
                kind,
                sha1,
            });
        }
    }

    files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(files)
}

/// 资源包、光影包通常用 zip，模组用 jar；大小写不敏感，`.jar.disabled` 这类不算。
fn is_resource_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("jar") || extension.eq_ignore_ascii_case("zip")
        })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{ResourceKind, scan_instance};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("emcsync-scan-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, content: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn maps_folders_to_kinds() {
        assert_eq!(ResourceKind::from_folder("mods"), Some(ResourceKind::Mod));
        assert_eq!(
            ResourceKind::from_folder("resourcepacks"),
            Some(ResourceKind::ResourcePack)
        );
        assert_eq!(
            ResourceKind::from_folder("shaderpacks"),
            Some(ResourceKind::ShaderPack)
        );
        assert_eq!(ResourceKind::from_folder("saves"), None);
    }

    #[test]
    fn scans_three_folders_and_skips_noise() {
        let dir = temp_dir("three-folders");
        write(&dir.join("mods/sodium.jar"), b"sodium");
        write(&dir.join("mods/readme.txt"), b"nope");
        write(&dir.join("mods/.hidden.jar"), b"nope");
        write(&dir.join("mods/broken.jar.disabled"), b"nope");
        write(&dir.join("mods/nested/deep.jar"), b"nope");
        write(&dir.join("resourcepacks/pack.zip"), b"pack");
        write(&dir.join("shaderpacks/shader.ZIP"), b"shader");

        let files = scan_instance(&dir).unwrap();

        let names: Vec<_> = files.iter().map(|file| file.file_name.as_str()).collect();
        assert_eq!(names, vec!["pack.zip", "shader.ZIP", "sodium.jar"]);

        let kinds: Vec<_> = files.iter().map(|file| file.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ResourceKind::ResourcePack,
                ResourceKind::ShaderPack,
                ResourceKind::Mod,
            ]
        );

        assert!(
            files
                .iter()
                .all(|file| file.sha1.len() == 40
                    && file.sha1.chars().all(|c| c.is_ascii_hexdigit()))
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_folders_are_empty() {
        let dir = temp_dir("empty");
        assert!(scan_instance(&dir).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
