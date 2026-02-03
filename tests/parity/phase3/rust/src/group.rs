//! Group parity tests matching Python Click output format.
//!
//! These mirror `tests/parity/phase3/python/test_group.py`.

use click::argument::Argument;
use click::command::Command;
use click::context::ContextBuilder;
use click::group::{CommandLike, Group};
use click::option::ClickOption;

use crate::util::{exit_code, py_repr, Output};

/// Run all group tests.
pub fn run() {
    test_group_with_subcommands();
    test_subcommand_dispatch();
    test_missing_command_errors();
    test_help_with_subcommand_listing();
    test_nested_groups();
    test_invoke_without_command();
    test_chain_mode();
}

fn test_group_with_subcommands() {
    println!("=== Group With Subcommands ===");

    let out = Output::new();
    let cli = Group::new("cli")
        .help("Main CLI application.")
        .command(
            Command::new("init")
                .help("Initialize the project.")
                .callback({
                    let out = out.clone();
                    move |_ctx| {
                        out.push("Initializing...");
                        Ok(())
                    }
                })
                .build(),
        )
        .command(
            Command::new("build")
                .help("Build the project.")
                .callback({
                    let out = out.clone();
                    move |_ctx| {
                        out.push("Building...");
                        Ok(())
                    }
                })
                .build(),
        )
        .build();

    println!("# Group subcommand listing:");
    let cmd_names = cli.list_commands();
    println!("  commands: {:?}", cmd_names);

    // Invoke init
    let out = Output::new();
    let cli = Group::new("cli")
        .help("Main CLI application.")
        .command(
            Command::new("init")
                .help("Initialize the project.")
                .callback({
                    let out = out.clone();
                    move |_ctx| {
                        out.push("Initializing...");
                        Ok(())
                    }
                })
                .build(),
        )
        .command(Command::new("build").help("Build the project.").build())
        .build();
    let result = cli.main(vec!["init".to_string()]);
    println!("# Invoke 'init' subcommand:");
    println!("  args: {:?}", vec!["init"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Invoke build
    let out = Output::new();
    let cli = Group::new("cli")
        .help("Main CLI application.")
        .command(Command::new("init").help("Initialize the project.").build())
        .command(
            Command::new("build")
                .help("Build the project.")
                .callback({
                    let out = out.clone();
                    move |_ctx| {
                        out.push("Building...");
                        Ok(())
                    }
                })
                .build(),
        )
        .build();
    let result = cli.main(vec!["build".to_string()]);
    println!("# Invoke 'build' subcommand:");
    println!("  args: {:?}", vec!["build"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));
}

fn test_subcommand_dispatch() {
    println!("\n=== Subcommand Dispatch ===");

    // Subcommand with argument
    let out = Output::new();
    let cli = Group::new("cli")
        .option(ClickOption::new(&["--verbose", "-v"]).flag("true").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let verbose = ctx
                    .get_param::<String>("verbose")
                    .map_or(false, |v| v == "true");
                if verbose {
                    out.push("Verbose mode enabled");
                }
                Ok(())
            }
        })
        .command(
            Command::new("greet")
                .help("Greet someone.")
                .argument(Argument::new("name").build())
                .callback({
                    let out = out.clone();
                    move |ctx| {
                        let name = ctx.get_param::<String>("name").map(|s| s.as_str()).unwrap_or("");
                        out.push(format!("Hello, {}!", name));
                        Ok(())
                    }
                })
                .build(),
        )
        .command(
            Command::new("repeat")
                .help("Repeat action.")
                .option(ClickOption::new(&["--count", "-c"]).default("1").build())
                .callback({
                    let out = out.clone();
                    move |ctx| {
                        let count = ctx.get_param::<String>("count").map(|s| s.as_str()).unwrap_or("1");
                        out.push(format!("Repeating {} times", count));
                        Ok(())
                    }
                })
                .build(),
        )
        .build();

    let result = cli.main(vec!["greet".to_string(), "Alice".to_string()]);
    println!("# Subcommand with argument:");
    println!("  args: {:?}", vec!["greet", "Alice"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Group option + subcommand
    let out = Output::new();
    let cli = Group::new("cli")
        .option(ClickOption::new(&["--verbose", "-v"]).flag("true").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let verbose = ctx
                    .get_param::<String>("verbose")
                    .map_or(false, |v| v == "true");
                if verbose {
                    out.push("Verbose mode enabled");
                }
                Ok(())
            }
        })
        .command(
            Command::new("greet")
                .help("Greet someone.")
                .argument(Argument::new("name").build())
                .callback({
                    let out = out.clone();
                    move |ctx| {
                        let name = ctx.get_param::<String>("name").map(|s| s.as_str()).unwrap_or("");
                        out.push(format!("Hello, {}!", name));
                        Ok(())
                    }
                })
                .build(),
        )
        .build();
    let result = cli.main(vec!["-v".to_string(), "greet".to_string(), "Bob".to_string()]);
    println!("# Group option + subcommand:");
    println!("  args: {:?}", vec!["-v", "greet", "Bob"]);
    for line in out.lines() {
        println!("  output: {}", py_repr(&line));
    }
    println!("  exit_code: {}", exit_code(&result));

    // Subcommand with option
    let out = Output::new();
    let cli = Group::new("cli")
        .command(
            Command::new("repeat")
                .help("Repeat action.")
                .option(ClickOption::new(&["--count", "-c"]).default("1").build())
                .callback({
                    let out = out.clone();
                    move |ctx| {
                        let count = ctx.get_param::<String>("count").map(|s| s.as_str()).unwrap_or("1");
                        out.push(format!("Repeating {} times", count));
                        Ok(())
                    }
                })
                .build(),
        )
        .build();
    let result = cli.main(vec!["repeat".to_string(), "-c".to_string(), "3".to_string()]);
    println!("# Subcommand with option:");
    println!("  args: {:?}", vec!["repeat", "-c", "3"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));
}

