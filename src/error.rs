//! Error types for click-rs.
//!
//! This module provides a comprehensive error type hierarchy that mirrors Python Click's
//! exception system. All errors can be displayed to users with helpful context and hints.

use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// The type of parameter that caused an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    /// A positional argument
    Argument,
    /// A command-line option (flag)
    Option,
    /// A generic parameter (unspecified type)
    Parameter,
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamType::Argument => write!(f, "argument"),
            ParamType::Option => write!(f, "option"),
            ParamType::Parameter => write!(f, "parameter"),
        }
    }
}

/// Context information for error formatting.
///
/// This struct holds contextual information that can be attached to errors
/// to provide better error messages and help hints.
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// The command path (e.g., "myapp subcommand")
    pub command_path: Option<String>,
    /// The usage string for the command
    pub usage: Option<String>,
    /// Available help option names (e.g., ["--help", "-h"])
    pub help_option_names: Vec<String>,
    /// Whether color output is enabled
    pub color: Option<bool>,
}

impl ErrorContext {
    /// Create a new empty error context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the command path.
    pub fn with_command_path(mut self, path: impl Into<String>) -> Self {
        self.command_path = Some(path.into());
        self
    }

    /// Set the usage string.
    pub fn with_usage(mut self, usage: impl Into<String>) -> Self {
        self.usage = Some(usage.into());
        self
    }

    /// Set the help option names.
    pub fn with_help_options(mut self, options: Vec<String>) -> Self {
        self.help_option_names = options;
        self
    }

    /// Generate the "Try 'COMMAND --help' for help" hint.
    pub fn help_hint(&self) -> Option<String> {
        if let (Some(cmd_path), Some(help_opt)) =
            (&self.command_path, self.help_option_names.first())
        {
            Some(format!("Try '{} {}' for help.", cmd_path, help_opt))
        } else {
            None
        }
    }
}

/// Join parameter hints into a display string.
///
/// All hints are quoted consistently, whether single or multiple.
fn join_param_hints(hints: &[String]) -> String {
    hints
        .iter()
        .map(|h| format!("'{}'", h))
        .collect::<Vec<_>>()
        .join(" / ")
}

/// The main error type for click-rs.
///
/// This enum represents all possible errors that can occur during CLI parsing
/// and execution. It is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking compatibility.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ClickError {
    /// A general usage error with the command.
    ///
    /// This is the base error type for command usage problems.
    /// Exit code: 2
    #[error("{message}")]
    UsageError {
        /// The error message
        message: String,
        /// Optional context for formatting
        ctx: Option<Box<ErrorContext>>,
    },

    /// A parameter received an invalid value.
    ///
    /// This error is raised when a callback or type conversion fails.
    /// Exit code: 2
    #[error("{message}")]
    BadParameter {
        /// The error message describing what went wrong
        message: String,
        /// The name of the parameter (e.g., "--count" or "FILENAME")
        param_name: Option<String>,
        /// A hint to display instead of param_name (can be multiple values)
        param_hint: Option<Vec<String>>,
        /// Optional context for formatting
        ctx: Option<Box<ErrorContext>>,
    },

    /// A required parameter was not provided.
    ///
    /// This error is raised when a required option or argument is missing.
    /// Exit code: 2
    #[error("{}", format_missing_param_message(.param_type, .param_name.as_deref(), .param_hint.as_deref(), .message.as_deref()))]
    MissingParameter {
        /// Optional additional message
        message: Option<String>,
        /// The name of the missing parameter
        param_name: Option<String>,
        /// A hint to display for the parameter
        param_hint: Option<Vec<String>>,
        /// The type of parameter (argument, option, or generic parameter)
        param_type: ParamType,
        /// Optional context for formatting
        ctx: Option<Box<ErrorContext>>,
    },

    /// An unknown option was provided.
    ///
    /// This error includes possible corrections if similar options exist.
    /// Exit code: 2
    #[error("{}", format_no_such_option(.option_name, .possibilities.as_deref()))]
    NoSuchOption {
        /// The option name that was not recognized
        option_name: String,
        /// Similar option names that might be what the user meant
        possibilities: Option<Vec<String>>,
        /// Optional context for formatting
        ctx: Option<Box<ErrorContext>>,
    },

    /// An option was used incorrectly.
    ///
    /// For example, wrong number of arguments for an option.
    /// Exit code: 2
    #[error("{message}")]
    BadOptionUsage {
        /// The option that was used incorrectly
        option_name: String,
        /// The error message
        message: String,
        /// Optional context for formatting
        ctx: Option<Box<ErrorContext>>,
    },

    /// An argument was used incorrectly.
    ///
    /// For example, wrong number of values for an argument.
    /// Exit code: 2
    #[error("{message}")]
    BadArgumentUsage {
        /// The error message
        message: String,
        /// Optional context for formatting
        ctx: Option<Box<ErrorContext>>,
    },

    /// A file operation failed.
    ///
    /// Exit code: 1
    #[error("Could not open file '{filename}': {hint}")]
    FileError {
        /// The path to the file that caused the error
        filename: PathBuf,
        /// A description of what went wrong
        hint: String,
    },

    /// The user aborted the operation.
    ///
    /// This is typically raised when the user presses Ctrl+C or answers "no"
    /// to a confirmation prompt.
    /// Exit code: 1
    #[error("Aborted!")]
    Abort,

    /// Exit with a specific code.
    ///
    /// This is used to signal that the application should exit with the given
    /// status code. A code of 0 indicates success.
    #[error("Exit with code {code}")]
    Exit {
        /// The exit code
        code: i32,
    },
}

