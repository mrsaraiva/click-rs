//! Port of Python Click's repo example.
//!
//! This example demonstrates how to build a complex command-line interface
//! similar to git or other version control systems.
//!
//! Run with: cargo run --example repo -- [--repo-home PATH] [--config KEY VALUE]... <command>
//!
//! Features demonstrated:
//! - Group with multiple subcommands
//! - pass_context and pass_obj patterns
//! - Environment variable reading (REPO_HOME)
//! - Multiple/variadic options (--config)
//! - Boolean flag options (--verbose, --shallow/--deep)
//! - Optional arguments with defaults
//! - Password prompting (simulated)
//! - Multi-line message input

use click::{
    echo, make_pass_decorator, Argument, ClickError, ClickOption, Command, CommandLike, Group,
    Result,
};
use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::sync::Arc;

/// Repository state object that gets passed between commands.
#[derive(Debug, Clone)]
struct Repo {
    home: String,
    config: HashMap<String, String>,
    verbose: bool,
}

impl Repo {
    fn new(home: &str) -> Self {
        Self {
            home: home.to_string(),
            config: HashMap::new(),
            verbose: false,
        }
    }

    fn set_config(&mut self, key: &str, value: &str) {
        self.config.insert(key.to_string(), value.to_string());
        if self.verbose {
            eprintln!("  config[{}] = {}", key, value);
        }
    }
}

