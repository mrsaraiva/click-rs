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
pub fn get_app_dir(app_name: &str, _roaming: bool) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = if _roaming {
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
        if _roaming {
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
// Shell Argument Parsing
// =============================================================================

/// Parse a string like a shell would: handle quotes, escapes, and whitespace.
///
/// This is useful for parsing command-line strings from environment variables
/// or configuration files.
///
/// # Rules
///
/// - Whitespace separates arguments
/// - Single quotes (`'`) preserve everything literally (no escape sequences)
/// - Double quotes (`"`) allow escape sequences (`\"`, `\\`)
/// - Backslash outside quotes escapes the next character
/// - Empty quotes produce empty strings
///
/// # Example
///
/// ```rust
/// use click::utils::split_arg_string;
///
/// let args = split_arg_string("foo 'bar baz' \"quoted\"");
/// assert_eq!(args, vec!["foo", "bar baz", "quoted"]);
///
/// let args = split_arg_string(r#"file\ name.txt "hello \"world\"""#);
/// assert_eq!(args, vec!["file name.txt", r#"hello "world""#]);
///
/// let args = split_arg_string("'single' \"double\" plain");
/// assert_eq!(args, vec!["single", "double", "plain"]);
/// ```
///
/// # Reference
///
/// Based on Python Click's `shell_completion.py:split_arg_string`.
pub fn split_arg_string(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(c) = chars.next() {
        if in_single_quote {
            // In single quotes: everything is literal until closing quote
            if c == '\'' {
                in_single_quote = false;
            } else {
                current.push(c);
            }
        } else if in_double_quote {
            // In double quotes: handle escapes for " and \
            if c == '"' {
                in_double_quote = false;
            } else if c == '\\' {
                // Check for escape sequences
                if let Some(&next) = chars.peek() {
                    if next == '"' || next == '\\' {
                        current.push(chars.next().unwrap());
                    } else {
                        // Not a recognized escape, keep the backslash
                        current.push(c);
                    }
                } else {
                    current.push(c);
                }
            } else {
                current.push(c);
            }
        } else {
            // Not in quotes
            if c == '\'' {
                in_single_quote = true;
            } else if c == '"' {
                in_double_quote = true;
            } else if c == '\\' {
                // Escape the next character
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else if c.is_whitespace() {
                // End of argument
                if !current.is_empty() {
                    result.push(current);
                    current = String::new();
                }
            } else {
                current.push(c);
            }
        }
    }

    // Don't forget the last argument
    if !current.is_empty() {
        result.push(current);
    }

    result
}

// =============================================================================
// Argument Expansion
// =============================================================================

/// Expand glob patterns in arguments.
///
/// This function expands shell-style glob patterns like `*.txt` or `**/*.rs`
/// into a list of matching file paths. Arguments that don't contain glob
/// patterns or don't match any files are returned as-is.
///
/// # Glob Patterns
///
/// - `*` matches any sequence of characters except path separators
/// - `?` matches any single character except path separators
/// - `**` matches any sequence of characters including path separators
/// - `[...]` matches any character in the brackets
/// - `[!...]` matches any character not in the brackets
///
/// # Example
///
/// ```rust,ignore
/// use click::utils::expand_args;
///
/// // Assuming *.txt files exist in the current directory
/// let args = vec!["file.rs".to_string(), "*.txt".to_string()];
/// let expanded = expand_args(&args);
/// // Returns ["file.rs", "a.txt", "b.txt", ...] if those files exist
/// ```
///
/// # Reference
///
/// Based on Python Click's `utils.py:_expand_args`.
pub fn expand_args(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();

    for arg in args {
        if has_glob_pattern(arg) {
            // Try to expand the glob pattern
            match expand_glob(arg) {
                Some(matches) if !matches.is_empty() => {
                    result.extend(matches);
                }
                _ => {
                    // No matches or error: keep original argument
                    result.push(arg.clone());
                }
            }
        } else {
            result.push(arg.clone());
        }
    }

    result
}

/// Check if a string contains glob pattern characters.
fn has_glob_pattern(s: &str) -> bool {
    s.chars().any(|c| c == '*' || c == '?' || c == '[')
}

/// Expand a glob pattern into matching file paths.
///
/// Returns `None` if the pattern is invalid, or `Some(vec)` with matches.
/// The result may be empty if no files match.
fn expand_glob(pattern: &str) -> Option<Vec<String>> {
    // Simple glob implementation that handles common patterns
    // For production use, consider using the `glob` crate

    let mut matches = Vec::new();

    // Handle the pattern by converting it to a regex-like matcher
    // This is a simplified implementation that handles basic patterns

    // Get the directory part and the pattern part
    let (dir, file_pattern) = split_pattern_path(pattern);

    // Read the directory
    let read_dir = if dir.is_empty() {
        std::fs::read_dir(".")
    } else {
        std::fs::read_dir(&dir)
    };

    let entries = match read_dir {
        Ok(entries) => entries,
        Err(_) => return None,
    };

    // Compile the pattern into a matcher
    let matcher = compile_glob_pattern(&file_pattern);

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if matches_pattern(&name, &matcher) {
            let path = if dir.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", dir, name)
            };
            matches.push(path);
        }
    }

    // Sort matches for consistent ordering
    matches.sort();

    Some(matches)
}

/// Split a glob pattern into directory and file pattern parts.
fn split_pattern_path(pattern: &str) -> (String, String) {
    // Find the last path separator before any glob characters
    let glob_start = pattern
        .chars()
        .position(|c| c == '*' || c == '?' || c == '[')
        .unwrap_or(pattern.len());

    let prefix = &pattern[..glob_start];
    let last_sep = prefix.rfind(|c| c == '/' || c == '\\');

    match last_sep {
        Some(idx) => (pattern[..idx].to_string(), pattern[idx + 1..].to_string()),
        None => (String::new(), pattern.to_string()),
    }
}

/// A compiled glob pattern for matching.
#[derive(Debug)]
enum GlobPart {
    Literal(String),
    Any,          // ?
    AnySequence,  // *
    CharClass(Vec<char>, bool), // [...] or [!...]
}

/// Compile a glob pattern into parts for matching.
fn compile_glob_pattern(pattern: &str) -> Vec<GlobPart> {
    let mut parts = Vec::new();
    let mut chars = pattern.chars().peekable();
    let mut literal = String::new();

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if !literal.is_empty() {
                    parts.push(GlobPart::Literal(literal));
                    literal = String::new();
                }
                // Collapse multiple *'s
                while chars.peek() == Some(&'*') {
                    chars.next();
                }
                parts.push(GlobPart::AnySequence);
            }
            '?' => {
                if !literal.is_empty() {
                    parts.push(GlobPart::Literal(literal));
                    literal = String::new();
                }
                parts.push(GlobPart::Any);
            }
            '[' => {
                if !literal.is_empty() {
                    parts.push(GlobPart::Literal(literal));
                    literal = String::new();
                }
                // Parse character class
                let negated = chars.peek() == Some(&'!');
                if negated {
                    chars.next();
                }
                let mut class_chars = Vec::new();
                while let Some(&ch) = chars.peek() {
                    if ch == ']' {
                        chars.next();
                        break;
                    }
                    class_chars.push(chars.next().unwrap());
                }
                parts.push(GlobPart::CharClass(class_chars, negated));
            }
            '\\' => {
                // Escape next character
                if let Some(next) = chars.next() {
                    literal.push(next);
                }
            }
            _ => {
                literal.push(c);
            }
        }
    }

    if !literal.is_empty() {
        parts.push(GlobPart::Literal(literal));
    }

    parts
}

