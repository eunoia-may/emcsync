//! 命令行定义与解析入口。

mod error;
mod help;

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::ui;

#[derive(Parser)]
#[command(
    name = "emcsync",
    version,
    disable_help_flag = true,
    disable_help_subcommand = true,
    // 没带子命令时走自己的错误文案，不要退回 clap 的英文 usage。
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// 同步
    Sync(SyncArgs),
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// 提取源，即原实例的目录
    #[arg(value_name = "source_dir")]
    pub source_dir: PathBuf,

    /// 复现目标，即待复现实例的目录
    #[arg(value_name = "target_dir")]
    pub target_dir: PathBuf,

    /// 要复现的资源属于哪个 Minecraft 版本
    #[arg(short = 'v', long, value_name = "mc_version")]
    pub version: String,

    /// 加载器
    #[arg(short = 'l', long, ignore_case = true, value_name = "loader")]
    pub loader: Loader,

    /// 同时下载的文件数
    #[arg(
        short = 'j',
        long,
        value_name = "jobs",
        default_value_t = 4,
        value_parser = clap::value_parser!(u8).range(1..=16)
    )]
    pub jobs: u8,

    /// 预同意，跳过询问环节
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// 只展示变化，不实际安装
    #[arg(short = 'd', long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum Loader {
    Fabric,
    Forge,
    NeoForge,
    Quilt,
}

impl Loader {
    /// Modrinth 用这套小写名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Fabric => "fabric",
            Self::Forge => "forge",
            Self::NeoForge => "neoforge",
            Self::Quilt => "quilt",
        }
    }
}

pub fn parse() -> Result<Command, clap::Error> {
    Cli::try_parse().map(|cli| cli.command)
}

/// help 的三种写法：`help`、`-h`、`--help`。
///
/// `help` 只在第一个位置算数，免得把名字真叫 help 的目录当成请求帮助。
pub fn help_requested<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut first = true;
    for arg in args {
        let arg = arg.as_ref();
        if arg == "--" {
            break;
        }
        if arg == "-h" || arg == "--help" || (first && arg == "help") {
            return true;
        }
        first = false;
    }
    false
}

pub fn help_text() -> String {
    help::render()
}

/// 解析失败时按 locale 输出错误，用法错误统一以 2 退出。
pub fn report(err: clap::Error) -> ExitCode {
    if matches!(err.kind(), clap::error::ErrorKind::DisplayVersion) {
        let _ = err.print();
        return ExitCode::SUCCESS;
    }

    match error::localize(&err) {
        Some(message) => ui::print_stderr(&format!("{message}\n")),
        // 少见的错误原样交给 clap，保持英文但信息完整。
        None => {
            let _ = err.print();
        }
    }
    ExitCode::from(2)
}
