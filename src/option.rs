//! Option parameter for click-rs.
//!
//! This module provides the `ClickOption` struct for named command-line parameters
//! (e.g., `--name`, `-n`). Options differ from arguments in that they are
//! typically optional and can have flags, boolean modes, and prompts.
//!
//! # Reference
//!
//! Based on Python Click's `core.py:Option` class (line 2641+).

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use crate::context::Context;
use crate::error::ClickError;
use crate::parameter::{Nargs, Parameter, ParameterCallback, ParameterConfig};
use crate::argument::AnyTypeConverter;
use crate::types::{StringType, TypeConverter, STRING};

// =============================================================================
// Option Name Parsing
// =============================================================================

/// Parse a single option name, returning (stripped_name, is_long).
///
/// - Long options start with "--" (e.g., "--name" -> ("name", true))
/// - Short options start with "-" and are single char (e.g., "-n" -> ("n", false))
///
/// # Errors
///
/// Returns an error if the name is not a valid option format.
pub fn parse_option_name(name: &str) -> Result<(String, bool), String> {
    if let Some(stripped) = name.strip_prefix("--") {
        if stripped.is_empty() {
            return Err("Long option name cannot be empty: '--'".to_string());
        }
        if stripped.starts_with('-') {
            return Err(format!(
                "Long option name cannot start with dash: '{}'",
                name
            ));
        }
        Ok((stripped.to_string(), true))
    } else if let Some(stripped) = name.strip_prefix('-') {
        if stripped.is_empty() {
            return Err("Short option name cannot be empty: '-'".to_string());
        }
        if stripped.len() != 1 {
            return Err(format!(
                "Short option must be a single character: '{}' (use '--{}' for long options)",
                name, stripped
            ));
        }
        if stripped.starts_with('-') {
            return Err(format!("Short option cannot be a dash: '{}'", name));
        }
        Ok((stripped.to_string(), false))
    } else {
        Err(format!(
            "Option name must start with '-' or '--': '{}'",
            name
        ))
    }
}

/// Split option names into (long_options, short_options).
///
/// Each name is validated and categorized. The first valid long option
/// (or first short option if no long) determines the parameter name.
///
/// # Returns
///
/// A tuple of (long_options, short_options) where each is a Vec of the
/// original names (with dashes).
pub fn split_option_names(names: &[&str]) -> Result<(Vec<String>, Vec<String>), String> {
    let mut long = Vec::new();
    let mut short = Vec::new();

    for name in names {
        let (_, is_long) = parse_option_name(name)?;
        if is_long {
            long.push(name.to_string());
        } else {
            short.push(name.to_string());
        }
    }

    if long.is_empty() && short.is_empty() {
        return Err("At least one option name is required".to_string());
    }

    Ok((long, short))
}

/// Derive the parameter name from option names.
///
/// Prefers long options over short. Strips dashes and replaces '-' with '_'.
fn derive_param_name(long: &[String], short: &[String]) -> String {
    let source = if !long.is_empty() {
        // Use the longest long option (most descriptive)
        long.iter()
            .max_by_key(|s| s.len())
            .map(|s| s.trim_start_matches('-'))
            .unwrap_or("")
    } else if !short.is_empty() {
        short
            .first()
            .map(|s| s.trim_start_matches('-'))
            .unwrap_or("")
    } else {
        ""
    };

    source.replace('-', "_")
}

// =============================================================================
// ClickOption Struct
// =============================================================================

/// A named command-line parameter (e.g., --name, -n).
///
/// Options are typically optional values that can be specified with flags.
/// They support various modes including:
/// - Regular value options (`--name VALUE`)
/// - Flags (`--verbose`)
/// - Boolean flags (`--flag/--no-flag`)
/// - Count mode (`-v -v -v` = 3)
/// - Prompts for interactive input
///
/// Note: Named `ClickOption` to avoid shadowing `std::option::Option`.
#[derive(Clone)]
pub struct ClickOption {
    /// Base parameter configuration.
    pub config: ParameterConfig,

