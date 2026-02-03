//! Terminal UI utilities for click-rs.
//!
//! This module provides terminal input/output functions for building interactive
//! command-line applications. It includes styled output, user prompts, progress bars,
//! and terminal utilities.
//!
//! # Features
//!
//! - **Output Functions**: `echo`, `secho`, and `style` for formatted terminal output
//! - **Input Functions**: `prompt`, `confirm`, `getchar`, and `pause` for user input
//! - **Progress Bars**: Visual progress indication with ETA and percentage
//! - **Terminal Utilities**: Screen clearing, size detection, TTY checks
//!
//! # Example
//!
//! ```rust,ignore
//! use click::termui::{echo, secho, style, prompt, confirm, Color};
//!
//! // Simple output
//! echo("Hello, world!", true, false, None);
//!
//! // Styled output
//! secho("Success!", Some(Color::Green), None, true, false, false, false, false, false, false, true, false, None);
//!
//! // Create a styled string
//! let styled = style("Error", Some(Color::Red), None, true, false, false, false, false, false, false, false);
//!
//! // Prompt for input
//! let name: String = prompt("Enter your name", Some("Anonymous"), false, false, |s| Ok(s.to_string())).unwrap();
//!
//! // Confirmation
//! if confirm("Continue?", Some(true), false).unwrap() {
//!     // proceed
//! }
//! ```

use std::io::{self, BufRead, Write};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Instant;

use crate::error::{ClickError, Result};

// ============================================================================
// Echo Macro
// ============================================================================

/// Convenience macro for printing to the terminal.
///
/// This macro wraps the `echo` function with a simpler syntax.
///
/// # Usage
///
/// ```rust,ignore
/// use click::echo;
///
/// // Simple message with newline
/// echo!("Hello, world!");
///
/// // Without newline
/// echo!("Prompt: ", nl = false);
///
/// // To stderr
/// echo!("Error occurred", err = true);
///
/// // Combined
/// echo!("Warning: ", nl = false, err = true);
///
/// // With formatting
/// echo!("Hello, {}!", name);
/// ```
#[macro_export]
macro_rules! echo {
    // Basic message
    ($msg:expr) => {
        $crate::termui::echo($msg, true, false, None)
    };
    // Message with format args
    ($fmt:expr, $($arg:tt)*) => {{
        // Check if first arg after format is a keyword arg
        echo!(@parse $fmt, $($arg)*)
    }};
    // Parse keyword arguments
    (@parse $fmt:expr, nl = $nl:expr) => {
        $crate::termui::echo($fmt, $nl, false, None)
    };
    (@parse $fmt:expr, err = $err:expr) => {
        $crate::termui::echo($fmt, true, $err, None)
    };
    (@parse $fmt:expr, nl = $nl:expr, err = $err:expr) => {
        $crate::termui::echo($fmt, $nl, $err, None)
    };
    (@parse $fmt:expr, err = $err:expr, nl = $nl:expr) => {
        $crate::termui::echo($fmt, $nl, $err, None)
    };
    (@parse $fmt:expr, color = $color:expr) => {
        $crate::termui::echo($fmt, true, false, $color)
    };
    (@parse $fmt:expr, nl = $nl:expr, color = $color:expr) => {
        $crate::termui::echo($fmt, $nl, false, $color)
    };
    (@parse $fmt:expr, err = $err:expr, color = $color:expr) => {
        $crate::termui::echo($fmt, true, $err, $color)
    };
    (@parse $fmt:expr, nl = $nl:expr, err = $err:expr, color = $color:expr) => {
        $crate::termui::echo($fmt, $nl, $err, $color)
    };
    // Format string with args (no keyword args)
    (@parse $fmt:expr, $($arg:tt)*) => {
        $crate::termui::echo(&format!($fmt, $($arg)*), true, false, None)
    };
}

// ============================================================================
// Color Constants
// ============================================================================

/// Terminal colors for styled output.
///
/// These colors correspond to standard ANSI terminal colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Black (ANSI code 30/40)
    Black,
    /// Red (ANSI code 31/41)
    Red,
    /// Green (ANSI code 32/42)
    Green,
    /// Yellow (ANSI code 33/43)
    Yellow,
    /// Blue (ANSI code 34/44)
    Blue,
    /// Magenta (ANSI code 35/45)
    Magenta,
    /// Cyan (ANSI code 36/46)
    Cyan,
    /// White (ANSI code 37/47)
    White,
    /// Bright Black (ANSI code 90/100)
    BrightBlack,
    /// Bright Red (ANSI code 91/101)
    BrightRed,
    /// Bright Green (ANSI code 92/102)
    BrightGreen,
    /// Bright Yellow (ANSI code 93/103)
    BrightYellow,
    /// Bright Blue (ANSI code 94/104)
    BrightBlue,
    /// Bright Magenta (ANSI code 95/105)
    BrightMagenta,
    /// Bright Cyan (ANSI code 96/106)
    BrightCyan,
    /// Bright White (ANSI code 97/107)
    BrightWhite,
    /// Reset color to default
    Reset,
}

impl Color {
    /// Get the ANSI foreground color code.
    pub fn fg_code(self) -> u8 {
        match self {
            Color::Black => 30,
            Color::Red => 31,
            Color::Green => 32,
            Color::Yellow => 33,
            Color::Blue => 34,
            Color::Magenta => 35,
            Color::Cyan => 36,
            Color::White => 37,
            Color::BrightBlack => 90,
            Color::BrightRed => 91,
            Color::BrightGreen => 92,
            Color::BrightYellow => 93,
            Color::BrightBlue => 94,
            Color::BrightMagenta => 95,
            Color::BrightCyan => 96,
            Color::BrightWhite => 97,
            Color::Reset => 39,
        }
    }