/// Format the message for a missing parameter error.
fn format_missing_param_message(
    param_type: &ParamType,
    param_name: Option<&str>,
    param_hint: Option<&[String]>,
    message: Option<&str>,
) -> String {
    let type_str = match param_type {
        ParamType::Argument => "Missing argument",
        ParamType::Option => "Missing option",
        ParamType::Parameter => "Missing parameter",
    };

    let hint_str = if let Some(hints) = param_hint {
        format!(" {}", join_param_hints(hints))
    } else if let Some(name) = param_name {
        format!(" '{}'", name)
    } else {
        String::new()
    };

    let msg_str = if let Some(msg) = message {
        format!(" {}", msg)
    } else {
        String::new()
    };

    format!("{}{}.{}", type_str, hint_str, msg_str)
}

/// Format the message for a "no such option" error.
fn format_no_such_option(option_name: &str, possibilities: Option<&[String]>) -> String {
    let base = format!("No such option: {}", option_name);

    match possibilities {
        Some(opts) if opts.len() == 1 => {
            format!("{} Did you mean '{}'?", base, opts[0])
        }
        Some(opts) if !opts.is_empty() => {
            // Preserve order from suggestion algorithm (e.g., ranked by similarity)
            // Quote each option consistently with single-option case
            let quoted: Vec<_> = opts.iter().map(|o| format!("'{}'", o)).collect();
            format!("{} (Possible options: {})", base, quoted.join(", "))
        }
        _ => base,
    }
}

impl ClickError {
    /// Get the exit code for this error.
    ///
    /// Returns the appropriate exit code based on the error type:
    /// - `Exit`: returns the specified code
    /// - `UsageError` variants: returns 2
    /// - Other errors: returns 1
    pub fn exit_code(&self) -> i32 {
        match self {
            ClickError::Exit { code } => *code,
            ClickError::UsageError { .. }
            | ClickError::BadParameter { .. }
            | ClickError::MissingParameter { .. }
            | ClickError::NoSuchOption { .. }
            | ClickError::BadOptionUsage { .. }
            | ClickError::BadArgumentUsage { .. } => 2,
            ClickError::FileError { .. } | ClickError::Abort => 1,
        }
    }

