//! Testing utilities for click-rs applications.
//!
//! This module provides utilities for testing CLI applications built with click-rs.
//! The main component is [`CliRunner`], which allows invoking commands with captured
//! output for assertions.
//!
//! # Reference
//!
//! Based on Python Click's `testing.py`.
//!
//! # Example
//!
//! ```rust
//! use click::testing::CliRunner;
//! use click::command::Command;
//!
//! let cmd = Command::new("hello")
//!     .callback(|_ctx| {
//!         // Your command logic here
//!         Ok(())
//!     })
//!     .build();
//!
//! let runner = CliRunner::new();
//! let result = runner.invoke(&cmd, &[]);
//!
//! assert_eq!(result.exit_code, 0);
//! assert!(result.is_success());
//! ```

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;

use crate::group::CommandLike;
use encoding_rs::Encoding;

#[derive(Debug)]
enum CaptureOutcome {
    Returned(Result<(), crate::ClickError>),
    Panicked(Box<dyn std::any::Any + Send + 'static>),
}

fn panic_message(panic: &(dyn std::any::Any + Send + 'static)) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic".to_string()
    }
}

fn restore_env(saved_env: &[(String, Option<String>)]) {
    for (key, value) in saved_env {
        match value {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
    }
}

/// A global lock used to serialize stdio redirection during output capture.
///
/// Output capture redirects process-global file descriptors, which is not safe
/// to do concurrently from multiple threads. The lock prevents tests (and other
/// concurrent invocations) from corrupting each other's stdio streams.
#[cfg(any(unix, windows))]
static IO_CAPTURE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(any(unix, windows))]
fn capture_lock() -> std::sync::MutexGuard<'static, ()> {
    IO_CAPTURE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("IO capture lock poisoned")
}

#[cfg(unix)]
fn run_with_capture<F>(input: &str, f: F) -> io::Result<(CaptureOutcome, Vec<u8>, Vec<u8>)>
where
    F: FnOnce() -> Result<(), crate::ClickError>,
{
    use nix::unistd::{dup, dup2_stderr, dup2_stdin, dup2_stdout};

    let _lock = capture_lock();

    // Pipes for stdout/stderr/stdin capture (os_pipe: safe, cross-platform).
    let (out_reader_pipe, out_writer_pipe) = os_pipe::pipe()?;
    let (err_reader_pipe, err_writer_pipe) = os_pipe::pipe()?;
    let (in_reader_pipe, in_writer_pipe) = os_pipe::pipe()?;

    // Save original fds (nix::dup returns OwnedFd with RAII cleanup).
    let saved_stdin = dup(io::stdin()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let saved_stdout = dup(io::stdout()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let saved_stderr = dup(io::stderr()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Write input then close stdin write end (PipeWriter Drop closes fd).
    {
        let mut writer = in_writer_pipe;
        let _ = writer.write_all(input.as_bytes());
    }

    // Redirect stdio using nix safe wrappers.
    let redir_ok = dup2_stdin(&in_reader_pipe)
        .and_then(|_| dup2_stdout(&out_writer_pipe))
        .and_then(|_| dup2_stderr(&err_writer_pipe))
        .is_ok();

    // Close pipe ends that have been duped into fd 0/1/2 (RAII Drop).
    drop(in_reader_pipe);
    drop(out_writer_pipe);
    drop(err_writer_pipe);

    if !redir_ok {
        let _ = dup2_stdin(&saved_stdin);
        let _ = dup2_stdout(&saved_stdout);
        let _ = dup2_stderr(&saved_stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "failed to redirect stdio",
        ));
    }

    // Reader threads use PipeReader directly (implements Read, no unsafe needed).
    let out_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut reader = out_reader_pipe;
        let _ = reader.read_to_end(&mut buf);
        buf
    });
    let err_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut reader = err_reader_pipe;
        let _ = reader.read_to_end(&mut buf);
        buf
    });

    let old_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let unwind = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(old_panic_hook);

    let _ = io::stdout().flush();
    let _ = io::stderr().flush();

    // Restore stdio (OwnedFd auto-closes via Drop after dup2).
    let _ = dup2_stdin(&saved_stdin);
    let _ = dup2_stdout(&saved_stdout);
    let _ = dup2_stderr(&saved_stderr);

    let stdout_bytes = out_thread.join().unwrap_or_default();
    let stderr_bytes = err_thread.join().unwrap_or_default();

    let outcome = match unwind {
        Ok(r) => CaptureOutcome::Returned(r),
        Err(panic) => CaptureOutcome::Panicked(panic),
    };

    Ok((outcome, stdout_bytes, stderr_bytes))
}

