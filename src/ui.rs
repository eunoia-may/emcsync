//! 终端输出。
//!
//! 颜色统一走 anstream：管道、`NO_COLOR`、`TERM=dumb` 时自动去掉 ANSI，不用手写判断。

use std::io::{self, IsTerminal};
use std::process::ExitCode;

use owo_colors::OwoColorize;
use rust_i18n::t;

pub fn print_stdout(text: &str) {
    anstream::print!("{text}");
}

pub fn print_stderr(text: &str) {
    anstream::eprint!("{text}");
}

/// 打印运行时错误并以 1 退出。
pub fn fail(err: &anyhow::Error) -> ExitCode {
    print_stderr(&format!("{}{err}\n", error_label(&t!("error.prefix"))));
    ExitCode::FAILURE
}

/// 交互确认需要终端，非 tty 时不能弹询问。安装确认落地后使用。
#[allow(dead_code)]
pub fn is_interactive() -> bool {
    io::stdin().is_terminal()
}

/// 命令、选项、建议值这类需要一眼看到的名字。
pub fn accent(text: &str) -> String {
    text.bright_cyan().to_string()
}

/// 段落标题。
pub fn heading(text: &str) -> String {
    text.bold().to_string()
}

/// 错误前缀。
pub fn error_label(text: &str) -> String {
    text.red().bold().to_string()
}