    /// Format the error message for display to the user.
    ///
    /// This method returns a user-friendly error message, potentially including
    /// usage information and help hints based on the error context.
    pub fn format_message(&self) -> String {
        match self {
            ClickError::BadParameter {
                message,
                param_name,
                param_hint,
                ..
            } => {
                let hint_str = if let Some(hints) = param_hint {
                    Some(join_param_hints(hints))
                } else {
                    param_name.as_ref().map(|n| format!("'{}'", n))
                };

                match hint_str {
                    Some(h) => format!("Invalid value for {}: {}", h, message),
                    None => format!("Invalid value: {}", message),
                }
            }
            _ => self.to_string(),
        }
    }

    /// Format the complete error output including usage and help hints.
    ///
    /// This method returns the full error output that should be shown to the user,
    /// including any usage information and "Try --help" hints.
    pub fn format_full(&self) -> String {
        let ctx = self.context();
        let mut parts = Vec::new();

        // Add usage information if available (for usage errors)
        if let Some(ctx) = ctx {
            if let Some(usage) = &ctx.usage {
                parts.push(usage.clone());
            }

            // Add help hint for usage errors
            if self.is_usage_error() {
                if let Some(hint) = ctx.help_hint() {
                    parts.push(hint);
                }
            }
        }

        // Add the error message
        parts.push(format!("Error: {}", self.format_message()));

        parts.join("\n")
    }

    /// Check if this is a usage error (exit code 2).
    pub fn is_usage_error(&self) -> bool {
        matches!(
            self,
            ClickError::UsageError { .. }
                | ClickError::BadParameter { .. }
                | ClickError::MissingParameter { .. }
                | ClickError::NoSuchOption { .. }
                | ClickError::BadOptionUsage { .. }
                | ClickError::BadArgumentUsage { .. }
        )
    }

    /// Get the error context, if any.
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            ClickError::UsageError { ctx, .. } => ctx.as_ref().map(|b| b.as_ref()),
            ClickError::BadParameter { ctx, .. } => ctx.as_ref().map(|b| b.as_ref()),
            ClickError::MissingParameter { ctx, .. } => ctx.as_ref().map(|b| b.as_ref()),
            ClickError::NoSuchOption { ctx, .. } => ctx.as_ref().map(|b| b.as_ref()),
            ClickError::BadOptionUsage { ctx, .. } => ctx.as_ref().map(|b| b.as_ref()),
            ClickError::BadArgumentUsage { ctx, .. } => ctx.as_ref().map(|b| b.as_ref()),
            _ => None,
        }
    }

    /// Attach context to this error.
    ///
    /// This method consumes the error and returns a new error with the given context.
    pub fn with_context(self, ctx: ErrorContext) -> Self {
        let boxed = Some(Box::new(ctx));
        match self {
            ClickError::UsageError { message, .. } => ClickError::UsageError {
                message,
                ctx: boxed,
            },
            ClickError::BadParameter {
                message,
                param_name,
                param_hint,
                ..
            } => ClickError::BadParameter {
                message,
                param_name,
                param_hint,
                ctx: boxed,
            },
            ClickError::MissingParameter {
                message,
                param_name,
                param_hint,
                param_type,
                ..
            } => ClickError::MissingParameter {
                message,
                param_name,
                param_hint,
                param_type,
                ctx: boxed,
            },
            ClickError::NoSuchOption {
                option_name,
                possibilities,
                ..
            } => ClickError::NoSuchOption {
                option_name,
                possibilities,
                ctx: boxed,
            },
            ClickError::BadOptionUsage {
                option_name,
                message,
                ..
            } => ClickError::BadOptionUsage {
                option_name,
                message,
                ctx: boxed,
            },
            ClickError::BadArgumentUsage { message, .. } => {
                ClickError::BadArgumentUsage { message, ctx: boxed }
            }
            // These errors don't have context
            other => other,
        }
    }
}