fn main() {
    // Create pass_repo decorator
    let pass_repo = || make_pass_decorator::<Repo>();

    // Clone subcommand
    let clone_cmd = Command::new("clone")
        .help("Clones a repository.\n\nThis will clone the repository at SRC into the folder DEST. If DEST\nis not provided this will automatically use the last path component\nof SRC and create that folder.")
        .argument(
            Argument::new("src")
                .help("Source repository URL")
                .build(),
        )
        .argument(
            Argument::new("dest")
                .help("Destination folder")
                .default("")
                .build(),
        )
        .option(
            ClickOption::new(&["--shallow", "--deep"])
                .help("Makes a checkout shallow or deep. Deep by default.")
                .flag("true")
                .default("false")
                .build(),
        )
        .option(
            ClickOption::new(&["--rev", "-r"])
                .help("Clone a specific revision instead of HEAD.")
                .default("HEAD")
                .build(),
        )
        .callback(pass_repo().decorate(|_repo: &Repo, ctx| {
            let src = ctx.get_param::<String>("src")
                .ok_or_else(|| ClickError::missing_argument("SRC"))?;
            let dest = ctx.get_param::<String>("dest")
                .map(|s| s.as_str())
                .unwrap_or("");
            let shallow = ctx.get_param::<String>("shallow")
                .map(|s| s == "true")
                .unwrap_or(false);
            let rev = ctx.get_param::<String>("rev")
                .map(|s| s.as_str())
                .unwrap_or("HEAD");

            // If dest is empty, derive from src
            let dest = if dest.is_empty() {
                src.rsplit('/').next().unwrap_or(".")
            } else {
                dest
            };

            echo(&format!("Cloning repo {} to {}", src, dest), true, false, None);
            if shallow {
                echo("Making shallow checkout", true, false, None);
            }
            echo(&format!("Checking out revision {}", rev), true, false, None);

            Ok(())
        }))
        .build();

    // Delete subcommand
    let delete_cmd = Command::new("delete")
        .help("Deletes a repository.\n\nThis will throw away the current repository.")
        .option(
            ClickOption::new(&["--yes", "-y"])
                .help("Confirm deletion without prompting")
                .flag("true")
                .build(),
        )
        .callback(pass_repo().decorate(|repo: &Repo, ctx| {
            let confirmed = ctx
                .get_param::<String>("yes")
                .map(|s| s == "true")
                .unwrap_or(false);

            if !confirmed {
                // Simple confirmation prompt
                print!("Do you want to continue? [y/N]: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let input = input.trim().to_lowercase();
                if input != "y" && input != "yes" {
                    echo("Aborted!", true, false, None);
                    return Ok(());
                }
            }

            echo(&format!("Destroying repo {}", repo.home), true, false, None);
            echo("Deleted!", true, false, None);
            Ok(())
        }))
        .build();

    // Setuser subcommand
    let setuser_cmd = Command::new("setuser")
        .help("Sets the user credentials.\n\nThis will override the current user config.")
        .option(
            ClickOption::new(&["--username"])
                .help("The developer's shown username.")
                .required()
                .build(),
        )
        .option(
            ClickOption::new(&["--email"])
                .help("The developer's email address")
                .required()
                .build(),
        )
        .option(
            ClickOption::new(&["--password"])
                .help("The login password.")
                .required()
                .build(),
        )
        .callback(pass_repo().decorate(|repo: &Repo, ctx| {
            let username = ctx
                .get_param::<String>("username")
                .ok_or_else(|| ClickError::missing_option("--username"))?;
            let email = ctx
                .get_param::<String>("email")
                .ok_or_else(|| ClickError::missing_option("--email"))?;
            let password = ctx
                .get_param::<String>("password")
                .ok_or_else(|| ClickError::missing_option("--password"))?;

            // In a real app, we would modify the repo object
            // For now, just echo the changes
            if repo.verbose {
                eprintln!("  config[username] = {}", username);
                eprintln!("  config[email] = {}", email);
                eprintln!("  config[password] = {}", "*".repeat(password.len()));
            }

            echo("Changed credentials.", true, false, None);
            Ok(())
        }))
        .build();

    // Commit subcommand
    let commit_cmd = Command::new("commit")
        .help("Commits outstanding changes.\n\nCommit changes to the given files into the repository. You will need to\n\"repo push\" to push up your changes to other repositories.\n\nIf a list of files is omitted, all changes reported by \"repo status\"\nwill be committed.")
        .option(
            ClickOption::new(&["--message", "-m"])
                .help("The commit message. If provided multiple times each argument gets converted into a new line.")
                .multiple()
                .build(),
        )
        .argument(
            Argument::new("files")
                .help("Files to commit")
                .nargs(click::Nargs::Variadic)
                .build(),
        )
        .callback(pass_repo().decorate(|_repo: &Repo, ctx| {
            let message = ctx.get_param::<Vec<String>>("message")
                .cloned()
                .unwrap_or_default();
            let files = ctx.get_param::<Vec<String>>("files")
                .cloned()
                .unwrap_or_default();

            let msg = if message.is_empty() {
                // In the Python version, this would open an editor
                // For simplicity, we'll just use a default message
                echo("(No message provided, would normally open editor)", true, false, None);
                "Default commit message".to_string()
            } else {
                message.join("\n")
            };

            let files_str = if files.is_empty() {
                "(all changed files)".to_string()
            } else {
                format!("{:?}", files)
            };

            echo(&format!("Files to be committed: {}", files_str), true, false, None);
            echo(&format!("Commit message:\n{}", msg), true, false, None);
            Ok(())
        }))
        .build();

    // Copy subcommand
    let copy_cmd = Command::new("copy")
        .short_help("Copies files.")
        .help("Copies one or multiple files to a new location. This copies all\nfiles from SRC to DST.")
        .option(
            ClickOption::new(&["--force"])
                .help("forcibly copy over an existing managed file")
                .flag("true")
                .build(),
        )
        .argument(
            Argument::new("src")
                .help("Source file(s)")
                .nargs(click::Nargs::Variadic)
                .build(),
        )
        .argument(
            Argument::new("dst")
                .help("Destination path")
                .build(),
        )
        .callback(pass_repo().decorate(|repo: &Repo, ctx| {
            let src = ctx.get_param::<Vec<String>>("src")
                .cloned()
                .unwrap_or_default();
            let dst = ctx.get_param::<String>("dst")
                .ok_or_else(|| ClickError::missing_argument("DST"))?;
            let force = ctx.get_param::<String>("force")
                .map(|s| s == "true")
                .unwrap_or(false);

            if force && repo.verbose {
                eprintln!("  (force mode enabled)");
            }

            for file in &src {
                echo(&format!("Copy from {} -> {}", file, dst), true, false, None);
            }
            Ok(())
        }))
        .build();

    // Build the main CLI group
    let cli = Group::new("repo")
        .help("Repo is a command line tool that showcases how to build complex\ncommand line interfaces with Click.\n\nThis tool is supposed to look like a distributed version control\nsystem to show how something like this can be structured.")
        .option(
            ClickOption::new(&["--repo-home"])
                .help("Changes the repository folder location.")
                .envvar("REPO_HOME")
                .default(".repo")
                .build(),
        )
        .option(
            ClickOption::new(&["--config"])
                .help("Overrides a config key/value pair.")
                .multiple()
                .nargs(click::Nargs::Count(2))
                .build(),
        )
        .option(
            ClickOption::new(&["--verbose", "-v"])
                .help("Enables verbose mode.")
                .flag("true")
                .build(),
        )
        .option(
            ClickOption::new(&["--version"])
                .help("Show the version and exit.")
                .flag("true")
                .eager()
                .metavar("__click_version__:repo, version 1.0")
                .build(),
        )
        .command(clone_cmd)
        .command(delete_cmd)
        .command(setuser_cmd)
        .command(commit_cmd)
        .command(copy_cmd)
        .build();

    // Get CLI args
    let args: Vec<String> = env::args().skip(1).collect();

    // Pre-process to extract repo settings for the Repo object
    let mut repo_home = env::var("REPO_HOME").unwrap_or_else(|_| ".repo".to_string());
    let mut verbose = false;
    let mut config_pairs: Vec<(String, String)> = Vec::new();

    // Simple pre-parsing for --repo-home, --verbose, --config
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--repo-home" {
            if let Some(val) = args.get(i + 1) {
                repo_home = val.clone();
                i += 1;
            }
        } else if args[i].starts_with("--repo-home=") {
            repo_home = args[i].trim_start_matches("--repo-home=").to_string();
        } else if args[i] == "-v" || args[i] == "--verbose" {
            verbose = true;
        } else if args[i] == "--config" {
            if let (Some(key), Some(val)) = (args.get(i + 1), args.get(i + 2)) {
                config_pairs.push((key.clone(), val.clone()));
                i += 2;
            }
        }
        i += 1;
    }

    // Create the Repo object
    let repo_path = std::path::Path::new(&repo_home)
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| repo_home.clone());
    let mut repo = Repo::new(&repo_path);
    repo.verbose = verbose;
    for (key, val) in config_pairs {
        repo.set_config(&key, &val);
    }

    // Run the CLI with the Repo object in context
    let result = run_with_repo(cli, args, repo);

    if let Err(e) = result {
        // Handle clean exit (e.g., --version)
        if let ClickError::Exit { code: 0 } = e {
            return;
        }
        eprintln!("{}", e.format_full());
        std::process::exit(e.exit_code());
    }
}

/// Run the CLI with a Repo object stored in the context.
fn run_with_repo(cli: Group, args: Vec<String>, repo: Repo) -> Result<()> {
    let prog_name = cli.name().unwrap_or("repo").to_string();

    // Create context with repo as the obj
    let ctx = cli.make_context(&prog_name, args, None)?;
    let mut ctx = ctx;

    // Store the Repo object in context
    ctx.set_obj(repo);

    let ctx = Arc::new(ctx);
    click::push_context(Arc::clone(&ctx));

    let result = cli.invoke(&ctx);

    click::pop_context();
    ctx.close();

    result
}