#[cfg(windows)]
fn run_with_capture<F>(input: &str, f: F) -> io::Result<(CaptureOutcome, Vec<u8>, Vec<u8>)>
where
    F: FnOnce() -> Result<(), crate::ClickError>,
{
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        GetStdHandle, SetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    let _lock = capture_lock();

    // Pipes for stdout/stderr/stdin capture (os_pipe: safe, cross-platform).
    let (out_reader_pipe, out_writer_pipe) = os_pipe::pipe()?;
    let (err_reader_pipe, err_writer_pipe) = os_pipe::pipe()?;
    let (in_reader_pipe, in_writer_pipe) = os_pipe::pipe()?;

    // SAFETY: GetStdHandle is a read-only query of the process-global stdio table.
    // The returned handles are borrowed (not owned) and we only use them to
    // restore the original state later. IO_CAPTURE_LOCK serializes access.
    let (saved_in, saved_out, saved_err) = unsafe {
        (
            GetStdHandle(STD_INPUT_HANDLE),
            GetStdHandle(STD_OUTPUT_HANDLE),
            GetStdHandle(STD_ERROR_HANDLE),
        )
    };

    if saved_in == 0
        || saved_out == 0
        || saved_err == 0
        || saved_in == INVALID_HANDLE_VALUE
        || saved_out == INVALID_HANDLE_VALUE
        || saved_err == INVALID_HANDLE_VALUE
    {
        return Err(io::Error::new(io::ErrorKind::Other, "GetStdHandle failed"));
    }

    // Write input then close stdin write end (PipeWriter Drop closes handle).
    {
        let mut writer = in_writer_pipe;
        let _ = writer.write_all(input.as_bytes());
    }

    // SAFETY: SetStdHandle replaces the process-global stdio handles. This is
    // serialized by IO_CAPTURE_LOCK, and we restore the original handles in all
    // exit paths (including panics) before releasing the lock.
    let redir_ok = unsafe {
        SetStdHandle(STD_INPUT_HANDLE, in_reader_pipe.as_raw_handle() as _) != 0
            && SetStdHandle(STD_OUTPUT_HANDLE, out_writer_pipe.as_raw_handle() as _) != 0
            && SetStdHandle(STD_ERROR_HANDLE, err_writer_pipe.as_raw_handle() as _) != 0
    };

    if !redir_ok {
        // SAFETY: Restoring original handles after failed redirect.
        unsafe {
            let _ = SetStdHandle(STD_INPUT_HANDLE, saved_in);
            let _ = SetStdHandle(STD_OUTPUT_HANDLE, saved_out);
            let _ = SetStdHandle(STD_ERROR_HANDLE, saved_err);
        }
        return Err(io::Error::last_os_error());
    }

    // Reader threads use PipeReader directly (implements Read, no unsafe needed).
    let out_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut reader = out_reader_pipe;
        let _ = reader.read_to_end(&mut buf);
        buf
    });
    let err_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut reader = err_reader_pipe;
        let _ = reader.read_to_end(&mut buf);
        buf
    });

    let old_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let unwind = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(old_panic_hook);

    let _ = io::stdout().flush();
    let _ = io::stderr().flush();

    // SAFETY: Restoring original handles and dropping redirected pipe ends
    // to signal EOF to reader threads.
    unsafe {
        let _ = SetStdHandle(STD_INPUT_HANDLE, saved_in);
        let _ = SetStdHandle(STD_OUTPUT_HANDLE, saved_out);
        let _ = SetStdHandle(STD_ERROR_HANDLE, saved_err);
    }

    // Close redirected pipe ends to signal EOF to readers (safe RAII Drop).
    drop(out_writer_pipe);
    drop(err_writer_pipe);
    drop(in_reader_pipe);

    let stdout_bytes = out_thread.join().unwrap_or_default();
    let stderr_bytes = err_thread.join().unwrap_or_default();

    let outcome = match unwind {
        Ok(r) => CaptureOutcome::Returned(r),
        Err(panic) => CaptureOutcome::Panicked(panic),
    };

    Ok((outcome, stdout_bytes, stderr_bytes))
}