    /// Long option names (e.g., ["--name", "--full-name"]).
    pub long: Vec<String>,

    /// Short option names (e.g., ["-n", "-N"]).
    pub short: Vec<String>,

    /// Whether this is a flag (no value required).
    pub is_flag: bool,

    /// Whether this is a boolean flag (--flag/--no-flag).
    pub is_bool_flag: bool,

    /// Value when flag is present (None means true for bool flags).
    pub flag_value: Option<String>,

    /// Secondary value for --no-flag (None means false for bool flags).
    pub secondary_value: Option<String>,

    /// Count mode (-v -v -v = 3).
    pub count: bool,

    /// Prompt text for interactive input (None = no prompt).
    pub prompt: Option<String>,

    /// Whether to ask twice (confirmation).
    pub confirmation_prompt: bool,

    /// Hide input (for passwords).
    pub hide_input: bool,

    /// Show default value in help text.
    pub show_default: bool,

    /// Show environment variable in help text.
    pub show_envvar: bool,

    /// Default value as string.
    pub default: Option<String>,

    /// Type name (for display/debugging).
    type_name: String,
    /// Type metavar (for help text).
    type_metavar: Option<String>,
    /// Type converter (type-erased).
    type_converter: Arc<dyn AnyTypeConverter>,
}

impl fmt::Debug for ClickOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClickOption")
            .field("config", &self.config)
            .field("long", &self.long)
            .field("short", &self.short)
            .field("is_flag", &self.is_flag)
            .field("is_bool_flag", &self.is_bool_flag)
            .field("flag_value", &self.flag_value)
            .field("secondary_value", &self.secondary_value)
            .field("count", &self.count)
            .field("prompt", &self.prompt)
            .field("confirmation_prompt", &self.confirmation_prompt)
            .field("hide_input", &self.hide_input)
            .field("show_default", &self.show_default)
            .field("show_envvar", &self.show_envvar)
            .field("default", &self.default)
            .field("type_name", &self.type_name)
            .field("has_type_converter", &true)
            .finish()
    }
}

impl ClickOption {
    /// Create a new option builder with the given names.
    ///
    /// Names should include the dashes (e.g., "--name", "-n").
    ///
    /// # Panics
    ///
    /// Panics if no valid option names are provided.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(names: &[&str]) -> OptionBuilder {
        OptionBuilder::new(names)
    }

    /// Get all option names (long and short) joined for display.
    pub fn opts_string(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        for s in &self.short {
            parts.push(s);
        }
        for l in &self.long {
            parts.push(l);
        }
        parts.join(", ")
    }

    /// Get the primary option string (first long or first short).
    pub fn primary_opt(&self) -> &str {
        self.long
            .first()
            .or(self.short.first())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Get the type converter for this option.
    pub fn type_converter(&self) -> &dyn AnyTypeConverter {
        self.type_converter.as_ref()
    }

    /// Convert a string value using this option's type converter.
    /// Returns the converted value as a boxed Any type.
    pub fn convert_any(&self, value: &str) -> Result<Box<dyn Any + Send + Sync>, String> {
        self.type_converter.convert_any(value)
    }

    /// Convert multiple string values using this option's type converter.
    /// Returns the converted value as a boxed Any type.
    pub fn convert_multi(&self, values: &[String]) -> Result<Box<dyn Any + Send + Sync>, String> {
        self.type_converter.convert_multi(values)
    }
}

impl Parameter for ClickOption {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn human_readable_name(&self) -> String {
        self.opts_string()
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
        // Custom metavar takes precedence
        if let Some(ref mv) = self.config.metavar {
            return Some(mv.clone());
        }
        // Otherwise use type's metavar
        if let Some(ref mv) = self.type_metavar {
            if mv.contains('|') && !(mv.starts_with('[') && mv.ends_with(']')) {
                return Some(format!("[{}]", mv));
            }
            return Some(mv.clone());
        }
        if !self.type_name.is_empty() {
            return Some(self.type_name.clone());
        }
        None
    }

