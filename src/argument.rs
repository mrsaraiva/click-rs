//! Positional argument parameter for click-rs.
//!
//! This module provides the `Argument` struct for positional command-line parameters.
//! Arguments are required by default and appear without dashes in the command line.
//!
//! # Reference
//!
//! Based on Python Click's `core.py:Argument` class (line 3319+).
//!
//! # Example
//!
//! ```
//! use click::argument::Argument;
//! use click::parameter::Parameter;
//!
//! let arg = Argument::new("filename")
//!     .help("The file to process")
//!     .build();
//!
//! assert!(arg.required());
//! assert_eq!(arg.human_readable_name(), "FILENAME");
//! ```

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use crate::context::Context;
use crate::error::ClickError;
use crate::parameter::{Nargs, Parameter, ParameterConfig};
use crate::types::{CompletionItem, StringType, TypeConverter};

// =============================================================================
// Type-erased Converter Wrapper
// =============================================================================

/// A type-erased wrapper for TypeConverter that stores converted values as `Box<dyn Any>`.
///
/// This allows arguments to work with any TypeConverter, not just those returning String.
pub trait AnyTypeConverter: Send + Sync {
    /// Returns the descriptive name of this type.
    fn name(&self) -> &str;

    /// Convert a string value to the target type, returning as Box<dyn Any>.
    fn convert_any(&self, value: &str) -> Result<Box<dyn Any + Send + Sync>, String>;

    /// Convert multiple string values to the target type, returning as Box<dyn Any>.
    ///
    /// By default this returns a Vec of the underlying type.
    fn convert_multi(&self, values: &[String]) -> Result<Box<dyn Any + Send + Sync>, String>;

    /// Returns the metavar for this type.
    fn get_metavar(&self) -> Option<String>;

    /// Split an environment variable value into multiple values.
    fn split_envvar_value(&self, value: &str) -> Vec<String>;

    /// Returns shell completion items for the given incomplete value.
    fn shell_complete(&self, incomplete: &str) -> Vec<CompletionItem>;
}

impl<T> AnyTypeConverter for T
where
    T: TypeConverter + Send + Sync,
    T::Value: Send + Sync + 'static,
{
    fn name(&self) -> &str {
        TypeConverter::name(self)
    }

    fn convert_any(&self, value: &str) -> Result<Box<dyn Any + Send + Sync>, String> {
        self.convert(value).map(|v| Box::new(v) as Box<dyn Any + Send + Sync>)
    }

    fn convert_multi(&self, values: &[String]) -> Result<Box<dyn Any + Send + Sync>, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(self.convert(value)?);
        }
        Ok(Box::new(converted))
    }

    fn get_metavar(&self) -> Option<String> {
        TypeConverter::get_metavar(self)
    }

    fn split_envvar_value(&self, value: &str) -> Vec<String> {
        TypeConverter::split_envvar_value(self, value)
    }

    fn shell_complete(&self, incomplete: &str) -> Vec<CompletionItem> {
        TypeConverter::shell_complete(self, incomplete)
    }
}

/// Custom shell completion callback type for arguments.
pub type ShellCompleteCallback = Arc<dyn Fn(&Context, &str) -> Vec<CompletionItem> + Send + Sync>;

// =============================================================================
// Argument Struct
// =============================================================================

/// A positional command-line argument.
///
/// Arguments are positional parameters that appear without dashes. Unlike options,
/// they are required by default and have a single name (no aliases).
///
/// # Key Differences from Options
///
/// - Arguments are positional (no `--` prefix)
/// - Required by default (unless a default is provided)
/// - Single name only (no short/long aliases)
/// - Name is displayed in uppercase in help text
///
/// # Example
///
/// ```
/// use click::argument::Argument;
/// use click::parameter::Parameter;
///
/// // Required argument
/// let filename = Argument::new("filename").build();
/// assert!(filename.required());
///
/// // Optional argument with default
/// let output = Argument::new("output")
///     .default("out.txt")
///     .build();
/// assert!(!output.required());
/// ```
pub struct Argument {
    /// Parameter configuration.
    pub config: ParameterConfig,