#[cfg(not(any(unix, windows)))]
fn run_with_capture<F>(_input: &str, f: F) -> io::Result<(CaptureOutcome, Vec<u8>, Vec<u8>)>
where
    F: FnOnce() -> Result<(), crate::ClickError>,
{
    let unwind = catch_unwind(AssertUnwindSafe(f));
    let outcome = match unwind {
        Ok(r) => CaptureOutcome::Returned(r),
        Err(panic) => CaptureOutcome::Panicked(panic),
    };
    Ok((outcome, Vec::new(), Vec::new()))
}

// =============================================================================
// CliRunner
// =============================================================================

/// A test runner for CLI applications.
///
/// `CliRunner` provides a controlled environment for testing CLI commands.
/// It captures stdout, stderr, and tracks exit codes.
///
/// # Example
///
/// ```rust
/// use click::testing::CliRunner;
/// use click::command::Command;
///
/// let cmd = Command::new("greet")
///     .callback(|ctx| {
///         let default_name = "World".to_string();
///         let name = ctx.get_param::<String>("name").unwrap_or(&default_name);
///         // Use name for greeting
///         Ok(())
///     })
///     .build();
///
/// let runner = CliRunner::new();
/// let result = runner.invoke(&cmd, &[]);
///
/// assert_eq!(result.exit_code, 0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct CliRunner {
    /// Environment variables to set during invocation.
    env: HashMap<String, String>,

    /// Environment variables to unset during invocation.
    env_unset: Vec<String>,

    /// Whether to echo input to output.
    echo_stdin: bool,

    /// Whether to mix stderr into stdout.
    mix_stderr: bool,

    /// Whether to capture panics as a failure instead of re-panicking.
    ///
    /// When enabled (default), panics inside the invoked command are caught and returned as
    /// `InvokeResult { exit_code: 1, exception_message: Some(...) }`.
    catch_panics: bool,

    /// Output charset for decoding captured bytes.
    charset: String,
}

impl CliRunner {
    /// Create a new CLI runner with default settings.
    ///
    /// # Example
    ///
    /// ```rust
    /// use click::testing::CliRunner;
    ///
    /// let runner = CliRunner::new();
    /// ```
    pub fn new() -> Self {
        Self {
            env: HashMap::new(),
            env_unset: Vec::new(),
            echo_stdin: false,
            mix_stderr: true,
            catch_panics: true,
            charset: "utf-8".to_string(),
        }
    }

    /// Set an environment variable for the invocation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use click::testing::CliRunner;
    ///
    /// let runner = CliRunner::new()
    ///     .env("MY_VAR", "value");
    /// ```
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Unset an environment variable for the invocation.
    pub fn env_unset(mut self, key: impl Into<String>) -> Self {
        self.env_unset.push(key.into());
        self
    }

    /// Clear all custom environment settings.
    pub fn env_clear(mut self) -> Self {
        self.env.clear();
        self.env_unset.clear();
        self
    }

    /// Set whether to echo stdin to output.
    pub fn echo_stdin(mut self, echo: bool) -> Self {
        self.echo_stdin = echo;
        self
    }

    /// Set whether to mix stderr into stdout.
    ///
    /// When true (default), `InvokeResult.output` contains `stdout + stderr`.
    /// The `InvokeResult.stderr` field is always populated with captured stderr.
    pub fn mix_stderr(mut self, mix: bool) -> Self {
        self.mix_stderr = mix;
        self
    }

    /// Set whether panics should be captured as a failure instead of re-panicking.
    pub fn catch_panics(mut self, catch: bool) -> Self {
        self.catch_panics = catch;
        self
    }

    /// Set the charset used to decode captured output.
    ///
    /// Defaults to `"utf-8"`. Unknown labels fall back to UTF-8.
    pub fn charset(mut self, charset: impl Into<String>) -> Self {
        self.charset = charset.into();
        self
    }

    /// Invoke a command and capture the result.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to invoke
    /// * `args` - Command-line arguments
    ///
    /// # Example
    ///
    /// ```rust
    /// use click::testing::CliRunner;
    /// use click::command::Command;
    ///
    /// let cmd = Command::new("hello")
    ///     .callback(|_| {
    ///         println!("Hello!");
    ///         Ok(())
    ///     })
    ///     .build();
    ///
    /// let result = CliRunner::new().invoke(&cmd, &[]);
    /// assert_eq!(result.exit_code, 0);
    /// ```
    pub fn invoke(&self, cmd: &dyn CommandLike, args: &[&str]) -> InvokeResult {
        self.invoke_with_input(cmd, args, None)
    }

