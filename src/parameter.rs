//! Base parameter abstraction for click-rs.
//!
//! This module provides the `Parameter` trait and common configuration types
//! for command-line parameters. Options and Arguments implement this trait
//! separately.
//!
//! # Reference
//!
//! Based on Python Click's `core.py:Parameter` class (line 2027+).

use std::fmt;

use crate::error::ClickError;

// =============================================================================
// Nargs Enum
// =============================================================================

/// Specifies how many arguments a parameter consumes.
///
/// This enum corresponds to Python Click's `nargs` parameter:
/// - `Count(1)` is the default (single value)
/// - `Variadic` corresponds to `nargs=-1`
/// - `Optional` corresponds to `nargs=?` (zero or one value)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nargs {
    /// Exactly N values (default is 1).
    Count(usize),
    /// Zero or more values (Python's `nargs=-1`).
    /// All remaining arguments are collected.
    Variadic,
    /// Optional single value (Python's `nargs=?`).
    /// Zero or one value; if not provided, uses the default.
    Optional,
}

impl Default for Nargs {
    fn default() -> Self {
        Nargs::Count(1)
    }
}

impl Nargs {
    /// Returns `true` if this nargs expects exactly one value.
    pub fn is_single(&self) -> bool {
        matches!(self, Nargs::Count(1))
    }

    /// Returns `true` if this nargs can accept multiple values.
    pub fn is_multi(&self) -> bool {
        match self {
            Nargs::Variadic => true,
            Nargs::Count(n) => *n > 1,
            Nargs::Optional => false,
        }
    }

    /// Returns `true` if this nargs accepts zero or more values.
    pub fn is_variadic(&self) -> bool {
        matches!(self, Nargs::Variadic)
    }

    /// Returns `true` if this nargs is optional (zero or one).
    pub fn is_optional(&self) -> bool {
        matches!(self, Nargs::Optional)
    }

    /// Returns the exact count if this is `Count(n)`, otherwise `None`.
    pub fn count(&self) -> Option<usize> {
        match self {
            Nargs::Count(n) => Some(*n),
            _ => None,
        }
    }
}

impl fmt::Display for Nargs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Nargs::Count(1) => write!(f, "1"),
            Nargs::Count(n) => write!(f, "{}", n),
            Nargs::Variadic => write!(f, "-1"),
            Nargs::Optional => write!(f, "?"),
        }
    }
}

// =============================================================================
// Parameter Trait
// =============================================================================

/// A trait for command-line parameters (options and arguments).
///
/// This trait defines the common interface for all parameter types.
/// Options and Arguments implement this trait with their specific behaviors.
pub trait Parameter: Send + Sync + fmt::Debug {
    /// The primary name of the parameter.
    ///
    /// For options, this is typically the long option name without dashes.
    /// For arguments, this is the argument name.
    fn name(&self) -> &str;

    /// Human-readable name for errors and help text.
    ///
    /// For options, this is usually the option flags (e.g., "--name / -n").
    /// For arguments, this is typically the metavar in uppercase.
    fn human_readable_name(&self) -> String;

    /// Number of arguments this parameter consumes.
    fn nargs(&self) -> Nargs;

    /// Whether this parameter can be specified multiple times.
    ///
    /// When true, the parameter collects values into a list/tuple.
    fn multiple(&self) -> bool;

    /// Whether this parameter should be processed before others.
    ///
    /// Eager parameters (like `--help` and `--version`) are processed
    /// first and can short-circuit command execution.
    fn is_eager(&self) -> bool;

    /// Whether this parameter's value should be exposed in `ctx.params`.
    ///
    /// When false, the parameter is still processed but its value
    /// is not stored in the context parameters.
    fn expose_value(&self) -> bool;

    /// Whether this parameter is required.
    ///
    /// Required parameters must be provided via CLI, environment, or default.
    fn required(&self) -> bool;

    /// Get environment variable name(s) for this parameter.
    ///
    /// Returns `None` if no environment variables are configured.
    /// Multiple environment variables can be specified; the first
    /// non-empty value is used.
    fn envvar(&self) -> Option<&[String]>;

    /// Get help text for this parameter.
    fn help(&self) -> Option<&str>;

    /// Whether this parameter is hidden from help.
    fn hidden(&self) -> bool;