    /// The default value (if any).
    pub default_value: Option<String>,

    /// Type converter for this argument (type-erased to support any return type).
    type_converter: Box<dyn AnyTypeConverter>,

    /// Custom shell completion callback.
    shell_complete_callback: Option<ShellCompleteCallback>,
}

impl fmt::Debug for Argument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Argument")
            .field("config", &self.config)
            .field("default_value", &self.default_value)
            .field("type_name", &self.type_converter.name())
            .field("has_shell_complete", &self.shell_complete_callback.is_some())
            .finish()
    }
}

impl Argument {
    /// Create a new argument builder with the given name.
    ///
    /// # Example
    ///
    /// ```
    /// use click::argument::Argument;
    ///
    /// let arg = Argument::builder("filename")
    ///     .help("The file to process")
    ///     .required(true)
    ///     .build();
    /// ```
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: &str) -> ArgumentBuilder {
        ArgumentBuilder::new(name)
    }

    /// Alias for `new()` - create a new argument builder.
    pub fn builder(name: &str) -> ArgumentBuilder {
        ArgumentBuilder::new(name)
    }

    /// Get the type converter for this argument.
    pub fn type_converter(&self) -> &dyn AnyTypeConverter {
        self.type_converter.as_ref()
    }

    /// Convert a string value using this argument's type converter.
    /// Returns the converted value as a boxed Any type.
    pub fn convert_any(&self, value: &str) -> Result<Box<dyn Any + Send + Sync>, String> {
        self.type_converter.convert_any(value)
    }

    /// Convert a string value using this argument's type converter.
    /// This is a convenience method that attempts to downcast to String.
    /// For non-String types, use `convert_any` instead.
    pub fn convert(&self, value: &str) -> Result<String, String> {
        let any_val = self.type_converter.convert_any(value)?;
        any_val
            .downcast::<String>()
            .map(|v| *v)
            .map_err(|_| "Type conversion returned non-String value; use convert_any() instead".to_string())
    }

    /// Get shell completions for this argument.
    ///
    /// If a custom shell_complete callback is set, it will be used.
    /// Otherwise, completions from the type converter are returned.
    pub fn get_completions(&self, ctx: &Context, incomplete: &str) -> Vec<CompletionItem> {
        if let Some(ref callback) = self.shell_complete_callback {
            callback(ctx, incomplete)
        } else {
            self.type_converter.shell_complete(incomplete)
        }
    }

    /// Check if this argument has a custom shell_complete callback.
    pub fn has_shell_complete_callback(&self) -> bool {
        self.shell_complete_callback.is_some()
    }

    /// Get the default value for this argument.
    pub fn default_value(&self) -> Option<&str> {
        self.default_value.as_deref()
    }

    /// Generate the metavar for this argument (used in help text).
    ///
    /// The metavar is formatted according to Click's conventions:
    /// - Custom metavar if set, otherwise uppercase name
    /// - Wrapped in `[]` if optional
    /// - Suffixed with `...` if variadic or multiple
    /// - Suffixed with `!` if deprecated
    ///
    /// Note: Unlike options, arguments use their name (uppercase) by default,
    /// not the type's metavar. This matches Python Click's behavior.
    pub fn make_metavar(&self) -> String {
        let mut var = if let Some(metavar) = &self.config.metavar {
            metavar.clone()
        } else if let Some(type_metavar) = self.type_converter.get_metavar() {
            if type_metavar.contains('|') {
                if type_metavar.contains('{') || type_metavar.contains('}') {
                    type_metavar
                } else {
                    format!("{{{}}}", type_metavar)
                }
            } else {
                self.config.name.to_uppercase()
            }
        } else {
            self.config.name.to_uppercase()
        };

        // Add deprecation marker
        if self.config.deprecated.is_some() {
            var.push('!');
        }

        // Wrap in brackets if optional
        if !self.config.required {
            var = format!("[{}]", var);
        }

        // Add ellipsis for variadic or count > 1
        match self.config.nargs {
            Nargs::Variadic => var.push_str("..."),
            Nargs::Count(n) if n != 1 => var.push_str("..."),
            _ => {}
        }

        var
    }
}