    /// Invoke a command with simulated stdin input.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to invoke
    /// * `args` - Command-line arguments
    /// * `input` - Optional stdin input
    ///
    /// # Example
    ///
    /// ```rust
    /// use click::testing::CliRunner;
    /// use click::command::Command;
    ///
    /// let cmd = Command::new("echo")
    ///     .callback(|_| Ok(()))
    ///     .build();
    ///
    /// let result = CliRunner::new().invoke_with_input(&cmd, &[], Some("test input"));
    /// ```
    pub fn invoke_with_input(
        &self,
        cmd: &dyn CommandLike,
        args: &[&str],
        input: Option<&str>,
    ) -> InvokeResult {
        // Save current environment
        let saved_env: Vec<(String, Option<String>)> = self
            .env
            .keys()
            .chain(self.env_unset.iter())
            .map(|k| (k.clone(), env::var(k).ok()))
            .collect();

        // Set test environment
        for (key, value) in &self.env {
            env::set_var(key, value);
        }
        for key in &self.env_unset {
            env::remove_var(key);
        }

        // Convert args to owned strings
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        // Always provide a stdin stream (empty by default) to avoid hanging on interactive reads.
        let input_str = input.unwrap_or("");

        let (outcome, stdout_bytes, stderr_bytes) = match run_with_capture(input_str, || {
            // Run in a child thread so Rust's test harness output capture (thread-local) doesn't
            // intercept stdout/stderr before our OS-level redirection does.
            thread::scope(|s| {
                let handle = s.spawn(|| cmd.main(args_owned));
                match handle.join() {
                    Ok(r) => r,
                    Err(panic) => std::panic::resume_unwind(panic),
                }
            })
        }) {
            Ok(v) => v,
            Err(e) => {
                restore_env(&saved_env);
                return InvokeResult::new(1, String::new(), String::new(), Some(e.to_string()));
            }
        };

        // Determine exit code and exception message
        let (exit_code, exception_message) = match outcome {
            CaptureOutcome::Returned(r) => match &r {
                Ok(()) => (0, None),
                Err(e) => (e.exit_code(), Some(e.to_string())),
            },
            CaptureOutcome::Panicked(panic) => {
                if !self.catch_panics {
                    restore_env(&saved_env);
                    std::panic::resume_unwind(panic);
                }
                (1, Some(format!("panic: {}", panic_message(&*panic))))
            }
        };

        let label = self.charset.trim().to_lowercase();
        let encoding = Encoding::for_label(label.as_bytes())
            .or_else(|| {
                if label == "latin-1" || label == "latin1" {
                    Encoding::for_label(b"iso-8859-1")
                } else {
                    None
                }
            })
            .unwrap_or(encoding_rs::UTF_8);
        let mut stdout = encoding.decode(&stdout_bytes).0.into_owned();
        let stderr = encoding.decode(&stderr_bytes).0.into_owned();

        // Echo stdin into stdout like Python Click's CliRunner when enabled.
        if self.echo_stdin && !input_str.is_empty() {
            stdout = format!("{}{}", input_str, stdout);
        }

        let output = if self.mix_stderr {
            format!("{}{}", stdout, stderr)
        } else {
            stdout
        };

        // Restore environment
        restore_env(&saved_env);

        InvokeResult::new(exit_code, output, stderr, exception_message)
    }

    /// Invoke a command within an isolated filesystem.
    ///
    /// This creates a temporary directory and runs the command there.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to invoke
    /// * `args` - Command-line arguments
    ///
    /// # Example
    ///
    /// ```rust
    /// use click::testing::CliRunner;
    /// use click::command::Command;
    ///
    /// let cmd = Command::new("test").build();
    /// let result = CliRunner::new().invoke_isolated(&cmd, &[]);
    /// ```
    pub fn invoke_isolated(&self, cmd: &dyn CommandLike, args: &[&str]) -> InvokeResult {
        let _isolated = IsolatedFilesystem::new().expect("Failed to create isolated filesystem");
        self.invoke(cmd, args)
    }
}

// =============================================================================
// InvokeResult
// =============================================================================