    /// Get the ANSI background color code.
    pub fn bg_code(self) -> u8 {
        match self {
            Color::Black => 40,
            Color::Red => 41,
            Color::Green => 42,
            Color::Yellow => 43,
            Color::Blue => 44,
            Color::Magenta => 45,
            Color::Cyan => 46,
            Color::White => 47,
            Color::BrightBlack => 100,
            Color::BrightRed => 101,
            Color::BrightGreen => 102,
            Color::BrightYellow => 103,
            Color::BrightBlue => 104,
            Color::BrightMagenta => 105,
            Color::BrightCyan => 106,
            Color::BrightWhite => 107,
            Color::Reset => 49,
        }
    }
}

// Color constants for convenience (Python Click compatibility)
pub const BLACK: Color = Color::Black;
pub const RED: Color = Color::Red;
pub const GREEN: Color = Color::Green;
pub const YELLOW: Color = Color::Yellow;
pub const BLUE: Color = Color::Blue;
pub const MAGENTA: Color = Color::Magenta;
pub const CYAN: Color = Color::Cyan;
pub const WHITE: Color = Color::White;
pub const BRIGHT_BLACK: Color = Color::BrightBlack;
pub const BRIGHT_RED: Color = Color::BrightRed;
pub const BRIGHT_GREEN: Color = Color::BrightGreen;
pub const BRIGHT_YELLOW: Color = Color::BrightYellow;
pub const BRIGHT_BLUE: Color = Color::BrightBlue;
pub const BRIGHT_MAGENTA: Color = Color::BrightMagenta;
pub const BRIGHT_CYAN: Color = Color::BrightCyan;
pub const BRIGHT_WHITE: Color = Color::BrightWhite;
pub const RESET: Color = Color::Reset;

// ============================================================================
// Terminal Detection
// ============================================================================

/// Check if a file descriptor refers to a TTY.
///
/// This checks if the given stream is connected to an interactive terminal.
///
/// # Arguments
///
/// * `stream` - The stream to check: "stdout", "stderr", or "stdin"
///
/// # Returns
///
/// `true` if the stream is a TTY, `false` otherwise.
///
/// # Note
///
/// This implementation uses environment variable heuristics for portability.
/// For more accurate detection, consider using the `atty` or `is-terminal` crate.
pub fn isatty(stream: &str) -> bool {
    // Check for common CI/non-interactive environment variables
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        return false;
    }

    // Check for explicit TERM settings
    if let Ok(term) = std::env::var("TERM") {
        if term == "dumb" {
            return false;
        }
        // If TERM is set to something reasonable, likely a TTY
        if !term.is_empty() {
            return true;
        }
    }

    // Check for common terminal programs
    if std::env::var("TERM_PROGRAM").is_ok() {
        return true;
    }

    // Check for TTY-related environment
    if std::env::var("TTY").is_ok() || std::env::var("SSH_TTY").is_ok() {
        return true;
    }

    // Platform-specific checks
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;

        // Use the isatty syscall wrapper
        extern "C" {
            fn isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int;
        }

        let fd = match stream {
            "stdin" => std::io::stdin().as_raw_fd(),
            "stdout" => std::io::stdout().as_raw_fd(),
            "stderr" => std::io::stderr().as_raw_fd(),
            _ => return false,
        };

        unsafe { isatty(fd) != 0 }
    }

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;

        #[link(name = "kernel32")]
        extern "system" {
            fn GetConsoleMode(
                hConsoleHandle: *mut std::ffi::c_void,
                lpMode: *mut u32,
            ) -> std::os::raw::c_int;
        }

        let handle = match stream {
            "stdin" => std::io::stdin().as_raw_handle(),
            "stdout" => std::io::stdout().as_raw_handle(),
            "stderr" => std::io::stderr().as_raw_handle(),
            _ => return false,
        };

        let mut mode: u32 = 0;
        unsafe { GetConsoleMode(handle as *mut _, &mut mode) != 0 }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = stream;
        false
    }
}

/// Check if stdout is connected to a TTY.
pub fn stdout_isatty() -> bool {
    isatty("stdout")
}

/// Check if stderr is connected to a TTY.
pub fn stderr_isatty() -> bool {
    isatty("stderr")
}

/// Check if stdin is connected to a TTY.
pub fn stdin_isatty() -> bool {
    isatty("stdin")
}