impl Parameter for Argument {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn human_readable_name(&self) -> String {
        // Use metavar if set, otherwise uppercase name
        if let Some(metavar) = &self.config.metavar {
            metavar.clone()
        } else {
            self.config.name.to_uppercase()
        }
    }

    fn nargs(&self) -> Nargs {
        self.config.nargs
    }

    fn multiple(&self) -> bool {
        self.config.multiple
    }

    fn is_eager(&self) -> bool {
        self.config.is_eager
    }

    fn expose_value(&self) -> bool {
        self.config.expose_value
    }

    fn required(&self) -> bool {
        self.config.required
    }

    fn envvar(&self) -> Option<&[String]> {
        self.config.envvar.as_deref()
    }

    fn help(&self) -> Option<&str> {
        self.config.help.as_deref()
    }

    fn hidden(&self) -> bool {
        self.config.hidden
    }

    fn get_metavar(&self) -> Option<String> {
        Some(self.make_metavar())
    }

    fn get_help_record(&self) -> Option<(String, String)> {
        // Hidden arguments don't appear in help
        if self.config.hidden {
            return None;
        }

        let metavar = self.make_metavar();
        let help = self.config.help.clone().unwrap_or_default();

        Some((metavar, help))
    }

    fn param_type_name(&self) -> &str {
        "argument"
    }
}

// =============================================================================
// ArgumentBuilder
// =============================================================================

/// Builder for creating `Argument` instances.
///
/// Use `Argument::new(name)` to create a builder, then chain methods
/// to configure the argument, and finally call `build()` to create
/// the `Argument`.
///
/// # Example
///
/// ```
/// use click::argument::Argument;
/// use click::parameter::Nargs;
///
/// let arg = Argument::new("files")
///     .help("Files to process")
///     .nargs(Nargs::Variadic)
///     .required(false)
///     .build();
/// ```
pub struct ArgumentBuilder {
    config: ParameterConfig,
    default_value: Option<String>,
    type_converter: Option<Box<dyn AnyTypeConverter>>,
    /// Track if the user explicitly set required
    required_explicitly_set: bool,
    /// Custom shell completion callback
    shell_complete_callback: Option<ShellCompleteCallback>,
}

impl ArgumentBuilder {
    /// Create a new argument builder with the given name.
    fn new(name: &str) -> Self {
        Self {
            config: ParameterConfig::new(name),
            default_value: None,
            type_converter: None,
            required_explicitly_set: false,
            shell_complete_callback: None,
        }
    }

    /// Set the help text for this argument.
    pub fn help(mut self, help: &str) -> Self {
        self.config.help = Some(help.to_string());
        self
    }

    /// Set whether this argument is required.
    ///
    /// By default, arguments are required unless a default value is provided.
    pub fn required(mut self, required: bool) -> Self {
        self.config.required = required;
        self.required_explicitly_set = true;
        self
    }

    /// Set the default value for this argument.
    ///
    /// Setting a default value automatically makes the argument optional
    /// (unless `required(true)` is explicitly called).
    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    /// Set an environment variable for this argument.
    ///
    /// If the argument is not provided on the command line, the value
    /// will be read from this environment variable.
    pub fn envvar(mut self, name: &str) -> Self {
        self.config.envvar = Some(vec![name.to_string()]);
        self
    }

    /// Set multiple environment variables for this argument.
    ///
    /// The first non-empty environment variable value is used.
    pub fn envvars(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.config.envvar = Some(names.into_iter().map(|n| n.into()).collect());
        self
    }