/// The result of invoking a command through [`CliRunner`].
///
/// Contains the exit code, captured output, and any exception that occurred.
#[derive(Debug, Clone)]
pub struct InvokeResult {
    /// The exit code from the command (0 for success).
    pub exit_code: i32,

    /// Captured stdout (and stderr if `mix_stderr` is true).
    pub output: String,

    /// Captured stderr (always captured, even if mixed into `output`).
    pub stderr: String,

    /// The error message if an exception occurred.
    pub exception_message: Option<String>,
}

impl InvokeResult {
    /// Create a new InvokeResult for testing purposes.
    #[doc(hidden)]
    pub fn new(
        exit_code: i32,
        output: String,
        stderr: String,
        exception_message: Option<String>,
    ) -> Self {
        Self {
            exit_code,
            output,
            stderr,
            exception_message,
        }
    }

    /// Check if the command succeeded (exit code 0).
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if the command failed (exit code != 0).
    pub fn is_failure(&self) -> bool {
        self.exit_code != 0
    }

    /// Get the output lines.
    pub fn output_lines(&self) -> Vec<&str> {
        self.output.lines().collect()
    }

    /// Check if the output contains a substring.
    pub fn output_contains(&self, substring: &str) -> bool {
        self.output.contains(substring)
    }

    /// Check if the stderr contains a substring.
    pub fn stderr_contains(&self, substring: &str) -> bool {
        self.stderr.contains(substring)
    }

    /// Get the combined output (stdout + stderr).
    ///
    /// If `mix_stderr` was true, this is the same as `output`.
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            return self.output.clone();
        }

        // If output already includes stderr (mix_stderr mode), avoid duplication.
        if self.output.ends_with(&self.stderr) {
            return self.output.clone();
        }

        format!("{}{}", self.output, self.stderr)
    }
}

// =============================================================================
// IsolatedFilesystem
// =============================================================================

/// A temporary filesystem context for isolated testing.
///
/// Creates a temporary directory that is automatically cleaned up when dropped.
/// The current working directory is changed to the temporary directory for the
/// duration of the test.
///
/// # Example
///
/// ```rust
/// use click::testing::IsolatedFilesystem;
/// use std::fs;
///
/// {
///     let isolated = IsolatedFilesystem::new().unwrap();
///     let path = isolated.path();
///
///     // Create files in the isolated directory
///     fs::write(path.join("test.txt"), "hello").unwrap();
///
///     assert!(path.join("test.txt").exists());
/// }
/// // Directory is automatically cleaned up here
/// ```
#[derive(Debug)]
pub struct IsolatedFilesystem {
    /// The temporary directory path.
    path: PathBuf,
    /// The original working directory to restore on drop.
    original_cwd: PathBuf,
}

impl IsolatedFilesystem {
    /// Generate a unique suffix for directory names to avoid test interference.
    fn unique_suffix() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        timestamp ^ count
    }

    /// Get current directory, falling back to temp dir if current dir is invalid.
    ///
    /// This can happen when running tests in parallel and another test
    /// deletes the directory that was the current working directory.
    fn get_current_dir_safe() -> io::Result<PathBuf> {
        match env::current_dir() {
            Ok(cwd) => Ok(cwd),
            Err(_) => {
                // Fall back to temp dir if current dir is invalid
                let temp = env::temp_dir();
                let _ = env::set_current_dir(&temp);
                Ok(temp)
            }
        }
    }

    /// Create a new isolated filesystem.
    ///
    /// This creates a temporary directory and changes to it.
    pub fn new() -> io::Result<Self> {
        let original_cwd = Self::get_current_dir_safe()?;
        let path = env::temp_dir().join(format!(
            "click_test_{}_{}",
            std::process::id(),
            Self::unique_suffix()
        ));

        // Create the directory
        fs::create_dir_all(&path)?;

        // Change to it
        env::set_current_dir(&path)?;

        Ok(Self { path, original_cwd })
    }

    /// Create an isolated filesystem with a specific name.
    pub fn with_name(name: &str) -> io::Result<Self> {
        let original_cwd = Self::get_current_dir_safe()?;
        let path = env::temp_dir().join(format!(
            "click_test_{}_{}_{}",
            name,
            std::process::id(),
            Self::unique_suffix()
        ));

        // Create the directory fresh
        fs::create_dir_all(&path)?;
        env::set_current_dir(&path)?;

        Ok(Self { path, original_cwd })
    }

    /// Get the path to the temporary directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a file in the isolated filesystem.
    pub fn create_file(&self, name: &str, content: &str) -> io::Result<PathBuf> {
        let file_path = self.path.join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Create a directory in the isolated filesystem.
    pub fn create_dir(&self, name: &str) -> io::Result<PathBuf> {
        let dir_path = self.path.join(name);
        fs::create_dir_all(&dir_path)?;
        Ok(dir_path)
    }

    /// Read a file from the isolated filesystem.
    pub fn read_file(&self, name: &str) -> io::Result<String> {
        fs::read_to_string(self.path.join(name))
    }

    /// Check if a file exists in the isolated filesystem.
    pub fn file_exists(&self, name: &str) -> bool {
        self.path.join(name).exists()
    }

    /// List files in the isolated filesystem.
    pub fn list_files(&self) -> io::Result<Vec<String>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                files.push(name.to_string());
            }
        }
        files.sort();
        Ok(files)
    }
}