    /// Get the metavar for help text (e.g., "FILE", "TEXT").
    ///
    /// If not explicitly set, the type's metavar is used.
    fn get_metavar(&self) -> Option<String>;

    /// Generate help record for formatting.
    ///
    /// Returns a tuple of (option_string, help_string) for help display,
    /// or `None` if this parameter should not appear in help.
    fn get_help_record(&self) -> Option<(String, String)>;

    /// Get the parameter type name (for error messages).
    fn param_type_name(&self) -> &str {
        "parameter"
    }
}

// =============================================================================
// ParameterConfig Struct
// =============================================================================

/// Common configuration for all parameter types.
///
/// This struct holds the shared settings between Options and Arguments.
/// It uses a builder pattern for convenient construction.
#[derive(Debug, Clone)]
pub struct ParameterConfig {
    /// The parameter name.
    pub name: String,
    /// Number of arguments consumed.
    pub nargs: Nargs,
    /// Whether the parameter can be specified multiple times.
    pub multiple: bool,
    /// Whether this parameter should be processed before others.
    pub is_eager: bool,
    /// Whether this parameter's value is exposed in ctx.params.
    pub expose_value: bool,
    /// Whether this parameter is required.
    pub required: bool,
    /// Environment variable name(s) for this parameter.
    pub envvar: Option<Vec<String>>,
    /// Help text for this parameter.
    pub help: Option<String>,
    /// Whether this parameter is hidden from help.
    pub hidden: bool,
    /// Custom metavar for help text.
    pub metavar: Option<String>,
    /// Whether this parameter is deprecated.
    pub deprecated: Option<DeprecationInfo>,
}

/// Information about a deprecated parameter.
#[derive(Debug, Clone, Default)]
pub struct DeprecationInfo {
    /// Custom deprecation message (if not using the default).
    pub message: Option<String>,
}

impl DeprecationInfo {
    /// Create a new deprecation info with default message.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new deprecation info with a custom message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
        }
    }
}

impl Default for ParameterConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            nargs: Nargs::default(),
            multiple: false,
            is_eager: false,
            expose_value: true,
            required: false,
            envvar: None,
            help: None,
            hidden: false,
            metavar: None,
            deprecated: None,
        }
    }
}

impl ParameterConfig {
    /// Create a new parameter configuration with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the number of arguments consumed.
    pub fn nargs(mut self, nargs: Nargs) -> Self {
        self.nargs = nargs;
        self
    }

    /// Set whether the parameter can be specified multiple times.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Set whether this parameter should be processed before others.
    pub fn eager(mut self, eager: bool) -> Self {
        self.is_eager = eager;
        self
    }

    /// Set whether this parameter's value is exposed in ctx.params.
    pub fn expose_value(mut self, expose: bool) -> Self {
        self.expose_value = expose;
        self
    }

    /// Set whether this parameter is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set a single environment variable for this parameter.
    pub fn envvar(mut self, var: impl Into<String>) -> Self {
        self.envvar = Some(vec![var.into()]);
        self
    }

