//! Integration tests for the derive macros.

use click_derive::Command;

/// A simple greeter command
#[derive(Command)]
#[command(name = "greet")]
struct Greet {
    /// Name to greet
    #[argument]
    name: String,

    /// Number of times to greet
    #[option(short, long, default = 1)]
    count: i32,

    /// Be verbose
    #[option(short, long)]
    verbose: bool,
}

#[test]
fn test_command_generation() {
    let cmd = Greet::command();

    // Verify command name
    assert_eq!(cmd.name.as_deref(), Some("greet"));

    // Verify we have arguments and options
    assert!(!cmd.arguments.is_empty());
    assert!(!cmd.options.is_empty());
}

#[test]
fn test_command_help() {
    let cmd = Greet::command();
    let ctx = click::ContextBuilder::new().info_name("greet").build();
    let help = cmd.get_help(&ctx);

    // Help should contain relevant information
    assert!(help.contains("greet"), "Help should contain command name");
    assert!(help.contains("NAME") || help.contains("name"), "Help should contain argument");
    assert!(help.contains("--count") || help.contains("-c"), "Help should contain count option");
    assert!(help.contains("--verbose") || help.contains("-v"), "Help should contain verbose option");
}

#[test]
fn test_command_with_callback() {
    use std::sync::{Arc, Mutex};

    let output = Arc::new(Mutex::new(Vec::new()));
    let output_clone = output.clone();

    let cmd = Greet::command_with_run(move |greet, _ctx| {
        for _ in 0..greet.count {
            output_clone.lock().unwrap().push(format!("Hello, {}!", greet.name));
        }
        if greet.verbose {
            output_clone.lock().unwrap().push("(verbose mode)".to_string());
        }
        Ok(())
    });

    // Test with default count
    cmd.main(vec!["World".to_string()]).unwrap();
    let lines = output.lock().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "Hello, World!");
    drop(lines);

    // Reset and test with count
    output.lock().unwrap().clear();
    let output_clone = output.clone();
    let cmd = Greet::command_with_run(move |greet, _ctx| {
        for _ in 0..greet.count {
            output_clone.lock().unwrap().push(format!("Hello, {}!", greet.name));
        }
        Ok(())
    });

    cmd.main(vec!["--count".to_string(), "3".to_string(), "Alice".to_string()]).unwrap();
    let lines = output.lock().unwrap();
    assert_eq!(lines.len(), 3);
    for line in lines.iter() {
        assert_eq!(line, "Hello, Alice!");
    }
}

#[test]
fn test_command_from_context() {
    let cmd = Greet::command();
    let ctx = cmd
        .make_context("greet", vec!["--count".to_string(), "2".to_string(), "Bob".to_string()], None)
        .unwrap();

    let greet = Greet::from_context(&ctx).unwrap();
    assert_eq!(greet.name, "Bob");
    assert_eq!(greet.count, 2);
    assert!(!greet.verbose);
}

/// Test command with optional argument
#[derive(Command)]
#[command(name = "optional")]
struct OptionalArgs {
    /// Optional filename
    #[argument(required = false)]
    filename: Option<String>,

    /// Optional output
    #[option(short, long)]
    output: Option<String>,
}

#[test]
fn test_optional_argument() {
    let cmd = OptionalArgs::command();

    // Without argument
    let ctx = cmd.make_context("optional", vec![], None).unwrap();
    let args = OptionalArgs::from_context(&ctx).unwrap();
    assert!(args.filename.is_none());
    assert!(args.output.is_none());

    // With argument
    let ctx = cmd.make_context("optional", vec!["test.txt".to_string()], None).unwrap();
    let args = OptionalArgs::from_context(&ctx).unwrap();
    assert_eq!(args.filename, Some("test.txt".to_string()));
}

/// Test command with multiple values
#[derive(Command)]
#[command(name = "multi")]
struct MultiValues {
    /// Input files
    #[argument(multiple)]
    files: Vec<String>,

    /// Tags
    #[option(short, long, multiple)]
    tags: Vec<String>,
}

#[test]
fn test_multiple_values() {
    let cmd = MultiValues::command();

    let ctx = cmd
        .make_context(
            "multi",
            vec![
                "-t".to_string(), "tag1".to_string(),
                "-t".to_string(), "tag2".to_string(),
                "file1.txt".to_string(),
                "file2.txt".to_string(),
            ],
            None,
        )
        .unwrap();

    let args = MultiValues::from_context(&ctx).unwrap();
    assert_eq!(args.files, vec!["file1.txt", "file2.txt"]);
    assert_eq!(args.tags, vec!["tag1", "tag2"]);
}

/// Test command with count option
#[derive(Command)]
#[command(name = "verbose")]
struct VerboseCmd {
    /// Verbosity level
    #[option(short, long, count)]
    verbose: usize,
}

#[test]
fn test_count_option() {
    let cmd = VerboseCmd::command();

    // No -v flags
    let ctx = cmd.make_context("verbose", vec![], None).unwrap();
    let args = VerboseCmd::from_context(&ctx).unwrap();
    assert_eq!(args.verbose, 0);

    // Multiple -v flags
    let ctx = cmd
        .make_context("verbose", vec!["-v".to_string(), "-v".to_string(), "-v".to_string()], None)
        .unwrap();
    let args = VerboseCmd::from_context(&ctx).unwrap();
    assert_eq!(args.verbose, 3);
}

/// Test command with doc comment as help
#[derive(Command)]
#[command(name = "documented")]
/// This is a documented command.
///
/// It has multiple lines of documentation.
struct DocumentedCmd {
    /// This argument has help from doc comment
    #[argument]
    #[allow(dead_code)]
    input: String,
}

#[test]
fn test_doc_comment_help() {
    let cmd = DocumentedCmd::command();
    let ctx = click::ContextBuilder::new().info_name("documented").build();
    let help = cmd.get_help(&ctx);

    assert!(help.contains("documented"), "Help should contain command name");
}

/// Test command with hidden option
#[derive(Command)]
#[command(name = "hidden")]
struct HiddenCmd {
    /// A visible option
    #[option(long)]
    #[allow(dead_code)]
    visible: String,

    /// A hidden option
    #[option(long, hidden)]
    #[allow(dead_code)]
    secret: String,
}

#[test]
fn test_hidden_option() {
    let cmd = HiddenCmd::command();
    let ctx = click::ContextBuilder::new().info_name("hidden").build();
    let help = cmd.get_help(&ctx);

    assert!(help.contains("--visible"), "Help should contain visible option");
    assert!(!help.contains("--secret"), "Help should not contain hidden option");
}

// Note: Environment variable support in options is defined at the struct level
// but full envvar parsing integration requires additional work in the parser.
// The derive macro sets up the envvar attribute correctly; the parser needs
// to be enhanced to read from environment variables when CLI args are missing.
