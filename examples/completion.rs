//! Shell completion example.
//!
//! This example demonstrates shell completion functionality with custom completers.
//!
//! Equivalent to Python Click's examples/completion/completion.py
//!
//! Features demonstrated:
//! - Group with subcommands
//! - Nested groups
//! - Custom shell completion for arguments using `shell_complete` callbacks
//! - Path type for directory arguments

use std::env;
use std::fs;

use click::{
    Argument, ClickError, ClickOption, Command, CompletionItem, Context, Group, PathType, Result,
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
                .help("Environment variable name")
                .shell_complete(|_ctx, incomplete| {
                    // Return environment variable names matching the incomplete prefix
                    env::vars()
                        .filter(|(key, _)| key.to_lowercase().starts_with(&incomplete.to_lowercase()))
                        .take(10)
                        .map(|(key, _)| CompletionItem::new(key))
                        .collect()
                })
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
    // User database for shell completion
    let users = [
        ("bob", "butcher"),
        ("alice", "baker"),
        ("jerry", "candlestick maker"),
    ];

    Command::new("select-user")
        .help("Choose a user")
        .argument(
            Argument::new("user")
                .help("User to select")
                .shell_complete(move |_ctx, incomplete| {
                    // Return users matching the incomplete prefix with their occupation as help
                    users
                        .iter()
                        .filter(|(name, _)| name.starts_with(incomplete))
                        .map(|(name, occupation)| {
                            CompletionItem::new(name.to_string()).with_help(occupation.to_string())
                        })
                        .collect()
                })
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