    /// Set multiple environment variables for this parameter.
    pub fn envvars(mut self, vars: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.envvar = Some(vars.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Set the help text.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Set whether this parameter is hidden from help.
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Set a custom metavar for help text.
    pub fn metavar(mut self, metavar: impl Into<String>) -> Self {
        self.metavar = Some(metavar.into());
        self
    }

    /// Mark this parameter as deprecated.
    pub fn deprecated(mut self, deprecated: bool) -> Self {
        self.deprecated = if deprecated {
            Some(DeprecationInfo::default())
        } else {
            None
        };
        self
    }

    /// Mark this parameter as deprecated with a custom message.
    pub fn deprecated_with_message(mut self, message: impl Into<String>) -> Self {
        self.deprecated = Some(DeprecationInfo::with_message(message));
        self
    }

    /// Validate the configuration.
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), ClickError> {
        // A deprecated parameter cannot be required
        if self.deprecated.is_some() && self.required {
            return Err(ClickError::usage(format!(
                "The parameter '{}' is deprecated and required. \
                 A deprecated parameter cannot be required.",
                self.name
            )));
        }
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nargs_default() {
        let nargs = Nargs::default();
        assert_eq!(nargs, Nargs::Count(1));
        assert!(nargs.is_single());
        assert!(!nargs.is_multi());
        assert!(!nargs.is_variadic());
        assert!(!nargs.is_optional());
    }

    #[test]
    fn test_nargs_count() {
        let nargs = Nargs::Count(3);
        assert!(!nargs.is_single());
        assert!(nargs.is_multi());
        assert!(!nargs.is_variadic());
        assert!(!nargs.is_optional());
        assert_eq!(nargs.count(), Some(3));
    }

    #[test]
    fn test_nargs_variadic() {
        let nargs = Nargs::Variadic;
        assert!(!nargs.is_single());
        assert!(nargs.is_multi());
        assert!(nargs.is_variadic());
        assert!(!nargs.is_optional());
        assert_eq!(nargs.count(), None);
    }

    #[test]
    fn test_nargs_optional() {
        let nargs = Nargs::Optional;
        assert!(!nargs.is_single());
        assert!(!nargs.is_multi());
        assert!(!nargs.is_variadic());
        assert!(nargs.is_optional());
        assert_eq!(nargs.count(), None);
    }

    #[test]
    fn test_nargs_display() {
        assert_eq!(Nargs::Count(1).to_string(), "1");
        assert_eq!(Nargs::Count(3).to_string(), "3");
        assert_eq!(Nargs::Variadic.to_string(), "-1");
        assert_eq!(Nargs::Optional.to_string(), "?");
    }

    #[test]
    fn test_parameter_config_builder() {
        let config = ParameterConfig::new("name")
            .nargs(Nargs::Count(2))
            .multiple(true)
            .eager(true)
            .expose_value(false)
            .required(true)
            .envvar("MY_VAR")
            .help("Help text")
            .hidden(false)
            .metavar("VALUE");

        assert_eq!(config.name, "name");
        assert_eq!(config.nargs, Nargs::Count(2));
        assert!(config.multiple);
        assert!(config.is_eager);
        assert!(!config.expose_value);
        assert!(config.required);
        assert_eq!(config.envvar, Some(vec!["MY_VAR".to_string()]));
        assert_eq!(config.help, Some("Help text".to_string()));
        assert!(!config.hidden);
        assert_eq!(config.metavar, Some("VALUE".to_string()));
    }

    #[test]
    fn test_parameter_config_envvars() {
        let config = ParameterConfig::new("name").envvars(["VAR1", "VAR2", "VAR3"]);

        assert_eq!(
            config.envvar,
            Some(vec![
                "VAR1".to_string(),
                "VAR2".to_string(),
                "VAR3".to_string()
            ])
        );
    }

    #[test]
    fn test_parameter_config_default() {
        let config = ParameterConfig::default();

        assert_eq!(config.name, "");
        assert_eq!(config.nargs, Nargs::Count(1));
        assert!(!config.multiple);
        assert!(!config.is_eager);
        assert!(config.expose_value);
        assert!(!config.required);
        assert!(config.envvar.is_none());
        assert!(config.help.is_none());
        assert!(!config.hidden);
        assert!(config.metavar.is_none());
        assert!(config.deprecated.is_none());
    }

    #[test]
    fn test_parameter_config_deprecated() {
        let config = ParameterConfig::new("old_option").deprecated(true);
        assert!(config.deprecated.is_some());
        assert!(config.deprecated.as_ref().unwrap().message.is_none());

        let config =
            ParameterConfig::new("old_option").deprecated_with_message("Use --new-option instead");
        assert!(config.deprecated.is_some());
        assert_eq!(
            config.deprecated.as_ref().unwrap().message,
            Some("Use --new-option instead".to_string())
        );
    }

    #[test]
    fn test_parameter_config_validate_deprecated_required() {
        let config = ParameterConfig::new("option")
            .deprecated(true)
            .required(true);

        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("deprecated"));
        assert!(err.to_string().contains("required"));
    }

    #[test]
    fn test_parameter_config_validate_ok() {
        let config = ParameterConfig::new("option").required(true);

        assert!(config.validate().is_ok());

        let config = ParameterConfig::new("option").deprecated(true);

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_deprecation_info() {
        let info = DeprecationInfo::new();
        assert!(info.message.is_none());

        let info = DeprecationInfo::with_message("Custom message");
        assert_eq!(info.message, Some("Custom message".to_string()));
    }
}
