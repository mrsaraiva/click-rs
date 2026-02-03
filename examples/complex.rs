//! Complex CLI example - demonstrating plugin-style command loading.
//!
//! This example demonstrates a complex CLI application with:
//! - Auto environment variable prefix (COMPLEX_)
//! - Custom context object passed to subcommands
//! - Plugin-style commands (init, status)
//!
//! Equivalent to Python Click's examples/complex/complex/ (consolidated into single file)
//!
//! Note: The Python version dynamically loads commands from the filesystem.
//! This Rust version statically registers commands for simplicity.

use std::env;
use std::io::{self, Write};
use click::{
    Argument, ClickOption, Command, Context, Group, PathType,
    Result, group::CommandLike,
};

// =============================================================================
// Environment - shared state passed between commands
// =============================================================================

/// Shared application state passed to subcommands via context.
#[derive(Debug, Clone)]
struct Environment {
    /// Enable verbose output.
    verbose: bool,
    /// The home directory for operations.
    home: String,
}

impl Environment {
    fn new() -> Self {
        Self {
            verbose: false,
            home: env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        }
    }

    /// Log a message to stderr.
    fn log(&self, msg: &str) {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(handle, "{}", msg);
    }

    /// Log a message only if verbose mode is enabled.
    fn vlog(&self, msg: &str) {
        if self.verbose {
            self.log(msg);
        }
    }
}

// =============================================================================
// CLI Builder
// =============================================================================

/// Build the main CLI group.
fn build_cli() -> Group {
    Group::new("complex")
        .help("A complex command line interface.")
        .invoke_without_command(true)
        .option(
            ClickOption::new(&["--home"])
                .type_any(PathType::new().exists(true).file_okay(false).resolve_path(true))
                .help("Changes the folder to operate on.")
                .envvar("COMPLEX_HOME")
                .build(),
        )
        .option(
            ClickOption::new(&["-v", "--verbose"])
                .flag("true")
                .help("Enables verbose mode.")
                .envvar("COMPLEX_VERBOSE")
                .build(),
        )
        .callback(cli_callback)
        .command(build_init_command())
        .command(build_status_command())
        .build()
}

fn cli_callback(ctx: &Context) -> Result<()> {
    // Create environment with parsed options
    let mut env = Environment::new();

    if let Some(verbose) = ctx.get_param::<String>("verbose") {
        env.verbose = verbose == "true";
    }

    if let Some(home) = ctx.get_param::<String>("home") {
        env.home = home.clone();
    }

    // In Python Click, we'd store this via pass_context decorator.
    // Here we just log that we're ready if verbose.
    env.vlog(&format!("Complex CLI initialized with home: {}", env.home));

    Ok(())
}

// =============================================================================
// init command
// =============================================================================

fn build_init_command() -> Command {
    Command::new("init")
        .short_help("Initializes a repo.")
        .help("Initializes a repository.")
        .argument(
            Argument::new("path")
                .default(".")
                .help("Path to initialize (defaults to current directory)")
                .build(),
        )
        .callback(init_callback)
        .build()
}

fn init_callback(ctx: &Context) -> Result<()> {
    // Get environment from parent context or create new one
    let env = get_environment_from_context(ctx);

    let path = ctx
        .get_param::<String>("path")
        .cloned()
        .unwrap_or_else(|| env.home.clone());

    env.log(&format!("Initialized the repository in {}", path));
    Ok(())
}

// =============================================================================
// status command
// =============================================================================

fn build_status_command() -> Command {
    Command::new("status")
        .short_help("Shows file changes.")
        .help("Shows file changes in the current working directory.")
        .callback(status_callback)
        .build()
}

fn status_callback(ctx: &Context) -> Result<()> {
    let env = get_environment_from_context(ctx);

    env.log("Changed files: none");
    env.vlog("bla bla bla, debug info");

    Ok(())
}

// =============================================================================
// Helper functions
// =============================================================================

/// Extract Environment from context chain.
///
/// Since we can't easily store arbitrary objects in Context in the current
/// implementation, we reconstruct the Environment from context parameters.
fn get_environment_from_context(ctx: &Context) -> Environment {
    let mut env = Environment::new();

    // Check current context
    if let Some(verbose) = ctx.get_param::<String>("verbose") {
        env.verbose = verbose == "true";
    }
    if let Some(home) = ctx.get_param::<String>("home") {
        env.home = home.clone();
    }

    // Check parent context
    if let Some(parent) = ctx.parent() {
        if let Some(verbose) = parent.get_param::<String>("verbose") {
            env.verbose = verbose == "true";
        }
        if let Some(home) = parent.get_param::<String>("home") {
            env.home = home.clone();
        }
    }

    // Check environment variables as fallback
    if let Ok(home) = env::var("COMPLEX_HOME") {
        if env.home == "." || env.home == env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default() {
            env.home = home;
        }
    }
    if let Ok(verbose) = env::var("COMPLEX_VERBOSE") {
        if verbose == "1" || verbose.to_lowercase() == "true" {
            env.verbose = true;
        }
    }

    env
}

fn main() {
    let cli = build_cli();
    let args: Vec<String> = env::args().skip(1).collect();

    if let Err(e) = cli.main(args) {
        eprintln!("{}", e.format_full());
        std::process::exit(e.exit_code());
    }
}
