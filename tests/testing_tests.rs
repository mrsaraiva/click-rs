//! Tests for the testing utilities module.

use click::command::Command;
use click::error::ClickError;
use click::option::ClickOption;
use click::testing::{make_test_context, CliRunner, InvokeResult, IsolatedFilesystem};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// =============================================================================
// CliRunner Construction Tests
// =============================================================================

#[test]
fn test_cli_runner_new() {
    let runner = CliRunner::new();
    // Default runner should be usable
    let _ = runner;
}

#[test]
fn test_cli_runner_env_single() {
    let runner = CliRunner::new().env("MY_VAR", "my_value");
    // Runner should accept environment variables
    let _ = runner;
}

#[test]
fn test_cli_runner_env_multiple() {
    let runner = CliRunner::new()
        .env("VAR1", "value1")
        .env("VAR2", "value2")
        .env("VAR3", "value3");
    let _ = runner;
}

#[test]
fn test_cli_runner_env_unset() {
    let runner = CliRunner::new().env_unset("SOME_VAR");
    let _ = runner;
}

#[test]
fn test_cli_runner_env_clear() {
    let runner = CliRunner::new()
        .env("VAR1", "value1")
        .env_unset("VAR2")
        .env_clear();
    // After clear, should be back to defaults
    let _ = runner;
}

#[test]
fn test_cli_runner_echo_stdin() {
    let runner = CliRunner::new().echo_stdin(true);
    let _ = runner;
}

#[test]
fn test_cli_runner_mix_stderr() {
    let runner_mixed = CliRunner::new().mix_stderr(true);
    let runner_separate = CliRunner::new().mix_stderr(false);
    let _ = (runner_mixed, runner_separate);
}

#[test]
fn test_cli_runner_charset_decoding() {
    let cmd = Command::new("latin")
        .callback(|_ctx| {
            let mut out = std::io::stdout();
            out.write_all(&[0xE9]).unwrap(); // "é" in latin-1
            Ok(())
        })
        .build();

    let runner = CliRunner::new().charset("latin-1");
    let result = runner.invoke(&cmd, &[]);

    assert_eq!(result.exit_code, 0);
    assert!(result.output.contains('é'));
}

#[test]
fn test_cli_runner_chaining() {
    let runner = CliRunner::new()
        .env("VAR1", "value1")
        .env("VAR2", "value2")
        .env_unset("VAR3")
        .echo_stdin(true)
        .mix_stderr(false);
    let _ = runner;
}

// =============================================================================
// CliRunner Invoke Tests
// =============================================================================

#[test]
fn test_invoke_simple_command() {
    let cmd = Command::new("test").callback(|_ctx| Ok(())).build();

    let runner = CliRunner::new();
    let result = runner.invoke(&cmd, &[]);

    assert_eq!(result.exit_code, 0);
    assert!(result.exception_message.is_none());
}

#[test]
fn test_invoke_command_with_args() {
    let cmd = Command::new("greet")
        .option(ClickOption::new(&["--name"]).default("World").build())
        .callback(|_ctx| Ok(()))
        .build();

    let runner = CliRunner::new();
    let result = runner.invoke(&cmd, &["--name", "Test"]);

    assert_eq!(result.exit_code, 0);
}

#[test]
fn test_invoke_failing_command() {
    let cmd = Command::new("fail")
        .callback(|_ctx| Err(ClickError::usage("Intentional failure")))
        .build();

    let runner = CliRunner::new();
    let result = runner.invoke(&cmd, &[]);

    assert_eq!(result.exit_code, 2); // Usage errors have exit code 2
    assert!(result.exception_message.is_some());
}

#[test]
fn test_invoke_missing_required_arg() {
    let cmd = Command::new("test")
        .option(ClickOption::new(&["--required"]).required().build())
        .build();

    let runner = CliRunner::new();
    let result = runner.invoke(&cmd, &[]);

    assert!(result.is_failure());
    assert!(result.exception_message.is_some());
}

#[test]
fn test_invoke_with_env_override() {
    let captured = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_clone = Arc::clone(&captured);

    let cmd = Command::new("test")
        .callback(move |_ctx| {
            if let Ok(val) = std::env::var("CLICK_TEST_VAR") {
                *captured_clone.lock().unwrap() = val;
            }
            Ok(())
        })
        .build();

    let runner = CliRunner::new().env("CLICK_TEST_VAR", "test_value");
    let _result = runner.invoke(&cmd, &[]);

    // The environment variable should have been set during invocation
    // Note: This depends on the command actually checking the env var
}

#[test]
fn test_invoke_with_input() {
    let cmd = Command::new("test").callback(|_ctx| Ok(())).build();

    let runner = CliRunner::new();
    let result = runner.invoke_with_input(&cmd, &[], Some("input data"));

    assert_eq!(result.exit_code, 0);
}

#[test]
fn test_invoke_isolated() {
    let cmd = Command::new("test").callback(|_ctx| Ok(())).build();

    let runner = CliRunner::new();
    let result = runner.invoke_isolated(&cmd, &[]);

    assert_eq!(result.exit_code, 0);
}