    fn get_help_record(&self) -> Option<(String, String)> {
        if self.hidden() {
            return None;
        }

        // Build the option string
        let mut opt_parts = Vec::new();

        // Add short options first
        for s in &self.short {
            opt_parts.push(s.clone());
        }

        // Add long options
        for l in &self.long {
            opt_parts.push(l.clone());
        }

        let mut opt_str = opt_parts.join(", ");

        // Add metavar for non-flag options
        if !self.is_flag && !self.count {
            if let Some(metavar) = self.get_metavar() {
                opt_str.push(' ');
                opt_str.push_str(&metavar);
            }
        }

        // Build help string with extras
        let mut help = self.help().unwrap_or("").to_string();
        let mut extras = Vec::new();

        // Add envvar info
        if self.show_envvar {
            if let Some(envvars) = self.envvar() {
                extras.push(format!("env var: {}", envvars.join(", ")));
            }
        }

        // Add default info
        if self.show_default {
            if let Some(ref default) = self.default {
                if !default.is_empty() {
                    extras.push(format!("default: {}", default));
                }
            }
        }

        // Add required marker
        if self.required() {
            extras.push("required".to_string());
        }

        // Append extras to help
        if !extras.is_empty() {
            let extra_str = extras.join("; ");
            if help.is_empty() {
                help = format!("[{}]", extra_str);
            } else {
                help = format!("{}  [{}]", help, extra_str);
            }
        }

        Some((opt_str, help))
    }

    fn param_type_name(&self) -> &str {
        "option"
    }
}

// =============================================================================
// OptionBuilder
// =============================================================================

/// Builder for constructing ClickOption instances.
pub struct OptionBuilder {
    long: Vec<String>,
    short: Vec<String>,
    name: String,
    help: Option<String>,
    is_flag: bool,
    is_bool_flag: bool,
    flag_value: Option<String>,
    secondary_value: Option<String>,
    count: bool,
    required: bool,
    default: Option<String>,
    envvar: Option<Vec<String>>,
    prompt: Option<String>,
    confirmation_prompt: bool,
    hide_input: bool,
    multiple: bool,
    hidden: bool,
    eager: bool,
    show_default: bool,
    show_envvar: bool,
    metavar: Option<String>,
    type_name: String,
    type_metavar: Option<String>,
    type_converter: Option<Arc<dyn AnyTypeConverter>>,
    nargs: Nargs,
    callback: Option<ParameterCallback>,
}

impl OptionBuilder {
    /// Create a new option builder with the given names.
    ///
    /// # Panics
    ///
    /// Panics if the names cannot be parsed as valid option names.
    pub fn new(names: &[&str]) -> Self {
        let (long, short) = split_option_names(names).expect("Invalid option names");
        let name = derive_param_name(&long, &short);

        Self {
            long,
            short,
            name,
            help: None,
            is_flag: false,
            is_bool_flag: false,
            flag_value: None,
            secondary_value: None,
            count: false,
            required: false,
            default: None,
            envvar: None,
            prompt: None,
            confirmation_prompt: false,
            hide_input: false,
            multiple: false,
            hidden: false,
            eager: false,
            show_default: false,
            show_envvar: false,
            metavar: None,
            type_name: TypeConverter::name(&STRING).to_string(),
            type_metavar: TypeConverter::get_metavar(&STRING),
            type_converter: None,
            nargs: Nargs::Count(1),
            callback: None,
        }
    }

