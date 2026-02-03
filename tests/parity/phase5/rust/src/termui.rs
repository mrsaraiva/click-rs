//! Terminal UI parity tests.
//!
//! Mirrors `tests/parity/phase5/python/test_termui.py`.

use click::termui::{echo_via_pager, strip_ansi_codes, style, Color};
use click::testing::CliRunner;
use click::Command;

use crate::util::py_repr;

pub fn run() {
    test_style_and_strip();
    test_echo_and_pager();
}

fn test_style_and_strip() {
    println!("=== Style / Strip ANSI ===");
    let styled = style(
        "Hello",
        Some(Color::Red),
        None,
        true,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
    );
    let stripped = strip_ansi_codes(&styled);
    println!("  has_ansi: {}", styled.contains("\x1b["));
    println!("  stripped: {}", py_repr(&stripped));
}

fn test_echo_and_pager() {
    println!("\n=== Echo / Pager ===");

    let cmd = Command::new("cli")
        .callback(|_ctx| {
            println!("out");
            eprintln!("err");
            echo_via_pager("paged", None);
            Ok(())
        })
        .build();

    let runner = CliRunner::new().env("CI", "1").mix_stderr(false);
    let result = runner.invoke(&cmd, &[]);

    println!("  exit_code: {}", result.exit_code);
    println!("  stdout: {}", py_repr(&result.output));
    println!("  stderr: {}", py_repr(&result.stderr));

    // Keep parity runner output clean if the command errored unexpectedly.
    if result.exit_code != 0 {
        eprintln!(
            "unexpected error: {}",
            result.exception_message.unwrap_or_else(|| "unknown".to_string())
        );
        std::process::exit(1);
    }
}