// Convenience constructors
impl ClickError {
    /// Create a new usage error.
    pub fn usage(message: impl Into<String>) -> Self {
        ClickError::UsageError {
            message: message.into(),
            ctx: None,
        }
    }

    /// Create a new bad parameter error.
    pub fn bad_parameter(message: impl Into<String>) -> Self {
        ClickError::BadParameter {
            message: message.into(),
            param_name: None,
            param_hint: None,
            ctx: None,
        }
    }

    /// Create a new bad parameter error with a parameter name.
    pub fn bad_parameter_named(message: impl Into<String>, param_name: impl Into<String>) -> Self {
        ClickError::BadParameter {
            message: message.into(),
            param_name: Some(param_name.into()),
            param_hint: None,
            ctx: None,
        }
    }

    /// Create a new missing parameter error.
    pub fn missing_option(name: impl Into<String>) -> Self {
        ClickError::MissingParameter {
            message: None,
            param_name: Some(name.into()),
            param_hint: None,
            param_type: ParamType::Option,
            ctx: None,
        }
    }

    /// Create a new missing argument error.
    pub fn missing_argument(name: impl Into<String>) -> Self {
        ClickError::MissingParameter {
            message: None,
            param_name: Some(name.into()),
            param_hint: None,
            param_type: ParamType::Argument,
            ctx: None,
        }
    }

    /// Create a new "no such option" error.
    pub fn no_such_option(option_name: impl Into<String>) -> Self {
        ClickError::NoSuchOption {
            option_name: option_name.into(),
            possibilities: None,
            ctx: None,
        }
    }

    /// Create a new "no such option" error with suggestions.
    pub fn no_such_option_with_suggestions(
        option_name: impl Into<String>,
        possibilities: Vec<String>,
    ) -> Self {
        ClickError::NoSuchOption {
            option_name: option_name.into(),
            possibilities: Some(possibilities),
            ctx: None,
        }
    }

    /// Create a new bad option usage error.
    pub fn bad_option_usage(option_name: impl Into<String>, message: impl Into<String>) -> Self {
        ClickError::BadOptionUsage {
            option_name: option_name.into(),
            message: message.into(),
            ctx: None,
        }
    }

    /// Create a new bad argument usage error.
    pub fn bad_argument_usage(message: impl Into<String>) -> Self {
        ClickError::BadArgumentUsage {
            message: message.into(),
            ctx: None,
        }
    }

    /// Create a new file error.
    pub fn file_error(filename: impl Into<PathBuf>, hint: impl Into<String>) -> Self {
        ClickError::FileError {
            filename: filename.into(),
            hint: hint.into(),
        }
    }

    /// Create an abort error.
    pub fn abort() -> Self {
        ClickError::Abort
    }

    /// Create an exit error with the given code.
    pub fn exit(code: i32) -> Self {
        ClickError::Exit { code }
    }
}

