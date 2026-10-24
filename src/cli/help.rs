//! 手写的 help 文本。
//!
//! clap 自动 help 已关掉：需要跟随 locale，文案也与 `规范-命令设计与简单演示.md` 一致。

use rust_i18n::t;

use crate::ui;

// 左侧名字一栏的宽度，取该栏最长的一项再加两列空隙。
const COMMAND_WIDTH: usize = 8;
const ARGUMENT_WIDTH: usize = 14;
const OPTION_WIDTH: usize = 28;

pub fn render() -> String {
    let program = env!("CARGO_PKG_NAME");
    let mut text = String::new();

    text.push_str(&format!(
        "{} {}\n{}\n\n",
        ui::heading(program),
        env!("CARGO_PKG_VERSION"),
        t!("app.about")
    ));

    text.push_str(&section(&t!("help.usage_heading")));
    text.push_str(&format!(
        "  {} {}\n",
        ui::accent(program),
        t!("help.sync_usage")
    ));
    text.push_str(&format!(
        "  {} {}\n\n",
        ui::accent(program),
        t!("help.help_usage")
    ));

    text.push_str(&section(&t!("help.commands_heading")));
    text.push_str(&item("sync", &t!("help.sync_about"), COMMAND_WIDTH));
    text.push_str(&item("help", &t!("help.help_about"), COMMAND_WIDTH));
    text.push('\n');

    text.push_str(&section(&t!("help.args_heading")));
    text.push_str(&item(
        "<source_dir>",
        &t!("help.source_dir"),
        ARGUMENT_WIDTH,
    ));
    text.push_str(&item(
        "<target_dir>",
        &t!("help.target_dir"),
        ARGUMENT_WIDTH,
    ));
    text.push('\n');

    text.push_str(&section(&t!("help.options_heading")));
    text.push_str(&item(
        "-v, --version <mc_version>",
        &t!("help.opt_version"),
        OPTION_WIDTH,
    ));
    text.push_str(&item(
        "-l, --loader <loader>",
        &t!("help.opt_loader"),
        OPTION_WIDTH,
    ));
    text.push_str(&item(
        "-j, --jobs <jobs>",
        &t!("help.opt_jobs"),
        OPTION_WIDTH,
    ));
    text.push_str(&item("-y, --yes", &t!("help.opt_yes"), OPTION_WIDTH));
    text.push_str(&item(
        "-d, --dry-run",
        &t!("help.opt_dry_run"),
        OPTION_WIDTH,
    ));
    text.push_str(&item("-h, --help", &t!("help.opt_help"), OPTION_WIDTH));
    text.push_str(&item(
        "-V, --version",
        &t!("help.opt_program_version"),
        OPTION_WIDTH,
    ));

    text
}

fn section(title: &str) -> String {
    format!("{}\n", ui::heading(title))
}

fn item(name: &str, desc: &str, width: usize) -> String {
    // 先补齐宽度再加颜色，避免 ANSI 转义把对齐算进去。
    let padded = format!("{name:<width$}");
    format!("  {}{desc}\n", ui::accent(&padded))
}