#[test]
fn test_invoke_catches_panics_when_enabled() {
    let cmd = Command::new("panic")
        .callback(|_ctx| {
            println!("before");
            panic!("boom");
        })
        .build();

    let runner = CliRunner::new().catch_panics(true).mix_stderr(false);
    let result = runner.invoke(&cmd, &[]);

    assert_eq!(result.exit_code, 1);
    assert!(result
        .exception_message
        .unwrap_or_default()
        .contains("boom"));
}

#[test]
#[should_panic]
fn test_invoke_rethrows_panics_when_disabled() {
    let cmd = Command::new("panic")
        .callback(|_ctx| {
            println!("before");
            panic!("boom");
        })
        .build();

    let runner = CliRunner::new().catch_panics(false);
    let _ = runner.invoke(&cmd, &[]);
}

// =============================================================================
// InvokeResult Tests
// =============================================================================

#[test]
fn test_invoke_result_is_success() {
    let result = InvokeResult::new(0, String::new(), String::new(), None);

    assert!(result.is_success());
    assert!(!result.is_failure());
}

#[test]
fn test_invoke_result_is_failure() {
    let result = InvokeResult::new(1, String::new(), String::new(), None);

    assert!(!result.is_success());
    assert!(result.is_failure());
}

#[test]
fn test_invoke_result_various_exit_codes() {
    for code in [0, 1, 2, 127, 255] {
        let result = InvokeResult::new(code, String::new(), String::new(), None);

        if code == 0 {
            assert!(result.is_success());
        } else {
            assert!(result.is_failure());
        }
    }
}

#[test]
fn test_invoke_result_output_contains() {
    let result = InvokeResult::new(0, "Hello, World!".to_string(), String::new(), None);

    assert!(result.output_contains("Hello"));
    assert!(result.output_contains("World"));
    assert!(result.output_contains("Hello, World!"));
    assert!(!result.output_contains("Goodbye"));
}

#[test]
fn test_invoke_result_stderr_contains() {
    let result = InvokeResult::new(
        0,
        String::new(),
        "Error: something went wrong".to_string(),
        None,
    );

    assert!(result.stderr_contains("Error"));
    assert!(result.stderr_contains("something"));
    assert!(!result.stderr_contains("success"));
}

#[test]
fn test_invoke_result_output_lines() {
    let result = InvokeResult::new(0, "line1\nline2\nline3".to_string(), String::new(), None);

    let lines = result.output_lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line1");
    assert_eq!(lines[1], "line2");
    assert_eq!(lines[2], "line3");
}

#[test]
fn test_invoke_result_output_lines_empty() {
    let result = InvokeResult::new(0, String::new(), String::new(), None);

    let lines = result.output_lines();
    assert!(lines.is_empty());
}

#[test]
fn test_invoke_result_combined_output() {
    let result = InvokeResult::new(
        0,
        "stdout content".to_string(),
        "stderr content".to_string(),
        None,
    );

    let combined = result.combined_output();
    assert!(combined.contains("stdout content"));
    assert!(combined.contains("stderr content"));
}

#[test]
fn test_invoke_result_combined_output_empty_stderr() {
    let result = InvokeResult::new(0, "only stdout".to_string(), String::new(), None);

    let combined = result.combined_output();
    assert_eq!(combined, "only stdout");
}

// =============================================================================
// IsolatedFilesystem Tests
// =============================================================================

#[test]
fn test_isolated_filesystem_creation() {
    let isolated = IsolatedFilesystem::new().unwrap();
    let path = isolated.path();

    assert!(path.exists());
    assert!(path.is_dir());
}

#[test]
fn test_isolated_filesystem_with_name() {
    let isolated = IsolatedFilesystem::with_name("mytest").unwrap();
    let path = isolated.path();

    assert!(path.exists());
    assert!(path.to_string_lossy().contains("mytest"));
}

#[test]
fn test_isolated_filesystem_cleanup() {
    let path;
    {
        let isolated = IsolatedFilesystem::new().unwrap();
        path = isolated.path().to_path_buf();
        assert!(path.exists());
    }
    // After drop, directory should be cleaned up
    assert!(!path.exists());
}

#[test]
fn test_isolated_filesystem_create_file() {
    let isolated = IsolatedFilesystem::new().unwrap();

    let file_path = isolated.create_file("test.txt", "hello world").unwrap();

    assert!(file_path.exists());
    assert!(isolated.file_exists("test.txt"));
    assert_eq!(isolated.read_file("test.txt").unwrap(), "hello world");
}

#[test]
fn test_isolated_filesystem_create_nested_file() {
    let isolated = IsolatedFilesystem::new().unwrap();

    let file_path = isolated
        .create_file("subdir/nested/file.txt", "nested content")
        .unwrap();

    assert!(file_path.exists());
    assert!(isolated.file_exists("subdir/nested/file.txt"));
    assert_eq!(
        isolated.read_file("subdir/nested/file.txt").unwrap(),
        "nested content"
    );
}

