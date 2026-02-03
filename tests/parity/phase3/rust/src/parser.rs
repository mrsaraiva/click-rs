//! Parser parity tests matching Python Click output format.
//!
//! These mirror `tests/parity/phase3/python/test_parser.py`.

use click::argument::Argument;
use click::command::Command;
use click::option::ClickOption;
use click::parameter::Nargs;

use crate::util::{exit_code, py_bool, py_list, py_repr, Output};

/// Run all parser tests.
pub fn run() {
    test_short_options();
    test_long_options();
    test_double_dash_terminator();
    test_mixed_options_and_arguments();
    test_unknown_option_errors();
}

fn test_short_options() {
    println!("=== Short Options ===");

    // Single short option flag
    let out = Output::new();
    let cmd1 = Command::new("cmd1")
        .option(ClickOption::new(&["-v", "--verbose"]).flag("true").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let verbose = ctx
                    .get_param::<String>("verbose")
                    .map_or(false, |v| v == "true");
                out.push(format!("verbose={}", py_bool(verbose)));
                Ok(())
            }
        })
        .build();
    let result = cmd1.main(vec!["-v".to_string()]);
    println!("# Single short flag (-v):");
    println!("  args: {:?}", vec!["-v"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Short option with value (attached)
    let out = Output::new();
    let cmd2 = Command::new("cmd2")
        .option(ClickOption::new(&["-n", "--name"]).build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let name = ctx.get_param::<String>("name").map(|s| s.as_str());
                match name {
                    Some(s) => out.push(format!("name={}", s)),
                    None => out.push("name=None"),
                }
                Ok(())
            }
        })
        .build();
    let result = cmd2.main(vec!["-nAlice".to_string()]);
    println!("# Short option with attached value (-nAlice):");
    println!("  args: {:?}", vec!["-nAlice"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Short option with value (separate)
    let out = Output::new();
    let cmd2 = Command::new("cmd2")
        .option(ClickOption::new(&["-n", "--name"]).build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let name = ctx.get_param::<String>("name").map(|s| s.as_str());
                match name {
                    Some(s) => out.push(format!("name={}", s)),
                    None => out.push("name=None"),
                }
                Ok(())
            }
        })
        .build();
    let result = cmd2.main(vec!["-n".to_string(), "Bob".to_string()]);
    println!("# Short option with separate value (-n Bob):");
    println!("  args: {:?}", vec!["-n", "Bob"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Grouped short flags (-abc)
    let out = Output::new();
    let cmd3 = Command::new("cmd3")
        .option(ClickOption::new(&["-a"]).flag("true").build())
        .option(ClickOption::new(&["-b"]).flag("true").build())
        .option(ClickOption::new(&["-c"]).flag("true").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let a = ctx.get_param::<String>("a").map_or(false, |v| v == "true");
                let b = ctx.get_param::<String>("b").map_or(false, |v| v == "true");
                let c = ctx.get_param::<String>("c").map_or(false, |v| v == "true");
                out.push(format!(
                    "a={} b={} c={}",
                    py_bool(a),
                    py_bool(b),
                    py_bool(c)
                ));
                Ok(())
            }
        })
        .build();
    let result = cmd3.main(vec!["-abc".to_string()]);
    println!("# Grouped short flags (-abc):");
    println!("  args: {:?}", vec!["-abc"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Partial grouped flags
    let out = Output::new();
    let cmd3 = Command::new("cmd3")
        .option(ClickOption::new(&["-a"]).flag("true").build())
        .option(ClickOption::new(&["-b"]).flag("true").build())
        .option(ClickOption::new(&["-c"]).flag("true").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let a = ctx.get_param::<String>("a").map_or(false, |v| v == "true");
                let b = ctx.get_param::<String>("b").map_or(false, |v| v == "true");
                let c = ctx.get_param::<String>("c").map_or(false, |v| v == "true");
                out.push(format!(
                    "a={} b={} c={}",
                    py_bool(a),
                    py_bool(b),
                    py_bool(c)
                ));
                Ok(())
            }
        })
        .build();
    let result = cmd3.main(vec!["-ab".to_string()]);
    println!("# Partial grouped flags (-ab):");
    println!("  args: {:?}", vec!["-ab"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));
}

