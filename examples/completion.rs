//! Shell completion demo, aligned with Python Click's completion example.

use std::env;
use std::fs;

use click::{run_with_completion, ClickError, Result};

#[click::command(name = "ls", help = "List directory contents")]
fn ls(
    #[option(
        short = 'd',
        long = "dir",
        help = "Directory to list",
        shell_complete = click::complete::directories
    )]
    dir: Option<String>,
) -> Result<()> {
    let dir = dir.as_deref().unwrap_or(".");
    match fs::read_dir(dir) {
        Ok(entries) => {
            let names: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect();
            println!("{}", names.join("\n"));
            Ok(())
        }
        Err(e) => Err(ClickError::usage(format!(
            "Cannot read directory '{}': {}",
            dir, e
        ))),
    }
}

#[click::command(name = "show-env", help = "A command to print environment variables")]
fn show_env(
    #[argument(help = "Environment variable name", shell_complete = click::complete::env_vars)]
    envvar: String,
) -> Result<()> {
    println!("Environment variable: {}", envvar);
    match env::var(&envvar) {
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

#[click::command(name = "select-user", help = "Choose a user")]
fn select_user(
    #[argument(
        help = "User to select",
        shell_complete = click::complete::items_with_help(&[
            ("bob", "butcher"),
            ("alice", "baker"),
            ("jerry", "candlestick maker"),
        ])
    )]
    user: String,
) -> Result<()> {
    println!("Chosen user is {}", user);
    Ok(())
}

#[click::group(
    name = "group",
    help = "A group that holds a subcommand",
    commands = [select_user]
)]
fn group() -> Result<()> {
    Ok(())
}

#[click::group(
    name = "completion",
    help = "Shell completion demo CLI",
    commands = [ls, show_env],
    groups = [group]
)]
fn completion() -> Result<()> {
    Ok(())
}

fn main() {
    let cli = completion_group();
    run_with_completion(&cli, "completion", "_COMPLETION_COMPLETE");
}