/// A specialized Result type for click-rs operations.
pub type Result<T> = std::result::Result<T, ClickError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_codes() {
        assert_eq!(ClickError::usage("test").exit_code(), 2);
        assert_eq!(ClickError::bad_parameter("test").exit_code(), 2);
        assert_eq!(ClickError::missing_option("--foo").exit_code(), 2);
        assert_eq!(ClickError::missing_argument("FILE").exit_code(), 2);
        assert_eq!(ClickError::no_such_option("--bar").exit_code(), 2);
        assert_eq!(ClickError::bad_option_usage("--x", "msg").exit_code(), 2);
        assert_eq!(ClickError::bad_argument_usage("msg").exit_code(), 2);
        assert_eq!(ClickError::file_error("test.txt", "not found").exit_code(), 1);
        assert_eq!(ClickError::abort().exit_code(), 1);
        assert_eq!(ClickError::exit(0).exit_code(), 0);
        assert_eq!(ClickError::exit(42).exit_code(), 42);
    }

    #[test]
    fn test_usage_error_display() {
        let err = ClickError::usage("invalid command");
        assert_eq!(err.to_string(), "invalid command");
    }

    #[test]
    fn test_bad_parameter_format() {
        let err = ClickError::bad_parameter_named("must be positive", "--count");
        assert_eq!(
            err.format_message(),
            "Invalid value for '--count': must be positive"
        );

        let err = ClickError::bad_parameter("must be positive");
        assert_eq!(err.format_message(), "Invalid value: must be positive");
    }

    #[test]
    fn test_missing_parameter_display() {
        let err = ClickError::missing_option("--name");
        assert_eq!(err.to_string(), "Missing option '--name'.");

        let err = ClickError::missing_argument("FILE");
        assert_eq!(err.to_string(), "Missing argument 'FILE'.");
    }

    #[test]
    fn test_no_such_option_display() {
        let err = ClickError::no_such_option("--hlep");
        assert_eq!(err.to_string(), "No such option: --hlep");

        let err = ClickError::no_such_option_with_suggestions("--hlep", vec!["--help".to_string()]);
        assert_eq!(err.to_string(), "No such option: --hlep Did you mean '--help'?");

        // Order is preserved from suggestion algorithm (not sorted), each option quoted
        let err = ClickError::no_such_option_with_suggestions(
            "--hlep",
            vec!["--help".to_string(), "--hello".to_string()],
        );
        assert_eq!(
            err.to_string(),
            "No such option: --hlep (Possible options: '--help', '--hello')"
        );
    }

    #[test]
    fn test_file_error_display() {
        let err = ClickError::file_error("/path/to/file.txt", "permission denied");
        assert_eq!(
            err.to_string(),
            "Could not open file '/path/to/file.txt': permission denied"
        );
    }

    #[test]
    fn test_abort_display() {
        let err = ClickError::abort();
        assert_eq!(err.to_string(), "Aborted!");
    }

    #[test]
    fn test_exit_display() {
        let err = ClickError::exit(0);
        assert_eq!(err.to_string(), "Exit with code 0");

        let err = ClickError::exit(1);
        assert_eq!(err.to_string(), "Exit with code 1");
    }

    #[test]
    fn test_context_help_hint() {
        let ctx = ErrorContext::new()
            .with_command_path("myapp")
            .with_help_options(vec!["--help".to_string(), "-h".to_string()]);

        assert_eq!(ctx.help_hint(), Some("Try 'myapp --help' for help.".to_string()));
    }

    #[test]
    fn test_format_full_with_context() {
        let ctx = ErrorContext::new()
            .with_command_path("myapp")
            .with_usage("Usage: myapp [OPTIONS] FILE")
            .with_help_options(vec!["--help".to_string()]);

        let err = ClickError::missing_argument("FILE").with_context(ctx);
        let output = err.format_full();

        assert!(output.contains("Usage: myapp [OPTIONS] FILE"));
        assert!(output.contains("Try 'myapp --help' for help."));
        assert!(output.contains("Error: Missing argument 'FILE'."));
    }

    #[test]
    fn test_is_usage_error() {
        assert!(ClickError::usage("test").is_usage_error());
        assert!(ClickError::bad_parameter("test").is_usage_error());
        assert!(ClickError::missing_option("--foo").is_usage_error());
        assert!(ClickError::no_such_option("--bar").is_usage_error());
        assert!(ClickError::bad_option_usage("--x", "msg").is_usage_error());
        assert!(ClickError::bad_argument_usage("msg").is_usage_error());

        assert!(!ClickError::file_error("f", "h").is_usage_error());
        assert!(!ClickError::abort().is_usage_error());
        assert!(!ClickError::exit(0).is_usage_error());
    }

    #[test]
    fn test_param_type_display() {
        assert_eq!(ParamType::Argument.to_string(), "argument");
        assert_eq!(ParamType::Option.to_string(), "option");
        assert_eq!(ParamType::Parameter.to_string(), "parameter");
    }
}
