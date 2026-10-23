//! 命令行入口：初始化语言、拦截 help、解析参数并分发。

mod cli;
mod report;
mod ui;

use std::process::ExitCode;

use anyhow::Result;
use emcsync_cli::{matching, modrinth, scan};
use rust_i18n::t;

rust_i18n::i18n!("locales", fallback = "en");

#[tokio::main]
async fn main() -> ExitCode {
    init_locale();

    // clap 的自动 help 已关闭，help 的三种写法在解析前统一处理。
    if cli::help_requested(std::env::args_os().skip(1)) {
        ui::print_stdout(&cli::help_text());
        return ExitCode::SUCCESS;
    }

    match cli::parse() {
        Ok(cli::Command::Sync(args)) => match sync(args).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => ui::fail(&err),
        },
        Err(err) => cli::report(err),
    }
}

fn init_locale() {
    rust_i18n::set_locale(pick_locale(sys_locale::get_locale().as_deref()));
}

fn pick_locale(raw: Option<&str>) -> &'static str {
    match raw {
        Some(locale) if locale.to_ascii_lowercase().starts_with("zh") => "zh-CN",
        _ => "en",
    }
}

async fn sync(args: cli::SyncArgs) -> Result<()> {
    if !args.source_dir.is_dir() {
        anyhow::bail!(
            "{}",
            t!(
                "sync.source_missing",
                path = args.source_dir.display().to_string()
            )
        );
    }

    if !args.target_dir.is_dir() {
        anyhow::bail!(
            "{}",
            t!(
                "sync.target_missing",
                path = args.target_dir.display().to_string()
            )
        );
    }

    if matches!(args.prefer, cli::Platform::CurseForge) {
        ui::warn(&t!("sync.curseforge_deferred"));
    }

    ui::status(&t!("sync.matching"));
    let source = scan::scan_instance(&args.source_dir)?;
    let target = scan::scan_instance(&args.target_dir)?;

    if source.is_empty() {
        ui::warn(&t!("sync.empty_source"));
        return Ok(());
    }

    let client = modrinth::Client::new()?;
    let result = matching::match_resources(&client, &source, &target).await?;
    report::print_match(&result);

    if args.dry_run {
        return Ok(());
    }

    // 版本兼容查询与安装尚未落地；目标已齐全时不必再等下一步。
    if result.missing.is_empty() {
        ui::print_stdout(&format!("{}\n", t!("sync.done")));
        return Ok(());
    }

    anyhow::bail!("{}", t!("sync.install_not_implemented"));
}

#[cfg(test)]
mod tests {
    use super::pick_locale;

    #[test]
    fn zh_locales_map_to_zh_cn() {
        assert_eq!(pick_locale(Some("zh-CN")), "zh-CN");
        assert_eq!(pick_locale(Some("zh_TW")), "zh-CN");
    }

    #[test]
    fn other_locales_fall_back_to_en() {
        assert_eq!(pick_locale(Some("en-US")), "en");
        assert_eq!(pick_locale(None), "en");
    }
}
