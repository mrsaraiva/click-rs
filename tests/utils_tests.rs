//! Tests for the utilities module.

use click::utils::{
    expand_path, format_filename, get_app_dir, get_extension, get_os_args,
    get_os_args_skip_program, get_text_stderr, get_text_stdout, home_dir,
    join_with_conjunction, make_safe_filename, pluralize, safecall, strip_extension,
};
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};

// =============================================================================
// Stream Function Tests
// =============================================================================

#[test]
fn test_get_text_stdout() {
    let mut stdout = get_text_stdout();
    // Should be writable
    assert!(write!(stdout, "").is_ok());
}

#[test]
fn test_get_text_stderr() {
    let mut stderr = get_text_stderr();
    // Should be writable
    assert!(write!(stderr, "").is_ok());
}

// =============================================================================
// Path Formatting Tests
// =============================================================================

#[test]
fn test_format_filename_absolute() {
    let path = Path::new("/usr/local/bin/test");
    let formatted = format_filename(path);

    // Should use forward slashes
    assert!(!formatted.contains('\\'));
}

#[test]
fn test_format_filename_relative() {
    let path = Path::new("./relative/path");
    let formatted = format_filename(path);

    assert_eq!(formatted, "./relative/path");
}

#[test]
fn test_format_filename_home_shortening() {
    if let Some(home) = home_dir() {
        let path = home.join("documents").join("file.txt");
        let formatted = format_filename(&path);

        // Should start with ~ if path is under home
        assert!(
            formatted.starts_with("~/") || formatted.starts_with("~\\"),
            "Expected path to start with ~/, got: {}",
            formatted
        );
        assert!(formatted.contains("documents"));
    }
}

// =============================================================================
// App Directory Tests
// =============================================================================

#[test]
fn test_get_app_dir_non_roaming() {
    let app_dir = get_app_dir("testapp", false);

    // Should return a valid path
    assert!(!app_dir.as_os_str().is_empty());

    // Should contain the app name
    assert!(
        app_dir.to_string_lossy().contains("testapp")
            || app_dir.to_string_lossy().contains(".testapp"),
        "App dir should contain app name: {}",
        app_dir.display()
    );
}

#[test]
fn test_get_app_dir_roaming() {
    let app_dir = get_app_dir("testapp", true);

    // Should return a valid path
    assert!(!app_dir.as_os_str().is_empty());

    // Should contain the app name
    assert!(
        app_dir.to_string_lossy().contains("testapp"),
        "App dir should contain app name: {}",
        app_dir.display()
    );
}

#[test]
fn test_get_app_dir_different_apps() {
    let dir1 = get_app_dir("app1", false);
    let dir2 = get_app_dir("app2", false);

    // Different apps should have different directories
    assert_ne!(dir1, dir2);
}

// =============================================================================
// Path Expansion Tests
// =============================================================================

#[test]
fn test_expand_path_no_expansion() {
    let path = expand_path("/absolute/path");
    assert_eq!(path, PathBuf::from("/absolute/path"));

    let path = expand_path("relative/path");
    assert_eq!(path, PathBuf::from("relative/path"));
}

#[test]
fn test_expand_path_tilde_home() {
    if let Some(home) = home_dir() {
        let path = expand_path("~");
        assert_eq!(path, home);
    }
}

#[test]
fn test_expand_path_tilde_subdir() {
    if let Some(home) = home_dir() {
        let path = expand_path("~/documents");
        assert_eq!(path, home.join("documents"));
    }
}

#[test]
fn test_expand_path_env_var_dollar() {
    env::set_var("CLICK_TEST_EXPAND", "/test/path");

    let path = expand_path("$CLICK_TEST_EXPAND/subdir");
    assert_eq!(path, PathBuf::from("/test/path/subdir"));

    env::remove_var("CLICK_TEST_EXPAND");
}

#[test]
fn test_expand_path_env_var_braces() {
    env::set_var("CLICK_TEST_EXPAND2", "/another/path");

    let path = expand_path("${CLICK_TEST_EXPAND2}/subdir");
    assert_eq!(path, PathBuf::from("/another/path/subdir"));

    env::remove_var("CLICK_TEST_EXPAND2");
}

