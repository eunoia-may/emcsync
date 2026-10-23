//! 命令行入口：初始化语言、拦截 help、解析参数并分发。

mod cli;
mod ui;

use std::process::ExitCode;

use anyhow::Result;
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
    // 骨架阶段先打印解析结果，确认参数都到位；实现同步流程后替换。
    eprintln!("{args:#?}");
    anyhow::bail!("{}", t!("sync.not_implemented"));
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
