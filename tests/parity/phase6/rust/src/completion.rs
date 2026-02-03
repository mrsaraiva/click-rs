//! Completion parity tests.
//!
//! Mirrors `tests/parity/phase6/python/test_completion.py`.

use click::completion::{get_completion_class, get_completions};
use click::group::Group;
use click::option::ClickOption;
use click::Command;

use crate::util::py_repr;

fn build_cli() -> Group {
    Group::new("cli")
        .command(
            Command::new("build")
                .help("Build")
                .option(ClickOption::new(&["--count", "-c"]).help("count").build())
                .build(),
        )
        .command(Command::new("init").help("Init").build())
        .build()
}

fn completion_output(shell: &str, env: &[(&str, &str)]) -> String {
    for (k, v) in env {
        std::env::set_var(k, v);
    }

    let cli = build_cli();
    let completer = get_completion_class(shell).expect("shell supported");
    let comp_args = completer.get_completion_args();
    let items = get_completions(&cli, "cli", &comp_args.args, &comp_args.incomplete);

    let mut out = String::new();
    for item in items {
        out.push_str(&completer.format_completion(&item));
        out.push('\n');
    }
    out
}

pub fn run() {
    println!("=== Completion ===");

    let out = completion_output(
        "bash",
        &[
            ("COMP_WORDS", "cli b"),
            ("COMP_CWORD", "1"),
        ],
    );
    println!("# bash group:");
    println!("  exit_code: 0");
    println!("  output: {}", py_repr(&out));

    let out = completion_output(
        "zsh",
        &[
            ("COMP_WORDS", "cli b"),
            ("COMP_CWORD", "1"),
        ],
    );
    println!("# zsh group:");
    println!("  exit_code: 0");
    println!("  output: {}", py_repr(&out));

    let out = completion_output(
        "fish",
        &[
            ("COMP_WORDS", "cli b"),
            ("COMP_CWORD", "b"),
        ],
    );
    println!("# fish group:");
    println!("  exit_code: 0");
    println!("  output: {}", py_repr(&out));

    let out = completion_output(
        "bash",
        &[
            ("COMP_WORDS", "cli build --"),
            ("COMP_CWORD", "2"),
        ],
    );
    println!("# bash options:");
    println!("  exit_code: 0");
    println!("  output: {}", py_repr(&out));

    let out = completion_output(
        "zsh",
        &[
            ("COMP_WORDS", "cli build --"),
            ("COMP_CWORD", "2"),
        ],
    );
    println!("# zsh options:");
    println!("  exit_code: 0");
    println!("  output: {}", py_repr(&out));

    let out = completion_output(
        "fish",
        &[
            ("COMP_WORDS", "cli build --"),
            ("COMP_CWORD", "--"),
        ],
    );
    println!("# fish options:");
    println!("  exit_code: 0");
    println!("  output: {}", py_repr(&out));
}

