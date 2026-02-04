//! Command building block for click-rs.
//!
//! This module provides the [`Command`] struct, which is the basic building block
//! of command-line interfaces in click-rs. A command handles argument parsing
//! and invokes a callback function with the parsed values.
//!
//! # Reference
//!
//! Based on Python Click's `core.py:Command` class (line 873+).
//!
//! # Example
//!
//! ```
//! use click::command::Command;
//! use click::context::Context;
//! use click::option::ClickOption;
//! use click::argument::Argument;
//!
//! let cmd = Command::new("greet")
//!     .help("Greet someone by name")
//!     .option(
//!         ClickOption::new(&["--name", "-n"])
//!             .help("Name to greet")
//!             .default("World")
//!             .build()
//!     )
//!     .argument(
//!         Argument::new("count")
//!             .help("Number of times to greet")
//!             .build()
//!     )
//!     .callback(|ctx| {
//!         // Access parsed values from ctx.params()
//!         Ok(())
//!     })
//!     .build();
//!
//! assert_eq!(cmd.name, Some("greet".to_string()));
//! assert!(!cmd.hidden);
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use crate::argument::Argument;
use crate::context::{push_context, pop_context, Context, ContextBuilder};
use crate::error::{ClickError, ErrorContext};
use crate::option::ClickOption;
use crate::parameter::{Nargs, Parameter};
use crate::parser::{OptionAction, OptionParser, ParsedValue, NARGS_OPTIONAL};

// =============================================================================
// CommandCallback Type
// =============================================================================

/// Type for command callbacks.
///
/// The callback receives a reference to the [`Context`] containing parsed
/// parameter values and returns a `Result` indicating success or failure.
pub type CommandCallback = Box<dyn Fn(&Context) -> Result<(), ClickError> + Send + Sync>;

// =============================================================================
// Command Struct
// =============================================================================

/// A command is the basic building block of click-rs CLIs.
///
/// Commands handle argument parsing and invoke a callback with the parsed values.
/// They can be nested within [`Group`]s to create subcommand hierarchies.
///
/// # Construction
///
/// Use [`Command::new`] to create a [`CommandBuilder`] for fluent construction:
///
/// ```
/// use click::command::Command;
///
/// let cmd = Command::new("mycommand")
///     .help("Description of the command")
///     .build();
/// ```
pub struct Command {
    /// The name of the command.
    ///
    /// When registered on a [`Group`], this becomes the subcommand name.
    /// Use [`Context::info_name`] to get the actual invocation name.
    pub name: Option<String>,

    /// The callback to execute when the command is invoked.
    ///
    /// May be `None` if the command is just a container for subcommands.
    pub callback: Option<CommandCallback>,

    /// The options for this command.
    pub options: Vec<ClickOption>,

    /// The arguments for this command.
    pub arguments: Vec<Argument>,

    /// The help text for this command.
    ///
    /// Displayed in the help output after the usage line.
    pub help: Option<String>,

    /// Text printed at the end of the help page.
    pub epilog: Option<String>,

    /// Short help shown in parent command listings.
    ///
    /// If not set, derived from the first line of `help`.
    pub short_help: Option<String>,

    /// The metavar for options (default: "[OPTIONS]").
    pub options_metavar: String,

    /// Whether to add a --help option automatically.
    pub add_help_option: bool,

    /// Show help if no arguments are provided.
    pub no_args_is_help: bool,

    /// Hide this command from help output.
    pub hidden: bool,

    /// Mark this command as deprecated.
    ///
    /// If `Some`, the command shows a deprecation warning when invoked.
    /// The string can be empty for a generic message, or contain custom text.
    pub deprecated: Option<String>,

    /// Whether to allow extra arguments (default: false).
    pub allow_extra_args: bool,

    /// Whether to allow interspersed arguments (default: true).
    pub allow_interspersed_args: bool,

    /// Whether to ignore unknown options (default: false).
    pub ignore_unknown_options: bool,

    /// Cached help option (created lazily).
    help_option: Option<ClickOption>,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .field("options", &format!("<{} options>", self.options.len()))
            .field("arguments", &format!("<{} arguments>", self.arguments.len()))
            .field("help", &self.help)
            .field("epilog", &self.epilog)
            .field("short_help", &self.short_help)
            .field("options_metavar", &self.options_metavar)
            .field("add_help_option", &self.add_help_option)
            .field("no_args_is_help", &self.no_args_is_help)
            .field("hidden", &self.hidden)
            .field("deprecated", &self.deprecated)
            .field("allow_extra_args", &self.allow_extra_args)
            .field("allow_interspersed_args", &self.allow_interspersed_args)
            .field("ignore_unknown_options", &self.ignore_unknown_options)
            .finish()
    }
}

impl Default for Command {
    fn default() -> Self {
        Self {
            name: None,
            callback: None,
            options: Vec::new(),
            arguments: Vec::new(),
            help: None,
            epilog: None,
            short_help: None,
            options_metavar: "[OPTIONS]".to_string(),
            add_help_option: true,
            no_args_is_help: false,
            hidden: false,
            deprecated: None,
            allow_extra_args: false,
            allow_interspersed_args: true,
            ignore_unknown_options: false,
            help_option: None,
        }
    }
}

impl Command {
    const VERSION_METAVAR_PREFIX: &'static str = "__click_version__:";