    /// Set the help text for this option.
    pub fn help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }

    /// Make this a simple flag with the given value when present.
    ///
    /// When the flag is used, its value will be set to the provided value.
    pub fn flag(mut self, value: &str) -> Self {
        self.is_flag = true;
        self.flag_value = Some(value.to_string());
        self
    }

    /// Make this a boolean flag (--flag/--no-flag style).
    ///
    /// The option will accept both --flag (true) and --no-flag (false).
    pub fn bool_flag(mut self) -> Self {
        self.is_flag = true;
        self.is_bool_flag = true;
        self.flag_value = Some("true".to_string());
        self.secondary_value = Some("false".to_string());
        self
    }

    /// Enable count mode (-v -v -v = 3).
    ///
    /// Each occurrence of the flag increments an integer counter.
    pub fn count(mut self) -> Self {
        self.count = true;
        self.is_flag = true;
        self.default = Some("0".to_string());
        self
    }

    /// Mark this option as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set a default value.
    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Set an environment variable for this option.
    pub fn envvar(mut self, name: &str) -> Self {
        self.envvar = Some(vec![name.to_string()]);
        self
    }

    /// Set multiple environment variables (first found is used).
    pub fn envvars(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.envvar = Some(names.into_iter().map(|n| n.into()).collect());
        self
    }

    /// Set a prompt text for interactive input.
    pub fn prompt(mut self, text: &str) -> Self {
        self.prompt = Some(text.to_string());
        self
    }

    /// Enable confirmation prompt (ask twice).
    pub fn confirmation_prompt(mut self, confirm: bool) -> Self {
        self.confirmation_prompt = confirm;
        self
    }

    /// Hide input (for passwords).
    pub fn hide_input(mut self, hide: bool) -> Self {
        self.hide_input = hide;
        self
    }

    /// Allow this option to be specified multiple times.
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    /// Set a callback invoked after conversion.
    pub fn callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Context, &dyn Parameter, Arc<dyn Any + Send + Sync>)
                -> Result<Arc<dyn Any + Send + Sync>, ClickError>
            + Send
            + Sync
            + 'static,
    {
        self.callback = Some(Arc::new(callback));
        self
    }

    /// Hide this option from help output.
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Make this an eager option (processed before others).
    ///
    /// Eager options like --help and --version are processed first
    /// and can short-circuit command execution.
    pub fn eager(mut self) -> Self {
        self.eager = true;
        self
    }

    /// Show the default value in help text.
    pub fn show_default(mut self) -> Self {
        self.show_default = true;
        self
    }

    /// Show the environment variable in help text.
    pub fn show_envvar(mut self) -> Self {
        self.show_envvar = true;
        self
    }

    /// Set a custom metavar for help text.
    pub fn metavar(mut self, metavar: &str) -> Self {
        self.metavar = Some(metavar.to_string());
        self
    }

    /// Set the type converter for this option.
    pub fn type_<T: TypeConverter<Value = String> + Send + Sync + 'static>(
        mut self,
        type_: T,
    ) -> Self {
        self.type_name = type_.name().to_string();
        self.type_metavar = type_.get_metavar();
        self.type_converter = Some(Arc::new(type_));
        self
    }

    /// Set the type using any TypeConverter (storing name and metavar).
    pub fn type_any<V: Send + Sync + 'static, T: TypeConverter<Value = V> + Send + Sync + 'static>(
        mut self,
        type_: T,
    ) -> Self {
        self.type_name = type_.name().to_string();
        self.type_metavar = type_.get_metavar();
        self.type_converter = Some(Arc::new(type_));
        self
    }

    /// Set how many arguments this option consumes.
    pub fn nargs(mut self, nargs: Nargs) -> Self {
        self.nargs = nargs;
        self
    }

    /// Build the ClickOption.
    pub fn build(self) -> ClickOption {
        let config = ParameterConfig {
            name: self.name,
            nargs: self.nargs,
            multiple: self.multiple,
            is_eager: self.eager,
            expose_value: true,
            required: self.required,
            envvar: self.envvar,
            help: self.help,
            hidden: self.hidden,
            metavar: self.metavar,
            deprecated: None,
            callback: self.callback,
        };

        let type_converter: Arc<dyn AnyTypeConverter> =
            self.type_converter.unwrap_or_else(|| Arc::new(StringType));

        ClickOption {
            config,
            long: self.long,
            short: self.short,
            is_flag: self.is_flag,
            is_bool_flag: self.is_bool_flag,
            flag_value: self.flag_value,
            secondary_value: self.secondary_value,
            count: self.count,
            prompt: self.prompt,
            confirmation_prompt: self.confirmation_prompt,
            hide_input: self.hide_input,
            show_default: self.show_default,
            show_envvar: self.show_envvar,
            default: self.default,
            type_name: self.type_name,
            type_metavar: self.type_metavar,
            type_converter,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Choice, INT};

    #[test]
    fn test_parse_option_name_long() {
        let (name, is_long) = parse_option_name("--name").unwrap();
        assert_eq!(name, "name");
        assert!(is_long);

        let (name, is_long) = parse_option_name("--full-name").unwrap();
        assert_eq!(name, "full-name");
        assert!(is_long);
    }

    #[test]
    fn test_parse_option_name_short() {
        let (name, is_long) = parse_option_name("-n").unwrap();
        assert_eq!(name, "n");
        assert!(!is_long);

        let (name, is_long) = parse_option_name("-v").unwrap();
        assert_eq!(name, "v");
        assert!(!is_long);
    }

    #[test]
    fn test_parse_option_name_errors() {
        assert!(parse_option_name("--").is_err());
        assert!(parse_option_name("-").is_err());
        assert!(parse_option_name("---name").is_err());
        assert!(parse_option_name("-ab").is_err()); // Multi-char short
        assert!(parse_option_name("name").is_err()); // No dash
    }

    #[test]
    fn test_split_option_names() {
        let (long, short) = split_option_names(&["--name", "-n", "--full-name"]).unwrap();
        assert_eq!(long, vec!["--name", "--full-name"]);
        assert_eq!(short, vec!["-n"]);
    }

    #[test]
    fn test_split_option_names_only_short() {
        let (long, short) = split_option_names(&["-n", "-N"]).unwrap();
        assert!(long.is_empty());
        assert_eq!(short, vec!["-n", "-N"]);
    }

    #[test]
    fn test_split_option_names_empty() {
        let result = split_option_names(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_param_name() {
        let long = vec!["--name".to_string(), "--full-name".to_string()];
        let short = vec!["-n".to_string()];
        // Should pick longest long option: --full-name -> full_name
        assert_eq!(derive_param_name(&long, &short), "full_name");

        let long = vec![];
        let short = vec!["-n".to_string()];
        assert_eq!(derive_param_name(&long, &short), "n");
    }

    #[test]
    fn test_option_builder_basic() {
        let opt = ClickOption::new(&["--name", "-n"])
            .help("The name to greet")
            .build();

        assert_eq!(opt.name(), "name");
        assert_eq!(opt.long, vec!["--name"]);
        assert_eq!(opt.short, vec!["-n"]);
        assert_eq!(opt.help(), Some("The name to greet"));
        assert!(!opt.is_flag);
        assert!(!opt.required());
    }

    #[test]
    fn test_option_flag() {
        let opt = ClickOption::new(&["--verbose", "-v"]).flag("true").build();

        assert!(opt.is_flag);
        assert_eq!(opt.flag_value, Some("true".to_string()));
    }

    #[test]
    fn test_option_bool_flag() {
        let opt = ClickOption::new(&["--debug"]).bool_flag().build();

        assert!(opt.is_flag);
        assert!(opt.is_bool_flag);
        assert_eq!(opt.flag_value, Some("true".to_string()));
        assert_eq!(opt.secondary_value, Some("false".to_string()));
    }

    #[test]
    fn test_option_count() {
        let opt = ClickOption::new(&["--verbose", "-v"]).count().build();

        assert!(opt.count);
        assert!(opt.is_flag);
        assert_eq!(opt.default, Some("0".to_string()));
    }

    #[test]
    fn test_option_help_record_value() {
        let opt = ClickOption::new(&["-n", "--name"])
            .help("Your name")
            .build();

        let (opts, help) = opt.get_help_record().unwrap();
        assert_eq!(opts, "-n, --name TEXT");
        assert_eq!(help, "Your name");
    }

    #[test]
    fn test_option_help_record_flag() {
        let opt = ClickOption::new(&["-v", "--verbose"])
            .flag("true")
            .help("Enable verbose mode")
            .build();

        let (opts, help) = opt.get_help_record().unwrap();
        assert_eq!(opts, "-v, --verbose");
        assert_eq!(help, "Enable verbose mode");
    }

    #[test]
    fn test_option_help_record_count() {
        let opt = ClickOption::new(&["--verbose", "-v"])
            .count()
            .help("Increase verbosity")
            .build();

        let (opts, help) = opt.get_help_record().unwrap();
        // Short options are listed first, then long options
        assert_eq!(opts, "-v, --verbose");
        assert_eq!(help, "Increase verbosity");
    }

    #[test]
    fn test_option_help_record_with_default() {
        let opt = ClickOption::new(&["--name"])
            .default("World")
            .show_default()
            .build();

        let (opts, help) = opt.get_help_record().unwrap();
        assert_eq!(opts, "--name TEXT");
        assert!(help.contains("default: World"));
    }

    #[test]
    fn test_option_help_record_with_envvar() {
        let opt = ClickOption::new(&["--name"])
            .envvar("MY_NAME")
            .show_envvar()
            .build();

        let (opts, help) = opt.get_help_record().unwrap();
        assert_eq!(opts, "--name TEXT");
        assert!(help.contains("env var: MY_NAME"));
    }

    #[test]
    fn test_option_help_record_required() {
        let opt = ClickOption::new(&["--name"]).required().build();

        let (opts, help) = opt.get_help_record().unwrap();
        assert_eq!(opts, "--name TEXT");
        assert!(help.contains("required"));
    }

    #[test]
    fn test_option_hidden() {
        let opt = ClickOption::new(&["--secret"]).hidden().build();

        assert!(opt.hidden());
        assert!(opt.get_help_record().is_none());
    }

    #[test]
    fn test_option_with_custom_type() {
        let opt = ClickOption::new(&["--count"]).type_any(INT).build();

        assert_eq!(opt.get_metavar(), Some("INTEGER".to_string()));
    }

    #[test]
    fn test_option_with_choice_type() {
        let opt = ClickOption::new(&["--format"])
            .type_(Choice::new(["json", "xml", "csv"]))
            .build();

        assert_eq!(opt.get_metavar(), Some("[json|xml|csv]".to_string()));
    }

    #[test]
    fn test_option_with_metavar_override() {
        let opt = ClickOption::new(&["--file"]).metavar("PATH").build();

        assert_eq!(opt.get_metavar(), Some("PATH".to_string()));
    }

    #[test]
    fn test_option_prompt() {
        let opt = ClickOption::new(&["--password"])
            .prompt("Enter password")
            .hide_input(true)
            .confirmation_prompt(true)
            .build();

        assert_eq!(opt.prompt, Some("Enter password".to_string()));
        assert!(opt.hide_input);
        assert!(opt.confirmation_prompt);
    }

    #[test]
    fn test_option_multiple() {
        let opt = ClickOption::new(&["--file", "-f"]).multiple().build();

        assert!(opt.multiple());
    }

    #[test]
    fn test_option_eager() {
        let opt = ClickOption::new(&["--help", "-h"]).eager().build();

        assert!(opt.is_eager());
    }

    #[test]
    fn test_option_human_readable_name() {
        let opt = ClickOption::new(&["-n", "--name", "--full-name"]).build();

        assert_eq!(opt.human_readable_name(), "-n, --name, --full-name");
    }

    #[test]
    fn test_option_primary_opt() {
        let opt = ClickOption::new(&["-n", "--name"]).build();
        assert_eq!(opt.primary_opt(), "--name");

        let opt = ClickOption::new(&["-n", "-N"]).build();
        assert_eq!(opt.primary_opt(), "-n");
    }

    #[test]
    fn test_option_param_type_name() {
        let opt = ClickOption::new(&["--name"]).build();
        assert_eq!(opt.param_type_name(), "option");
    }
}
