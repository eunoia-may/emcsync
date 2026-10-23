use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use rust_i18n::t;

rust_i18n::i18n!("locales", fallback = "en");

#[derive(Parser)]
#[command(name = "emcsync-cli", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 同步
    Sync(SyncArgs),
}

#[derive(Debug, Args)]
struct SyncArgs {
    /// 提取源，即原实例的目录
    source_dir: PathBuf,

    /// 复现目标，即待复现实例的目录
    target_dir: PathBuf,

    /// 要复现的资源属于哪个 Minecraft 版本
    #[arg(short = 'v', long)]
    version: String,

    /// 加载器
    #[arg(short = 'l', long, ignore_case = true)]
    loader: Loader,

    /// 优先从哪个平台获取资源
    #[arg(short = 'p', long, ignore_case = true)]
    prefer: Platform,

    /// 预同意，跳过询问环节
    #[arg(short = 'y', long)]
    yes: bool,

    /// 只展示变化，不实际安装
    #[arg(short = 'd', long = "dry-run")]
    dry_run: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
enum Loader {
    Fabric,
    Forge,
    NeoForge,
    Quilt,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
enum Platform {
    Modrinth,
    CurseForge,
}

fn init_locale() {
    let locale = sys_locale::get_locale().unwrap_or_default();
    let locale = if locale.to_ascii_lowercase().starts_with("zh") {
        "zh-CN"
    } else {
        "en"
    };
    rust_i18n::set_locale(locale);
}

#[tokio::main]
async fn main() -> Result<()> {
    init_locale();

    match Cli::parse().command {
        Command::Sync(args) => {
            // 骨架阶段先打印解析结果，确认参数都到位；实现同步逻辑后删掉。
            eprintln!("{args:#?}");
            anyhow::bail!("{}", t!("sync.not_implemented"));
        }
    }
}