    /// Create a new command builder with the given name.
    ///
    /// # Example
    ///
    /// ```
    /// use click::command::Command;
    ///
    /// let cmd = Command::new("hello")
    ///     .help("Say hello")
    ///     .build();
    /// ```
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: &str) -> CommandBuilder {
        CommandBuilder::new(name)
    }

    /// Get the help option if enabled.
    ///
    /// Returns `None` if `add_help_option` is false or if the help option
    /// names conflict with existing parameters.
    pub fn get_help_option(&self, ctx: &Context) -> Option<ClickOption> {
        if !self.add_help_option {
            return None;
        }

        let help_names = self.get_help_option_names(ctx);
        if help_names.is_empty() {
            return None;
        }

        // If we have a cached help option, return a clone
        if let Some(ref help_opt) = self.help_option {
            return Some(help_opt.clone());
        }

        // Create the help option
        Some(make_help_option(&help_names))
    }

    /// Get the names available for the help option.
    ///
    /// Filters out any names that are already used by existing parameters.
    fn get_help_option_names(&self, ctx: &Context) -> Vec<String> {
        let mut all_names: std::collections::HashSet<String> =
            ctx.help_option_names().iter().cloned().collect();

        // Remove names used by existing options
        for opt in &self.options {
            for name in &opt.long {
                all_names.remove(name);
            }
            for name in &opt.short {
                all_names.remove(name);
            }
        }

        all_names.into_iter().collect()
    }

    /// Get the total number of parameters (options + arguments).
    pub fn param_count(&self) -> usize {
        self.options.len() + self.arguments.len()
    }

    /// Get all parameters (options and arguments) for this command.
    ///
    /// Returns a vector of trait objects. Options are returned first,
    /// followed by arguments, in the order they were added.
    pub fn get_params(&self, _ctx: &Context) -> Vec<&dyn Parameter> {
        let mut params: Vec<&dyn Parameter> = Vec::with_capacity(self.param_count());
        for opt in &self.options {
            params.push(opt);
        }
        for arg in &self.arguments {
            params.push(arg);
        }
        params
    }

    /// Create a context for this command.
    ///
    /// This sets up parsing state and applies command defaults.
    ///
    /// # Arguments
    ///
    /// * `info_name` - The name to display in help/usage (usually the command name)
    /// * `args` - The arguments to parse
    /// * `parent` - Optional parent context for nested commands
    ///
    /// # Example
    ///
    /// ```
    /// use click::command::Command;
    ///
    /// let cmd = Command::new("hello").build();
    /// let ctx = cmd.make_context("hello", vec![], None).unwrap();
    /// assert_eq!(ctx.info_name(), Some("hello"));
    /// ```
    pub fn make_context(
        &self,
        info_name: &str,
        args: Vec<String>,
        parent: Option<Arc<Context>>,
    ) -> Result<Context, ClickError> {
        let mut builder = ContextBuilder::new()
            .info_name(info_name)
            .allow_extra_args(self.allow_extra_args)
            .allow_interspersed_args(self.allow_interspersed_args)
            .ignore_unknown_options(self.ignore_unknown_options);

        if let Some(parent) = parent {
            builder = builder.parent(parent);
        }

        let mut ctx = builder.build();

        // Parse arguments and populate context
        self.parse_args(&mut ctx, args)?;

        Ok(ctx)
    }

    /// Parse arguments and populate the context.
    ///
    /// This method creates a parser, adds all parameters to it, parses the
    /// arguments, and stores the results in the context.
    pub fn parse_args(&self, ctx: &mut Context, args: Vec<String>) -> Result<(), ClickError> {
        // Check for no_args_is_help
        if args.is_empty() && self.no_args_is_help && !ctx.resilient_parsing() {
            // Return help instead of an error
            return Err(ClickError::Exit { code: 0 });
        }

        // Create the parser
        let mut parser = OptionParser::new()
            .allow_interspersed_args(ctx.allow_interspersed_args())
            .ignore_unknown_options(ctx.ignore_unknown_options());

        // Add help option if enabled
        let help_opt = self.get_help_option(ctx);

        // Add options to parser
        for opt in &self.options {
            self.add_option_to_parser(&mut parser, opt);
        }
        if let Some(ref help) = help_opt {
            self.add_option_to_parser(&mut parser, help);
        }

        // Add arguments to parser
        for arg in &self.arguments {
            self.add_argument_to_parser(&mut parser, arg);
        }

        // Parse arguments
        let (opts, remaining, _order) = parser.parse_args(args)?;

        // Process eager options first (like --help)
        // This must happen before validation so --help works with missing required params
        if let Some(ref help) = help_opt {
            if let Some(ParsedValue::Single(val)) = opts.get(help.name()) {
                if val == "true" {
                    // Help was requested - print help and exit
                    // Store in context for later use
                    ctx.params_mut().insert(
                        help.name().to_string(),
                        Arc::new(true) as Arc<dyn std::any::Any + Send + Sync>,
                    );
                    // In Click, help triggers an eager exit. We signal this with Exit.
                    // The caller (main) should catch this and print help.
                    return Err(ClickError::Exit { code: 0 });
                }
            }
        }

        // Process other eager options
        for opt in &self.options {
            if opt.is_eager() {
                if let Some(ParsedValue::Single(val)) = opts.get(opt.name()) {
                    if val == "true" || !val.is_empty() {
                        // Eager option was triggered - process and potentially exit
                        // For now, just mark it in context
                        ctx.params_mut().insert(
                            opt.name().to_string(),
                            Arc::new(val.clone()) as Arc<dyn std::any::Any + Send + Sync>,
                        );

                        // Special-case version option: eager print + exit, before validation.
                        if opt
                            .config
                            .metavar
                            .as_deref()
                            .is_some_and(|m| m.starts_with(Self::VERSION_METAVAR_PREFIX))
                        {
                            return Err(ClickError::Exit { code: 0 });
                        }
                    }
                }
            }
        }

        // Process options (validates required options)
        for opt in &self.options {
            self.process_option_value(ctx, opt, &opts)?;
        }
        if let Some(ref help) = help_opt {
            self.process_option_value(ctx, help, &opts)?;
        }

        // Process arguments (validates required arguments)
        for arg in &self.arguments {
            self.process_argument_value(ctx, arg, &opts)?;
        }

        // Handle extra arguments
        if !remaining.is_empty() && !ctx.allow_extra_args() && !ctx.resilient_parsing() {
            let extra_args = remaining.join(" ");
            return Err(ClickError::usage(if remaining.len() == 1 {
                format!("Got unexpected extra argument ({})", extra_args)
            } else {
                format!("Got unexpected extra arguments ({})", extra_args)
            }));
        }

        // Store remaining args in context
        ctx.args_mut().extend(remaining);

        Ok(())
    }

    /// Add a ClickOption to the parser.
    fn add_option_to_parser(&self, parser: &mut OptionParser, opt: &ClickOption) {
        let mut opts: Vec<&str> = Vec::new();
        for s in &opt.short {
            opts.push(s.as_str());
        }
        for l in &opt.long {
            opts.push(l.as_str());
        }

        let (action, nargs, const_value, flag_needs_value) = if opt.count {
            (OptionAction::Count, 0, None, false)
        } else if opt.is_flag {
            let const_val = opt.flag_value.as_deref().unwrap_or("true");
            (OptionAction::StoreConst, 0, Some(const_val), false)
        } else if opt.multiple() {
            let (nargs_val, fnv) = match opt.nargs() {
                Nargs::Count(n) => (n as i32, false),
                Nargs::Variadic => (-1, false),
                Nargs::Optional => (1, true), // Optional option values use flag_needs_value
            };
            (OptionAction::Append, nargs_val, None, fnv)
        } else {
            let (nargs_val, fnv) = match opt.nargs() {
                Nargs::Count(n) => (n as i32, false),
                Nargs::Variadic => (-1, false),
                Nargs::Optional => (1, true), // Optional option values use flag_needs_value
            };
            (OptionAction::Store, nargs_val, None, fnv)
        };

        parser.add_option_ex(&opts, opt.name(), action, nargs, const_value, flag_needs_value);
    }

    /// Add an Argument to the parser.
    fn add_argument_to_parser(&self, parser: &mut OptionParser, arg: &Argument) {
        let nargs_val = match arg.nargs() {
            Nargs::Count(n) => n as i32,
            Nargs::Variadic => -1,
            Nargs::Optional => NARGS_OPTIONAL, // Use special marker for optional args
        };

        parser.add_argument(arg.name(), nargs_val);
    }

    /// Resolve an environment variable value for a parameter, if configured.
    fn resolve_envvar_value(&self, ctx: &Context, param: &dyn Parameter) -> Option<String> {
        let mut candidates: Vec<String> = Vec::new();

        if let Some(envvars) = param.envvar() {
            candidates.extend(envvars.iter().cloned());
        } else if let Some(prefix) = ctx.auto_envvar_prefix() {
            let normalized = param.name().to_uppercase().replace('-', "_");
            candidates.push(format!("{}_{}", prefix, normalized));
        }

        for name in candidates {
            if let Ok(value) = std::env::var(&name) {
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }

        None
    }

    /// Process a parsed option value and store it in the context.
    fn process_option_value(
        &self,
        ctx: &mut Context,
        opt: &ClickOption,
        opts: &HashMap<String, ParsedValue>,
    ) -> Result<(), ClickError> {
        let name = opt.name();
        let parsed_value = opts.get(name);

        // Check for FlagNeedsValue marker from Append action
        // Uses internal namespace prefix to avoid collision with user options
        let flag_needs_value_key = format!("__click_internal_flag_needs_value_{}", name);
        let had_flag_needs_value = opts.get(&flag_needs_value_key).is_some();

        let make_error_ctx = || {
            ErrorContext::new()
                .with_command_path(ctx.command_path())
                .with_usage(self.get_usage(ctx))
                .with_help_options(ctx.help_option_names().to_vec())
        };

        let convert_single = |value: &str| -> Result<Arc<dyn std::any::Any + Send + Sync>, ClickError> {
            opt.convert_any(value)
                .map(Arc::from)
                .map_err(|msg| {
                    ClickError::bad_parameter_named(msg, opt.human_readable_name())
                        .with_context(make_error_ctx())
                })
        };

        let convert_multi = |values: &[String]| -> Result<Arc<dyn std::any::Any + Send + Sync>, ClickError> {
            opt.convert_multi(values)
                .map(Arc::from)
                .map_err(|msg| {
                    ClickError::bad_parameter_named(msg, opt.human_readable_name())
                        .with_context(make_error_ctx())
                })
        };

        // Convert ParsedValue to a boxed value for storage
        let envvar_value = if matches!(parsed_value, Some(ParsedValue::Unset) | None) {
            self.resolve_envvar_value(ctx, opt)
        } else {
            None
        };

        let value: Option<Arc<dyn std::any::Any + Send + Sync>> = match parsed_value {
            Some(ParsedValue::Count(n)) => Some(Arc::new(*n)),
            Some(ParsedValue::Flag(b)) => Some(Arc::new(*b)),
            Some(ParsedValue::Single(s)) => Some(convert_single(s)?),
            Some(ParsedValue::Multiple(v)) => {
                let mut values = v.clone();
                if had_flag_needs_value {
                    if values.is_empty() {
                        if let Some(ref flag_val) = opt.flag_value {
                            values.push(flag_val.clone());
                        } else if let Some(ref default) = opt.default {
                            values.push(default.clone());
                        } else {
                            values.push(String::new());
                        }
                    } else if let Some(ref flag_val) = opt.flag_value {
                        values.push(flag_val.clone());
                    } else {
                        values.push(String::new());
                    }
                }
                Some(convert_multi(&values)?)
            }
            Some(ParsedValue::FlagNeedsValue) => {
                let fallback = opt
                    .flag_value
                    .as_ref()
                    .or(opt.default.as_ref())
                    .cloned()
                    .unwrap_or_else(String::new);
                Some(convert_single(&fallback)?)
            }
            Some(ParsedValue::Unset) | None => {
                if let Some(envval) = envvar_value {
                    if opt.count {
                        let parsed = envval.parse::<usize>().map_err(|_| {
                            ClickError::bad_parameter_named(
                                format!("'{}' is not a valid integer.", envval),
                                opt.human_readable_name(),
                            )
                            .with_context(make_error_ctx())
                        })?;
                        Some(Arc::new(parsed))
                    } else if opt.nargs().is_multi() || opt.multiple() {
                        let values = opt.type_converter().split_envvar_value(&envval);
                        if values.is_empty() {
                            None
                        } else {
                            Some(convert_multi(&values)?)
                        }
                    } else {
                        Some(convert_single(&envval)?)
                    }
                } else if opt.count {
                    Some(Arc::new(0usize))
                } else if let Some(ref default) = opt.default {
                    if opt.nargs().is_multi() || opt.multiple() {
                        Some(convert_multi(&vec![default.clone()])?)
                    } else {
                        Some(convert_single(default)?)
                    }
                } else {
                    None
                }
            }
        };

        // Check if required option is missing
        if value.is_none() && opt.required() && !ctx.resilient_parsing() {
            let error_ctx = ErrorContext::new()
                .with_command_path(ctx.command_path())
                .with_usage(self.get_usage(ctx))
                .with_help_options(ctx.help_option_names().to_vec());
            return Err(ClickError::missing_option(opt.human_readable_name()).with_context(error_ctx));
        }

        // Store in context if expose_value is true
        if opt.expose_value() {
            if let Some(v) = value {
                ctx.params_mut().insert(name.to_string(), v);
            }
        }

        Ok(())
    }

    /// Process a parsed argument value and store it in the context.
    fn process_argument_value(
        &self,
        ctx: &mut Context,
        arg: &Argument,
        opts: &HashMap<String, ParsedValue>,
    ) -> Result<(), ClickError> {
        let name = arg.name();
        let parsed_value = opts.get(name);

        let envvar_value = if matches!(parsed_value, Some(ParsedValue::Unset) | None) {
            self.resolve_envvar_value(ctx, arg)
        } else {
            None
        };

        let make_error_ctx = || {
            ErrorContext::new()
                .with_command_path(ctx.command_path())
                .with_usage(self.get_usage(ctx))
                .with_help_options(ctx.help_option_names().to_vec())
        };

        let convert_single = |value: &str| -> Result<Arc<dyn std::any::Any + Send + Sync>, ClickError> {
            arg.convert_any(value)
                .map(Arc::from)
                .map_err(|msg| {
                    ClickError::bad_parameter_named(msg, arg.human_readable_name())
                        .with_context(make_error_ctx())
                })
        };

        let convert_multi = |values: &[String]| -> Result<Arc<dyn std::any::Any + Send + Sync>, ClickError> {
            arg.type_converter()
                .convert_multi(values)
                .map(Arc::from)
                .map_err(|msg| {
                    ClickError::bad_parameter_named(msg, arg.human_readable_name())
                        .with_context(make_error_ctx())
                })
        };

        // Convert ParsedValue to a boxed value for storage
        let value: Option<Arc<dyn std::any::Any + Send + Sync>> = match parsed_value {
            Some(ParsedValue::Single(s)) => Some(convert_single(s)?),
            Some(ParsedValue::Multiple(v)) => Some(convert_multi(v)?),
            Some(ParsedValue::Count(n)) => Some(Arc::new(*n)),
            Some(ParsedValue::Flag(b)) => Some(Arc::new(*b)),
            Some(ParsedValue::FlagNeedsValue) | Some(ParsedValue::Unset) | None => {
                if let Some(envval) = envvar_value {
                    if arg.nargs().is_multi() || arg.multiple() {
                        let values = arg.type_converter().split_envvar_value(&envval);
                        if values.is_empty() {
                            None
                        } else {
                            Some(convert_multi(&values)?)
                        }
                    } else {
                        Some(convert_single(&envval)?)
                    }
                } else {
                    arg.default_value()
                        .map(|d| {
                            if arg.nargs().is_multi() || arg.multiple() {
                                convert_multi(&vec![d.to_string()]).map_err(|e| e)
                            } else {
                                convert_single(d).map_err(|e| e)
                            }
                        })
                        .transpose()?
                }
            }
        };

        // Check if required argument is missing
        if value.is_none() && arg.required() && !ctx.resilient_parsing() {
            let error_ctx = ErrorContext::new()
                .with_command_path(ctx.command_path())
                .with_usage(self.get_usage(ctx))
                .with_help_options(ctx.help_option_names().to_vec());
            return Err(ClickError::missing_argument(arg.human_readable_name()).with_context(error_ctx));
        }

        // Store in context if expose_value is true
        if arg.expose_value() {
            if let Some(v) = value {
                ctx.params_mut().insert(name.to_string(), v);
            }
        }

        Ok(())
    }

    /// Invoke the command callback with the context.
    ///
    /// Returns `Ok(())` if no callback is set.
    pub fn invoke(&self, ctx: &Context) -> Result<(), ClickError> {
        // Show deprecation warning
        if let Some(ref deprecated) = self.deprecated {
            let extra = if deprecated.is_empty() {
                String::new()
            } else {
                format!(" {}", deprecated)
            };
            eprintln!(
                "DeprecationWarning: The command '{}' is deprecated.{}",
                self.name.as_deref().unwrap_or(""),
                extra
            );
        }

        // Invoke callback
        if let Some(ref callback) = self.callback {
            callback(ctx)
        } else {
            Ok(())
        }
    }

    /// Main entry point - make context, parse args, and invoke.
    ///
    /// This is the primary way to run a command as a CLI application.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments (typically `std::env::args().skip(1)`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use click::command::Command;
    ///
    /// let cmd = Command::new("hello")
    ///     .callback(|_ctx| {
    ///         println!("Hello, world!");
    ///         Ok(())
    ///     })
    ///     .build();
    ///
    /// let args: Vec<String> = std::env::args().skip(1).collect();
    /// if let Err(e) = cmd.main(args) {
    ///     eprintln!("{}", e.format_full());
    ///     std::process::exit(e.exit_code());
    /// }
    /// ```
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn main(&self, args: Vec<String>) -> Result<(), ClickError> {
        let prog_name = self.name.clone().unwrap_or_else(|| {
            std::env::args()
                .next()
                .unwrap_or_else(|| "program".to_string())
        });

        let args_for_eager = args.clone();

        // Try to make context - this may fail early for --help
        let ctx_result = self.make_context(&prog_name, args, None);

        match ctx_result {
            Ok(ctx) => {
                // Note: Context contains RefCell for close_callbacks which makes it !Send+!Sync.
                // This is fine for single-threaded CLI usage.
                let ctx = Arc::new(ctx);

                // Push context onto thread-local stack
                push_context(Arc::clone(&ctx));

                // Invoke the command
                let result = self.invoke(&ctx);

                // Pop context
                pop_context();

                // Run close callbacks
                ctx.close();

                result
            }
            Err(ClickError::Exit { code: 0 }) => {
                // Help (or other eager exits) was requested.
                //
                // Version is implemented as an eager option that signals Exit(0) and stores the
                // output string in the option metavar with a reserved prefix.
                if let Some(version_output) = self.get_version_output_from_args(&args_for_eager) {
                    println!("{}", version_output);
                    return Ok(());
                }

                // Default: print help.
                // Create a minimal context for help formatting.
                let ctx = ContextBuilder::new().info_name(&prog_name).build();
                println!("{}", self.get_help(&ctx));
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn arg_matches_opt(arg: &str, opt: &str) -> bool {
        if arg == opt {
            return true;
        }
        if opt.starts_with("--") && arg.starts_with(opt) && arg.get(opt.len()..opt.len() + 1) == Some("=") {
            return true;
        }
        if opt.starts_with('-') && opt.len() == 2 && !opt.starts_with("--") {
            let needle = opt.chars().nth(1).unwrap_or('\0');
            if arg.starts_with('-') && !arg.starts_with("--") {
                return arg.chars().skip(1).any(|c| c == needle);
            }
        }
        false
    }

    /// Check if a version option was triggered and return the version output.
    ///
    /// This is used internally by `main()` and `Group::main()` to handle
    /// version options that trigger `Exit { code: 0 }`.
    pub fn get_version_output_from_args(&self, args: &[String]) -> Option<String> {
        for opt in &self.options {
            let meta = opt.config.metavar.as_deref()?;
            let output = meta.strip_prefix(Self::VERSION_METAVAR_PREFIX)?;

            let mut names = opt.long.iter().chain(opt.short.iter());
            if names.any(|n| args.iter().any(|a| Self::arg_matches_opt(a, n))) {
                return Some(output.to_string());
            }
        }
        None
    }

    /// Format the usage line.
    ///
    /// Returns a string like: `Usage: COMMAND [OPTIONS] ARGUMENT`
    pub fn get_usage(&self, ctx: &Context) -> String {
        let mut pieces = Vec::new();

        // Add options metavar
        if !self.options_metavar.is_empty() {
            pieces.push(self.options_metavar.clone());
        }

        // Add argument metavars
        for arg in &self.arguments {
            if !arg.hidden() {
                pieces.push(arg.make_metavar());
            }
        }

        format!("Usage: {} {}", ctx.command_path(), pieces.join(" "))
    }

    /// Format the help text.
    ///
    /// Returns the complete help output including usage, description,
    /// options, arguments, and epilog.
    pub fn get_help(&self, ctx: &Context) -> String {
        let mut parts = Vec::new();

        // Usage line
        parts.push(self.get_usage(ctx));

        // Help text
        if let Some(ref help) = self.help {
            let text = help.lines().next().unwrap_or("");
            if !text.is_empty() {
                parts.push(String::new()); // blank line
                let help_text = if let Some(ref dep) = self.deprecated {
                    if dep.is_empty() {
                        format!("{}  (DEPRECATED)", text)
                    } else {
                        format!("{}  (DEPRECATED: {})", text, dep)
                    }
                } else {
                    text.to_string()
                };
                parts.push(format!("  {}", help_text));
            }
        } else if let Some(ref dep) = self.deprecated {
            parts.push(String::new());
            let dep_msg = if dep.is_empty() {
                "(DEPRECATED)".to_string()
            } else {
                format!("(DEPRECATED: {})", dep)
            };
            parts.push(format!("  {}", dep_msg));
        }

        // Options section
        let opt_records: Vec<(String, String)> = self
            .options
            .iter()
            .filter_map(|opt| opt.get_help_record())
            .collect();

        // Add help option record
        let help_opt = self.get_help_option(ctx);
        let help_record = help_opt.as_ref().and_then(|h| h.get_help_record());

        if !opt_records.is_empty() || help_record.is_some() {
            parts.push(String::new());
            parts.push("Options:".to_string());

            for (opt_str, help) in &opt_records {
                parts.push(format!("  {}  {}", opt_str, help));
            }
            if let Some((opt_str, help)) = help_record {
                parts.push(format!("  {}  {}", opt_str, help));
            }
        }

        // Arguments section (if any have help text)
        let arg_records: Vec<(String, String)> = self
            .arguments
            .iter()
            .filter_map(|arg| arg.get_help_record())
            .filter(|(_, help)| !help.is_empty())
            .collect();

        if !arg_records.is_empty() {
            parts.push(String::new());
            parts.push("Arguments:".to_string());
            for (metavar, help) in &arg_records {
                parts.push(format!("  {}  {}", metavar, help));
            }
        }

        // Epilog
        if let Some(ref epilog) = self.epilog {
            parts.push(String::new());
            parts.push(epilog.clone());
        }

        parts.join("\n")
    }

    /// Get the short help text for command listings.
    ///
    /// Returns `short_help` if set, otherwise derives from the first
    /// sentence of `help`.
    pub fn get_short_help(&self) -> String {
        if let Some(ref short_help) = self.short_help {
            let text = short_help.clone();
            if let Some(ref dep) = self.deprecated {
                if dep.is_empty() {
                    format!("{} (DEPRECATED)", text)
                } else {
                    format!("{} (DEPRECATED: {})", text, dep)
                }
            } else {
                text
            }
        } else if let Some(ref help) = self.help {
            // Get first line/sentence
            let text = help
                .lines()
                .next()
                .unwrap_or("")
                .split('.')
                .next()
                .unwrap_or("")
                .trim();
            if let Some(ref dep) = self.deprecated {
                if dep.is_empty() {
                    format!("{} (DEPRECATED)", text)
                } else {
                    format!("{} (DEPRECATED: {})", text, dep)
                }
            } else {
                text.to_string()
            }
        } else if let Some(ref dep) = self.deprecated {
            if dep.is_empty() {
                "(DEPRECATED)".to_string()
            } else {
                format!("(DEPRECATED: {})", dep)
            }
        } else {
            String::new()
        }
    }
}

// =============================================================================
// CommandBuilder
// =============================================================================

/// Builder for creating [`Command`] instances.
///
/// Use [`Command::new`] to create a builder, then chain methods to configure
/// the command, and finally call [`build`](CommandBuilder::build) to create
/// the command.
pub struct CommandBuilder {
    name: String,
    callback: Option<CommandCallback>,
    options: Vec<ClickOption>,
    arguments: Vec<Argument>,
    help: Option<String>,
    epilog: Option<String>,
    short_help: Option<String>,
    options_metavar: String,
    add_help_option: bool,
    help_option: Option<ClickOption>,
    no_args_is_help: bool,
    hidden: bool,
    deprecated: Option<String>,
    allow_extra_args: bool,
    allow_interspersed_args: bool,
    ignore_unknown_options: bool,
}

impl CommandBuilder {
    /// Create a new command builder with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            callback: None,
            options: Vec::new(),
            arguments: Vec::new(),
            help: None,
            epilog: None,
            short_help: None,
            options_metavar: "[OPTIONS]".to_string(),
            add_help_option: true,
            help_option: None,
            no_args_is_help: false,
            hidden: false,
            deprecated: None,
            allow_extra_args: false,
            allow_interspersed_args: true,
            ignore_unknown_options: false,
        }
    }

    /// Set the callback function for this command.
    ///
    /// The callback receives a reference to the [`Context`] and should return
    /// `Ok(())` on success or a [`ClickError`] on failure.
    pub fn callback<F>(mut self, f: F) -> Self
    where
        F: Fn(&Context) -> Result<(), ClickError> + Send + Sync + 'static,
    {
        self.callback = Some(Box::new(f));
        self
    }

    /// Add an option to this command.
    pub fn option(mut self, opt: ClickOption) -> Self {
        self.options.push(opt);
        self
    }

    /// Add an argument to this command.
    pub fn argument(mut self, arg: Argument) -> Self {
        self.arguments.push(arg);
        self
    }

    /// Set the help text for this command.
    pub fn help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }

    /// Set the epilog (text shown after help).
    pub fn epilog(mut self, epilog: &str) -> Self {
        self.epilog = Some(epilog.to_string());
        self
    }

    /// Set the short help text for command listings.
    pub fn short_help(mut self, short_help: &str) -> Self {
        self.short_help = Some(short_help.to_string());
        self
    }

    /// Set the options metavar (default: "[OPTIONS]").
    pub fn options_metavar(mut self, metavar: &str) -> Self {
        self.options_metavar = metavar.to_string();
        self
    }

    /// Set whether to add a --help option (default: true).
    pub fn add_help_option(mut self, add: bool) -> Self {
        self.add_help_option = add;
        self
    }

    /// Override the automatically generated help option.
    ///
    /// This lets you customize the help option names (e.g. `-h`, `--help`) and
    /// its help text while keeping it eager.
    ///
    /// Setting a custom help option implicitly enables `add_help_option`.
    pub fn help_option(mut self, opt: ClickOption) -> Self {
        self.add_help_option = true;
        self.help_option = Some(opt);
        self
    }

    /// Set whether to show help if no args provided (default: false).
    pub fn no_args_is_help(mut self, value: bool) -> Self {
        self.no_args_is_help = value;
        self
    }

    /// Hide this command from help output.
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Mark this command as deprecated.
    ///
    /// Pass an empty string for a generic deprecation message,
    /// or a custom message for more details.
    pub fn deprecated(mut self, message: &str) -> Self {
        self.deprecated = Some(message.to_string());
        self
    }

    /// Set whether extra arguments are allowed.
    pub fn allow_extra_args(mut self, allow: bool) -> Self {
        self.allow_extra_args = allow;
        self
    }

    /// Set whether interspersed arguments are allowed.
    pub fn allow_interspersed_args(mut self, allow: bool) -> Self {
        self.allow_interspersed_args = allow;
        self
    }

    /// Set whether unknown options should be ignored.
    pub fn ignore_unknown_options(mut self, ignore: bool) -> Self {
        self.ignore_unknown_options = ignore;
        self
    }

    /// Build the command.
    pub fn build(self) -> Command {
        Command {
            name: Some(self.name),
            callback: self.callback,
            options: self.options,
            arguments: self.arguments,
            help: self.help,
            epilog: self.epilog,
            short_help: self.short_help,
            options_metavar: self.options_metavar,
            add_help_option: self.add_help_option,
            no_args_is_help: self.no_args_is_help,
            hidden: self.hidden,
            deprecated: self.deprecated,
            allow_extra_args: self.allow_extra_args,
            allow_interspersed_args: self.allow_interspersed_args,
            ignore_unknown_options: self.ignore_unknown_options,
            help_option: self.help_option,
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create the standard help option.
fn make_help_option(names: &[String]) -> ClickOption {
    // Convert to &str for the option builder
    let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();

    ClickOption::new(&name_refs)
        .flag("true")
        .eager()
        .help("Show this message and exit.")
        .build()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::INT;

    #[test]
    fn test_command_creation_defaults() {
        let cmd = Command::new("test").build();

        assert_eq!(cmd.name, Some("test".to_string()));
        assert!(cmd.callback.is_none());
        assert!(cmd.options.is_empty());
        assert!(cmd.arguments.is_empty());
        assert!(cmd.help.is_none());
        assert!(cmd.epilog.is_none());
        assert!(cmd.short_help.is_none());
        assert_eq!(cmd.options_metavar, "[OPTIONS]");
        assert!(cmd.add_help_option);
        assert!(!cmd.no_args_is_help);
        assert!(!cmd.hidden);
        assert!(cmd.deprecated.is_none());
        assert!(!cmd.allow_extra_args);
        assert!(cmd.allow_interspersed_args);
        assert!(!cmd.ignore_unknown_options);
    }

    #[test]
    fn test_command_builder_chain() {
        let cmd = Command::new("hello")
            .help("Say hello to someone")
            .epilog("Example: hello --name World")
            .short_help("Say hello")
            .options_metavar("[OPTS]")
            .add_help_option(false)
            .no_args_is_help(true)
            .hidden()
            .deprecated("Use 'greet' instead")
            .allow_extra_args(true)
            .allow_interspersed_args(false)
            .ignore_unknown_options(true)
            .build();

        assert_eq!(cmd.name, Some("hello".to_string()));
        assert_eq!(cmd.help, Some("Say hello to someone".to_string()));
        assert_eq!(cmd.epilog, Some("Example: hello --name World".to_string()));
        assert_eq!(cmd.short_help, Some("Say hello".to_string()));
        assert_eq!(cmd.options_metavar, "[OPTS]");
        assert!(!cmd.add_help_option);
        assert!(cmd.no_args_is_help);
        assert!(cmd.hidden);
        assert_eq!(cmd.deprecated, Some("Use 'greet' instead".to_string()));
        assert!(cmd.allow_extra_args);
        assert!(!cmd.allow_interspersed_args);
        assert!(cmd.ignore_unknown_options);
    }

    #[test]
    fn test_command_with_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let cmd = Command::new("test")
            .callback(move |_ctx| {
                called_clone.store(true, Ordering::SeqCst);
                Ok(())
            })
            .build();

        assert!(cmd.callback.is_some());

        // Create a minimal context and invoke
        let ctx = ContextBuilder::new().info_name("test").build();
        let result = cmd.invoke(&ctx);
        assert!(result.is_ok());
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_command_with_option() {
        let cmd = Command::new("greet")
            .option(
                ClickOption::new(&["--name", "-n"])
                    .help("Name to greet")
                    .default("World")
                    .build(),
            )
            .build();

        assert_eq!(cmd.options.len(), 1);
        assert_eq!(cmd.options[0].name(), "name");
    }

    #[test]
    fn test_command_with_argument() {
        let cmd = Command::new("cat")
            .argument(Argument::new("file").help("File to read").build())
            .build();

        assert_eq!(cmd.arguments.len(), 1);
        assert_eq!(cmd.arguments[0].name(), "file");
    }

    #[test]
    fn test_command_with_multiple_params() {
        let cmd = Command::new("copy")
            .option(
                ClickOption::new(&["--recursive", "-r"])
                    .flag("true")
                    .help("Copy recursively")
                    .build(),
            )
            .argument(Argument::new("src").help("Source path").build())
            .argument(Argument::new("dst").help("Destination path").build())
            .build();

        assert_eq!(cmd.options.len(), 1);
        assert_eq!(cmd.arguments.len(), 2);
    }

    #[test]
    fn test_make_context_basic() {
        let cmd = Command::new("hello").build();
        let ctx = cmd.make_context("hello", vec![], None);

        assert!(ctx.is_ok());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.info_name(), Some("hello"));
    }

    #[test]
    fn test_parse_args_with_option() {
        let cmd = Command::new("greet")
            .option(
                ClickOption::new(&["--name", "-n"])
                    .default("World")
                    .build(),
            )
            .build();

        let ctx = cmd.make_context("greet", vec!["--name".to_string(), "Alice".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let name = ctx.get_param::<String>("name");
        assert_eq!(name, Some(&"Alice".to_string()));
    }

    #[test]
    fn test_parse_args_with_argument() {
        let cmd = Command::new("cat")
            .argument(Argument::new("file").build())
            .build();

        let ctx = cmd.make_context("cat", vec!["test.txt".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let file = ctx.get_param::<String>("file");
        assert_eq!(file, Some(&"test.txt".to_string()));
    }

    #[test]
    fn test_parse_args_missing_required() {
        let cmd = Command::new("cat")
            .argument(Argument::new("file").build())
            .build();

        let ctx = cmd.make_context("cat", vec![], None);
        assert!(ctx.is_err());

        let err = ctx.unwrap_err();
        assert!(matches!(err, ClickError::MissingParameter { .. }));
    }

    #[test]
    fn test_parse_args_extra_args_error() {
        let cmd = Command::new("hello").build();

        let ctx = cmd.make_context("hello", vec!["extra".to_string()], None);
        assert!(ctx.is_err());

        let err = ctx.unwrap_err();
        assert!(matches!(err, ClickError::UsageError { .. }));
    }

    #[test]
    fn test_parse_args_extra_args_allowed() {
        let cmd = Command::new("hello").allow_extra_args(true).build();

        let ctx = cmd.make_context("hello", vec!["extra".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        assert_eq!(ctx.args(), &["extra".to_string()]);
    }

    #[test]
    fn test_get_usage() {
        let cmd = Command::new("copy")
            .argument(Argument::new("src").build())
            .argument(Argument::new("dst").build())
            .build();

        let ctx = ContextBuilder::new().info_name("copy").build();
        let usage = cmd.get_usage(&ctx);

        assert!(usage.contains("Usage:"));
        assert!(usage.contains("copy"));
        assert!(usage.contains("[OPTIONS]"));
        assert!(usage.contains("SRC"));
        assert!(usage.contains("DST"));
    }

    #[test]
    fn test_get_help() {
        let cmd = Command::new("greet")
            .help("Greet someone")
            .option(
                ClickOption::new(&["--name", "-n"])
                    .help("Name to greet")
                    .build(),
            )
            .epilog("Example: greet --name World")
            .build();

        let ctx = ContextBuilder::new().info_name("greet").build();
        let help = cmd.get_help(&ctx);

        assert!(help.contains("Usage:"));
        assert!(help.contains("Greet someone"));
        assert!(help.contains("Options:"));
        assert!(help.contains("--name"));
        assert!(help.contains("Name to greet"));
        assert!(help.contains("Example:"));
    }

    #[test]
    fn test_get_short_help() {
        // Explicit short_help
        let cmd = Command::new("test").short_help("Test command").build();
        assert_eq!(cmd.get_short_help(), "Test command");

        // Derived from help
        let cmd = Command::new("test")
            .help("This is a test. It does things.")
            .build();
        assert_eq!(cmd.get_short_help(), "This is a test");

        // With deprecation
        let cmd = Command::new("test")
            .short_help("Test command")
            .deprecated("Use 'new-test' instead")
            .build();
        assert!(cmd.get_short_help().contains("DEPRECATED"));
        assert!(cmd.get_short_help().contains("Use 'new-test' instead"));
    }

    #[test]
    fn test_make_help_option() {
        let help_opt = make_help_option(&["--help".to_string(), "-h".to_string()]);

        assert!(help_opt.is_flag);
        assert!(help_opt.is_eager());
        assert_eq!(help_opt.help(), Some("Show this message and exit."));
    }

    #[test]
    fn test_command_debug() {
        let cmd = Command::new("test").build();
        let debug_str = format!("{:?}", cmd);

        assert!(debug_str.contains("Command"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_invoke_with_deprecation() {
        let cmd = Command::new("old")
            .deprecated("Use 'new' instead")
            .callback(|_| Ok(()))
            .build();

        let ctx = ContextBuilder::new().info_name("old").build();
        // This will print a deprecation warning to stderr
        let result = cmd.invoke(&ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_flag_option() {
        let cmd = Command::new("test")
            .option(
                ClickOption::new(&["--verbose", "-v"])
                    .flag("true")
                    .build(),
            )
            .build();

        let ctx = cmd.make_context("test", vec!["--verbose".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let verbose = ctx.get_param::<String>("verbose");
        assert_eq!(verbose, Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_count_option() {
        let cmd = Command::new("test")
            .option(ClickOption::new(&["--verbose", "-v"]).count().build())
            .build();

        let ctx = cmd.make_context(
            "test",
            vec!["-v".to_string(), "-v".to_string(), "-v".to_string()],
            None,
        );
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let verbose = ctx.get_param::<usize>("verbose");
        assert_eq!(verbose, Some(&3));
    }

    #[test]
    fn test_option_type_conversion() {
        let cmd = Command::new("test")
            .option(ClickOption::new(&["--count"]).type_any(INT).build())
            .build();

        let ctx = cmd.make_context("test", vec!["--count".to_string(), "42".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let count = ctx.get_param::<i64>("count");
        assert_eq!(count, Some(&42));
    }

    #[test]
    fn test_option_with_default() {
        let cmd = Command::new("greet")
            .option(
                ClickOption::new(&["--name"])
                    .default("World")
                    .build(),
            )
            .build();

        // Without providing the option
        let ctx = cmd.make_context("greet", vec![], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let name = ctx.get_param::<String>("name");
        assert_eq!(name, Some(&"World".to_string()));
    }

    #[test]
    fn test_option_envvar_value() {
        std::env::set_var("CLICK_TEST_COUNT", "9");
        let cmd = Command::new("test")
            .option(
                ClickOption::new(&["--count"])
                    .envvar("CLICK_TEST_COUNT")
                    .type_any(INT)
                    .build(),
            )
            .build();

        let ctx = cmd.make_context("test", vec![], None);
        std::env::remove_var("CLICK_TEST_COUNT");

        assert!(ctx.is_ok());
        let ctx = ctx.unwrap();
        let count = ctx.get_param::<i64>("count");
        assert_eq!(count, Some(&9));
    }

    #[test]
    fn test_argument_type_conversion() {
        let cmd = Command::new("greet")
            .argument(Argument::new("count").type_(INT).build())
            .build();

        let ctx = cmd.make_context("greet", vec!["7".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let count = ctx.get_param::<i64>("count");
        assert_eq!(count, Some(&7));
    }

    #[test]
    fn test_argument_type_conversion_error() {
        let cmd = Command::new("greet")
            .argument(Argument::new("count").type_(INT).build())
            .build();

        let ctx = cmd.make_context("greet", vec!["nope".to_string()], None);
        assert!(matches!(ctx, Err(ClickError::BadParameter { .. })));
    }

    #[test]
    fn test_argument_with_default() {
        let cmd = Command::new("greet")
            .argument(Argument::new("name").default("World").build())
            .build();

        // Without providing the argument
        let ctx = cmd.make_context("greet", vec![], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let name = ctx.get_param::<String>("name");
        assert_eq!(name, Some(&"World".to_string()));
    }

    #[test]
    fn test_argument_auto_envvar_prefix() {
        std::env::set_var("MYAPP_NAME", "Alice");
        let cmd = Command::new("greet")
            .argument(Argument::new("name").build())
            .build();

        let mut ctx = ContextBuilder::new()
            .info_name("greet")
            .auto_envvar_prefix("MYAPP")
            .build();

        let result = cmd.parse_args(&mut ctx, vec![]);
        std::env::remove_var("MYAPP_NAME");

        assert!(result.is_ok());
        let name = ctx.get_param::<String>("name");
        assert_eq!(name, Some(&"Alice".to_string()));
    }

    // -------------------------------------------------------------------------
    // Optional positional argument tests (fix for Nargs::Optional)
    // -------------------------------------------------------------------------

    #[test]
    fn test_optional_argument_with_value() {
        use crate::parameter::Nargs;

        let cmd = Command::new("test")
            .argument(
                Argument::new("file")
                    .nargs(Nargs::Optional)
                    .default("default.txt")
                    .build(),
            )
            .build();

        // With a value provided
        let ctx = cmd.make_context("test", vec!["input.txt".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let file = ctx.get_param::<String>("file");
        assert_eq!(file, Some(&"input.txt".to_string()));
    }

    #[test]
    fn test_optional_argument_without_value() {
        use crate::parameter::Nargs;

        let cmd = Command::new("test")
            .argument(
                Argument::new("file")
                    .nargs(Nargs::Optional)
                    .default("default.txt")
                    .build(),
            )
            .build();

        // Without providing the optional argument - should use default
        let ctx = cmd.make_context("test", vec![], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let file = ctx.get_param::<String>("file");
        assert_eq!(file, Some(&"default.txt".to_string()));
    }

    #[test]
    fn test_required_followed_by_optional_argument() {
        use crate::parameter::Nargs;

        let cmd = Command::new("copy")
            .argument(Argument::new("src").build()) // required
            .argument(
                Argument::new("dst")
                    .nargs(Nargs::Optional)
                    .default(".")
                    .build(),
            )
            .build();

        // Only providing required arg
        let ctx = cmd.make_context("copy", vec!["source.txt".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let src = ctx.get_param::<String>("src");
        let dst = ctx.get_param::<String>("dst");
        assert_eq!(src, Some(&"source.txt".to_string()));
        assert_eq!(dst, Some(&".".to_string()));
    }

    #[test]
    fn test_required_followed_by_optional_with_both_values() {
        use crate::parameter::Nargs;

        let cmd = Command::new("copy")
            .argument(Argument::new("src").build()) // required
            .argument(
                Argument::new("dst")
                    .nargs(Nargs::Optional)
                    .default(".")
                    .build(),
            )
            .build();

        // Providing both args
        let ctx = cmd.make_context(
            "copy",
            vec!["source.txt".to_string(), "dest.txt".to_string()],
            None,
        );
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let src = ctx.get_param::<String>("src");
        let dst = ctx.get_param::<String>("dst");
        assert_eq!(src, Some(&"source.txt".to_string()));
        assert_eq!(dst, Some(&"dest.txt".to_string()));
    }

    // -------------------------------------------------------------------------
    // Multi-value argument error test (fix for issue 2)
    // -------------------------------------------------------------------------

    #[test]
    fn test_multi_value_argument_incomplete_error() {
        use crate::parameter::Nargs;

        let cmd = Command::new("point")
            .argument(Argument::new("coords").nargs(Nargs::Count(3)).build())
            .build();

        // Only providing 2 of 3 required values
        let ctx = cmd.make_context(
            "point",
            vec!["1".to_string(), "2".to_string()],
            None,
        );

        // Should fail with an error about missing values
        assert!(ctx.is_err());
        let err = ctx.unwrap_err();
        assert!(err.to_string().contains("takes 3 values"));
    }

    // -------------------------------------------------------------------------
    // Optional option value test (fix for issue 4)
    // -------------------------------------------------------------------------

    #[test]
    fn test_optional_option_value_with_flag_value() {
        use crate::parameter::Nargs;

        let cmd = Command::new("test")
            .option(
                ClickOption::new(&["--opt"])
                    .nargs(Nargs::Optional)
                    .flag("flagval") // Used when option is used without value
                    .default("default")
                    .build(),
            )
            .build();

        // Using --opt followed by another option (would trigger FlagNeedsValue)
        let ctx = cmd.make_context("test", vec!["--opt".to_string()], None);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        let opt = ctx.get_param::<String>("opt");
        // Should use the flag_value since --opt was used without an argument
        assert_eq!(opt, Some(&"flagval".to_string()));
    }

    // -------------------------------------------------------------------------
    // Eager option (--help) tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_help_with_missing_required_arg() {
        // --help should work even when a required argument is missing
        let cmd = Command::new("test")
            .argument(Argument::new("required_file").build())
            .build();

        // Without --help, missing required arg should fail
        let ctx = cmd.make_context("test", vec![], None);
        assert!(ctx.is_err());

        // With --help, should exit cleanly (Exit code 0)
        let ctx = cmd.make_context("test", vec!["--help".to_string()], None);
        assert!(matches!(ctx, Err(ClickError::Exit { code: 0 })));
    }

    #[test]
    fn test_help_with_missing_required_option() {
        // --help should work even when a required option is missing
        let cmd = Command::new("test")
            .option(
                ClickOption::new(&["--name", "-n"])
                    .required()
                    .build(),
            )
            .build();

        // Without --help, missing required option should fail
        let ctx = cmd.make_context("test", vec![], None);
        assert!(ctx.is_err());

        // With --help, should exit cleanly (Exit code 0)
        let ctx = cmd.make_context("test", vec!["--help".to_string()], None);
        assert!(matches!(ctx, Err(ClickError::Exit { code: 0 })));
    }
}
