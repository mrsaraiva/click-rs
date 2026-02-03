//! Testing parity tests.
//!
//! Mirrors `tests/parity/phase6/python/test_testing.py`.

use click::testing::CliRunner;
use click::Command;

use crate::util::py_repr;

pub fn run() {
    println!("=== CliRunner ===");

    let cmd = Command::new("cli")
        .callback(|_ctx| {
            println!("out");
            eprintln!("err");
            Ok(())
        })
        .build();

    let runner = CliRunner::new();
    let result = runner.mix_stderr(false).invoke(&cmd, &[]);

    println!("  exit_code: {}", result.exit_code);
    println!("  stdout: {}", py_repr(&result.output));
    println!("  stderr: {}", py_repr(&result.stderr));

    if result.exit_code != 0 {
        eprintln!(
            "unexpected error: {}",
            result.exception_message.unwrap_or_else(|| "unknown".to_string())
        );
        std::process::exit(1);
    }
}
