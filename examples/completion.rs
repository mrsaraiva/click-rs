//! Shell completion example.
//!
//! This example demonstrates shell completion functionality with custom completers.
//!
//! Equivalent to Python Click's examples/completion/completion.py
//!
//! Features demonstrated:
//! - Group with subcommands
//! - Nested groups
//! - Custom shell completion for arguments (simulated via help text)
//! - Path type for directory arguments
//!
//! Note: Custom shell_complete callbacks are not yet implemented in click-rs.
//! This example shows the command structure; completion relies on built-in behavior.

use std::env;
use std::fs;

use click::{
    Argument, ClickError, ClickOption, Command, Context, Group, PathType, Result,
    completion::make_completion_option,
    group::CommandLike,
};

/// Build the main CLI group.
fn build_cli() -> Group {
    Group::new("completion")
        .help("Shell completion demo CLI")
        .command(build_ls_command())
        .command(build_show_env_command())
        .command(build_group())
        .build()
}

/// Build the `ls` command that lists directory contents.
fn build_ls_command() -> Command {
    Command::new("ls")
        .help("List directory contents")
        .option(
            ClickOption::new(&["--dir", "-d"])
                .type_any(PathType::new().dir_okay(true).file_okay(false))
                .help("Directory to list")
                .build(),
        )
        .callback(ls_callback)
        .build()
}

fn ls_callback(ctx: &Context) -> Result<()> {
    let dir = ctx
        .get_param::<String>("dir")
        .map(|s| s.as_str())
        .unwrap_or(".");

    match fs::read_dir(dir) {
        Ok(entries) => {
            let names: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            println!("{}", names.join("\n"));
            Ok(())
        }
        Err(e) => Err(ClickError::usage(format!("Cannot read directory '{}': {}", dir, e))),
    }
}

/// Build the `show-env` command that prints environment variables.
fn build_show_env_command() -> Command {
    Command::new("show-env")
        .help("A command to print environment variables")
        .argument(
            Argument::new("envvar")
                .help("Environment variable name (completion shows available vars)")
                .build(),
        )
        .callback(show_env_callback)
        .build()
}

fn show_env_callback(ctx: &Context) -> Result<()> {
    let envvar = ctx
        .get_param::<String>("envvar")
        .ok_or_else(|| ClickError::missing_argument("ENVVAR"))?;

    println!("Environment variable: {}", envvar);
    match env::var(envvar) {
        Ok(value) => {
            println!("Value: {}", value);
            Ok(())
        }
        Err(_) => Err(ClickError::usage(format!(
            "Environment variable '{}' is not set",
            envvar
        ))),
    }
}

/// Build a nested group with subcommands.
fn build_group() -> Group {
    Group::new("group")
        .help("A group that holds a subcommand")
        .command(build_select_user_command())
        .build()
}

/// Build the `select-user` command inside the nested group.
fn build_select_user_command() -> Command {
    // In Python Click, this uses a custom shell_complete function.
    // click-rs doesn't yet support custom completers, so we document the users in help.
    Command::new("select-user")
        .help("Choose a user (bob=butcher, alice=baker, jerry=candlestick maker)")
        .argument(
            Argument::new("user")
                .help("User to select")
                .build(),
        )
        .callback(select_user_callback)
        .build()
}

fn select_user_callback(ctx: &Context) -> Result<()> {
    let user = ctx
        .get_param::<String>("user")
        .ok_or_else(|| ClickError::missing_argument("USER"))?;

    println!("Chosen user is {}", user);
    Ok(())
}

fn main() {
    let cli = build_cli();
    let prog_name = "completion";
    let complete_var = "_COMPLETION_COMPLETE";

    // Check for shell completion request
    let completion_opt = make_completion_option(complete_var);
    if completion_opt.handle_completion(&cli, prog_name) {
        return;
    }

    // Normal execution
    let args: Vec<String> = env::args().skip(1).collect();

    if let Err(e) = cli.main(args) {
        eprintln!("{}", e.format_full());
        std::process::exit(e.exit_code());
    }
}
