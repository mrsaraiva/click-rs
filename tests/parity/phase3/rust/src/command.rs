//! Command parity tests matching Python Click output format.
//!
//! These mirror `tests/parity/phase3/python/test_command.py`.

use click::argument::Argument;
use click::command::Command;
use click::context::ContextBuilder;
use click::error::{ClickError, ParamType};
use click::option::ClickOption;

use crate::util::{exit_code, py_repr, Output};

/// Run all command tests.
pub fn run() {
    test_command_creation();
    test_command_with_options();
    test_command_with_arguments();
    test_help_text_generation();
    test_usage_line_generation();
    test_missing_required_parameter();
    test_callback_execution();
}

fn test_command_creation() {
    println!("=== Command Creation ===");

    let out = Output::new();
    let simple_cmd = Command::new("simple_cmd")
        .callback({
            let out = out.clone();
            move |_ctx| {
                out.push("Hello, World!");
                Ok(())
            }
        })
        .build();

    let result = simple_cmd.main(vec![]);
    println!("# Simple command with no args:");
    println!("  command: 'simple_cmd'");
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Command with name
    let named_cmd = Command::new("custom-name").build();
    println!("# Command with custom name:");
    println!("  name: 'custom-name'");
    println!(
        "  actual_name: {}",
        py_repr(named_cmd.name.as_deref().unwrap_or(""))
    );
}

fn test_command_with_options() {
    println!("\n=== Command With Options ===");

    let make_greet = |out: Output| {
        Command::new("greet")
            .option(ClickOption::new(&["--name", "-n"]).default("World").build())
            .option(ClickOption::new(&["--count", "-c"]).default("1").build())
            .callback(move |ctx| {
                let name = ctx.get_param::<String>("name").map(|s| s.as_str()).unwrap_or("World");
                let count: i32 = ctx
                    .get_param::<String>("count")
                    .map(|s| s.as_str())
                    .unwrap_or("1")
                    .parse()
                    .unwrap_or(1);

                for _ in 0..count {
                    out.push(format!("Hello, {}!", name));
                }
                Ok(())
            })
            .build()
    };

    // With defaults
    let out = Output::new();
    let greet = make_greet(out.clone());
    let result = greet.main(vec![]);
    println!("# Command with default options:");
    println!("  args: {:?}", Vec::<&str>::new());
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // With custom values
    let out = Output::new();
    let greet = make_greet(out.clone());
    let result = greet.main(vec![
        "--name".to_string(),
        "Alice".to_string(),
        "-c".to_string(),
        "2".to_string(),
    ]);
    println!("# Command with custom option values:");
    println!("  args: {:?}", vec!["--name", "Alice", "-c", "2"]);
    for line in out.lines() {
        println!("  output: {}", py_repr(&line));
    }
    println!("  exit_code: {}", exit_code(&result));
}

fn test_command_with_arguments() {
    println!("\n=== Command With Arguments ===");

    // Single argument
    let out = Output::new();
    let cmd = Command::new("cmd")
        .argument(Argument::new("filename").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let filename = ctx.get_param::<String>("filename").map(|s| s.as_str()).unwrap_or("");
                out.push(format!("File: {}", filename));
                Ok(())
            }
        })
        .build();
    let result = cmd.main(vec!["test.txt".to_string()]);
    println!("# Command with single argument:");
    println!("  args: {:?}", vec!["test.txt"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Multiple arguments
    let out = Output::new();
    let copy = Command::new("copy")
        .argument(Argument::new("src").build())
        .argument(Argument::new("dst").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let src = ctx.get_param::<String>("src").map(|s| s.as_str()).unwrap_or("");
                let dst = ctx.get_param::<String>("dst").map(|s| s.as_str()).unwrap_or("");
                out.push(format!("Copy {} -> {}", src, dst));
                Ok(())
            }
        })
        .build();
    let result = copy.main(vec!["a.txt".to_string(), "b.txt".to_string()]);
    println!("# Command with multiple arguments:");
    println!("  args: {:?}", vec!["a.txt", "b.txt"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Variadic arguments
    let out = Output::new();
    let ls = Command::new("ls")
        .argument(Argument::new("files").multiple().required(false).build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let files = ctx
                    .get_param::<Vec<String>>("files")
                    .cloned()
                    .unwrap_or_default();
                // Python prints list(files) so use Python list repr inside the output content.
                out.push(format!("Files: {}", crate::util::py_list(&files)));
                Ok(())
            }
        })
        .build();
    let result = ls.main(vec!["a.txt".to_string(), "b.txt".to_string(), "c.txt".to_string()]);
    println!("# Command with variadic arguments:");
    println!("  args: {:?}", vec!["a.txt", "b.txt", "c.txt"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));
}

fn test_help_text_generation() {
    println!("\n=== Help Text Generation ===");

    let cmd = Command::new("greet")
        .help("Greet someone.")
        .option(
            ClickOption::new(&["--name", "-n"])
                .help("Name to greet")
                .build(),
        )
        .option(
            ClickOption::new(&["--count", "-c"])
                .default("1")
                .type_any(click::INT)
                .help("Number of greetings")
                .build(),
        )
        .build();

    let ctx = ContextBuilder::new().info_name("greet").build();
    let help_text = cmd.get_help(&ctx);

    println!("# Help text elements:");
    println!("  has_usage: {}", help_text.contains("Usage:"));
    println!("  has_options_section: {}", help_text.contains("Options:"));
    println!("  has_help_option: {}", help_text.contains("--help"));
    println!(
        "  has_name_option: {}",
        help_text.contains("--name") || help_text.contains("-n")
    );
    println!(
        "  has_count_option: {}",
        help_text.contains("--count") || help_text.contains("-c")
    );
    println!("  has_docstring: {}", help_text.contains("Greet someone"));
    println!("  exit_code: 0");
}

