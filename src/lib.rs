//! 资源匹配与安装的核心逻辑。
//!
//! 命令行入口在 `src/main.rs`。这里只放与终端无关的逻辑，方便单测。

pub mod hash;
pub mod matching;
pub mod modrinth;
pub mod platform;
pub mod scan;
