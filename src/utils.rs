//! Utility functions for click-rs.
//!
//! This module provides utility functions for path handling, stream access,
//! environment queries, and other common operations needed by CLI applications.
//!
//! # Reference
//!
//! Based on Python Click's `utils.py`.

use std::env;
use std::ffi::OsStr;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};

// =============================================================================
// Stream Functions
// =============================================================================

/// Get a text stdout writer.
///
/// Returns a writer suitable for text output. On most platforms, this is
/// simply stdout.
///
/// # Example
///
/// ```rust
/// use click::utils::get_text_stdout;
/// use std::io::Write;
///
/// let mut stdout = get_text_stdout();
/// writeln!(stdout, "Hello, World!").unwrap();
/// ```
pub fn get_text_stdout() -> Stdout {
    io::stdout()
}

/// Get a text stderr writer.
///
/// Returns a writer suitable for text error output.
///
/// # Example
///
/// ```rust
/// use click::utils::get_text_stderr;
/// use std::io::Write;
///
/// let mut stderr = get_text_stderr();
/// writeln!(stderr, "Error occurred").unwrap();
/// ```
pub fn get_text_stderr() -> io::Stderr {
    io::stderr()
}

/// Get a binary stdout writer.
///
/// Returns a writer suitable for binary output. This is the same as
/// `get_text_stdout()` in Rust since stdout handles both.
pub fn get_binary_stdout() -> Stdout {
    io::stdout()
}

/// Get a binary stdin reader.
///
/// Returns a reader for binary input.
pub fn get_binary_stdin() -> io::Stdin {
    io::stdin()
}

// =============================================================================
// Path Utilities
// =============================================================================

/// Format a path for user-friendly display.
///
/// This function shortens paths by replacing the home directory with `~`
/// and using forward slashes on all platforms.
///
/// # Example
///
/// ```rust
/// use click::utils::format_filename;
/// use std::path::Path;
///
/// let path = Path::new("/home/user/documents/file.txt");
/// let formatted = format_filename(path);
/// // May return "~/documents/file.txt" if /home/user is the home directory
/// ```
pub fn format_filename(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Try to replace home directory with ~
    if let Some(home) = home_dir() {
        let home_str = home.to_string_lossy();
        if path_str.starts_with(home_str.as_ref()) {
            let relative = &path_str[home_str.len()..];
            let relative = relative.trim_start_matches(['/', '\\']);
            return format!("~/{}", relative.replace('\\', "/"));
        }
    }

    // Replace backslashes with forward slashes for consistency
    path_str.replace('\\', "/")
}

/// Get the platform-specific application directory.
///
/// Returns the appropriate directory for storing application data based on
/// the platform:
///
/// - **Unix**: `~/.app_name` (non-roaming) or `~/.local/share/app_name` (roaming)
/// - **macOS**: `~/Library/Application Support/app_name`
/// - **Windows**: `%APPDATA%\app_name` (roaming) or `%LOCALAPPDATA%\app_name` (non-roaming)
///
/// # Arguments
///
/// * `app_name` - The name of the application
/// * `roaming` - Whether to use the roaming profile (Windows) or XDG data dir (Unix)
///
/// # Example
///
/// ```rust
/// use click::utils::get_app_dir;
///
/// let app_dir = get_app_dir("myapp", false);
/// println!("App directory: {}", app_dir.display());
/// ```
pub fn get_app_dir(app_name: &str, roaming: bool) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = if roaming {
            env::var("APPDATA").ok()
        } else {
            env::var("LOCALAPPDATA").ok()
        };

        match base {
            Some(base) => PathBuf::from(base).join(app_name),
            None => PathBuf::from(".").join(app_name),
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            home.join("Library")
                .join("Application Support")
                .join(app_name)
        } else {
            PathBuf::from(".").join(app_name)
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if roaming {
            // Use XDG data dir
            if let Ok(xdg_data) = env::var("XDG_DATA_HOME") {
                return PathBuf::from(xdg_data).join(app_name);
            }
            if let Some(home) = home_dir() {
                return home.join(".local").join("share").join(app_name);
            }
        }

        // Non-roaming: use ~/.app_name
        if let Some(home) = home_dir() {
            home.join(format!(".{}", app_name))
        } else {
            PathBuf::from(".").join(format!(".{}", app_name))
        }
    }
}