#[test]
fn test_expand_path_undefined_env_var() {
    env::remove_var("CLICK_UNDEFINED_VAR");

    let path = expand_path("$CLICK_UNDEFINED_VAR/subdir");
    // Undefined variable should be removed
    assert_eq!(path, PathBuf::from("/subdir"));
}

#[test]
fn test_expand_path_multiple_vars() {
    env::set_var("CLICK_VAR1", "/first");
    env::set_var("CLICK_VAR2", "second");

    let path = expand_path("$CLICK_VAR1/$CLICK_VAR2/file");
    assert_eq!(path, PathBuf::from("/first/second/file"));

    env::remove_var("CLICK_VAR1");
    env::remove_var("CLICK_VAR2");
}

// =============================================================================
// OS Args Tests
// =============================================================================

#[test]
fn test_get_os_args() {
    let args = get_os_args();

    // Should have at least the program name
    assert!(!args.is_empty());
}

#[test]
fn test_get_os_args_skip_program() {
    let args = get_os_args_skip_program();

    // This is the args without the program name
    // In tests, we typically don't have extra args
    let _ = args;
}

// =============================================================================
// Home Directory Tests
// =============================================================================

#[test]
fn test_home_dir() {
    // home_dir should return Some on most systems
    // Even if it doesn't, it shouldn't panic
    let _ = home_dir();
}

#[test]
fn test_home_dir_with_env() {
    // If HOME is set, we should get a result
    if env::var("HOME").is_ok() {
        let home = home_dir();
        assert!(home.is_some(), "home_dir() should return Some when HOME is set");
    }
}

// =============================================================================
// String Utility Tests
// =============================================================================

#[test]
fn test_pluralize_one() {
    assert_eq!(pluralize(1, "file", "files"), "file");
    assert_eq!(pluralize(1, "item", "items"), "item");
}

#[test]
fn test_pluralize_zero() {
    assert_eq!(pluralize(0, "file", "files"), "files");
    assert_eq!(pluralize(0, "item", "items"), "items");
}

#[test]
fn test_pluralize_many() {
    assert_eq!(pluralize(2, "file", "files"), "files");
    assert_eq!(pluralize(5, "item", "items"), "items");
    assert_eq!(pluralize(100, "person", "people"), "people");
}

#[test]
fn test_join_with_conjunction_empty() {
    let items: Vec<&str> = vec![];
    assert_eq!(join_with_conjunction(&items, ", ", " and "), "");
}

#[test]
fn test_join_with_conjunction_one() {
    let items = vec!["apple"];
    assert_eq!(join_with_conjunction(&items, ", ", " and "), "apple");
}

#[test]
fn test_join_with_conjunction_two() {
    let items = vec!["apple", "banana"];
    assert_eq!(join_with_conjunction(&items, ", ", " and "), "apple and banana");
}

#[test]
fn test_join_with_conjunction_three() {
    let items = vec!["apple", "banana", "cherry"];
    assert_eq!(join_with_conjunction(&items, ", ", " and "), "apple, banana and cherry");
}

#[test]
fn test_join_with_conjunction_or() {
    let items = vec!["red", "green", "blue"];
    assert_eq!(join_with_conjunction(&items, ", ", " or "), "red, green or blue");
}

#[test]
fn test_safecall_normal() {
    assert_eq!(safecall("normal text"), "normal text");
    assert_eq!(safecall("with spaces"), "with spaces");
    assert_eq!(safecall("with-dashes"), "with-dashes");
}

#[test]
fn test_safecall_newlines() {
    assert_eq!(safecall("line1\nline2"), "line1\nline2");
}

#[test]
fn test_safecall_tabs() {
    assert_eq!(safecall("col1\tcol2"), "col1\tcol2");
}

#[test]
fn test_safecall_control_chars() {
    // Bell character
    assert_eq!(safecall("before\x07after"), "before\\x07after");

    // Null character
    assert_eq!(safecall("before\x00after"), "before\\x00after");
}

// =============================================================================
// Filename Safety Tests
// =============================================================================

#[test]
fn test_make_safe_filename_normal() {
    assert_eq!(make_safe_filename("normal.txt"), "normal.txt");
    assert_eq!(make_safe_filename("file_name.txt"), "file_name.txt");
}

#[test]
fn test_make_safe_filename_spaces() {
    assert_eq!(make_safe_filename("file name.txt"), "file_name.txt");
    assert_eq!(make_safe_filename("multiple   spaces"), "multiple_spaces");
}