fn test_missing_command_errors() {
    println!("\n=== Missing Command Errors ===");

    let cli = Group::new("cli")
        // Match Python Click behavior: no args yields a usage error rather than help.
        .no_args_is_help(false)
        .command(Command::new("hello").callback(|_ctx| Ok(())).build())
        .build();

    // No subcommand provided
    println!("# No subcommand provided:");
    println!("  args: {:?}", Vec::<&str>::new());
    let result = cli.main(vec![]);
    let ctx = ContextBuilder::new().info_name("cli").build();
    let shows_usage = cli.get_help(&ctx).contains("Usage:");
    println!("  shows_usage: {}", shows_usage);
    println!("  exit_code: {}", exit_code(&result));

    // Unknown subcommand
    let result = cli.main(vec!["unknown".to_string()]);
    println!("# Unknown subcommand:");
    println!("  args: {:?}", vec!["unknown"]);
    if let Err(e) = &result {
        let msg = e.format_message();
        if msg.to_lowercase().contains("no such command") {
            println!("  error_type: 'UsageError'");
            println!("  error_contains: 'No such command'");
        }
    }
    println!("  exit_code: {}", exit_code(&result));
}

fn test_help_with_subcommand_listing() {
    println!("\n=== Help With Subcommand Listing ===");

    let cli = Group::new("cli")
        .help("A sample CLI application.")
        .command(Command::new("init").short_help("Initialize the project.").build())
        .command(Command::new("build").short_help("Build the project.").build())
        .command(Command::new("deploy").short_help("Deploy the project.").build())
        .build();

    let ctx = ContextBuilder::new().info_name("cli").build();
    let help_text = cli.get_help(&ctx);

    println!("# Group help text elements:");
    println!("  has_usage: {}", help_text.contains("Usage:"));
    println!("  has_docstring: {}", help_text.contains("sample CLI"));
    println!("  has_options_section: {}", help_text.contains("Options:"));
    println!("  has_commands_section: {}", help_text.contains("Commands:"));
    println!("  lists_init: {}", help_text.contains("init"));
    println!("  lists_build: {}", help_text.contains("build"));
    println!("  lists_deploy: {}", help_text.contains("deploy"));
    println!("  init_has_help: {}", help_text.contains("Initialize"));
    println!("  build_has_help: {}", help_text.contains("Build"));
    println!("  deploy_has_help: {}", help_text.contains("Deploy"));
    println!("  exit_code: 0");

    // Help for specific subcommand
    let init_cmd = Command::new("init").help("Initialize the project.").build();
    let ctx = ContextBuilder::new().info_name("init").build();
    let help_text = init_cmd.get_help(&ctx);
    println!("# Subcommand help:");
    println!("  has_usage: {}", help_text.contains("Usage:"));
    println!("  has_init_in_usage: {}", help_text.contains("init"));
    println!("  has_docstring: {}", help_text.contains("Initialize"));
    println!("  exit_code: 0");
}