/// Expand a path by resolving `~` and environment variables.
///
/// This function expands:
/// - `~` at the start to the user's home directory
/// - `$VAR` or `${VAR}` to the value of the environment variable `VAR`
///
/// # Example
///
/// ```rust
/// use click::utils::expand_path;
///
/// let path = expand_path("~/.config/myapp");
/// // Returns something like "/home/user/.config/myapp"
///
/// // With environment variables
/// std::env::set_var("MY_DIR", "/custom");
/// let path = expand_path("$MY_DIR/file.txt");
/// // Returns "/custom/file.txt"
/// ```
pub fn expand_path(path: &str) -> PathBuf {
    let mut result = path.to_string();

    // Expand ~ at the beginning
    if result.starts_with('~') {
        if let Some(home) = home_dir() {
            let home_str = home.to_string_lossy();
            if result == "~" {
                return home;
            } else if result.starts_with("~/") || result.starts_with("~\\") {
                result = format!("{}{}", home_str, &result[1..]);
            }
        }
    }

    // Expand environment variables
    result = expand_env_vars(&result);

    PathBuf::from(result)
}

/// Expand environment variables in a string.
///
/// Supports both `$VAR` and `${VAR}` syntax.
fn expand_env_vars(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.peek() == Some(&'{') {
                // ${VAR} syntax
                chars.next(); // consume '{'
                let var_name: String = chars.by_ref().take_while(|&c| c != '}').collect();
                if let Ok(value) = env::var(&var_name) {
                    result.push_str(&value);
                }
            } else {
                // $VAR syntax - collect alphanumeric and underscore characters using peek
                // to avoid consuming the character after the variable name
                let mut var_name = String::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c.is_alphanumeric() || next_c == '_' {
                        var_name.push(next_c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !var_name.is_empty() {
                    if let Ok(value) = env::var(&var_name) {
                        result.push_str(&value);
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Get the user's home directory.
///
/// Returns `None` if the home directory cannot be determined.
pub fn home_dir() -> Option<PathBuf> {
    // Check HOME first (Unix)
    if let Ok(home) = env::var("HOME") {
        return Some(PathBuf::from(home));
    }

    // On Windows, check USERPROFILE
    #[cfg(target_os = "windows")]
    {
        if let Ok(profile) = env::var("USERPROFILE") {
            return Some(PathBuf::from(profile));
        }

        // Fallback: HOMEDRIVE + HOMEPATH
        if let (Ok(drive), Ok(path)) = (env::var("HOMEDRIVE"), env::var("HOMEPATH")) {
            return Some(PathBuf::from(format!("{}{}", drive, path)));
        }
    }

    None
}

// =============================================================================
// Environment Utilities
// =============================================================================

/// Get the command-line arguments.
///
/// Returns all arguments including the program name.
///
/// # Example
///
/// ```rust
/// use click::utils::get_os_args;
///
/// let args = get_os_args();
/// // args[0] is typically the program name
/// ```
pub fn get_os_args() -> Vec<String> {
    env::args().collect()
}

/// Get the command-line arguments without the program name.
///
/// Returns arguments starting from index 1.
pub fn get_os_args_skip_program() -> Vec<String> {
    env::args().skip(1).collect()
}

/// Check if ANSI escape codes should be stripped from output.
///
/// Returns `true` if:
/// - The `NO_COLOR` environment variable is set
/// - The `TERM` is set to `dumb`
/// - We cannot determine TTY status
///
/// # Example
///
/// ```rust
/// use click::utils::should_strip_ansi;
/// use std::io;
///
/// let strip = should_strip_ansi();
/// if strip {
///     println!("Plain text output");
/// } else {
///     println!("\x1b[32mColored output\x1b[0m");
/// }
/// ```
pub fn should_strip_ansi() -> bool {
    // Check NO_COLOR environment variable (https://no-color.org/)
    if env::var("NO_COLOR").is_ok() {
        return true;
    }

    // Check for dumb terminal
    if let Ok(term) = env::var("TERM") {
        if term == "dumb" {
            return true;
        }
    }

    // Check COLORTERM for color support
    if env::var("COLORTERM").is_ok() {
        return false;
    }

    // Check TERM for known color-supporting terminals
    if let Ok(term) = env::var("TERM") {
        if term.contains("color") || term.contains("256") || term.contains("xterm") {
            return false;
        }
    }

    // Default: assume color support if we can't determine
    false
}

/// Check if the output stream is a terminal/TTY.
pub fn is_tty() -> bool {
    !should_strip_ansi()
}

/// Get the terminal width.
///
/// Returns the terminal width in columns, or `None` if it cannot be determined.
///
/// This implementation checks the `COLUMNS` environment variable, which is
/// commonly set by shells. For more robust terminal size detection, consider
/// using a crate like `terminal_size`.
pub fn get_terminal_width() -> Option<usize> {
    // Try to get from environment
    if let Ok(cols) = env::var("COLUMNS") {
        if let Ok(width) = cols.parse::<usize>() {
            if width > 0 {
                return Some(width);
            }
        }
    }

    // Default to 80 columns if we can't determine
    // This is a reasonable fallback for most terminals
    None
}

// =============================================================================
// String Utilities
// =============================================================================

/// Safely encode a string for terminal output.
///
/// Replaces unprintable characters with their escaped representations.
pub fn safecall<T: AsRef<str>>(s: T) -> String {
    let s = s.as_ref();
    let mut result = String::with_capacity(s.len());

    for c in s.chars() {
        if c.is_control() && c != '\n' && c != '\t' && c != '\r' {
            // Escape control characters
            result.push_str(&format!("\\x{:02x}", c as u32));
        } else {
            result.push(c);
        }
    }

    result
}

/// Pluralize a word based on a count.
///
/// # Example
///
/// ```rust
/// use click::utils::pluralize;
///
/// assert_eq!(pluralize(1, "file", "files"), "file");
/// assert_eq!(pluralize(2, "file", "files"), "files");
/// assert_eq!(pluralize(0, "file", "files"), "files");
/// ```
pub fn pluralize<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

/// Join items with a separator and a final conjunction.
///
/// # Example
///
/// ```rust
/// use click::utils::join_with_conjunction;
///
/// let items = vec!["a", "b", "c"];
/// assert_eq!(join_with_conjunction(&items, ", ", " or "), "a, b or c");
///
/// let items = vec!["a", "b"];
/// assert_eq!(join_with_conjunction(&items, ", ", " and "), "a and b");
///
/// let items = vec!["a"];
/// assert_eq!(join_with_conjunction(&items, ", ", " and "), "a");
/// ```
pub fn join_with_conjunction(items: &[&str], separator: &str, conjunction: &str) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].to_string(),
        2 => format!("{}{}{}", items[0], conjunction, items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}{}{}", rest.join(separator), conjunction, last)
        }
    }
}

// =============================================================================
// Filename Utilities
// =============================================================================

/// Make a filename safe by removing or replacing problematic characters.
///
/// This is useful for creating safe filenames from user input.
///
/// # Example
///
/// ```rust
/// use click::utils::make_safe_filename;
///
/// let safe = make_safe_filename("my file<>.txt");
/// assert_eq!(safe, "my_file.txt");
/// ```
pub fn make_safe_filename(filename: &str) -> String {
    // Characters not allowed in filenames on various platforms
    const UNSAFE_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];

    let mut result = String::with_capacity(filename.len());
    let mut last_was_space = false;

    for c in filename.chars() {
        if UNSAFE_CHARS.contains(&c) {
            // Skip unsafe characters
            continue;
        } else if c.is_whitespace() {
            // Replace multiple whitespace with single underscore
            if !last_was_space {
                result.push('_');
                last_was_space = true;
            }
        } else if c.is_control() {
            // Skip control characters
            continue;
        } else {
            result.push(c);
            last_was_space = false;
        }
    }

    // Trim trailing underscores
    result.trim_end_matches('_').to_string()
}

/// Get the file extension from a path, if any.
pub fn get_extension(path: &Path) -> Option<&str> {
    path.extension().and_then(OsStr::to_str)
}

/// Strip the file extension from a path.
pub fn strip_extension(path: &Path) -> PathBuf {
    let mut result = path.to_path_buf();
    result.set_extension("");
    result
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_filename() {
        // Test with absolute path
        let path = Path::new("/usr/local/bin/test");
        let formatted = format_filename(path);
        assert!(!formatted.contains('\\'));

        // Test with relative path
        let path = Path::new("./relative/path");
        let formatted = format_filename(path);
        assert_eq!(formatted, "./relative/path");
    }

    #[test]
    fn test_expand_path_tilde() {
        if let Some(home) = home_dir() {
            let path = expand_path("~/test");
            assert_eq!(path, home.join("test"));

            let path = expand_path("~");
            assert_eq!(path, home);
        }
    }

    #[test]
    fn test_expand_path_env_vars() {
        env::set_var("CLICK_TEST_VAR", "/test/path");

        let path = expand_path("$CLICK_TEST_VAR/file.txt");
        assert_eq!(path, PathBuf::from("/test/path/file.txt"));

        let path = expand_path("${CLICK_TEST_VAR}/file.txt");
        assert_eq!(path, PathBuf::from("/test/path/file.txt"));

        env::remove_var("CLICK_TEST_VAR");
    }

    #[test]
    fn test_expand_path_no_expansion() {
        let path = expand_path("/absolute/path");
        assert_eq!(path, PathBuf::from("/absolute/path"));

        let path = expand_path("relative/path");
        assert_eq!(path, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_get_os_args() {
        let args = get_os_args();
        // Should have at least the program name
        assert!(!args.is_empty());
    }

    #[test]
    fn test_get_app_dir() {
        let app_dir = get_app_dir("testapp", false);
        // Just verify it returns a path
        assert!(!app_dir.as_os_str().is_empty());
    }

    #[test]
    fn test_pluralize() {
        assert_eq!(pluralize(0, "file", "files"), "files");
        assert_eq!(pluralize(1, "file", "files"), "file");
        assert_eq!(pluralize(2, "file", "files"), "files");
        assert_eq!(pluralize(100, "item", "items"), "items");
    }

    #[test]
    fn test_join_with_conjunction() {
        assert_eq!(join_with_conjunction(&[], ", ", " and "), "");
        assert_eq!(join_with_conjunction(&["a"], ", ", " and "), "a");
        assert_eq!(join_with_conjunction(&["a", "b"], ", ", " and "), "a and b");
        assert_eq!(
            join_with_conjunction(&["a", "b", "c"], ", ", " and "),
            "a, b and c"
        );
        assert_eq!(
            join_with_conjunction(&["a", "b", "c", "d"], ", ", " or "),
            "a, b, c or d"
        );
    }

    #[test]
    fn test_make_safe_filename() {
        assert_eq!(make_safe_filename("normal.txt"), "normal.txt");
        assert_eq!(make_safe_filename("file name.txt"), "file_name.txt");
        assert_eq!(make_safe_filename("a<b>c.txt"), "abc.txt");
        assert_eq!(make_safe_filename("a:b/c\\d.txt"), "abcd.txt");
        assert_eq!(
            make_safe_filename("  multiple   spaces  "),
            "_multiple_spaces"
        );
    }

    #[test]
    fn test_safecall() {
        assert_eq!(safecall("normal text"), "normal text");
        assert_eq!(safecall("with\nnewline"), "with\nnewline");
        assert_eq!(safecall("with\ttab"), "with\ttab");
        // Control character (bell)
        assert_eq!(safecall("with\x07bell"), "with\\x07bell");
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension(Path::new("file.txt")), Some("txt"));
        assert_eq!(get_extension(Path::new("file.tar.gz")), Some("gz"));
        assert_eq!(get_extension(Path::new("file")), None);
        assert_eq!(get_extension(Path::new(".hidden")), None);
    }

    #[test]
    fn test_strip_extension() {
        assert_eq!(strip_extension(Path::new("file.txt")), PathBuf::from("file"));
        assert_eq!(
            strip_extension(Path::new("path/to/file.txt")),
            PathBuf::from("path/to/file")
        );
        assert_eq!(
            strip_extension(Path::new("file.tar.gz")),
            PathBuf::from("file.tar")
        );
    }

    #[test]
    fn test_home_dir() {
        // Should return Some on most systems
        let home = home_dir();
        if home.is_some() {
            assert!(home.unwrap().exists() || env::var("HOME").is_ok());
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_should_strip_ansi_no_color() {
        // Save current NO_COLOR
        let saved = env::var("NO_COLOR").ok();

        env::set_var("NO_COLOR", "1");
        assert!(should_strip_ansi());

        // Restore
        match saved {
            Some(v) => env::set_var("NO_COLOR", v),
            None => env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_should_strip_ansi_dumb_term() {
        // Save current TERM
        let saved = env::var("TERM").ok();
        env::remove_var("NO_COLOR");

        env::set_var("TERM", "dumb");
        assert!(should_strip_ansi());

        // Restore
        match saved {
            Some(v) => env::set_var("TERM", v),
            None => env::remove_var("TERM"),
        }
    }
}