#[test]
fn test_isolated_filesystem_create_dir() {
    let isolated = IsolatedFilesystem::new().unwrap();

    let dir_path = isolated.create_dir("mydir").unwrap();

    assert!(dir_path.exists());
    assert!(dir_path.is_dir());
}

#[test]
fn test_isolated_filesystem_create_nested_dir() {
    let isolated = IsolatedFilesystem::new().unwrap();

    let dir_path = isolated.create_dir("a/b/c").unwrap();

    assert!(dir_path.exists());
    assert!(dir_path.is_dir());
}

#[test]
fn test_isolated_filesystem_file_exists() {
    let isolated = IsolatedFilesystem::new().unwrap();

    assert!(!isolated.file_exists("nonexistent.txt"));

    isolated.create_file("exists.txt", "").unwrap();
    assert!(isolated.file_exists("exists.txt"));
}

#[test]
fn test_isolated_filesystem_list_files() {
    let isolated = IsolatedFilesystem::new().unwrap();

    isolated.create_file("c.txt", "").unwrap();
    isolated.create_file("a.txt", "").unwrap();
    isolated.create_file("b.txt", "").unwrap();

    let files = isolated.list_files().unwrap();

    // Should be sorted
    assert_eq!(files, vec!["a.txt", "b.txt", "c.txt"]);
}

#[test]
fn test_isolated_filesystem_list_files_empty() {
    let isolated = IsolatedFilesystem::new().unwrap();

    let files = isolated.list_files().unwrap();
    assert!(files.is_empty());
}

#[test]
fn test_isolated_filesystem_list_files_with_dirs() {
    let isolated = IsolatedFilesystem::new().unwrap();

    isolated.create_file("file.txt", "").unwrap();
    isolated.create_dir("subdir").unwrap();

    let files = isolated.list_files().unwrap();

    // Both files and directories should be listed
    assert!(files.contains(&"file.txt".to_string()));
    assert!(files.contains(&"subdir".to_string()));
}

// =============================================================================
// make_test_context Tests
// =============================================================================

#[test]
fn test_make_test_context() {
    let ctx = make_test_context("mytest");

    assert_eq!(ctx.info_name(), Some("mytest"));
}

#[test]
fn test_make_test_context_different_names() {
    let ctx1 = make_test_context("cmd1");
    let ctx2 = make_test_context("cmd2");

    assert_eq!(ctx1.info_name(), Some("cmd1"));
    assert_eq!(ctx2.info_name(), Some("cmd2"));
}

// =============================================================================
// Callback Execution Tests
// =============================================================================

#[test]
fn test_callback_is_invoked() {
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = Arc::clone(&called);

    let cmd = Command::new("test")
        .callback(move |_ctx| {
            called_clone.store(true, Ordering::SeqCst);
            Ok(())
        })
        .build();

    let runner = CliRunner::new();
    let _result = runner.invoke(&cmd, &[]);

    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_callback_receives_params() {
    let received_name = Arc::new(std::sync::Mutex::new(String::new()));
    let received_name_clone = Arc::clone(&received_name);

    let cmd = Command::new("test")
        .option(ClickOption::new(&["--name"]).default("default").build())
        .callback(move |ctx| {
            if let Some(name) = ctx.get_param::<String>("name") {
                *received_name_clone.lock().unwrap() = name.clone();
            }
            Ok(())
        })
        .build();

    let runner = CliRunner::new();
    let _result = runner.invoke(&cmd, &["--name", "TestValue"]);

    assert_eq!(*received_name.lock().unwrap(), "TestValue");
}

#[test]
fn test_multiple_invocations() {
    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    let cmd = Command::new("test")
        .callback(move |_ctx| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .build();

    let runner = CliRunner::new();

    runner.invoke(&cmd, &[]);
    runner.invoke(&cmd, &[]);
    runner.invoke(&cmd, &[]);

    assert_eq!(count.load(Ordering::SeqCst), 3);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_usage_error_exit_code() {
    let cmd = Command::new("test")
        .callback(|_ctx| Err(ClickError::usage("Usage error")))
        .build();

    let runner = CliRunner::new();
    let result = runner.invoke(&cmd, &[]);

    assert_eq!(result.exit_code, 2);
}

#[test]
fn test_abort_error_exit_code() {
    let cmd = Command::new("test")
        .callback(|_ctx| Err(ClickError::abort()))
        .build();

    let runner = CliRunner::new();
    let result = runner.invoke(&cmd, &[]);

    assert_eq!(result.exit_code, 1);
}

#[test]
fn test_exit_error_custom_code() {
    let cmd = Command::new("test")
        .callback(|_ctx| Err(ClickError::exit(42)))
        .build();

    let runner = CliRunner::new();
    let result = runner.invoke(&cmd, &[]);

    assert_eq!(result.exit_code, 42);
}

#[test]
fn test_exception_preserved() {
    let cmd = Command::new("test")
        .callback(|_ctx| Err(ClickError::usage("Test error message")))
        .build();

    let runner = CliRunner::new();
    let result = runner.invoke(&cmd, &[]);

    assert!(result.exception_message.is_some());
    let exception = result.exception_message.unwrap();
    assert!(exception.to_string().contains("Test error message"));
}
