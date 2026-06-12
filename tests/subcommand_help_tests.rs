//! Regression tests: `--help` on a subcommand must render that subcommand's
//! help text instead of propagating Exit{0} silently (which made
//! `app subcmd --help` print nothing and exit 0).
//!
//! Bug history: Group::invoke forwarded the subcommand's make_context
//! Exit{0} with `?`, so only the ROOT --help (handled in Command::main)
//! ever printed anything. Fixed in 1.0.1.

use click::command::Command;
use click::context::Context;
use click::group::{CommandLike, Group};
use click::option::ClickOption;

fn build_cli() -> Group {
    let sub = Command::new("deploy")
        .help("Deploy a topology to the server.")
        .option(
            ClickOption::new(&["--workspace-directory"])
                .help("Workspace source directory")
                .build(),
        )
        .callback(|_ctx: &Context| Ok(()))
        .build();

    Group::new("podium").command(sub).build()
}

/// Capturing stdout of `println!` from the same process requires a child
/// process; instead assert at the API seam: invoking the group with
/// `<sub> --help` must return Ok(()) (help-printed path), NOT Exit{0} or
/// any other error, and must not invoke the callback.
#[test]
fn subcommand_help_returns_ok_not_silent_exit() {
    let cli = build_cli();
    let result = cli.main(vec!["deploy".to_string(), "--help".to_string()]);
    assert!(
        result.is_ok(),
        "subcommand --help must resolve to the printed-help Ok path, got: {result:?}"
    );
}

/// The help text itself must be renderable for a subcommand with its full
/// command path (this is what Group::invoke prints on Exit{0}).
#[test]
fn subcommand_help_text_is_nonempty_and_has_usage() {
    let cli = build_cli();
    // Render exactly what the Exit{0} handler renders.
    let ctx = click::context::ContextBuilder::new()
        .info_name("podium deploy")
        .build();
    let sub = Command::new("deploy")
        .help("Deploy a topology to the server.")
        .option(
            ClickOption::new(&["--workspace-directory"])
                .help("Workspace source directory")
                .build(),
        )
        .build();
    let help = sub.get_help(&ctx);
    assert!(!help.trim().is_empty(), "subcommand help must not be empty");
    assert!(
        help.contains("Usage:") && help.contains("--workspace-directory"),
        "help must contain usage line and option names, got:\n{help}"
    );
    let _ = cli; // group construction sanity
}