/// Get the terminal size as (width, height).
///
/// Returns a default of (80, 24) if detection fails or if not connected to a TTY.
///
/// # Returns
///
/// A tuple of (width, height) in characters.
pub fn get_terminal_size() -> (usize, usize) {
    // Try environment variables first (portable)
    if let (Ok(cols), Ok(rows)) = (std::env::var("COLUMNS"), std::env::var("LINES")) {
        if let (Ok(w), Ok(h)) = (cols.parse::<usize>(), rows.parse::<usize>()) {
            if w > 0 && h > 0 {
                return (w, h);
            }
        }
    }

    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;
        use std::os::unix::io::AsRawFd;

        #[repr(C)]
        struct Winsize {
            ws_row: u16,
            ws_col: u16,
            ws_xpixel: u16,
            ws_ypixel: u16,
        }

        // TIOCGWINSZ value varies by platform
        #[cfg(target_os = "linux")]
        const TIOCGWINSZ: std::os::raw::c_ulong = 0x5413;
        #[cfg(target_os = "macos")]
        const TIOCGWINSZ: std::os::raw::c_ulong = 0x40087468;
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        const TIOCGWINSZ: std::os::raw::c_ulong = 0x5413; // Default to Linux

        extern "C" {
            fn ioctl(fd: std::os::raw::c_int, request: std::os::raw::c_ulong, ...) -> std::os::raw::c_int;
        }

        let fd = std::io::stdout().as_raw_fd();
        let mut ws = MaybeUninit::<Winsize>::uninit();

        unsafe {
            if ioctl(fd, TIOCGWINSZ, ws.as_mut_ptr()) == 0 {
                let ws = ws.assume_init();
                if ws.ws_col > 0 && ws.ws_row > 0 {
                    return (ws.ws_col as usize, ws.ws_row as usize);
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;

        #[repr(C)]
        struct Coord {
            x: i16,
            y: i16,
        }

        #[repr(C)]
        struct SmallRect {
            left: i16,
            top: i16,
            right: i16,
            bottom: i16,
        }

        #[repr(C)]
        struct ConsoleScreenBufferInfo {
            dw_size: Coord,
            dw_cursor_position: Coord,
            w_attributes: u16,
            sr_window: SmallRect,
            dw_maximum_window_size: Coord,
        }

        #[link(name = "kernel32")]
        extern "system" {
            fn GetConsoleScreenBufferInfo(
                hConsoleOutput: *mut std::ffi::c_void,
                lpConsoleScreenBufferInfo: *mut ConsoleScreenBufferInfo,
            ) -> std::os::raw::c_int;
        }

        let handle = std::io::stdout().as_raw_handle();
        let mut csbi = std::mem::MaybeUninit::<ConsoleScreenBufferInfo>::uninit();

        unsafe {
            if GetConsoleScreenBufferInfo(handle as *mut _, csbi.as_mut_ptr()) != 0 {
                let csbi = csbi.assume_init();
                let width = (csbi.sr_window.right - csbi.sr_window.left + 1) as usize;
                let height = (csbi.sr_window.bottom - csbi.sr_window.top + 1) as usize;
                if width > 0 && height > 0 {
                    return (width, height);
                }
            }
        }
    }

    // Default fallback
    (80, 24)
}

/// Clear the terminal screen.
///
/// Uses ANSI escape sequences to clear the screen and move cursor to home.
/// On non-TTY outputs, this is a no-op.
pub fn clear() {
    if stdout_isatty() {
        // ANSI escape: clear screen and move to home
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }
}

// ============================================================================
// Styling Functions
// ============================================================================

/// Determine if ANSI codes should be used.
///
/// Returns true if:
/// - `color` is explicitly `Some(true)`
/// - `color` is `None` and the output stream is a TTY
fn should_use_color(color: Option<bool>, err: bool) -> bool {
    match color {
        Some(true) => true,
        Some(false) => false,
        None => {
            if err {
                stderr_isatty()
            } else {
                stdout_isatty()
            }
        }
    }
}

/// Style text with ANSI escape codes.
///
/// Creates a styled string with the specified formatting. If the terminal
/// doesn't support colors or formatting is disabled, returns the text unchanged.
///
/// # Arguments
///
/// * `text` - The text to style
/// * `fg` - Foreground color
/// * `bg` - Background color
/// * `bold` - Bold text
/// * `dim` - Dim (faint) text
/// * `underline` - Underlined text
/// * `overline` - Overlined text (not widely supported)
/// * `italic` - Italic text
/// * `blink` - Blinking text
/// * `strikethrough` - Strikethrough text
/// * `reset` - Whether to reset all styles at the end
///
/// # Returns
///
/// The styled string with ANSI escape codes.
#[allow(clippy::too_many_arguments)]
pub fn style(
    text: &str,
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    dim: bool,
    underline: bool,
    overline: bool,
    italic: bool,
    blink: bool,
    strikethrough: bool,
    reset: bool,
) -> String {
    let mut codes = Vec::new();

    // Collect style codes
    if bold {
        codes.push("1".to_string());
    }
    if dim {
        codes.push("2".to_string());
    }
    if italic {
        codes.push("3".to_string());
    }
    if underline {
        codes.push("4".to_string());
    }
    if blink {
        codes.push("5".to_string());
    }
    if overline {
        codes.push("53".to_string()); // ANSI overline
    }
    if strikethrough {
        codes.push("9".to_string());
    }

    // Add colors
    if let Some(color) = fg {
        codes.push(color.fg_code().to_string());
    }
    if let Some(color) = bg {
        codes.push(color.bg_code().to_string());
    }

    if codes.is_empty() {
        return text.to_string();
    }

    let style_start = format!("\x1b[{}m", codes.join(";"));
    let style_end = if reset { "\x1b[0m" } else { "" };

    format!("{}{}{}", style_start, text, style_end)
}

/// Print a message to the terminal.
///
/// # Arguments
///
/// * `message` - The message to print
/// * `nl` - Whether to append a newline (default: true)
/// * `err` - Whether to print to stderr instead of stdout
/// * `color` - Force color on/off, or None to auto-detect
pub fn echo(message: &str, nl: bool, err: bool, color: Option<bool>) {
    let output = if should_use_color(color, err) {
        message.to_string()
    } else {
        // Strip ANSI codes if color is disabled
        strip_ansi_codes(message)
    };

    if err {
        if nl {
            eprintln!("{}", output);
            let _ = io::stderr().flush();
        } else {
            eprint!("{}", output);
            let _ = io::stderr().flush();
        }
    } else if nl {
        println!("{}", output);
        let _ = io::stdout().flush();
    } else {
        print!("{}", output);
        let _ = io::stdout().flush();
    }
}

/// Print a styled message to the terminal.
///
/// This is a convenience function that combines `style` and `echo`.
///
/// # Arguments
///
/// * `message` - The message to print
/// * `fg` - Foreground color
/// * `bg` - Background color
/// * `bold` - Bold text
/// * `dim` - Dim text
/// * `underline` - Underlined text
/// * `overline` - Overlined text
/// * `italic` - Italic text
/// * `blink` - Blinking text
/// * `strikethrough` - Strikethrough text
/// * `reset` - Whether to reset styles at end
/// * `nl` - Whether to append a newline
/// * `err` - Whether to print to stderr
/// * `color` - Force color on/off, or None to auto-detect
#[allow(clippy::too_many_arguments)]
pub fn secho(
    message: &str,
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    dim: bool,
    underline: bool,
    overline: bool,
    italic: bool,
    blink: bool,
    strikethrough: bool,
    reset: bool,
    nl: bool,
    err: bool,
    color: Option<bool>,
) {
    let styled = if should_use_color(color, err) {
        style(
            message,
            fg,
            bg,
            bold,
            dim,
            underline,
            overline,
            italic,
            blink,
            strikethrough,
            reset,
        )
    } else {
        message.to_string()
    };

    if err {
        if nl {
            eprintln!("{}", styled);
        } else {
            eprint!("{}", styled);
            let _ = io::stderr().flush();
        }
    } else if nl {
        println!("{}", styled);
    } else {
        print!("{}", styled);
        let _ = io::stdout().flush();
    }
}

// ============================================================================
// Pager Support
// ============================================================================

/// Display text via a pager.
///
/// Pipes the given text to a pager program ($PAGER, less, or more).
/// Falls back to `echo()` if no pager is available or if not connected to a TTY.
///
/// # Arguments
///
/// * `text` - The text to display
/// * `color` - Whether to preserve ANSI color codes (None = auto-detect)
///
/// # Example
///
/// ```rust,ignore
/// use click::termui::echo_via_pager;
///
/// let long_text = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
/// echo_via_pager(&long_text, None);
/// ```
pub fn echo_via_pager(text: &str, color: Option<bool>) {
    // If not a TTY, just echo the text
    if !stdin_isatty() || !stdout_isatty() {
        echo(text, true, false, color);
        return;
    }

    // Determine the pager command
    let pager = std::env::var("PAGER")
        .ok()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| {
            // Try to find less or more
            if which_pager("less").is_some() {
                "less".to_string()
            } else if which_pager("more").is_some() {
                "more".to_string()
            } else {
                String::new()
            }
        });

    if pager.is_empty() {
        // No pager available, fall back to echo
        echo(text, true, false, color);
        return;
    }

    // Prepare the text (strip ANSI codes if color is disabled)
    let output_text = if color == Some(false) {
        strip_ansi_codes(text)
    } else {
        text.to_string()
    };

    // Build the pager command with proper arguments
    let mut parts = pager.split_whitespace();
    let cmd_name = match parts.next() {
        Some(name) => name,
        None => {
            echo(&output_text, true, false, color);
            return;
        }
    };

    let mut cmd = ProcessCommand::new(cmd_name);

    // Add any additional arguments from $PAGER
    for arg in parts {
        cmd.arg(arg);
    }

    // If using less and color is enabled, add -R flag for raw control characters
    if cmd_name == "less" && color != Some(false) {
        cmd.arg("-R");
    }

    // Try to spawn the pager and pipe text to it
    match cmd.stdin(Stdio::piped()).spawn() {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(output_text.as_bytes());
            }
            // Wait for pager to finish
            let _ = child.wait();
        }
        Err(_) => {
            // Pager failed to start, fall back to echo
            echo(&output_text, true, false, color);
        }
    }
}