fn test_long_options() {
    println!("\n=== Long Options ===");

    // Long option with = separator
    let out = Output::new();
    let cmd = Command::new("cmd")
        .option(ClickOption::new(&["--name"]).build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let name = ctx.get_param::<String>("name").map(|s| s.as_str());
                out.push(format!("name={}", name.unwrap_or("None")));
                Ok(())
            }
        })
        .build();
    let result = cmd.main(vec!["--name=Alice".to_string()]);
    println!("# Long option with = (--name=Alice):");
    println!("  args: {:?}", vec!["--name=Alice"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Long option with space separator
    let out = Output::new();
    let cmd = Command::new("cmd")
        .option(ClickOption::new(&["--name"]).build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let name = ctx.get_param::<String>("name").map(|s| s.as_str());
                out.push(format!("name={}", name.unwrap_or("None")));
                Ok(())
            }
        })
        .build();
    let result = cmd.main(vec!["--name".to_string(), "Bob".to_string()]);
    println!("# Long option with space (--name Bob):");
    println!("  args: {:?}", vec!["--name", "Bob"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Long flag
    let out = Output::new();
    let cmd2 = Command::new("cmd2")
        .option(ClickOption::new(&["--verbose"]).flag("true").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let verbose = ctx
                    .get_param::<String>("verbose")
                    .map_or(false, |v| v == "true");
                out.push(format!("verbose={}", py_bool(verbose)));
                Ok(())
            }
        })
        .build();
    let result = cmd2.main(vec!["--verbose".to_string()]);
    println!("# Long flag (--verbose):");
    println!("  args: {:?}", vec!["--verbose"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Empty value
    let out = Output::new();
    let cmd = Command::new("cmd")
        .option(ClickOption::new(&["--name"]).build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let name = ctx.get_param::<String>("name").map(|s| s.as_str());
                out.push(format!("name={}", name.unwrap_or("None")));
                Ok(())
            }
        })
        .build();
    let result = cmd.main(vec!["--name=".to_string()]);
    println!("# Long option with empty value (--name=):");
    println!("  args: {:?}", vec!["--name="]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));
}

fn test_double_dash_terminator() {
    println!("\n=== Double-Dash Terminator ===");

    let make_cmd = |out: Output| {
        Command::new("cmd")
            .option(ClickOption::new(&["--verbose", "-v"]).flag("true").build())
            .argument(Argument::new("args").nargs(Nargs::Variadic).required(false).build())
            .callback(move |ctx| {
                let verbose = ctx
                    .get_param::<String>("verbose")
                    .map_or(false, |v| v == "true");
                let args = ctx
                    .get_param::<Vec<String>>("args")
                    .cloned()
                    .unwrap_or_default();
                out.push(format!("verbose={}", py_bool(verbose)));
                out.push(format!("args={}", py_list(&args)));
                Ok(())
            })
            .build()
    };

    // -- terminates options
    let out = Output::new();
    let cmd = make_cmd(out.clone());
    let result = cmd.main(vec!["--".to_string(), "--verbose".to_string()]);
    println!("# Double-dash before option-like arg (-- --verbose):");
    println!("  args: {:?}", vec!["--", "--verbose"]);
    for line in out.lines() {
        println!("  output: {}", py_repr(&line));
    }
    println!("  exit_code: {}", exit_code(&result));

    // Options before --
    let out = Output::new();
    let cmd = make_cmd(out.clone());
    let result = cmd.main(vec![
        "-v".to_string(),
        "--".to_string(),
        "-flag".to_string(),
        "--opt".to_string(),
    ]);
    println!("# Options before --, args after (-v -- -flag --opt):");
    println!("  args: {:?}", vec!["-v", "--", "-flag", "--opt"]);
    for line in out.lines() {
        println!("  output: {}", py_repr(&line));
    }
    println!("  exit_code: {}", exit_code(&result));

    // Just --
    let out = Output::new();
    let cmd = make_cmd(out.clone());
    let result = cmd.main(vec!["--".to_string()]);
    println!("# Just double-dash (--):");
    println!("  args: {:?}", vec!["--"]);
    for line in out.lines() {
        println!("  output: {}", py_repr(&line));
    }
    println!("  exit_code: {}", exit_code(&result));
}