impl Drop for IsolatedFilesystem {
    fn drop(&mut self) {
        // Restore original working directory
        let _ = env::set_current_dir(&self.original_cwd);

        // Clean up the temporary directory
        let _ = fs::remove_dir_all(&self.path);
    }
}

// =============================================================================
// EchoingStdin (for simulated input)
// =============================================================================

/// A wrapper that echoes input to output as it's read.
#[derive(Debug)]
pub struct EchoingStdin<R: Read, W: Write> {
    input: R,
    output: W,
}

impl<R: Read, W: Write> EchoingStdin<R, W> {
    /// Create a new echoing stdin wrapper.
    pub fn new(input: R, output: W) -> Self {
        Self { input, output }
    }
}

impl<R: Read, W: Write> Read for EchoingStdin<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.input.read(buf)?;
        if n > 0 {
            self.output.write_all(&buf[..n])?;
        }
        Ok(n)
    }
}

// =============================================================================
// Test Utilities
// =============================================================================

/// Create a mock context for testing.
///
/// This is useful when you need to test callback functions in isolation.
pub fn make_test_context(info_name: &str) -> crate::context::Context {
    crate::context::ContextBuilder::new()
        .info_name(info_name)
        .build()
}

/// Assert that a result indicates success.
#[macro_export]
macro_rules! assert_success {
    ($result:expr) => {
        assert!(
            $result.is_success(),
            "Expected success but got exit code {} with output:\n{}",
            $result.exit_code,
            $result.combined_output()
        );
    };
}

/// Assert that a result indicates failure.
#[macro_export]
macro_rules! assert_failure {
    ($result:expr) => {
        assert!(
            $result.is_failure(),
            "Expected failure but got success with output:\n{}",
            $result.output
        );
    };
    ($result:expr, $code:expr) => {
        assert_eq!(
            $result.exit_code,
            $code,
            "Expected exit code {} but got {} with output:\n{}",
            $code,
            $result.exit_code,
            $result.combined_output()
        );
    };
}

