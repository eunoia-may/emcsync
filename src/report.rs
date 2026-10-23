//! 匹配与安装计划的终端展示。
//!
//! 四类按“待安装、此版本不存在、已存在、无法匹配”的顺序输出，类内按名称排序。

use emcsync_cli::matching::{MatchResult, Resolved};
use emcsync_cli::plan::{InstallItem, Plan};
use rust_i18n::t;

use crate::ui;

pub fn print_plan(result: &MatchResult, plan: &Plan) {
    let mut blocks = Vec::new();

    if !plan.installable.is_empty() {
        blocks.push(installable_block(
            &t!("sync.heading_missing"),
            &plan.installable,
        ));
    }

    if !plan.unavailable.is_empty() {
        blocks.push(plain_block(
            &t!("sync.heading_unavailable"),
            &plan.unavailable,
        ));
    }

    if !result.existing.is_empty() {
        blocks.push(plain_block(&t!("sync.heading_existing"), &result.existing));
    }

    if !result.unmatched.is_empty() {
        blocks.push(unmatched_block(
            &t!("sync.heading_unmatched"),
            &result.unmatched,
        ));
    }

    if plan.installable.is_empty() {
        blocks.push(format!("{}\n", t!("sync.nothing_missing")));
    }

    ui::print_stdout(&blocks.join("\n"));
}

/// 待安装的资源带平台，方便看出是从哪拿的。
fn installable_block(heading: &str, items: &[InstallItem]) -> String {
    let mut text = format!("{}\n", ui::heading(heading));

    for item in items {
        text.push_str(&format!(
            "  {} ({}) ({})\n",
            ui::accent(&item.project.slug),
            item.kind.label(),
            item.platform.label()
        ));
    }

    text
}

/// 此版本不存在、已存在的资源不显示平台，省一次反查。
fn plain_block(heading: &str, items: &[Resolved]) -> String {
    let mut text = format!("{}\n", ui::heading(heading));

    for item in items {
        text.push_str(&format!(
            "  {} ({})\n",
            ui::accent(&item.project.slug),
            item.file.kind.label()
        ));
    }

    text
}

fn unmatched_block(heading: &str, names: &[String]) -> String {
    let mut text = format!("{}\n", ui::heading(heading));

    for name in names {
        text.push_str(&format!("  {name}\n"));
    }

    text
}