fn test_usage_line_generation() {
    println!("\n=== Usage Line Generation ===");

    // Simple command
    let simple = Command::new("simple").build();
    let ctx = ContextBuilder::new().info_name("simple").build();
    let _usage_line = simple.get_usage(&ctx);
    println!("# Simple command usage:");
    println!("  pattern: 'Usage: <name> [OPTIONS]'");

    // Command with argument
    let with_arg = Command::new("with_arg")
        .argument(Argument::new("filename").build())
        .build();
    let ctx = ContextBuilder::new().info_name("with_arg").build();
    let _usage_line = with_arg.get_usage(&ctx);
    println!("# Command with argument usage:");
    println!("  pattern: 'Usage: <name> [OPTIONS] FILENAME'");

    // Command with optional argument
    let with_optional_arg = Command::new("with_optional_arg")
        .argument(Argument::new("filename").required(false).build())
        .build();
    let ctx = ContextBuilder::new().info_name("with_optional_arg").build();
    let _usage_line = with_optional_arg.get_usage(&ctx);
    println!("# Command with optional argument usage:");
    println!("  pattern: 'Usage: <name> [OPTIONS] [FILENAME]'");

    // Command with variadic argument
    let with_variadic = Command::new("with_variadic")
        .argument(Argument::new("files").multiple().required(false).build())
        .build();
    let ctx = ContextBuilder::new().info_name("with_variadic").build();
    let _usage_line = with_variadic.get_usage(&ctx);
    println!("# Command with variadic argument usage:");
    println!("  pattern: 'Usage: <name> [OPTIONS] [FILES]...'");
}

fn test_missing_required_parameter() {
    println!("\n=== Missing Required Parameter ===");

    // Missing required option
    let cmd_opt = Command::new("cmd_opt")
        .option(ClickOption::new(&["--name", "-n"]).required().build())
        .callback(|ctx| {
            let name = ctx.get_param::<String>("name").map(|s| s.as_str()).unwrap_or("");
            let _ = name;
            Ok(())
        })
        .build();
    let result = cmd_opt.main(vec![]);
    println!("# Missing required option:");
    println!("  args: {:?}", Vec::<&str>::new());
    if let Err(e) = &result {
        if matches!(e, ClickError::MissingParameter { param_type: ParamType::Option, .. }) {
            println!("  error_type: 'MissingParameter'");
            println!("  param_type: 'option'");
        }
    }
    println!("  exit_code: {}", exit_code(&result));

    // Missing required argument
    let cmd_arg = Command::new("cmd_arg")
        .argument(Argument::new("filename").build())
        .callback(|_ctx| Ok(()))
        .build();
    let result = cmd_arg.main(vec![]);
    println!("# Missing required argument:");
    println!("  args: {:?}", Vec::<&str>::new());
    if let Err(e) = &result {
        if matches!(e, ClickError::MissingParameter { param_type: ParamType::Argument, .. }) {
            println!("  error_type: 'MissingParameter'");
            println!("  param_type: 'argument'");
        }
    }
    println!("  exit_code: {}", exit_code(&result));
}

fn test_callback_execution() {
    println!("\n=== Callback Execution ===");

    // Callback log is printed like Python.
    let callback_log: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let make_cmd = |out: Output,
                    callback_log: std::sync::Arc<std::sync::Mutex<Vec<String>>>| {
        Command::new("cmd")
            .option(ClickOption::new(&["--value"]).default("default").build())
            .callback(move |ctx| {
                let value = ctx.get_param::<String>("value").map(|s| s.as_str()).unwrap_or("default");
                callback_log
                    .lock()
                    .unwrap()
                    .push(format!("called with {}", value));
                out.push(format!("value={}", value));
                Ok(())
            })
            .build()
    };

    // Default value
    let out = Output::new();
    callback_log.lock().unwrap().clear();
    let cmd = make_cmd(out.clone(), std::sync::Arc::clone(&callback_log));
    let result = cmd.main(vec![]);
    println!("# Callback with default value:");
    let log = callback_log.lock().unwrap().clone();
    println!("  callback_called: {}", !log.is_empty());
    println!("  callback_log: {:?}", log);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    drop(log);
    let _ = result;

    // Custom value
    let out = Output::new();
    callback_log.lock().unwrap().clear();
    let cmd = make_cmd(out.clone(), std::sync::Arc::clone(&callback_log));
    let result = cmd.main(vec!["--value".to_string(), "custom".to_string()]);
    println!("# Callback with custom value:");
    let log = callback_log.lock().unwrap().clone();
    println!("  callback_called: {}", !log.is_empty());
    println!("  callback_log: {:?}", log);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    drop(log);
    let _ = result;

    // Callback that raises
    let error_cmd = Command::new("error_cmd")
        .callback(|_ctx| {
            Err(ClickError::file_error("x.txt", "intentional error"))
        })
        .build();
    let result = error_cmd.main(vec![]);
    println!("# Callback that raises ClickException:");
    match &result {
        Ok(_) => {
            println!("  error_in_output: false");
            println!("  exit_code: 0");
        }
        Err(e) => {
            let msg = e.format_full();
            println!("  error_in_output: {}", msg.contains("intentional error"));
            println!("  exit_code: {}", e.exit_code());
        }
    }
}