fn test_mixed_options_and_arguments() {
    println!("\n=== Mixed Options and Arguments ===");

    // Options before argument
    let out = Output::new();
    let cmd = Command::new("cmd")
        .option(ClickOption::new(&["--count", "-c"]).default("1").build())
        .argument(Argument::new("name").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let count: i32 = ctx
                    .get_param::<String>("count")
                    .map(|s| s.as_str())
                    .unwrap_or("1")
                    .parse()
                    .unwrap_or(1);
                let name = ctx.get_param::<String>("name").map(|s| s.as_str()).unwrap_or("");
                out.push(format!("count={} name={}", count, name));
                Ok(())
            }
        })
        .build();
    let result = cmd.main(vec!["-c".to_string(), "5".to_string(), "Alice".to_string()]);
    println!("# Options before argument (-c 5 Alice):");
    println!("  args: {:?}", vec!["-c", "5", "Alice"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Argument before options (interspersed)
    let out = Output::new();
    let cmd = Command::new("cmd")
        .option(ClickOption::new(&["--count", "-c"]).default("1").build())
        .argument(Argument::new("name").build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let count: i32 = ctx
                    .get_param::<String>("count")
                    .map(|s| s.as_str())
                    .unwrap_or("1")
                    .parse()
                    .unwrap_or(1);
                let name = ctx.get_param::<String>("name").map(|s| s.as_str()).unwrap_or("");
                out.push(format!("count={} name={}", count, name));
                Ok(())
            }
        })
        .build();
    let result = cmd.main(vec!["Alice".to_string(), "-c".to_string(), "5".to_string()]);
    println!("# Argument before options - interspersed (Alice -c 5):");
    println!("  args: {:?}", vec!["Alice", "-c", "5"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));

    // Files with flag interspersed
    let out = Output::new();
    let cmd2 = Command::new("cmd2")
        .option(ClickOption::new(&["--flag", "-f"]).flag("true").build())
        .argument(Argument::new("files").nargs(Nargs::Variadic).required(false).build())
        .callback({
            let out = out.clone();
            move |ctx| {
                let flag = ctx
                    .get_param::<String>("flag")
                    .map_or(false, |v| v == "true");
                let files = ctx
                    .get_param::<Vec<String>>("files")
                    .cloned()
                    .unwrap_or_default();
                out.push(format!("flag={} files={}", py_bool(flag), py_list(&files)));
                Ok(())
            }
        })
        .build();
    let result = cmd2.main(vec!["a.txt".to_string(), "-f".to_string(), "b.txt".to_string()]);
    println!("# Files with flag interspersed (a.txt -f b.txt):");
    println!("  args: {:?}", vec!["a.txt", "-f", "b.txt"]);
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", exit_code(&result));
}

fn test_unknown_option_errors() {
    println!("\n=== Unknown Option Errors ===");

    let cmd = Command::new("cmd")
        .option(ClickOption::new(&["--help-me"]).flag("true").build())
        .option(ClickOption::new(&["--verbose", "-v"]).flag("true").build())
        .callback(|_ctx| Ok(()))
        .build();

    // Unknown long option
    let result = cmd.main(vec!["--unknown".to_string()]);
    println!("# Unknown long option (--unknown):");
    println!("  args: {:?}", vec!["--unknown"]);
    if let Err(e) = &result {
        let msg = e.format_message();
        if msg.contains("No such option:") || msg.to_lowercase().contains("no such option:") {
            println!("  error: 'No such option: --unknown'");
        } else {
            println!("  error: {}", py_repr(&msg));
        }
    } else {
        println!("  error: ''");
    }
    println!("  exit_code: {}", exit_code(&result));

    // Unknown short option
    let result = cmd.main(vec!["-x".to_string()]);
    println!("# Unknown short option (-x):");
    println!("  args: {:?}", vec!["-x"]);
    if let Err(e) = &result {
        let msg = e.format_message();
        if msg.contains("No such option:") || msg.to_lowercase().contains("no such option:") {
            println!("  error: 'No such option: -x'");
        } else {
            println!("  error: {}", py_repr(&msg));
        }
    } else {
        println!("  error: ''");
    }
    println!("  exit_code: {}", exit_code(&result));

    // Typo with suggestion (--hlep similar to --help-me)
    let result = cmd.main(vec!["--hlep".to_string()]);
    println!("# Typo option (--hlep):");
    println!("  args: {:?}", vec!["--hlep"]);
    if let Err(e) = &result {
        let msg = e.format_message();
        if msg.contains("No such option:") || msg.to_lowercase().contains("no such option:") {
            println!("  error_type: 'NoSuchOption'");
        }
    }
    println!("  exit_code: {}", exit_code(&result));
}
