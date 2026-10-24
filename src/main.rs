//! 命令行入口：初始化语言、拦截 help、解析参数并分发。

mod cli;
mod install;
mod report;
mod ui;

use std::process::ExitCode;

use anyhow::Result;
use emcsync::{matching, modrinth, plan, scan};
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

    if !result.missing.is_empty() {
        ui::status(&t!("sync.versions"));
    }

    let install_plan =
        plan::build(&client, &result.missing, &args.version, args.loader.label()).await?;
    let create = plan::missing_folders(&args.target_dir, &install_plan.installable);

    report::print_plan(&result, &install_plan);

    // 缺目录不单独提问，与安装确认合成一次。
    if !create.is_empty() {
        let folders = create
            .iter()
            .map(|kind| kind.folder())
            .collect::<Vec<_>>()
            .join(&t!("error.separator"));
        ui::print_stdout(&format!(
            "{}\n\n",
            t!("sync.creating_dirs", folders = folders)
        ));
    }

    if args.dry_run {
        return Ok(());
    }

    if install_plan.installable.is_empty() {
        ui::print_stdout(&format!("{}\n", t!("sync.done")));
        return Ok(());
    }

    if !args.yes {
        if !ui::is_interactive() {
            anyhow::bail!("{}", t!("sync.non_interactive"));
        }

        if !ui::confirm(&t!("sync.confirm_install"))? {
            ui::print_stdout(&format!("{}\n", t!("sync.cancelled")));
            return Ok(());
        }
    }

    if let Err(err) = install::install(
        &client,
        &install_plan.installable,
        &args.target_dir,
        &create,
    )
    .await
    {
        ui::print_stderr(&format!("{}\n", t!("sync.install_failed")));
        return Err(err);
    }

    ui::print_stdout(&format!("{}\n", t!("sync.done")));
    Ok(())
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