    /// Set how many values this argument consumes.
    pub fn nargs(mut self, n: Nargs) -> Self {
        self.config.nargs = n;
        // Variadic automatically enables multiple
        if matches!(n, Nargs::Variadic) {
            self.config.multiple = true;
        }
        self
    }

    /// Set a callback invoked after conversion.
    pub fn callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Context, &dyn Parameter, Box<dyn Any + Send + Sync>)
                -> Result<Box<dyn Any + Send + Sync>, ClickError>
            + Send
            + Sync
            + 'static,
    {
        self.config.callback = Some(Arc::new(callback));
        self
    }

    /// Make this argument variadic (consume all remaining arguments).
    ///
    /// Equivalent to `nargs(Nargs::Variadic)`.
    pub fn multiple(mut self) -> Self {
        self.config.nargs = Nargs::Variadic;
        self.config.multiple = true;
        self
    }

    /// Set the type converter for this argument.
    ///
    /// By default, arguments use `STRING` which passes values through unchanged.
    /// This method accepts any TypeConverter - the converted value will be stored
    /// as `Box<dyn Any>` in the context and can be retrieved with `ctx.get_param::<T>()`.
    ///
    /// # Example
    ///
    /// ```
    /// use click::argument::Argument;
    /// use click::types::{INT, FileType};
    ///
    /// // Integer argument
    /// let count = Argument::new("count")
    ///     .type_(INT)
    ///     .build();
    ///
    /// // File argument (opens the file)
    /// let input = Argument::new("input")
    ///     .type_(FileType::new())
    ///     .build();
    /// ```
    pub fn type_<T>(mut self, type_: T) -> Self
    where
        T: TypeConverter + Send + Sync + 'static,
        T::Value: Send + Sync + 'static,
    {
        self.type_converter = Some(Box::new(type_));
        self
    }

    /// Set a custom shell completion callback for this argument.
    ///
    /// The callback receives the current context and the incomplete string
    /// being typed, and should return a list of completion items.
    ///
    /// # Example
    ///
    /// ```
    /// use click::argument::Argument;
    /// use click::types::CompletionItem;
    ///
    /// let arg = Argument::new("filename")
    ///     .shell_complete(|_ctx, incomplete| {
    ///         // Return file completions matching the prefix
    ///         vec![
    ///             CompletionItem::new(format!("{}.txt", incomplete)),
    ///             CompletionItem::new(format!("{}.md", incomplete)),
    ///         ]
    ///     })
    ///     .build();
    /// ```
    pub fn shell_complete<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Context, &str) -> Vec<CompletionItem> + Send + Sync + 'static,
    {
        self.shell_complete_callback = Some(Arc::new(callback));
        self
    }

    /// Set a custom metavar for help text.
    ///
    /// By default, the uppercase name is used.
    pub fn metavar(mut self, metavar: &str) -> Self {
        self.config.metavar = Some(metavar.to_string());
        self
    }

    /// Hide this argument from help output.
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.config.hidden = hidden;
        self
    }

    /// Set whether this argument is eager (processed before others).
    pub fn eager(mut self, eager: bool) -> Self {
        self.config.is_eager = eager;
        self
    }

    /// Set whether this argument's value is exposed in ctx.params.
    pub fn expose_value(mut self, expose: bool) -> Self {
        self.config.expose_value = expose;
        self
    }

    /// Mark this argument as deprecated.
    pub fn deprecated(mut self, deprecated: bool) -> Self {
        self.config = self.config.deprecated(deprecated);
        self
    }

    /// Mark this argument as deprecated with a custom message.
    pub fn deprecated_with_message(mut self, message: impl Into<String>) -> Self {
        self.config = self.config.deprecated_with_message(message);
        self
    }

    /// Build the `Argument`.
    ///
    /// This method applies the following defaults:
    /// - If `required` was not explicitly set:
    ///   - If a default value is provided, the argument is optional
    ///   - Otherwise, the argument is required (if nargs > 0)
    /// - The type converter defaults to `STRING`
    pub fn build(mut self) -> Argument {
        // Apply Click's auto-detection logic for required status
        if !self.required_explicitly_set {
            if self.default_value.is_some() {
                // If a default is provided, argument is optional
                self.config.required = false;
            } else {
                // Otherwise, required if nargs > 0
                self.config.required = match self.config.nargs {
                    Nargs::Count(n) => n > 0,
                    Nargs::Variadic => true,
                    Nargs::Optional => false,
                };
            }
        }

        // Use STRING type by default
        let type_converter: Box<dyn AnyTypeConverter> =
            self.type_converter.unwrap_or_else(|| Box::new(StringType));

        Argument {
            config: self.config,
            default_value: self.default_value,
            type_converter,
            shell_complete_callback: self.shell_complete_callback,
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
    fn test_argument_creation_defaults() {
        let arg = Argument::new("filename").build();

        assert_eq!(arg.name(), "filename");
        assert!(arg.required());
        assert!(!arg.multiple());
        assert!(!arg.is_eager());
        assert!(arg.expose_value());
        assert_eq!(arg.nargs(), Nargs::Count(1));
        assert!(arg.default_value().is_none());
        assert_eq!(arg.param_type_name(), "argument");
    }

    #[test]
    fn test_argument_human_readable_name() {
        // Default: uppercase name
        let arg = Argument::new("filename").build();
        assert_eq!(arg.human_readable_name(), "FILENAME");

        // Custom metavar
        let arg = Argument::new("file").metavar("PATH").build();
        assert_eq!(arg.human_readable_name(), "PATH");
    }

    #[test]
    fn test_argument_with_default_is_optional() {
        let arg = Argument::new("output").default("out.txt").build();

        assert!(!arg.required());
        assert_eq!(arg.default_value(), Some("out.txt"));
    }

    #[test]
    fn test_argument_explicit_required_with_default() {
        // Can still make it required even with a default
        let arg = Argument::new("output")
            .default("out.txt")
            .required(true)
            .build();

        assert!(arg.required());
        assert_eq!(arg.default_value(), Some("out.txt"));
    }

    #[test]
    fn test_argument_explicit_optional_without_default() {
        let arg = Argument::new("output").required(false).build();

        assert!(!arg.required());
        assert!(arg.default_value().is_none());
    }

    #[test]
    fn test_argument_variadic() {
        let arg = Argument::new("files").multiple().build();

        assert!(arg.multiple());
        assert_eq!(arg.nargs(), Nargs::Variadic);
    }

    #[test]
    fn test_argument_nargs_variadic() {
        let arg = Argument::new("files").nargs(Nargs::Variadic).build();

        assert!(arg.multiple());
        assert_eq!(arg.nargs(), Nargs::Variadic);
    }

    #[test]
    fn test_argument_nargs_optional() {
        let arg = Argument::new("file").nargs(Nargs::Optional).build();

        // Optional nargs means the argument itself is not required
        assert!(!arg.required());
        assert_eq!(arg.nargs(), Nargs::Optional);
    }

    #[test]
    fn test_argument_nargs_count_zero() {
        let arg = Argument::new("flag").nargs(Nargs::Count(0)).build();

        // nargs=0 means not required
        assert!(!arg.required());
    }

    #[test]
    fn test_argument_help_record_required() {
        let arg = Argument::new("filename").help("The input file").build();

        let record = arg.get_help_record();
        assert!(record.is_some());

        let (metavar, help) = record.unwrap();
        assert_eq!(metavar, "FILENAME");
        assert_eq!(help, "The input file");
    }

    #[test]
    fn test_argument_help_record_optional() {
        let arg = Argument::new("filename")
            .required(false)
            .help("The input file")
            .build();

        let record = arg.get_help_record();
        assert!(record.is_some());

        let (metavar, help) = record.unwrap();
        assert_eq!(metavar, "[FILENAME]");
        assert_eq!(help, "The input file");
    }

    #[test]
    fn test_argument_help_record_variadic() {
        let arg = Argument::new("files")
            .multiple()
            .help("Files to process")
            .build();

        let record = arg.get_help_record();
        assert!(record.is_some());

        let (metavar, _) = record.unwrap();
        assert_eq!(metavar, "FILES...");
    }

    #[test]
    fn test_argument_help_record_optional_variadic() {
        let arg = Argument::new("files").multiple().required(false).build();

        let record = arg.get_help_record();
        let (metavar, _) = record.unwrap();
        assert_eq!(metavar, "[FILES]...");
    }

    #[test]
    fn test_argument_hidden() {
        let arg = Argument::new("secret").hidden(true).build();

        assert!(arg.hidden());
        assert!(arg.get_help_record().is_none());
    }

    #[test]
    fn test_argument_envvar() {
        let arg = Argument::new("filename").envvar("MY_FILE").build();

        let envvars = arg.envvar();
        assert!(envvars.is_some());
        assert_eq!(envvars.unwrap(), &["MY_FILE"]);
    }

    #[test]
    fn test_argument_multiple_envvars() {
        let arg = Argument::new("filename")
            .envvars(["MY_FILE", "FALLBACK_FILE"])
            .build();

        let envvars = arg.envvar();
        assert!(envvars.is_some());
        assert_eq!(envvars.unwrap(), &["MY_FILE", "FALLBACK_FILE"]);
    }

    #[test]
    fn test_argument_convert() {
        let arg = Argument::new("text").build();

        let result = arg.convert("hello world");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn test_argument_deprecated_marker() {
        let arg = Argument::new("old").deprecated(true).build();

        let metavar = arg.make_metavar();
        assert!(metavar.contains('!'));
    }

    #[test]
    fn test_argument_custom_metavar() {
        let arg = Argument::new("file").metavar("PATH").build();

        assert_eq!(arg.make_metavar(), "PATH");
    }

    #[test]
    fn test_argument_nargs_count_multiple() {
        let arg = Argument::new("pair").nargs(Nargs::Count(2)).build();

        let metavar = arg.make_metavar();
        assert_eq!(metavar, "PAIR...");
    }

    #[test]
    fn test_argument_debug() {
        let arg = Argument::new("test").build();
        let debug_str = format!("{:?}", arg);
        assert!(debug_str.contains("Argument"));
        assert!(debug_str.contains("test"));
    }

    // =========================================================================
    // Tests for Gap 1: Non-String type converters
    // =========================================================================

    #[test]
    fn test_argument_with_int_type() {
        use crate::types::INT;

        let arg = Argument::new("count").type_(INT).build();

        // Convert using convert_any
        let result = arg.convert_any("42");
        assert!(result.is_ok());

        // Downcast to i64
        let boxed = result.unwrap();
        let value = boxed.downcast_ref::<i64>();
        assert!(value.is_some());
        assert_eq!(*value.unwrap(), 42i64);
    }

    #[test]
    fn test_argument_with_int_type_error() {
        use crate::types::INT;

        let arg = Argument::new("count").type_(INT).build();

        let result = arg.convert_any("not-a-number");
        assert!(result.is_err());
    }

    #[test]
    fn test_argument_with_float_type() {
        use crate::types::FLOAT;

        let arg = Argument::new("value").type_(FLOAT).build();

        let result = arg.convert_any("3.14");
        assert!(result.is_ok());

        let boxed = result.unwrap();
        let value = boxed.downcast_ref::<f64>();
        assert!(value.is_some());
        assert!((value.unwrap() - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_argument_with_path_type() {
        use crate::types::PathType;
        use std::path::PathBuf;

        let arg = Argument::new("path").type_(PathType::new()).build();

        let result = arg.convert_any("/some/path");
        assert!(result.is_ok());

        let boxed = result.unwrap();
        let value = boxed.downcast_ref::<PathBuf>();
        assert!(value.is_some());
        assert_eq!(*value.unwrap(), PathBuf::from("/some/path"));
    }

    #[test]
    fn test_argument_with_choice_type() {
        use crate::types::Choice;

        let arg = Argument::new("format")
            .type_(Choice::new(["json", "xml", "yaml"]))
            .build();

        let result = arg.convert_any("json");
        assert!(result.is_ok());

        let boxed = result.unwrap();
        let value = boxed.downcast_ref::<String>();
        assert!(value.is_some());
        assert_eq!(value.unwrap(), "json");

        // Invalid choice
        let result = arg.convert_any("csv");
        assert!(result.is_err());
    }

    #[test]
    fn test_argument_string_convert_fallback() {
        // The convert() method should still work for String types
        let arg = Argument::new("text").build();
        let result = arg.convert("hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_argument_convert_non_string_returns_error() {
        use crate::types::INT;

        // When using convert() on non-String type, it should return an error
        let arg = Argument::new("count").type_(INT).build();
        let result = arg.convert("42");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-String"));
    }

    // =========================================================================
    // Tests for Gap 2: Custom shell_complete callback
    // =========================================================================

    #[test]
    fn test_argument_shell_complete_callback() {
        use crate::context::ContextBuilder;

        let arg = Argument::new("filename")
            .shell_complete(|_ctx, incomplete| {
                vec![
                    CompletionItem::new(format!("{}.txt", incomplete)),
                    CompletionItem::new(format!("{}.md", incomplete)),
                ]
            })
            .build();

        assert!(arg.has_shell_complete_callback());

        let ctx = ContextBuilder::new().build();
        let completions = arg.get_completions(&ctx, "test");

        assert_eq!(completions.len(), 2);
        assert_eq!(completions[0].value, "test.txt");
        assert_eq!(completions[1].value, "test.md");
    }

    #[test]
    fn test_argument_shell_complete_from_type() {
        use crate::context::ContextBuilder;
        use crate::types::Choice;

        // Without custom callback, completions come from the type converter
        let arg = Argument::new("format")
            .type_(Choice::new(["json", "xml", "yaml"]))
            .build();

        assert!(!arg.has_shell_complete_callback());

        let ctx = ContextBuilder::new().build();
        let completions = arg.get_completions(&ctx, "j");

        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].value, "json");
    }

    #[test]
    fn test_argument_shell_complete_overrides_type() {
        use crate::context::ContextBuilder;
        use crate::types::Choice;

        // Custom callback should override type's completions
        let arg = Argument::new("format")
            .type_(Choice::new(["json", "xml", "yaml"]))
            .shell_complete(|_ctx, _incomplete| {
                vec![CompletionItem::new("custom")]
            })
            .build();

        let ctx = ContextBuilder::new().build();
        let completions = arg.get_completions(&ctx, "j");

        // Should use custom callback, not Choice completions
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].value, "custom");
    }

    #[test]
    fn test_argument_no_shell_complete() {
        use crate::context::ContextBuilder;

        // Default STRING type has no completions
        let arg = Argument::new("text").build();

        assert!(!arg.has_shell_complete_callback());

        let ctx = ContextBuilder::new().build();
        let completions = arg.get_completions(&ctx, "test");

        assert!(completions.is_empty());
    }

    #[test]
    fn test_argument_shell_complete_with_context() {
        use crate::context::ContextBuilder;

        // Test that the callback receives the context
        let arg = Argument::new("name")
            .shell_complete(|ctx, incomplete| {
                // Use info_name from context in completion
                let prefix = ctx.info_name().unwrap_or("default");
                vec![CompletionItem::new(format!("{}_{}", prefix, incomplete))]
            })
            .build();

        let ctx = ContextBuilder::new().info_name("myapp").build();
        let completions = arg.get_completions(&ctx, "test");

        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].value, "myapp_test");
    }
}