#[test]
fn test_make_safe_filename_unsafe_chars() {
    assert_eq!(make_safe_filename("a<b>c.txt"), "abc.txt");
    assert_eq!(make_safe_filename("a:b.txt"), "ab.txt");
    assert_eq!(make_safe_filename("a/b\\c.txt"), "abc.txt");
    assert_eq!(make_safe_filename("a|b.txt"), "ab.txt");
    assert_eq!(make_safe_filename("a?b.txt"), "ab.txt");
    assert_eq!(make_safe_filename("a*b.txt"), "ab.txt");
    assert_eq!(make_safe_filename("a\"b.txt"), "ab.txt");
}

#[test]
fn test_make_safe_filename_leading_trailing_spaces() {
    assert_eq!(make_safe_filename("  file.txt  "), "_file.txt");
}

#[test]
fn test_make_safe_filename_all_unsafe() {
    assert_eq!(make_safe_filename("<>:/\\|?*"), "");
}

// =============================================================================
// File Extension Tests
// =============================================================================

#[test]
fn test_get_extension_normal() {
    assert_eq!(get_extension(Path::new("file.txt")), Some("txt"));
    assert_eq!(get_extension(Path::new("file.tar.gz")), Some("gz"));
    assert_eq!(get_extension(Path::new("document.pdf")), Some("pdf"));
}

#[test]
fn test_get_extension_none() {
    assert_eq!(get_extension(Path::new("file")), None);
    assert_eq!(get_extension(Path::new(".hidden")), None);
}

#[test]
fn test_get_extension_path() {
    assert_eq!(get_extension(Path::new("/path/to/file.txt")), Some("txt"));
    assert_eq!(get_extension(Path::new("./relative/file.md")), Some("md"));
}

#[test]
fn test_strip_extension_normal() {
    assert_eq!(strip_extension(Path::new("file.txt")), PathBuf::from("file"));
    assert_eq!(strip_extension(Path::new("file.tar.gz")), PathBuf::from("file.tar"));
}

#[test]
fn test_strip_extension_no_extension() {
    assert_eq!(strip_extension(Path::new("file")), PathBuf::from("file"));
}

#[test]
fn test_strip_extension_path() {
    assert_eq!(
        strip_extension(Path::new("/path/to/file.txt")),
        PathBuf::from("/path/to/file")
    );
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_expand_path_empty() {
    let path = expand_path("");
    assert_eq!(path, PathBuf::from(""));
}

#[test]
fn test_format_filename_empty() {
    let formatted = format_filename(Path::new(""));
    assert_eq!(formatted, "");
}

#[test]
fn test_make_safe_filename_empty() {
    assert_eq!(make_safe_filename(""), "");
}

#[test]
fn test_get_extension_empty() {
    assert_eq!(get_extension(Path::new("")), None);
}

#[test]
fn test_pluralize_edge_cases() {
    // Very large numbers
    assert_eq!(pluralize(usize::MAX, "item", "items"), "items");

    // With irregular plurals
    assert_eq!(pluralize(1, "mouse", "mice"), "mouse");
    assert_eq!(pluralize(2, "mouse", "mice"), "mice");
}

#[test]
fn test_join_with_conjunction_with_empty_items() {
    let items = vec!["", "a", ""];
    let result = join_with_conjunction(&items, ", ", " and ");
    // Empty strings are still included
    assert_eq!(result, ", a and ");
}

#[test]
fn test_expand_path_consecutive_vars() {
    env::set_var("CLICK_A", "a");
    env::set_var("CLICK_B", "b");

    let path = expand_path("$CLICK_A$CLICK_B");
    assert_eq!(path, PathBuf::from("ab"));

    env::remove_var("CLICK_A");
    env::remove_var("CLICK_B");
}

#[test]
fn test_make_safe_filename_unicode() {
    // Unicode characters should be preserved
    assert_eq!(make_safe_filename("file_日本語.txt"), "file_日本語.txt");
    assert_eq!(make_safe_filename("файл.txt"), "файл.txt");
}

#[test]
fn test_format_filename_preserves_unicode() {
    let path = Path::new("/path/to/日本語/file.txt");
    let formatted = format_filename(path);
    assert!(formatted.contains("日本語"));
}