/// Check if a pager command exists in PATH.
fn which_pager(name: &str) -> Option<String> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let full_path = std::path::Path::new(dir).join(name);
            if full_path.exists() {
                return Some(full_path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

// ============================================================================
// Launch Support
// ============================================================================

/// Open a URL or file path in the default application.
///
/// Uses platform-specific commands to launch the associated application:
/// - macOS: `open`
/// - Linux: `xdg-open`
/// - Windows: `start`
///
/// # Arguments
///
/// * `url` - The URL or file path to open
/// * `wait` - If true, wait for the application to finish before returning
/// * `locate` - If true, open a file manager showing the file location instead
///
/// # Returns
///
/// `Ok(())` on success, or an error if the launch failed.
///
/// # Example
///
/// ```rust,ignore
/// use click::termui::launch;
///
/// // Open a URL in the default browser
/// launch("https://example.com", false, false)?;
///
/// // Open a file in its default application
/// launch("/path/to/document.pdf", false, false)?;
///
/// // Show file location in file manager
/// launch("/path/to/file.txt", false, true)?;
/// ```
pub fn launch(url: &str, wait: bool, locate: bool) -> Result<()> {
    let (cmd, args) = get_launch_command(url, locate)?;

    let mut command = ProcessCommand::new(&cmd);
    command.args(&args);

    if wait {
        let status = command.status().map_err(|e| {
            ClickError::usage(format!("Failed to launch '{}': {}", url, e))
        })?;

        if !status.success() {
            return Err(ClickError::usage(format!(
                "Launch command failed with exit code: {:?}",
                status.code()
            )));
        }
    } else {
        // Spawn without waiting
        command.spawn().map_err(|e| {
            ClickError::usage(format!("Failed to launch '{}': {}", url, e))
        })?;
    }

    Ok(())
}

/// Get the platform-specific launch command and arguments.
fn get_launch_command(url: &str, locate: bool) -> Result<(String, Vec<String>)> {
    #[cfg(target_os = "macos")]
    {
        if locate {
            Ok(("open".to_string(), vec!["-R".to_string(), url.to_string()]))
        } else {
            Ok(("open".to_string(), vec![url.to_string()]))
        }
    }

    #[cfg(target_os = "linux")]
    {
        if locate {
            // Try to use a file manager that supports revealing files
            // First try dbus with nautilus/dolphin, fall back to xdg-open on parent dir
            let path = std::path::Path::new(url);
            if let Some(parent) = path.parent() {
                Ok((
                    "xdg-open".to_string(),
                    vec![parent.to_string_lossy().into_owned()],
                ))
            } else {
                Err(ClickError::usage(format!(
                    "Cannot locate file: {}",
                    url
                )))
            }
        } else {
            Ok(("xdg-open".to_string(), vec![url.to_string()]))
        }
    }

    #[cfg(target_os = "windows")]
    {
        if locate {
            // Use explorer with /select to highlight the file
            Ok((
                "explorer".to_string(),
                vec!["/select,".to_string() + url],
            ))
        } else {
            // Use cmd /c start for URLs and files
            Ok((
                "cmd".to_string(),
                vec!["/c".to_string(), "start".to_string(), "".to_string(), url.to_string()],
            ))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = locate; // suppress unused warning
        Err(ClickError::usage(format!(
            "Platform not supported for launch: {}",
            url
        )))
    }
}

/// Strip ANSI escape codes from a string.
///
/// # Arguments
///
/// * `text` - The text to strip
///
/// # Returns
///
/// The text with all ANSI escape codes removed.
pub fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip the escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter (end of sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

// ============================================================================
// Input Functions
// ============================================================================

/// Prompt the user for input with optional type conversion.
///
/// # Arguments
///
/// * `text` - The prompt text to display
/// * `default` - Optional default value if user presses Enter
/// * `hide_input` - Whether to hide user input (for passwords)
/// * `confirmation` - Whether to prompt twice and require matching input
/// * `type_converter` - Function to convert and validate the input
///
/// # Returns
///
/// The converted user input, or an error.
///
/// # Notes
///
/// Hidden input requires terminal raw mode. If raw mode is unavailable,
/// input will be visible with a warning.
pub fn prompt<T, F>(
    text: &str,
    default: Option<T>,
    hide_input: bool,
    confirmation: bool,
    type_converter: F,
) -> Result<T>
where
    T: Clone + std::fmt::Display,
    F: Fn(&str) -> std::result::Result<T, String>,
{
    loop {
        // Build prompt string
        let prompt_text = if let Some(ref def) = default {
            format!("{} [{}]: ", text, def)
        } else {
            format!("{}: ", text)
        };

        // Read input
        let input = if hide_input {
            read_hidden_input(&prompt_text)?
        } else {
            read_line(&prompt_text)?
        };

        // Handle empty input with default
        let value = if input.is_empty() {
            if let Some(def) = default.clone() {
                return Ok(def);
            } else {
                echo("Error: This field is required.", true, true, None);
                continue;
            }
        } else {
            input
        };

        // Convert value
        let converted = match type_converter(&value) {
            Ok(v) => v,
            Err(msg) => {
                echo(&format!("Error: {}", msg), true, true, None);
                continue;
            }
        };

        // Handle confirmation
        if confirmation {
            let confirm_prompt = "Repeat for confirmation: ".to_string();
            let confirm_input = if hide_input {
                read_hidden_input(&confirm_prompt)?
            } else {
                read_line(&confirm_prompt)?
            };

            if confirm_input != value {
                echo(
                    "Error: The two entered values do not match.",
                    true,
                    true,
                    None,
                );
                continue;
            }
        }

        return Ok(converted);
    }
}

/// Prompt for yes/no confirmation.
///
/// # Arguments
///
/// * `text` - The prompt text to display
/// * `default` - Optional default value (true=yes, false=no)
/// * `abort` - Whether to raise Abort error on "no" answer
///
/// # Returns
///
/// `true` if user answered yes, `false` if no.
/// Returns `Err(ClickError::Abort)` if `abort` is true and user answered no.
pub fn confirm(text: &str, default: Option<bool>, abort: bool) -> Result<bool> {
    let suffix = match default {
        Some(true) => " [Y/n]: ",
        Some(false) => " [y/N]: ",
        None => " [y/n]: ",
    };

    loop {
        let prompt_text = format!("{}{}", text, suffix);
        let input = read_line(&prompt_text)?;
        let input_lower = input.to_lowercase();

        let result = if input.is_empty() {
            default
        } else if input_lower == "y" || input_lower == "yes" {
            Some(true)
        } else if input_lower == "n" || input_lower == "no" {
            Some(false)
        } else {
            echo("Error: invalid input", true, true, None);
            continue;
        };

        match result {
            Some(true) => return Ok(true),
            Some(false) => {
                if abort {
                    return Err(ClickError::Abort);
                }
                return Ok(false);
            }
            None => {
                echo("Error: invalid input", true, true, None);
                continue;
            }
        }
    }
}

/// Read a single character from the terminal.
///
/// # Arguments
///
/// * `echo_char` - Whether to echo the character back to the terminal
///
/// # Returns
///
/// The character read from the terminal.
///
/// # Notes
///
/// This function attempts to use raw mode for immediate character reading.
/// If raw mode is unavailable, it falls back to reading a line and returning
/// the first character.
pub fn getchar(echo_char: bool) -> Result<char> {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Termios {
            c_iflag: u32,
            c_oflag: u32,
            c_cflag: u32,
            c_lflag: u32,
            c_cc: [u8; 32],
            c_ispeed: u32,
            c_ospeed: u32,
        }

        const ICANON: u32 = 0o000002;
        const ECHO: u32 = 0o000010;
        const VMIN: usize = 6;
        const VTIME: usize = 5;
        const TCSANOW: i32 = 0;

        extern "C" {
            fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
            fn tcsetattr(fd: i32, action: i32, termios: *const Termios) -> i32;
            fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
        }

        let stdin_fd = std::io::stdin().as_raw_fd();
        let mut old_termios = std::mem::MaybeUninit::<Termios>::uninit();

        unsafe {
            if tcgetattr(stdin_fd, old_termios.as_mut_ptr()) == 0 {
                let old_termios = old_termios.assume_init();
                let mut new_termios = old_termios;

                // Disable canonical mode and echo
                new_termios.c_lflag &= !(ICANON | ECHO);
                new_termios.c_cc[VMIN] = 1;
                new_termios.c_cc[VTIME] = 0;

                if tcsetattr(stdin_fd, TCSANOW, &new_termios) == 0 {
                    // Read a single byte
                    let mut buf = [0u8; 1];
                    let result = read(stdin_fd, buf.as_mut_ptr(), 1);

                    // Restore terminal attributes
                    tcsetattr(stdin_fd, TCSANOW, &old_termios);

                    if result == 1 {
                        let c = buf[0] as char;
                        if echo_char {
                            print!("{}", c);
                            let _ = io::stdout().flush();
                        }
                        return Ok(c);
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{BOOL, HANDLE, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Console::{
            GetConsoleMode, GetStdHandle, ReadConsoleInputW, SetConsoleMode, INPUT_RECORD,
            ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, KEY_EVENT, STD_INPUT_HANDLE,
        };

        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if handle != 0 && handle != INVALID_HANDLE_VALUE {
            let mut mode: u32 = 0;
            unsafe {
                if GetConsoleMode(handle, &mut mode) != 0 {
                    let new_mode = mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
                    if SetConsoleMode(handle, new_mode) != 0 {
                        struct RestoreConsoleMode {
                            handle: HANDLE,
                            mode: u32,
                        }

                        impl Drop for RestoreConsoleMode {
                            fn drop(&mut self) {
                                unsafe {
                                    let _ = SetConsoleMode(self.handle, self.mode);
                                }
                            }
                        }

                        let _restore = RestoreConsoleMode { handle, mode };

                        loop {
                            let mut rec = std::mem::MaybeUninit::<INPUT_RECORD>::uninit();
                            let mut read: u32 = 0;
                            let ok: BOOL =
                                ReadConsoleInputW(handle, rec.as_mut_ptr(), 1, &mut read);
                            if ok == 0 {
                                break;
                            }
                            if read == 0 {
                                continue;
                            }

                            let rec = rec.assume_init();
                            if rec.EventType as u32 != KEY_EVENT {
                                continue;
                            }

                            let key_event = rec.Event.KeyEvent;
                            if key_event.bKeyDown == 0 {
                                continue;
                            }

                            let u: u16 = key_event.uChar.UnicodeChar;
                            if u == 0 {
                                continue;
                            }

                            if let Some(c) = char::from_u32(u as u32) {
                                if echo_char {
                                    print!("{}", c);
                                    let _ = io::stdout().flush();
                                }
                                return Ok(c);
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: read a line and return the first character
    let input = read_line("")?;
    input.chars().next().ok_or(ClickError::Abort)
}

/// Pause until the user presses any key.
///
/// # Arguments
///
/// * `info` - Optional message to display (default: "Press any key to continue...")
pub fn pause(info: Option<&str>) {
    let message = info.unwrap_or("Press any key to continue...");
    echo(message, false, false, None);

    // Try to read a single character
    let _ = getchar(false);

    // Print newline
    println!();
}

/// Read a line of input from stdin.
fn read_line(prompt: &str) -> Result<String> {
    if !prompt.is_empty() {
        print!("{}", prompt);
        let _ = io::stdout().flush();
    }

    let stdin = io::stdin();
    let mut line = String::new();

    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| ClickError::usage(format!("Failed to read input: {}", e)))?;

    // Trim the trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }

    Ok(line)
}

/// Read hidden input (for passwords).
fn read_hidden_input(prompt: &str) -> Result<String> {
    if !prompt.is_empty() {
        print!("{}", prompt);
        let _ = io::stdout().flush();
    }

    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Termios {
            c_iflag: u32,
            c_oflag: u32,
            c_cflag: u32,
            c_lflag: u32,
            c_cc: [u8; 32],
            c_ispeed: u32,
            c_ospeed: u32,
        }

        const ECHO: u32 = 0o000010;
        const TCSANOW: i32 = 0;

        extern "C" {
            fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
            fn tcsetattr(fd: i32, action: i32, termios: *const Termios) -> i32;
        }

        let stdin_fd = std::io::stdin().as_raw_fd();
        let mut old_termios = std::mem::MaybeUninit::<Termios>::uninit();

        unsafe {
            if tcgetattr(stdin_fd, old_termios.as_mut_ptr()) == 0 {
                let old_termios = old_termios.assume_init();
                let mut new_termios = old_termios;

                // Disable echo
                new_termios.c_lflag &= !ECHO;

                if tcsetattr(stdin_fd, TCSANOW, &new_termios) == 0 {
                    // Read the line
                    let result = read_line("");

                    // Restore terminal
                    tcsetattr(stdin_fd, TCSANOW, &old_termios);

                    // Print newline since echo was disabled
                    println!();

                    return result;
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Console::{
            GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_ECHO_INPUT, STD_INPUT_HANDLE,
        };

        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if handle != 0 && handle != INVALID_HANDLE_VALUE {
            let mut mode: u32 = 0;
            unsafe {
                if GetConsoleMode(handle, &mut mode) != 0 {
                    let new_mode = mode & !ENABLE_ECHO_INPUT;
                    if SetConsoleMode(handle, new_mode) != 0 {
                        struct RestoreConsoleMode {
                            handle: HANDLE,
                            mode: u32,
                        }

                        impl Drop for RestoreConsoleMode {
                            fn drop(&mut self) {
                                unsafe {
                                    let _ = SetConsoleMode(self.handle, self.mode);
                                }
                            }
                        }

                        let _restore = RestoreConsoleMode { handle, mode };

                        let result = read_line("");

                        // Print newline since echo was disabled
                        println!();

                        return result;
                    }
                }
            }
        }
    }

    // Fallback: warn user and read normally
    echo("(Warning: Input will be visible)", true, true, None);
    read_line("")
}

// ============================================================================
// Progress Bar
// ============================================================================

/// A progress bar for displaying operation progress.
///
/// # Example
///
/// ```rust,ignore
/// use click::termui::ProgressBar;
///
/// let mut bar = ProgressBar::new(100, Some("Processing"), true, true, true, 40);
///
/// for i in 0..100 {
///     // Do work...
///     bar.update(1);
/// }
///
/// bar.finish();
/// ```
pub struct ProgressBar {
    /// Total length of the progress (number of items)
    length: usize,
    /// Current position
    position: usize,
    /// Optional label to display
    label: Option<String>,
    /// Whether to show ETA
    show_eta: bool,
    /// Whether to show percentage
    show_percent: bool,
    /// Whether to show position/length
    show_pos: bool,
    /// Width of the progress bar in characters
    width: usize,
    /// Start time for ETA calculation
    start_time: Instant,
    /// Whether the bar is finished
    finished: bool,
    /// Whether output is a TTY
    is_tty: bool,
    /// Last rendered output length (for TTY updates)
    #[allow(dead_code)]
    last_output_len: usize,
}

impl ProgressBar {
    /// Create a new progress bar.
    ///
    /// # Arguments
    ///
    /// * `length` - Total number of items to process
    /// * `label` - Optional label to display before the bar
    /// * `show_eta` - Whether to show estimated time remaining
    /// * `show_percent` - Whether to show percentage complete
    /// * `show_pos` - Whether to show position/length
    /// * `width` - Width of the bar portion in characters
    pub fn new(
        length: usize,
        label: Option<&str>,
        show_eta: bool,
        show_percent: bool,
        show_pos: bool,
        width: usize,
    ) -> Self {
        let bar = Self {
            length,
            position: 0,
            label: label.map(String::from),
            show_eta,
            show_percent,
            show_pos,
            width,
            start_time: Instant::now(),
            finished: false,
            is_tty: stdout_isatty(),
            last_output_len: 0,
        };

        // Initial render
        bar.render_internal();
        bar
    }

    /// Update the progress bar by advancing by `n` items.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of items completed since last update
    pub fn update(&mut self, n: usize) {
        if self.finished {
            return;
        }

        self.position = (self.position + n).min(self.length);
        self.render_internal();
    }

    /// Set the progress bar to a specific position.
    ///
    /// # Arguments
    ///
    /// * `pos` - New position
    pub fn set_position(&mut self, pos: usize) {
        if self.finished {
            return;
        }

        self.position = pos.min(self.length);
        self.render_internal();
    }

    /// Mark the progress bar as finished and render final state.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }

        self.position = self.length;
        self.finished = true;
        self.render_internal();

        // Print newline
        if self.is_tty {
            println!();
        }
    }

    /// Render the current progress bar state to a string.
    pub fn render(&self) -> String {
        let mut parts = Vec::new();

        // Label
        if let Some(ref label) = self.label {
            parts.push(label.clone());
        }

        // Calculate progress
        let progress = if self.length > 0 {
            self.position as f64 / self.length as f64
        } else {
            0.0
        };

        // Progress bar
        let filled = (progress * self.width as f64) as usize;
        let empty = self.width.saturating_sub(filled);
        let bar = format!("[{}{}]", "#".repeat(filled), "-".repeat(empty));
        parts.push(bar);

        // Percentage
        if self.show_percent {
            parts.push(format!("{:3.0}%", progress * 100.0));
        }

        // Position / Length
        if self.show_pos {
            parts.push(format!("{}/{}", self.position, self.length));
        }

        // ETA
        if self.show_eta && self.position > 0 && !self.finished {
            let elapsed = self.start_time.elapsed();
            let rate = self.position as f64 / elapsed.as_secs_f64();
            let remaining = self.length - self.position;
            let eta_secs = if rate > 0.0 {
                remaining as f64 / rate
            } else {
                0.0
            };

            if eta_secs < 3600.0 {
                let mins = (eta_secs / 60.0) as u64;
                let secs = (eta_secs % 60.0) as u64;
                parts.push(format!("eta {:02}:{:02}", mins, secs));
            } else {
                let hours = (eta_secs / 3600.0) as u64;
                let mins = ((eta_secs % 3600.0) / 60.0) as u64;
                parts.push(format!("eta {}h {:02}m", hours, mins));
            }
        }

        parts.join(" ")
    }

    /// Internal render method that updates the terminal.
    fn render_internal(&self) {
        let output = self.render();

        if self.is_tty {
            // Carriage return to beginning of line
            print!("\r{}", output);
            // Clear any remaining characters from previous output
            let clear_len = self.last_output_len.saturating_sub(output.len());
            if clear_len > 0 {
                print!("{}", " ".repeat(clear_len));
                print!("\r{}", output);
            }
            let _ = io::stdout().flush();
        } else {
            // Non-TTY: print on new lines
            println!("{}", output);
        }
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        if !self.finished && self.is_tty {
            // Ensure we leave the terminal in a clean state
            println!();
        }
    }
}

/// Wrap an iterator with a progress bar display.
///
/// # Arguments
///
/// * `iter` - The iterator to wrap
/// * `length` - Total length (for percentage/ETA), or None to infer from iterator
/// * `label` - Optional label to display
///
/// # Returns
///
/// An iterator that displays progress as items are consumed.
///
/// # Example
///
/// ```rust,ignore
/// use click::termui::progressbar;
///
/// for item in progressbar(items.iter(), Some(items.len()), Some("Processing")) {
///     // Process item
/// }
/// ```
pub fn progressbar<I>(
    iter: I,
    length: Option<usize>,
    label: Option<&str>,
) -> ProgressBarIter<I::IntoIter>
where
    I: IntoIterator,
    I::IntoIter: ExactSizeIterator,
{
    let iter = iter.into_iter();
    let len = length.unwrap_or_else(|| iter.len());

    ProgressBarIter {
        iter,
        bar: ProgressBar::new(len, label, true, true, true, 30),
    }
}

/// An iterator wrapper that displays a progress bar.
pub struct ProgressBarIter<I> {
    iter: I,
    bar: ProgressBar,
}

impl<I> Iterator for ProgressBarIter<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(item) => {
                self.bar.update(1);
                Some(item)
            }
            None => {
                self.bar.finish();
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for ProgressBarIter<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

// ============================================================================
// Editor Support
// ============================================================================

/// Open a text editor for the user to edit content.
///
/// # Arguments
///
/// * `text` - Initial text to populate the editor with
/// * `editor` - Editor command to use, or None to use $EDITOR/$VISUAL/vi
/// * `extension` - File extension for the temporary file
/// * `require_save` - If true, return None if user didn't save
///
/// # Returns
///
/// The edited text, or None if editing was cancelled.
pub fn edit_text(
    text: Option<&str>,
    editor: Option<&str>,
    extension: &str,
    require_save: bool,
) -> Result<Option<String>> {
    use std::fs;
    use std::process::Command;

    // Create temporary file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("click_edit_{}.{}", std::process::id(), extension));

    // Write initial content
    if let Some(initial) = text {
        fs::write(&temp_file, initial)
            .map_err(|e| ClickError::file_error(&temp_file, e.to_string()))?;
    }

    // Get modification time before editing
    let mtime_before = fs::metadata(&temp_file)
        .ok()
        .and_then(|m| m.modified().ok());

    // Determine editor
    let editor_cmd = editor
        .map(String::from)
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "vi".to_string());

    // Run editor
    let status = Command::new(&editor_cmd)
        .arg(&temp_file)
        .status()
        .map_err(|e| ClickError::usage(format!("Failed to run editor '{}': {}", editor_cmd, e)))?;

    if !status.success() {
        let _ = fs::remove_file(&temp_file);
        return Err(ClickError::usage(format!(
            "Editor '{}' exited with error",
            editor_cmd
        )));
    }

    // Check if file was modified
    if require_save {
        let mtime_after = fs::metadata(&temp_file)
            .ok()
            .and_then(|m| m.modified().ok());

        if mtime_before == mtime_after {
            let _ = fs::remove_file(&temp_file);
            return Ok(None);
        }
    }

    // Read edited content
    let content = fs::read_to_string(&temp_file)
        .map_err(|e| ClickError::file_error(&temp_file, e.to_string()))?;

    // Clean up
    let _ = fs::remove_file(&temp_file);

    Ok(Some(content))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_codes() {
        assert_eq!(Color::Red.fg_code(), 31);
        assert_eq!(Color::Red.bg_code(), 41);
        assert_eq!(Color::BrightGreen.fg_code(), 92);
        assert_eq!(Color::BrightGreen.bg_code(), 102);
        assert_eq!(Color::Reset.fg_code(), 39);
        assert_eq!(Color::Reset.bg_code(), 49);
    }

    #[test]
    fn test_style_basic() {
        let styled = style(
            "hello", None, None, false, false, false, false, false, false, false, false,
        );
        assert_eq!(styled, "hello");
    }

    #[test]
    fn test_style_with_color() {
        let styled = style(
            "hello",
            Some(Color::Red),
            None,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            true,
        );
        assert_eq!(styled, "\x1b[31mhello\x1b[0m");
    }

    #[test]
    fn test_style_bold() {
        let styled = style(
            "hello", None, None, true, false, false, false, false, false, false, true,
        );
        assert_eq!(styled, "\x1b[1mhello\x1b[0m");
    }

    #[test]
    fn test_style_multiple() {
        let styled = style(
            "hello",
            Some(Color::Green),
            Some(Color::Black),
            true,
            false,
            true,
            false,
            false,
            false,
            false,
            true,
        );
        // Should have: bold (1), underline (4), fg green (32), bg black (40)
        assert!(styled.starts_with("\x1b["));
        assert!(styled.contains("1"));
        assert!(styled.contains("4"));
        assert!(styled.contains("32"));
        assert!(styled.contains("40"));
        assert!(styled.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_strip_ansi_codes() {
        let styled = "\x1b[31mhello\x1b[0m world";
        let stripped = strip_ansi_codes(styled);
        assert_eq!(stripped, "hello world");

        let plain = "no codes here";
        assert_eq!(strip_ansi_codes(plain), "no codes here");
    }

    #[test]
    fn test_strip_ansi_codes_complex() {
        let styled = "\x1b[1;31;40mcomplex\x1b[0m";
        let stripped = strip_ansi_codes(styled);
        assert_eq!(stripped, "complex");
    }

    #[test]
    fn test_get_terminal_size_returns_valid() {
        let (width, height) = get_terminal_size();
        assert!(width > 0);
        assert!(height > 0);
    }

    #[test]
    fn test_progress_bar_render() {
        let bar = ProgressBar::new(100, Some("Test"), false, true, true, 20);
        let output = bar.render();
        assert!(output.contains("Test"));
        assert!(output.contains("["));
        assert!(output.contains("]"));
        assert!(output.contains("0/100"));
        assert!(output.contains("0%"));
    }

    #[test]
    fn test_progress_bar_update() {
        let mut bar = ProgressBar::new(100, None, false, true, false, 10);
        bar.update(50);
        let output = bar.render();
        assert!(output.contains("50%"));
    }

    #[test]
    fn test_progress_bar_finish() {
        let mut bar = ProgressBar::new(100, None, false, true, false, 10);
        bar.finish();
        let output = bar.render();
        assert!(output.contains("100%"));
        assert!(bar.finished);
    }

    #[test]
    fn test_progress_bar_zero_length() {
        let bar = ProgressBar::new(0, None, false, true, false, 10);
        let output = bar.render();
        assert!(output.contains("0%"));
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(BLACK, Color::Black);
        assert_eq!(RED, Color::Red);
        assert_eq!(GREEN, Color::Green);
        assert_eq!(YELLOW, Color::Yellow);
        assert_eq!(BLUE, Color::Blue);
        assert_eq!(MAGENTA, Color::Magenta);
        assert_eq!(CYAN, Color::Cyan);
        assert_eq!(WHITE, Color::White);
        assert_eq!(BRIGHT_BLACK, Color::BrightBlack);
        assert_eq!(BRIGHT_RED, Color::BrightRed);
        assert_eq!(BRIGHT_GREEN, Color::BrightGreen);
        assert_eq!(BRIGHT_YELLOW, Color::BrightYellow);
        assert_eq!(BRIGHT_BLUE, Color::BrightBlue);
        assert_eq!(BRIGHT_MAGENTA, Color::BrightMagenta);
        assert_eq!(BRIGHT_CYAN, Color::BrightCyan);
        assert_eq!(BRIGHT_WHITE, Color::BrightWhite);
        assert_eq!(RESET, Color::Reset);
    }

    #[test]
    fn test_style_all_options() {
        let styled = style(
            "test",
            Some(Color::Blue),
            Some(Color::White),
            true,  // bold
            true,  // dim
            true,  // underline
            true,  // overline
            true,  // italic
            true,  // blink
            true,  // strikethrough
            true,  // reset
        );
        assert!(styled.starts_with("\x1b["));
        assert!(styled.contains("1")); // bold
        assert!(styled.contains("2")); // dim
        assert!(styled.contains("3")); // italic
        assert!(styled.contains("4")); // underline
        assert!(styled.contains("5")); // blink
        assert!(styled.contains("9")); // strikethrough
        assert!(styled.contains("53")); // overline
        assert!(styled.contains("34")); // blue fg
        assert!(styled.contains("47")); // white bg
        assert!(styled.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_style_no_reset() {
        let styled = style(
            "hello",
            Some(Color::Red),
            None,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert!(styled.starts_with("\x1b[31m"));
        assert!(!styled.ends_with("\x1b[0m"));
    }
}
