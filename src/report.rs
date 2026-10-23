//! 匹配结果的终端展示。

use emcsync_cli::matching::{MatchResult, Resolved};
use rust_i18n::t;

use crate::ui;

pub fn print_match(result: &MatchResult) {
    let mut blocks = Vec::new();

    if !result.missing.is_empty() {
        blocks.push(matched_block(
            &t!("sync.heading_missing"),
            &result.missing,
            true,
        ));
    }

    if !result.existing.is_empty() {
        blocks.push(matched_block(
            &t!("sync.heading_existing"),
            &result.existing,
            false,
        ));
    }

    if !result.unmatched.is_empty() {
        blocks.push(unmatched_block(
            &t!("sync.heading_unmatched"),
            &result.unmatched,
        ));
    }

    if result.missing.is_empty() {
        blocks.push(format!("{}\n", t!("sync.nothing_missing")));
    }

    ui::print_stdout(&blocks.join("\n"));
}

fn matched_block(heading: &str, items: &[Resolved], with_platform: bool) -> String {
    let mut text = format!("{}\n", ui::heading(heading));

    for item in items {
        let slug = ui::accent(&item.project.slug);
        let kind = item.file.kind.label();

        if with_platform {
            let platform = item.platform.label();
            text.push_str(&format!("  {slug} ({kind}) ({platform})\n"));
        } else {
            text.push_str(&format!("  {slug} ({kind})\n"));
        }
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