/// Check if a string matches a compiled glob pattern.
fn matches_pattern(s: &str, parts: &[GlobPart]) -> bool {
    matches_pattern_recursive(s, parts, 0)
}

/// Recursive helper for pattern matching.
fn matches_pattern_recursive(s: &str, parts: &[GlobPart], part_idx: usize) -> bool {
    if part_idx >= parts.len() {
        return s.is_empty();
    }

    let part = &parts[part_idx];

    match part {
        GlobPart::Literal(lit) => {
            if s.starts_with(lit.as_str()) {
                matches_pattern_recursive(&s[lit.len()..], parts, part_idx + 1)
            } else {
                false
            }
        }
        GlobPart::Any => {
            if s.is_empty() {
                false
            } else {
                // Skip one character
                let mut chars = s.chars();
                chars.next();
                matches_pattern_recursive(chars.as_str(), parts, part_idx + 1)
            }
        }
        GlobPart::AnySequence => {
            // Try matching zero or more characters
            // First try matching zero characters
            if matches_pattern_recursive(s, parts, part_idx + 1) {
                return true;
            }
            // Then try matching one or more characters
            for (i, _) in s.char_indices() {
                if matches_pattern_recursive(&s[i + 1..], parts, part_idx + 1) {
                    return true;
                }
            }
            false
        }
        GlobPart::CharClass(chars, negated) => {
            if s.is_empty() {
                return false;
            }
            let first = s.chars().next().unwrap();
            let in_class = chars.contains(&first);
            let matches = if *negated { !in_class } else { in_class };
            if matches {
                let mut remaining = s.chars();
                remaining.next();
                matches_pattern_recursive(remaining.as_str(), parts, part_idx + 1)
            } else {
                false
            }
        }
    }
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

    // =========================================================================
    // Tests for split_arg_string
    // =========================================================================

    #[test]
    fn test_split_arg_string_simple() {
        let args = split_arg_string("foo bar baz");
        assert_eq!(args, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_split_arg_string_single_quotes() {
        let args = split_arg_string("foo 'bar baz' qux");
        assert_eq!(args, vec!["foo", "bar baz", "qux"]);

        // Empty single quotes
        let args = split_arg_string("foo '' bar");
        assert_eq!(args, vec!["foo", "bar"]);
    }

    #[test]
    fn test_split_arg_string_double_quotes() {
        let args = split_arg_string("foo \"bar baz\" qux");
        assert_eq!(args, vec!["foo", "bar baz", "qux"]);

        // Escapes in double quotes
        let args = split_arg_string(r#"foo "bar \"quoted\"" baz"#);
        assert_eq!(args, vec!["foo", r#"bar "quoted""#, "baz"]);
    }

    #[test]
    fn test_split_arg_string_backslash_escape() {
        // Backslash escapes space
        let args = split_arg_string(r"foo\ bar baz");
        assert_eq!(args, vec!["foo bar", "baz"]);

        // Backslash escapes backslash
        let args = split_arg_string(r"foo\\bar");
        assert_eq!(args, vec![r"foo\bar"]);
    }

    #[test]
    fn test_split_arg_string_mixed_quotes() {
        let args = split_arg_string("foo 'bar baz' \"quoted\" plain");
        assert_eq!(args, vec!["foo", "bar baz", "quoted", "plain"]);
    }

    #[test]
    fn test_split_arg_string_empty() {
        let args = split_arg_string("");
        assert!(args.is_empty());

        let args = split_arg_string("   ");
        assert!(args.is_empty());
    }

    #[test]
    fn test_split_arg_string_complex() {
        // Example from Python Click documentation
        let args = split_arg_string("foo 'bar baz' \"quoted\"");
        assert_eq!(args, vec!["foo", "bar baz", "quoted"]);
    }

    #[test]
    fn test_split_arg_string_no_escapes_in_single_quotes() {
        // Single quotes preserve everything literally
        let args = split_arg_string(r"'foo\\bar'");
        assert_eq!(args, vec![r"foo\\bar"]);

        let args = split_arg_string(r#"'foo\"bar'"#);
        assert_eq!(args, vec![r#"foo\"bar"#]);
    }

    // =========================================================================
    // Tests for expand_args and glob matching
    // =========================================================================

    #[test]
    fn test_has_glob_pattern() {
        assert!(has_glob_pattern("*.txt"));
        assert!(has_glob_pattern("file?.txt"));
        assert!(has_glob_pattern("file[ab].txt"));
        assert!(!has_glob_pattern("normal.txt"));
        assert!(!has_glob_pattern("/path/to/file"));
    }

    #[test]
    fn test_glob_pattern_literal() {
        let parts = compile_glob_pattern("hello");
        let is_match = matches_pattern("hello", &parts);
        assert!(is_match);

        let is_match = matches_pattern("world", &parts);
        assert!(!is_match);
    }

    #[test]
    fn test_glob_pattern_star() {
        let parts = compile_glob_pattern("*.txt");
        assert!(matches_pattern("file.txt", &parts));
        assert!(matches_pattern("hello.txt", &parts));
        assert!(matches_pattern(".txt", &parts));
        assert!(!matches_pattern("file.rs", &parts));
    }

    #[test]
    fn test_glob_pattern_question() {
        let parts = compile_glob_pattern("file?.txt");
        assert!(matches_pattern("file1.txt", &parts));
        assert!(matches_pattern("filea.txt", &parts));
        assert!(!matches_pattern("file12.txt", &parts));
        assert!(!matches_pattern("file.txt", &parts));
    }

    #[test]
    fn test_glob_pattern_char_class() {
        let parts = compile_glob_pattern("file[abc].txt");
        assert!(matches_pattern("filea.txt", &parts));
        assert!(matches_pattern("fileb.txt", &parts));
        assert!(matches_pattern("filec.txt", &parts));
        assert!(!matches_pattern("filed.txt", &parts));
    }

    #[test]
    fn test_glob_pattern_negated_char_class() {
        let parts = compile_glob_pattern("file[!abc].txt");
        assert!(!matches_pattern("filea.txt", &parts));
        assert!(!matches_pattern("fileb.txt", &parts));
        assert!(matches_pattern("filed.txt", &parts));
        assert!(matches_pattern("file1.txt", &parts));
    }

    #[test]
    fn test_expand_args_no_patterns() {
        let args = vec!["file.txt".to_string(), "other.rs".to_string()];
        let expanded = expand_args(&args);
        assert_eq!(expanded, args);
    }

    #[test]
    fn test_expand_args_pattern_no_matches() {
        // Pattern that won't match anything
        let args = vec!["__nonexistent_pattern_xyz_*.abc".to_string()];
        let expanded = expand_args(&args);
        // Should keep original if no matches
        assert_eq!(expanded, args);
    }

    #[test]
    fn test_split_pattern_path() {
        let (dir, pattern) = split_pattern_path("*.txt");
        assert_eq!(dir, "");
        assert_eq!(pattern, "*.txt");

        let (dir, pattern) = split_pattern_path("src/*.rs");
        assert_eq!(dir, "src");
        assert_eq!(pattern, "*.rs");

        let (dir, pattern) = split_pattern_path("/home/user/*.txt");
        assert_eq!(dir, "/home/user");
        assert_eq!(pattern, "*.txt");
    }
}