/// Assert that the output contains a substring.
#[macro_export]
macro_rules! assert_output_contains {
    ($result:expr, $substring:expr) => {
        assert!(
            $result.output_contains($substring),
            "Expected output to contain '{}' but got:\n{}",
            $substring,
            $result.output
        );
    };
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;

    #[test]
    fn test_cli_runner_new() {
        let runner = CliRunner::new();
        assert!(runner.env.is_empty());
        assert!(runner.env_unset.is_empty());
    }

    #[test]
    fn test_cli_runner_env() {
        let runner = CliRunner::new()
            .env("TEST_VAR", "test_value")
            .env("ANOTHER", "value");

        assert_eq!(runner.env.get("TEST_VAR"), Some(&"test_value".to_string()));
        assert_eq!(runner.env.get("ANOTHER"), Some(&"value".to_string()));
    }

    #[test]
    fn test_cli_runner_env_unset() {
        let runner = CliRunner::new().env("KEEP", "value").env_unset("REMOVE");

        assert_eq!(runner.env.len(), 1);
        assert_eq!(runner.env_unset.len(), 1);
    }

    #[test]
    fn test_cli_runner_env_clear() {
        let runner = CliRunner::new()
            .env("VAR1", "val1")
            .env("VAR2", "val2")
            .env_unset("VAR3")
            .env_clear();

        assert!(runner.env.is_empty());
        assert!(runner.env_unset.is_empty());
    }

    #[test]
    fn test_invoke_simple_command() {
        let cmd = Command::new("test").callback(|_ctx| Ok(())).build();

        let runner = CliRunner::new();
        let result = runner.invoke(&cmd, &[]);

        assert_eq!(result.exit_code, 0);
        assert!(result.exception_message.is_none());
    }

    #[test]
    fn test_invoke_failing_command() {
        let cmd = Command::new("fail")
            .callback(|_ctx| Err(crate::ClickError::usage("test error")))
            .build();

        let runner = CliRunner::new();
        let result = runner.invoke(&cmd, &[]);

        assert_eq!(result.exit_code, 2); // Usage errors have exit code 2
        assert!(result.exception_message.is_some());
    }

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
    fn test_invoke_result_output_contains() {
        let result = InvokeResult::new(0, "Hello, World!".to_string(), String::new(), None);

        assert!(result.output_contains("Hello"));
        assert!(result.output_contains("World"));
        assert!(!result.output_contains("Goodbye"));
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
    fn test_invoke_result_combined_output() {
        let result = InvokeResult::new(0, "stdout".to_string(), "stderr".to_string(), None);

        assert_eq!(result.combined_output(), "stdoutstderr");
    }

    #[test]
    fn test_isolated_filesystem() {
        let isolated = IsolatedFilesystem::new().unwrap();
        let iso_path = isolated.path().to_path_buf();

        // The isolated directory exists
        assert!(iso_path.exists());
        assert!(iso_path.starts_with(env::temp_dir()));

        // Create a file
        isolated.create_file("test.txt", "hello").unwrap();
        assert!(isolated.file_exists("test.txt"));
        assert_eq!(isolated.read_file("test.txt").unwrap(), "hello");

        // Drop the filesystem explicitly
        drop(isolated);

        // After drop, the directory is cleaned up
        // Note: We don't test current_dir restoration because it's racy with parallel tests
        assert!(
            !iso_path.exists(),
            "temp directory should be cleaned up after drop"
        );
    }

    #[test]
    fn test_isolated_filesystem_with_name() {
        let isolated = IsolatedFilesystem::with_name("custom").unwrap();
        let path = isolated.path();

        assert!(path.to_string_lossy().contains("custom"));
    }

    #[test]
    fn test_isolated_filesystem_create_dir() {
        let isolated = IsolatedFilesystem::new().unwrap();

        let dir_path = isolated.create_dir("subdir").unwrap();
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[test]
    fn test_isolated_filesystem_list_files() {
        let isolated = IsolatedFilesystem::new().unwrap();

        isolated.create_file("a.txt", "").unwrap();
        isolated.create_file("b.txt", "").unwrap();
        isolated.create_file("c.txt", "").unwrap();

        let files = isolated.list_files().unwrap();
        assert_eq!(files, vec!["a.txt", "b.txt", "c.txt"]);
    }

    #[test]
    fn test_isolated_filesystem_nested_file() {
        let isolated = IsolatedFilesystem::new().unwrap();

        isolated
            .create_file("dir/nested/file.txt", "content")
            .unwrap();
        assert!(isolated.file_exists("dir/nested/file.txt"));
        assert_eq!(
            isolated.read_file("dir/nested/file.txt").unwrap(),
            "content"
        );
    }

    #[test]
    fn test_make_test_context() {
        let ctx = make_test_context("test");
        assert_eq!(ctx.info_name(), Some("test"));
    }

    #[test]
    fn test_echoing_stdin() {
        let input = b"hello";
        let mut output = Vec::new();

        {
            let mut echoing = EchoingStdin::new(&input[..], &mut output);
            let mut buf = [0u8; 10];
            let n = echoing.read(&mut buf).unwrap();
            assert_eq!(n, 5);
            assert_eq!(&buf[..n], b"hello");
        }

        assert_eq!(&output, b"hello");
    }

    #[test]
    fn test_cli_runner_mix_stderr() {
        let runner = CliRunner::new().mix_stderr(false);
        assert!(!runner.mix_stderr);

        let runner = CliRunner::new().mix_stderr(true);
        assert!(runner.mix_stderr);
    }

    #[test]
    fn test_cli_runner_echo_stdin() {
        let runner = CliRunner::new().echo_stdin(true);
        assert!(runner.echo_stdin);
    }
}
