//! clap 错误的本地化：只翻常见几种，其余交给 clap 原样输出。

use clap::error::{ContextKind, ContextValue, Error, ErrorKind};
use rust_i18n::t;

use crate::ui;

pub fn localize(err: &Error) -> Option<String> {
    match err.kind() {
        ErrorKind::InvalidSubcommand => {
            let name = value(err, ContextKind::InvalidSubcommand, 0).unwrap_or_default();
            let mut lines = vec![t!("error.unknown_subcommand", name = name).to_string()];
            if let Some(suggestion) = value(err, ContextKind::SuggestedSubcommand, 0) {
                lines.push(t!("error.suggestion", value = ui::accent(&suggestion)).to_string());
            }
            Some(finish(lines))
        }
        ErrorKind::UnknownArgument => {
            let name = value(err, ContextKind::InvalidArg, 0).unwrap_or_default();
            let mut lines = vec![t!("error.unknown_argument", name = name).to_string()];
            if let Some(suggestion) = value(err, ContextKind::SuggestedArg, 0) {
                lines.push(t!("error.suggestion", value = ui::accent(&suggestion)).to_string());
            }
            Some(finish(lines))
        }
        ErrorKind::MissingSubcommand | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            Some(finish(vec![t!("error.missing_subcommand").to_string()]))
        }
        ErrorKind::MissingRequiredArgument => {
            let names = values(err, ContextKind::InvalidArg).join(&t!("error.separator"));
            Some(finish(vec![
                t!("error.missing_required", names = names).to_string(),
            ]))
        }
        ErrorKind::InvalidValue => {
            let arg = value(err, ContextKind::InvalidArg, 0).unwrap_or_default();
            let invalid = value(err, ContextKind::InvalidValue, 0).unwrap_or_default();
            let valid = values(err, ContextKind::ValidValue).join(&t!("error.separator"));
            let mut lines = vec![
                t!(
                    "error.invalid_value",
                    arg = arg,
                    value = ui::accent(&invalid),
                    valid = valid
                )
                .to_string(),
            ];
            if let Some(suggestion) = value(err, ContextKind::SuggestedValue, 0) {
                lines.push(t!("error.suggestion", value = ui::accent(&suggestion)).to_string());
            }
            Some(finish(lines))
        }
        _ => None,
    }
}

/// 首行是错误本身，其后是建议与查用法提示。
fn finish(mut lines: Vec<String>) -> String {
    lines.push(t!("error.help_hint", program = env!("CARGO_PKG_NAME")).to_string());
    let first = lines.remove(0);
    let mut message = format!("{}{first}", ui::error_label(&t!("error.prefix")));
    for line in lines {
        message.push('\n');
        message.push_str(&line);
    }
    message
}

fn value(err: &Error, kind: ContextKind, index: usize) -> Option<String> {
    values(err, kind).into_iter().nth(index)
}

fn values(err: &Error, kind: ContextKind) -> Vec<String> {
    match err.get(kind) {
        Some(ContextValue::String(text)) => vec![text.clone()],
        Some(ContextValue::Strings(texts)) => texts.clone(),
        Some(ContextValue::StyledStr(text)) => vec![text.to_string()],
        Some(ContextValue::StyledStrs(texts)) => texts.iter().map(ToString::to_string).collect(),
        _ => Vec::new(),
    }
}
