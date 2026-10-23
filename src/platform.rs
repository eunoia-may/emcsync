//! 资源平台。
//!
//! 目前只接 Modrinth；CurseForge 的枚举位留到真正接入时再加。

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Modrinth,
}

impl Platform {
    pub fn label(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
        }
    }
}