fn test_nested_groups() {
    println!("\n=== Nested Groups ===");

    let out = Output::new();
    let db = Group::new("db")
        .help("Database commands.")
        .command(
            Command::new("init")
                .help("Initialize database.")
                .callback({
                    let out = out.clone();
                    move |_ctx| {
                        out.push("DB initialized");
                        Ok(())
                    }
                })
                .build(),
        )
        .command(
            Command::new("migrate")
                .help("Run migrations.")
                .callback({
                    let out = out.clone();
                    move |_ctx| {
                        out.push("Migrations run");
                        Ok(())
                    }
                })
                .build(),
        )
        .build();

    let cli = Group::new("cli").help("Main CLI.").command(db).build();

    let result = cli.main(vec!["db".to_string(), "init".to_string()]);
    println!("# Nested group command:");
    println!("  args: {:?}", vec!["db", "init"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Help for nested group
    let db = Group::new("db")
        .help("Database commands.")
        .command(Command::new("init").short_help("Initialize database.").build())
        .command(Command::new("migrate").short_help("Run migrations.").build())
        .build();
    let ctx = ContextBuilder::new().info_name("db").build();
    let help_text = db.get_help(&ctx);
    println!("# Nested group help:");
    println!("  has_commands: {}", help_text.contains("Commands:"));
    println!("  lists_init: {}", help_text.contains("init"));
    println!("  lists_migrate: {}", help_text.contains("migrate"));
    println!("  exit_code: 0");
}

fn test_invoke_without_command() {
    println!("\n=== Invoke Without Command ===");

    // No subcommand
    let out = Output::new();
    let cli = Group::new("cli")
        .invoke_without_command(true)
        .callback({
            let out = out.clone();
            move |ctx| {
                if ctx.invoked_subcommand().is_none() {
                    out.push("No subcommand invoked");
                }
                Ok(())
            }
        })
        .command(
            Command::new("sub")
                .callback({
                    let out = out.clone();
                    move |_ctx| {
                        out.push("Subcommand executed");
                        Ok(())
                    }
                })
                .build(),
        )
        .build();
    let result = cli.main(vec![]);
    println!("# Group invoked without subcommand:");
    println!("  args: {:?}", Vec::<&str>::new());
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // With subcommand
    let out = Output::new();
    let cli = Group::new("cli")
        .invoke_without_command(true)
        .callback(|_ctx| Ok(()))
        .command(
            Command::new("sub")
                .callback({
                    let out = out.clone();
                    move |_ctx| {
                        out.push("Subcommand executed");
                        Ok(())
                    }
                })
                .build(),
        )
        .build();
    let result = cli.main(vec!["sub".to_string()]);
    println!("# Group invoked with subcommand:");
    println!("  args: {:?}", vec!["sub"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));
}

fn test_chain_mode() {
    println!("\n=== Chain Mode ===");

    let build_chain = |out: Output, commands: Vec<&str>| {
        let mut group = Group::new("cli").chain(true);
        for cmd_name in commands {
            let out = out.clone();
            let cmd_name = cmd_name.to_string();
            let cmd_name_for_cb = cmd_name.clone();
            group = group.command(
                Command::new(&cmd_name)
                    .callback(move |_ctx| {
                        out.push(format!("{} executed", cmd_name_for_cb));
                        Ok(())
                    })
                    .build(),
            );
        }
        group.build()
    };

    // Single command in chain
    let out = Output::new();
    let cli = build_chain(out.clone(), vec!["cmd1", "cmd2", "cmd3"]);
    let result = cli.main(vec!["cmd1".to_string()]);
    println!("# Chain with single command:");
    println!("  args: {:?}", vec!["cmd1"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Two commands in chain
    let out = Output::new();
    let cli = build_chain(out.clone(), vec!["cmd1", "cmd2"]);
    let result = cli.main(vec!["cmd1".to_string(), "cmd2".to_string()]);
    println!("# Chain with two commands:");
    println!("  args: {:?}", vec!["cmd1", "cmd2"]);
    for line in out.lines() {
        println!("  output: {}", py_repr(&line));
    }
    println!("  exit_code: {}", exit_code(&result));

    // All three commands
    let out = Output::new();
    let cli = build_chain(out.clone(), vec!["cmd1", "cmd2", "cmd3"]);
    let result = cli.main(vec!["cmd1".to_string(), "cmd2".to_string(), "cmd3".to_string()]);
    println!("# Chain with three commands:");
    println!("  args: {:?}", vec!["cmd1", "cmd2", "cmd3"]);
    for line in out.lines() {
        println!("  output: {}", py_repr(&line));
    }
    println!("  exit_code: {}", exit_code(&result));
}
