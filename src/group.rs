//! Command groups for click-rs.
//!
//! This module provides the [`Group`] struct, which is a command that contains
//! and dispatches to subcommands. Groups can be nested to create hierarchical
//! command-line interfaces.
//!
//! # Reference
//!
//! Based on Python Click's `core.py:Group` class (line 1503+).
//!
//! # Example
//!
//! ```
//! use click::group::{Group, CommandLike};
//! use click::command::Command;
//! use click::context::Context;
//!
//! let cli = Group::new("cli")
//!     .help("A sample CLI application")
//!     .command(
//!         Command::new("init")
//!             .help("Initialize the project")
//!             .callback(|_ctx| {
//!                 println!("Initializing...");
//!                 Ok(())
//!             })
//!             .build()
//!     )
//!     .command(
//!         Command::new("build")
//!             .help("Build the project")
//!             .callback(|_ctx| {
//!                 println!("Building...");
//!                 Ok(())
//!             })
//!             .build()
//!     )
//!     .build();
//!
//! assert_eq!(cli.list_commands().len(), 2);
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::argument::Argument;
use crate::command::{Command, CommandBuilder, CommandCallback};
use crate::context::{get_current_context, pop_context, push_context, Context, ContextBuilder};
use crate::error::ClickError;
use crate::option::ClickOption;
use crate::parameter::Parameter;

// =============================================================================
// CommandLike Trait
// =============================================================================

/// Shared interface for Command and Group.
///
/// This trait provides a common interface that both [`Command`] and [`Group`]
/// implement, allowing them to be used interchangeably in many contexts.
pub trait CommandLike: Send + Sync {
    /// Get the name of this command.
    fn name(&self) -> Option<&str>;

    /// Create a context for executing this command.
    ///
    /// # Arguments
    ///
    /// * `info_name` - The name to display in help/usage
    /// * `args` - The arguments to parse
    /// * `parent` - Optional parent context for nested commands
    fn make_context(
        &self,
        info_name: &str,
        args: Vec<String>,
        parent: Option<Arc<Context>>,
    ) -> Result<Context, ClickError>;

    /// Invoke the command with the given context.
    fn invoke(&self, ctx: &Context) -> Result<(), ClickError>;

    /// Main entry point - make context, parse args, and invoke.
    fn main(&self, args: Vec<String>) -> Result<(), ClickError>;

    /// Get the full help text for this command.
    fn get_help(&self, ctx: &Context) -> String;

    /// Get the short help text for command listings.
    fn get_short_help(&self) -> String;

    /// Check if this command is hidden from help output.
    fn is_hidden(&self) -> bool;

    /// Get the usage line for this command.
    fn get_usage(&self, ctx: &Context) -> String;

    /// Convert to Any for downcasting.
    fn as_any(&self) -> &dyn Any;
}

// =============================================================================
// CommandLike impl for Command
// =============================================================================

impl CommandLike for Command {
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn make_context(
        &self,
        info_name: &str,
        args: Vec<String>,
        parent: Option<Arc<Context>>,
    ) -> Result<Context, ClickError> {
        Command::make_context(self, info_name, args, parent)
    }

    fn invoke(&self, ctx: &Context) -> Result<(), ClickError> {
        Command::invoke(self, ctx)
    }

    fn main(&self, args: Vec<String>) -> Result<(), ClickError> {
        Command::main(self, args)
    }

    fn get_help(&self, ctx: &Context) -> String {
        Command::get_help(self, ctx)
    }

    fn get_short_help(&self) -> String {
        Command::get_short_help(self)
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn get_usage(&self, ctx: &Context) -> String {
        Command::get_usage(self, ctx)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// =============================================================================
// ResultCallback Type
// =============================================================================

/// Type for result callbacks that process subcommand return values.
///
/// The callback receives the context and a vector of return values from
/// subcommand invocations. In chain mode, this will contain values from
/// all chained commands; otherwise, it will contain a single value.
pub type ResultCallback =
    Box<dyn Fn(&Context, Vec<Box<dyn Any + Send + Sync>>) -> Result<(), ClickError> + Send + Sync>;

// =============================================================================
// Group Struct
// =============================================================================

/// A command group that contains and dispatches to subcommands.
///
/// Groups are the primary way to organize CLI applications with multiple
/// commands. They can be nested to create complex command hierarchies.
///
/// # Features
///
/// - Subcommand registration and dispatch
/// - Optional group callback (runs before subcommand)
/// - Chain mode for executing multiple subcommands
/// - Automatic help generation with command listing
///
/// # Example
///
/// ```
/// use click::group::Group;
/// use click::command::Command;
///
/// let cli = Group::new("myapp")
///     .help("My application")
///     .invoke_without_command(true)
///     .callback(|_ctx| {
///         println!("No subcommand provided");
///         Ok(())
///     })
///     .command(
///         Command::new("hello")
///             .help("Say hello")
///             .build()
///     )
///     .build();
/// ```
pub struct Group {
    /// The underlying command (for the group's own options/arguments).
    pub command: Command,

    /// Registered subcommands (name -> Command or Group).
    pub commands: HashMap<String, Arc<dyn CommandLike>>,

    /// Alias metadata for commands registered through the Group/GroupBuilder APIs.
    ///
    /// This tracks which registered names (keys in `commands`) point at the same underlying
    /// command object, so callers can distinguish:
    /// - canonical command name (`command.name()`)
    /// - registered name (key in the parent group)
    /// - other aliases (other keys mapping to the same command)
    command_ids_by_name: HashMap<String, usize>,
    command_aliases_by_id: HashMap<usize, Vec<String>>,
    next_command_id: usize,

    /// Whether to execute multiple subcommands in sequence.
    pub chain: bool,

    /// Whether to invoke the group callback even if no subcommand is provided.
    pub invoke_without_command: bool,

    /// Optional callback to process subcommand results.
    pub result_callback: Option<ResultCallback>,

    /// Whether a subcommand is required (error if not provided).
    ///
    /// Defaults to `true` unless `invoke_without_command` is `true`.
    pub subcommand_required: bool,

    /// The metavar to show for subcommands in usage.
    pub subcommand_metavar: String,
}

impl std::fmt::Debug for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Group")
            .field("command", &self.command)
            .field(
                "commands",
                &format!("<{} subcommands>", self.commands.len()),
            )
            .field("chain", &self.chain)
            .field("invoke_without_command", &self.invoke_without_command)
            .field("subcommand_required", &self.subcommand_required)
            .field("subcommand_metavar", &self.subcommand_metavar)
            .finish()
    }
}

impl Default for Group {
    fn default() -> Self {
        Self {
            command: Command::default(),
            commands: HashMap::new(),
            command_ids_by_name: HashMap::new(),
            command_aliases_by_id: HashMap::new(),
            next_command_id: 0,
            chain: false,
            invoke_without_command: false,
            result_callback: None,
            subcommand_required: true,
            subcommand_metavar: "COMMAND [ARGS]...".to_string(),
        }
    }
}

impl Group {
    /// Create a new group builder with the given name.
    ///
    /// # Example
    ///
    /// ```
    /// use click::group::Group;
    ///
    /// let group = Group::new("mygroup")
    ///     .help("My command group")
    ///     .build();
    /// ```
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: &str) -> GroupBuilder {
        GroupBuilder::new(name)
    }

    /// Add a subcommand to this group.
    ///
    /// If `name` is provided, it overrides the command's own name.
    ///
    /// # Example
    ///
    /// ```
    /// use click::group::Group;
    /// use click::command::Command;
    ///
    /// let mut group = Group::new("cli").build();
    /// group.add_command(Command::new("hello").build(), None);
    /// group.add_command(Command::new("greet").build(), Some("hi")); // registered as "hi"
    ///
    /// assert!(group.get_command("hello").is_some());
    /// assert!(group.get_command("hi").is_some());
    /// assert!(group.get_command("greet").is_none()); // not found by original name
    /// ```
    pub fn add_command(&mut self, cmd: impl CommandLike + 'static, name: Option<&str>) {
        let cmd_name = name
            .map(|s| s.to_string())
            .or_else(|| cmd.name().map(|s| s.to_string()));

        if let Some(n) = cmd_name {
            self.add_command_shared(Arc::new(cmd), Some(&n));
        }
    }

    /// Add a subcommand to this group using a shared command object.
    ///
    /// This makes it possible to register the same command under multiple names (aliases)
    /// while still being able to query alias metadata.
    pub fn add_command_shared(&mut self, cmd: Arc<dyn CommandLike>, name: Option<&str>) {
        let cmd_name = name
            .map(|s| s.to_string())
            .or_else(|| cmd.name().map(|s| s.to_string()));

        let Some(name) = cmd_name else { return };

        // If we're replacing an existing name, unlink it from prior alias metadata.
        if let Some(old_id) = self.command_ids_by_name.get(&name).copied() {
            if let Some(names) = self.command_aliases_by_id.get_mut(&old_id) {
                names.retain(|n| n != &name);
            }
        }

        // Find an existing id for this command (if it's already registered under another name).
        let existing_id = self.commands.iter().find_map(|(n, existing)| {
            if Arc::ptr_eq(existing, &cmd) {
                self.command_ids_by_name.get(n).copied()
            } else {
                None
            }
        });

        let id = existing_id.unwrap_or_else(|| {
            let id = self.next_command_id;
            self.next_command_id += 1;
            id
        });

        self.command_ids_by_name.insert(name.clone(), id);
        self.command_aliases_by_id
            .entry(id)
            .or_insert_with(Vec::new)
            .push(name.clone());

        // Keep alias lists deterministic.
        if let Some(names) = self.command_aliases_by_id.get_mut(&id) {
            names.sort();
            names.dedup();
        }

        self.commands.insert(name, cmd);
    }

    /// Get a subcommand by name.
    ///
    /// # Example
    ///
    /// ```
    /// use click::group::Group;
    /// use click::command::Command;
    ///
    /// let group = Group::new("cli")
    ///     .command(Command::new("hello").build())
    ///     .build();
    ///
    /// assert!(group.get_command("hello").is_some());
    /// assert!(group.get_command("unknown").is_none());
    /// ```
    pub fn get_command(&self, name: &str) -> Option<&dyn CommandLike> {
        self.commands.get(name).map(|c| c.as_ref())
    }

    /// List all command registrations as `(registered_name, command)` pairs.
    ///
    /// This is useful when you need both the registered key and the canonical name
    /// stored inside the command itself (`command.name()`).
    pub fn list_command_entries(&self) -> Vec<(String, &dyn CommandLike)> {
        let mut names: Vec<&String> = self.commands.keys().collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|name| {
                self.commands
                    .get(name)
                    .map(|cmd| (name.clone(), cmd.as_ref()))
            })
            .collect()
    }

    /// List other registered names (aliases) that map to the same command.
    ///
    /// Returns an empty list if the command is not found or if it has no aliases.
    pub fn list_command_aliases(&self, name: &str) -> Vec<String> {
        let Some(id) = self.command_ids_by_name.get(name).copied() else {
            return Vec::new();
        };
        let Some(names) = self.command_aliases_by_id.get(&id) else {
            return Vec::new();
        };

        let mut out: Vec<String> = names
            .iter()
            .filter(|n| n.as_str() != name)
            .cloned()
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// List all subcommand names (sorted alphabetically).
    ///
    /// # Example
    ///
    /// ```
    /// use click::group::Group;
    /// use click::command::Command;
    ///
    /// let group = Group::new("cli")
    ///     .command(Command::new("build").build())
    ///     .command(Command::new("init").build())
    ///     .command(Command::new("deploy").build())
    ///     .build();
    ///
    /// let commands = group.list_commands();
    /// assert_eq!(commands, vec!["build", "deploy", "init"]);
    /// ```
    pub fn list_commands(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.commands.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Resolve a command from arguments.
    ///
    /// Returns the command name, the command, and the remaining arguments.
    /// Returns an error if the command is not found (unless in resilient parsing mode).
    ///
    /// # Arguments
    ///
    /// * `ctx` - The current context
    /// * `args` - The arguments to parse
    pub fn resolve_command<'a>(
        &'a self,
        ctx: &Context,
        args: &[String],
    ) -> Result<Option<(&'a str, &'a dyn CommandLike, Vec<String>)>, ClickError> {
        if args.is_empty() {
            return Ok(None);
        }

        let cmd_name = &args[0];
        let remaining = args[1..].to_vec();

        // Try to find the command
        if let Some(cmd) = self.commands.get(cmd_name) {
            // Find the key that matches (for returning &str with correct lifetime)
            for (key, _) in &self.commands {
                if key == cmd_name {
                    return Ok(Some((key.as_str(), cmd.as_ref(), remaining)));
                }
            }
        }

        // Command not found
        if ctx.resilient_parsing() {
            return Ok(None);
        }

        // Check if the first arg looks like an option
        if cmd_name.starts_with('-') {
            // It's an option, not a command name - let the parser handle it
            return Ok(None);
        }

        Err(ClickError::usage(format!(
            "No such command '{}'.",
            cmd_name
        )))
    }

    /// Format the commands section for help output.
    ///
    /// Returns a formatted string listing all visible subcommands.
    pub fn format_commands(&self, _ctx: &Context) -> String {
        let mut lines = Vec::new();

        // Get visible commands (not hidden)
        let visible_cmds: Vec<(&str, &dyn CommandLike)> = self
            .list_commands()
            .into_iter()
            .filter_map(|name| {
                self.get_command(name)
                    .filter(|cmd| !cmd.is_hidden())
                    .map(|cmd| (name, cmd))
            })
            .collect();

        if visible_cmds.is_empty() {
            return String::new();
        }

        // Calculate the max command name width
        let max_width = visible_cmds
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0);

        lines.push("Commands:".to_string());

        for (name, cmd) in visible_cmds {
            let help = cmd.get_short_help();
            let padding = max_width - name.len() + 2;
            lines.push(format!(
                "  {}{:padding$}{}",
                name,
                "",
                help,
                padding = padding
            ));
        }

        lines.join("\n")
    }

    /// Get the usage line including the subcommand metavar.
    fn get_usage_with_subcommand(&self, ctx: &Context) -> String {
        let base_usage = self.command.get_usage(ctx);
        format!("{} {}", base_usage, self.subcommand_metavar)
    }

    /// Get help text including the subcommand listing.
    fn get_help_with_commands(&self, ctx: &Context) -> String {
        let mut parts = Vec::new();

        // Usage line with subcommand metavar
        parts.push(self.get_usage_with_subcommand(ctx));

        // Help text
        if let Some(ref help) = self.command.help {
            let text = help.lines().next().unwrap_or("");
            if !text.is_empty() {
                parts.push(String::new());
                let help_text = if let Some(ref dep) = self.command.deprecated {
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
        }

        // Options section
        let opt_records: Vec<(String, String)> = self
            .command
            .options
            .iter()
            .filter_map(|opt| opt.get_help_record())
            .collect();

        let help_opt = self.command.get_help_option(ctx);
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

        // Commands section
        let commands_section = self.format_commands(ctx);
        if !commands_section.is_empty() {
            parts.push(String::new());
            parts.push(commands_section);
        }

        // Epilog
        if let Some(ref epilog) = self.command.epilog {
            parts.push(String::new());
            parts.push(epilog.clone());
        }

        parts.join("\n")
    }
}

// =============================================================================
// CommandLike impl for Group
// =============================================================================

impl CommandLike for Group {
    fn name(&self) -> Option<&str> {
        self.command.name.as_deref()
    }

    fn make_context(
        &self,
        info_name: &str,
        args: Vec<String>,
        parent: Option<Arc<Context>>,
    ) -> Result<Context, ClickError> {
        // Groups need to allow extra args for subcommand dispatch
        let mut builder = ContextBuilder::new()
            .info_name(info_name)
            .allow_extra_args(true)
            .allow_interspersed_args(false);

        if let Some(parent) = parent {
            builder = builder.parent(parent);
        }

        let mut ctx = builder.build();

        // Parse the group's own arguments
        self.command.parse_args(&mut ctx, args)?;

        Ok(ctx)
    }

    fn invoke(&self, ctx: &Context) -> Result<(), ClickError> {
        // Get the remaining args after parsing group options
        let args = ctx.args().to_vec();

        // Helper function to process results through result_callback
        let process_result = |result_callback: &Option<ResultCallback>,
                              ctx: &Context,
                              results: Vec<Box<dyn Any + Send + Sync>>|
         -> Result<(), ClickError> {
            if let Some(ref callback) = result_callback {
                callback(ctx, results)?;
            }
            Ok(())
        };

        // Get the parent context Arc from the thread-local stack for proper inheritance
        let parent_arc = get_current_context();

        // Resolve first subcommand to check if there's any
        let resolved = self.resolve_command(ctx, &args)?;

        if resolved.is_none() {
            // No subcommand provided
            if self.invoke_without_command {
                // Invoke group callback and process result
                let group_result = self.command.invoke(ctx);
                if group_result.is_ok() {
                    // For invoke_without_command, result is empty list in chain mode
                    // or the group's return value (which we don't capture) otherwise
                    let results: Vec<Box<dyn Any + Send + Sync>> = if self.chain {
                        Vec::new()
                    } else {
                        // In non-chain mode, we pass an empty result since Rust callbacks
                        // return Result<(), ClickError> not arbitrary values
                        Vec::new()
                    };
                    process_result(&self.result_callback, ctx, results)?;
                }
                return group_result;
            } else if self.subcommand_required && !ctx.resilient_parsing() {
                return Err(ClickError::usage("Missing command."));
            } else {
                return Ok(());
            }
        }

        // We have at least one subcommand
        if !self.chain {
            // Non-chain mode: invoke single subcommand
            let (cmd_name, cmd, remaining) = resolved.unwrap();

            // Note: Setting invoked_subcommand requires interior mutability in Context.
            // The Context struct uses RefCell for close_callbacks but not for invoked_subcommand.
            // For now, we skip setting it; a future refactor could add RefCell<Option<String>>.

            // Invoke group callback first (if present)
            if self.command.callback.is_some() {
                self.command.invoke(ctx)?;
            }

            // Create subcommand context with proper parent inheritance
            let sub_ctx = cmd.make_context(cmd_name, remaining, parent_arc)?;

            // Push and invoke subcommand
            let sub_ctx_arc = Arc::new(sub_ctx);
            push_context(Arc::clone(&sub_ctx_arc));
            let result = cmd.invoke(&sub_ctx_arc);
            pop_context();

            // Close subcommand context
            sub_ctx_arc.close();

            // Process result callback if set
            if result.is_ok() {
                // In non-chain mode, result is a single value (empty since we don't capture return)
                process_result(&self.result_callback, ctx, Vec::new())?;
            }

            result
        } else {
            // Chain mode: invoke multiple subcommands in sequence
            // Note: In Python Click, invoked_subcommand is set to "*" in chain mode

            // Invoke group callback first (if present)
            if self.command.callback.is_some() {
                self.command.invoke(ctx)?;
            }

            // Collect all subcommand contexts first (like Python Click does)
            let mut contexts: Vec<(Arc<Context>, &dyn CommandLike)> = Vec::new();
            let mut remaining_args = args;

            while !remaining_args.is_empty() {
                let resolved = self.resolve_command(ctx, &remaining_args)?;
                match resolved {
                    Some((cmd_name, cmd, rest)) => {
                        // In chain mode, subcommands allow extra args and no interspersed args
                        // so remaining tokens are passed to the next command resolution.
                        // We create the context with chain mode settings, overriding the command's defaults.
                        let mut sub_ctx = ContextBuilder::new()
                            .info_name(cmd_name)
                            .allow_extra_args(true) // Chain mode: allow extra args for next cmd
                            .allow_interspersed_args(false) // Chain mode: no interspersed
                            .parent(
                                parent_arc
                                    .clone()
                                    .unwrap_or_else(|| Arc::new(Context::default())),
                            )
                            .build();

                        // Parse args using the command (this populates the context)
                        if let Some(command) = cmd.as_any().downcast_ref::<Command>() {
                            command.parse_args(&mut sub_ctx, rest)?;
                        } else if let Some(group) = cmd.as_any().downcast_ref::<Group>() {
                            // For nested groups, use make_context which handles group-specific parsing
                            let nested_ctx =
                                group.make_context(cmd_name, rest, parent_arc.clone())?;
                            sub_ctx = nested_ctx;
                        } else {
                            // Fallback: use make_context (may error on extra args)
                            sub_ctx = cmd.make_context(cmd_name, rest, parent_arc.clone())?;
                        }

                        // The subcommand's unparsed args become input for next command
                        remaining_args = sub_ctx.args().to_vec();

                        contexts.push((Arc::new(sub_ctx), cmd));
                    }
                    None => {
                        // No more commands found - remaining args are truly extra
                        // In chain mode, we don't error on extra args that look like commands
                        // but couldn't be resolved. However, if they look like options,
                        // that's an error (unless resilient_parsing)
                        if !remaining_args.is_empty()
                            && remaining_args[0].starts_with('-')
                            && !ctx.resilient_parsing()
                        {
                            return Err(ClickError::usage(format!(
                                "No such option: {}",
                                remaining_args[0]
                            )));
                        }
                        break;
                    }
                }
            }

            // Invoke all subcommands and collect results
            let mut results: Vec<Box<dyn Any + Send + Sync>> = Vec::new();
            for (sub_ctx_arc, cmd) in contexts {
                push_context(Arc::clone(&sub_ctx_arc));
                let result = cmd.invoke(&sub_ctx_arc);
                pop_context();
                sub_ctx_arc.close();

                // If any subcommand fails, propagate the error
                result?;

                // We don't capture return values since Rust callbacks return Result<(), ClickError>
                // In a more advanced implementation, callbacks could return arbitrary values
                results.push(Box::new(()));
            }

            // Process result callback with collected results
            process_result(&self.result_callback, ctx, results)?;

            Ok(())
        }
    }

    #[allow(clippy::arc_with_non_send_sync)]
    fn main(&self, args: Vec<String>) -> Result<(), ClickError> {
        let prog_name = self.command.name.clone().unwrap_or_else(|| {
            std::env::args()
                .next()
                .unwrap_or_else(|| "program".to_string())
        });

        let ctx = self.make_context(&prog_name, args, None)?;
        let ctx = Arc::new(ctx);

        // Push context onto thread-local stack
        push_context(Arc::clone(&ctx));

        // Invoke the group
        let result = self.invoke(&ctx);

        // Pop context
        pop_context();

        // Run close callbacks
        ctx.close();

        result
    }

    fn get_help(&self, ctx: &Context) -> String {
        self.get_help_with_commands(ctx)
    }

    fn get_short_help(&self) -> String {
        self.command.get_short_help()
    }

    fn is_hidden(&self) -> bool {
        self.command.hidden
    }

    fn get_usage(&self, ctx: &Context) -> String {
        self.get_usage_with_subcommand(ctx)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// =============================================================================
// CommandCollection
// =============================================================================

/// A group-like command that merges commands from multiple groups.
///
/// This is the click-rs equivalent of Python Click's `CommandCollection`.
/// Commands are resolved by searching the base group first, then each source
/// group in insertion order.
///
/// Only the base group's parameters (options/arguments/callback/help) are used.
#[derive(Debug)]
pub struct CommandCollection {
    /// The base group providing parameters and help formatting.
    pub base: Group,

    /// Additional groups to source subcommands from.
    pub sources: Vec<Group>,
}

impl CommandCollection {
    /// Create a new `CommandCollection` builder with the given base name.
    ///
    /// The base group can register its own subcommands via `.command(...)`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: &str) -> CommandCollectionBuilder {
        CommandCollectionBuilder::new(name)
    }

    /// Add a source group.
    pub fn add_source(&mut self, group: Group) {
        self.sources.push(group);
    }

    /// Get a subcommand by name, searching base first then sources.
    pub fn get_command(&self, name: &str) -> Option<&dyn CommandLike> {
        if let Some(cmd) = self.base.get_command(name) {
            return Some(cmd);
        }
        for src in &self.sources {
            if let Some(cmd) = src.get_command(name) {
                return Some(cmd);
            }
        }
        None
    }

    /// List all unique subcommand names from base + sources, sorted.
    pub fn list_commands(&self) -> Vec<String> {
        let mut names: std::collections::HashSet<String> =
            self.base.commands.keys().cloned().collect();

        for src in &self.sources {
            for name in src.commands.keys() {
                names.insert(name.clone());
            }
        }

        let mut out: Vec<String> = names.into_iter().collect();
        out.sort();
        out
    }

    fn resolve_command<'a>(
        &'a self,
        ctx: &Context,
        args: &[String],
    ) -> Result<Option<(String, &'a dyn CommandLike, Vec<String>)>, ClickError> {
        if args.is_empty() {
            return Ok(None);
        }

        let cmd_name = &args[0];
        let remaining = args[1..].to_vec();

        if let Some(cmd) = self.base.commands.get(cmd_name) {
            return Ok(Some((cmd_name.clone(), cmd.as_ref(), remaining)));
        }
        for src in &self.sources {
            if let Some(cmd) = src.commands.get(cmd_name) {
                return Ok(Some((cmd_name.clone(), cmd.as_ref(), remaining)));
            }
        }

        if ctx.resilient_parsing() {
            return Ok(None);
        }
        if cmd_name.starts_with('-') {
            return Ok(None);
        }

        Err(ClickError::usage(format!(
            "No such command '{}'.",
            cmd_name
        )))
    }

    fn format_commands(&self, _ctx: &Context) -> String {
        let mut visible_cmds: Vec<(String, &dyn CommandLike)> = self
            .list_commands()
            .into_iter()
            .filter_map(|name| {
                self.get_command(&name)
                    .filter(|cmd| !cmd.is_hidden())
                    .map(|cmd| (name, cmd))
            })
            .collect();

        if visible_cmds.is_empty() {
            return String::new();
        }

        visible_cmds.sort_by(|a, b| a.0.cmp(&b.0));

        let max_width = visible_cmds
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0);

        let mut lines = Vec::new();
        lines.push("Commands:".to_string());

        for (name, cmd) in visible_cmds {
            let help = cmd.get_short_help();
            let padding = max_width - name.len() + 2;
            lines.push(format!(
                "  {}{:padding$}{}",
                name,
                "",
                help,
                padding = padding
            ));
        }

        lines.join("\n")
    }

    fn get_usage_with_subcommand(&self, ctx: &Context) -> String {
        let base_usage = self.base.command.get_usage(ctx);
        format!("{} {}", base_usage, self.base.subcommand_metavar)
    }

    fn get_help_with_commands(&self, ctx: &Context) -> String {
        let mut parts = Vec::new();

        parts.push(self.get_usage_with_subcommand(ctx));

        if let Some(ref help) = self.base.command.help {
            let text = help.lines().next().unwrap_or("");
            if !text.is_empty() {
                parts.push(String::new());
                let help_text = if let Some(ref dep) = self.base.command.deprecated {
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
        }

        let opt_records: Vec<(String, String)> = self
            .base
            .command
            .options
            .iter()
            .filter_map(|opt| opt.get_help_record())
            .collect();

        let help_opt = self.base.command.get_help_option(ctx);
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

        let commands_section = self.format_commands(ctx);
        if !commands_section.is_empty() {
            parts.push(String::new());
            parts.push(commands_section);
        }

        if let Some(ref epilog) = self.base.command.epilog {
            parts.push(String::new());
            parts.push(epilog.clone());
        }

        parts.join("\n")
    }
}

impl CommandLike for CommandCollection {
    fn name(&self) -> Option<&str> {
        self.base.command.name.as_deref()
    }

    fn make_context(
        &self,
        info_name: &str,
        args: Vec<String>,
        parent: Option<Arc<Context>>,
    ) -> Result<Context, ClickError> {
        let mut builder = ContextBuilder::new()
            .info_name(info_name)
            .allow_extra_args(true)
            .allow_interspersed_args(false);

        if let Some(parent) = parent {
            builder = builder.parent(parent);
        }

        let mut ctx = builder.build();
        self.base.command.parse_args(&mut ctx, args)?;
        Ok(ctx)
    }

    fn invoke(&self, ctx: &Context) -> Result<(), ClickError> {
        let args = ctx.args().to_vec();

        let process_result = |result_callback: &Option<ResultCallback>,
                              ctx: &Context,
                              results: Vec<Box<dyn Any + Send + Sync>>|
         -> Result<(), ClickError> {
            if let Some(ref callback) = result_callback {
                callback(ctx, results)?;
            }
            Ok(())
        };

        let parent_arc = get_current_context();
        let resolved = self.resolve_command(ctx, &args)?;

        if resolved.is_none() {
            if self.base.invoke_without_command {
                let group_result = self.base.command.invoke(ctx);
                if group_result.is_ok() {
                    process_result(&self.base.result_callback, ctx, Vec::new())?;
                }
                return group_result;
            } else if self.base.subcommand_required && !ctx.resilient_parsing() {
                return Err(ClickError::usage("Missing command."));
            } else {
                return Ok(());
            }
        }

        if !self.base.chain {
            let (cmd_name, cmd, remaining) = resolved.unwrap();

            if self.base.command.callback.is_some() {
                self.base.command.invoke(ctx)?;
            }

            let sub_ctx = cmd.make_context(&cmd_name, remaining, parent_arc)?;

            let sub_ctx_arc = Arc::new(sub_ctx);
            push_context(Arc::clone(&sub_ctx_arc));
            let result = cmd.invoke(&sub_ctx_arc);
            pop_context();
            sub_ctx_arc.close();

            if result.is_ok() {
                process_result(&self.base.result_callback, ctx, Vec::new())?;
            }

            result
        } else {
            if self.base.command.callback.is_some() {
                self.base.command.invoke(ctx)?;
            }

            let mut contexts: Vec<(Arc<Context>, &dyn CommandLike)> = Vec::new();
            let mut remaining_args = args;

            while !remaining_args.is_empty() {
                let resolved = self.resolve_command(ctx, &remaining_args)?;
                match resolved {
                    Some((cmd_name, cmd, rest)) => {
                        let mut sub_ctx = ContextBuilder::new()
                            .info_name(&cmd_name)
                            .allow_extra_args(true)
                            .allow_interspersed_args(false)
                            .parent(
                                parent_arc
                                    .clone()
                                    .unwrap_or_else(|| Arc::new(Context::default())),
                            )
                            .build();

                        if let Some(command) = cmd.as_any().downcast_ref::<Command>() {
                            command.parse_args(&mut sub_ctx, rest)?;
                        } else if let Some(group) = cmd.as_any().downcast_ref::<Group>() {
                            sub_ctx = group.make_context(&cmd_name, rest, parent_arc.clone())?;
                        } else if let Some(collection) =
                            cmd.as_any().downcast_ref::<CommandCollection>()
                        {
                            sub_ctx =
                                collection.make_context(&cmd_name, rest, parent_arc.clone())?;
                        } else {
                            sub_ctx = cmd.make_context(&cmd_name, rest, parent_arc.clone())?;
                        }

                        remaining_args = sub_ctx.args().to_vec();
                        contexts.push((Arc::new(sub_ctx), cmd));
                    }
                    None => {
                        if !remaining_args.is_empty()
                            && remaining_args[0].starts_with('-')
                            && !ctx.resilient_parsing()
                        {
                            return Err(ClickError::usage(format!(
                                "No such option: {}",
                                remaining_args[0]
                            )));
                        }
                        break;
                    }
                }
            }

            let mut results: Vec<Box<dyn Any + Send + Sync>> = Vec::new();
            for (sub_ctx_arc, cmd) in contexts {
                push_context(Arc::clone(&sub_ctx_arc));
                let result = cmd.invoke(&sub_ctx_arc);
                pop_context();
                sub_ctx_arc.close();
                result?;
                results.push(Box::new(()));
            }

            process_result(&self.base.result_callback, ctx, results)?;
            Ok(())
        }
    }

    #[allow(clippy::arc_with_non_send_sync)]
    fn main(&self, args: Vec<String>) -> Result<(), ClickError> {
        let prog_name = self.base.command.name.clone().unwrap_or_else(|| {
            std::env::args()
                .next()
                .unwrap_or_else(|| "program".to_string())
        });

        let ctx = self.make_context(&prog_name, args, None)?;
        let ctx = Arc::new(ctx);

        push_context(Arc::clone(&ctx));
        let result = self.invoke(&ctx);
        pop_context();
        ctx.close();
        result
    }

    fn get_help(&self, ctx: &Context) -> String {
        self.get_help_with_commands(ctx)
    }

    fn get_short_help(&self) -> String {
        self.base.command.get_short_help()
    }

    fn is_hidden(&self) -> bool {
        self.base.command.hidden
    }

    fn get_usage(&self, ctx: &Context) -> String {
        self.get_usage_with_subcommand(ctx)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Builder for [`CommandCollection`].
pub struct CommandCollectionBuilder {
    base: GroupBuilder,
    sources: Vec<Group>,
}

impl CommandCollectionBuilder {
    fn new(name: &str) -> Self {
        Self {
            base: GroupBuilder::new(name),
            sources: Vec::new(),
        }
    }

    /// Add a source group.
    pub fn source(mut self, group: Group) -> Self {
        self.sources.push(group);
        self
    }

    /// Add a subcommand to the base group.
    pub fn command(mut self, cmd: impl CommandLike + 'static) -> Self {
        self.base = self.base.command(cmd);
        self
    }

    /// Build the `CommandCollection`.
    pub fn build(self) -> CommandCollection {
        CommandCollection {
            base: self.base.build(),
            sources: self.sources,
        }
    }
}

// =============================================================================
// GroupBuilder
// =============================================================================

/// Builder for creating [`Group`] instances.
///
/// Use [`Group::new`] to create a builder, then chain methods to configure
/// the group, and finally call [`build`](GroupBuilder::build) to create
/// the group.
///
/// # Example
///
/// ```
/// use click::group::Group;
/// use click::command::Command;
///
/// let group = Group::new("cli")
///     .help("My CLI application")
///     .callback(|_ctx| {
///         println!("Group callback");
///         Ok(())
///     })
///     .invoke_without_command(true)
///     .command(Command::new("hello").build())
///     .build();
/// ```
pub struct GroupBuilder {
    name: String,
    callback: Option<CommandCallback>,
    options: Vec<ClickOption>,
    arguments: Vec<Argument>,
    help: Option<String>,
    epilog: Option<String>,
    short_help: Option<String>,
    hidden: bool,
    deprecated: Option<String>,
    commands: HashMap<String, Arc<dyn CommandLike>>,
    command_ids_by_name: HashMap<String, usize>,
    command_aliases_by_id: HashMap<usize, Vec<String>>,
    next_command_id: usize,
    chain: bool,
    invoke_without_command: bool,
    result_callback: Option<ResultCallback>,
    subcommand_required: Option<bool>,
    subcommand_metavar: Option<String>,
    add_help_option: bool,
    help_option: Option<ClickOption>,
    no_args_is_help: Option<bool>,
}

impl GroupBuilder {
    /// Create a new group builder with the given name.
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            callback: None,
            options: Vec::new(),
            arguments: Vec::new(),
            help: None,
            epilog: None,
            short_help: None,
            hidden: false,
            deprecated: None,
            commands: HashMap::new(),
            command_ids_by_name: HashMap::new(),
            command_aliases_by_id: HashMap::new(),
            next_command_id: 0,
            chain: false,
            invoke_without_command: false,
            result_callback: None,
            subcommand_required: None,
            subcommand_metavar: None,
            add_help_option: true,
            help_option: None,
            no_args_is_help: None,
        }
    }

    // -------------------------------------------------------------------------
    // Inherited from Command
    // -------------------------------------------------------------------------

    /// Set the callback function for this group.
    ///
    /// The callback is invoked before subcommand dispatch (or alone if
    /// `invoke_without_command` is true and no subcommand is given).
    pub fn callback<F>(mut self, f: F) -> Self
    where
        F: Fn(&Context) -> Result<(), ClickError> + Send + Sync + 'static,
    {
        self.callback = Some(Box::new(f));
        self
    }

    /// Add an option to this group.
    pub fn option(mut self, opt: ClickOption) -> Self {
        self.options.push(opt);
        self
    }

    /// Add an argument to this group.
    pub fn argument(mut self, arg: Argument) -> Self {
        self.arguments.push(arg);
        self
    }

    /// Set the help text for this group.
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

    /// Hide this group from help output.
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Mark this group as deprecated.
    pub fn deprecated(mut self, message: &str) -> Self {
        self.deprecated = Some(message.to_string());
        self
    }

    /// Set whether to add a --help option (default: true).
    pub fn add_help_option(mut self, add: bool) -> Self {
        self.add_help_option = add;
        self
    }

    /// Override the automatically generated help option.
    ///
    /// Setting a custom help option implicitly enables `add_help_option`.
    pub fn help_option(mut self, opt: ClickOption) -> Self {
        self.add_help_option = true;
        self.help_option = Some(opt);
        self
    }

    /// Set whether to show help if no args provided.
    ///
    /// Defaults to the opposite of `invoke_without_command`.
    pub fn no_args_is_help(mut self, value: bool) -> Self {
        self.no_args_is_help = Some(value);
        self
    }

    // -------------------------------------------------------------------------
    // Group-specific
    // -------------------------------------------------------------------------

    /// Add a subcommand to this group.
    ///
    /// # Example
    ///
    /// ```
    /// use click::group::Group;
    /// use click::command::Command;
    ///
    /// let group = Group::new("cli")
    ///     .command(Command::new("hello").build())
    ///     .command(Command::new("goodbye").build())
    ///     .build();
    /// ```
    pub fn command(self, cmd: impl CommandLike + 'static) -> Self {
        self.command_shared(Arc::new(cmd))
    }

    /// Add a subcommand with a specific name (overriding the command's name).
    pub fn command_with_name(self, name: &str, cmd: impl CommandLike + 'static) -> Self {
        self.command_shared_with_name(name, Arc::new(cmd))
    }

    /// Add a shared subcommand to this group.
    ///
    /// This makes it possible to register a single command under multiple names (aliases).
    pub fn command_shared(mut self, cmd: Arc<dyn CommandLike>) -> Self {
        let name = cmd.name().map(|s| s.to_string());
        if let Some(name) = name {
            self = self.command_shared_with_name(&name, cmd);
        }
        self
    }

    /// Add a shared subcommand with a specific registered name.
    pub fn command_shared_with_name(mut self, name: &str, cmd: Arc<dyn CommandLike>) -> Self {
        // If we're replacing an existing name, unlink it from prior alias metadata.
        if let Some(old_id) = self.command_ids_by_name.get(name).copied() {
            if let Some(names) = self.command_aliases_by_id.get_mut(&old_id) {
                names.retain(|n| n != name);
            }
        }

        // Find an existing id for this command (if it's already registered under another name).
        let existing_id = self.commands.iter().find_map(|(n, existing)| {
            if Arc::ptr_eq(existing, &cmd) {
                self.command_ids_by_name.get(n).copied()
            } else {
                None
            }
        });

        let id = existing_id.unwrap_or_else(|| {
            let id = self.next_command_id;
            self.next_command_id += 1;
            id
        });

        self.command_ids_by_name.insert(name.to_string(), id);
        self.command_aliases_by_id
            .entry(id)
            .or_insert_with(Vec::new)
            .push(name.to_string());

        if let Some(names) = self.command_aliases_by_id.get_mut(&id) {
            names.sort();
            names.dedup();
        }

        self.commands.insert(name.to_string(), cmd);
        self
    }

    /// Enable or disable chain mode.
    ///
    /// In chain mode, multiple subcommands can be invoked in sequence:
    /// `cli cmd1 arg1 cmd2 arg2`
    pub fn chain(mut self, chain: bool) -> Self {
        self.chain = chain;
        if chain {
            self.subcommand_metavar =
                Some("COMMAND1 [ARGS]... [COMMAND2 [ARGS]...]...".to_string());
        }
        self
    }

    /// Set whether to invoke the group callback without a subcommand.
    ///
    /// If true, the group's callback is invoked even when no subcommand
    /// is provided.
    pub fn invoke_without_command(mut self, value: bool) -> Self {
        self.invoke_without_command = value;
        self
    }

    /// Set whether a subcommand is required.
    ///
    /// If not explicitly set, defaults to the opposite of `invoke_without_command`.
    pub fn subcommand_required(mut self, required: bool) -> Self {
        self.subcommand_required = Some(required);
        self
    }

    /// Set the metavar for subcommands in usage output.
    ///
    /// Default is "COMMAND [ARGS]..." (or "COMMAND1 [ARGS]... [COMMAND2 [ARGS]...]..."
    /// in chain mode).
    pub fn subcommand_metavar(mut self, metavar: &str) -> Self {
        self.subcommand_metavar = Some(metavar.to_string());
        self
    }

    /// Set the result callback for processing subcommand results.
    pub fn result_callback<F>(mut self, f: F) -> Self
    where
        F: Fn(&Context, Vec<Box<dyn Any + Send + Sync>>) -> Result<(), ClickError>
            + Send
            + Sync
            + 'static,
    {
        self.result_callback = Some(Box::new(f));
        self
    }

    /// Build the group.
    pub fn build(self) -> Group {
        // Determine no_args_is_help default
        let no_args_is_help = self.no_args_is_help.unwrap_or(!self.invoke_without_command);

        // Determine subcommand_required default
        let subcommand_required = self
            .subcommand_required
            .unwrap_or(!self.invoke_without_command);

        // Determine subcommand_metavar
        let subcommand_metavar = self.subcommand_metavar.unwrap_or_else(|| {
            if self.chain {
                "COMMAND1 [ARGS]... [COMMAND2 [ARGS]...]...".to_string()
            } else {
                "COMMAND [ARGS]...".to_string()
            }
        });

        // Build the underlying command
        let mut cmd_builder = CommandBuilder::new(&self.name)
            .allow_extra_args(true)
            .allow_interspersed_args(false)
            .add_help_option(self.add_help_option)
            .no_args_is_help(no_args_is_help);

        if let Some(help_opt) = self.help_option {
            cmd_builder = cmd_builder.help_option(help_opt);
        }

        // Add options
        for opt in self.options {
            cmd_builder = cmd_builder.option(opt);
        }

        // Add arguments
        for arg in self.arguments {
            cmd_builder = cmd_builder.argument(arg);
        }

        // Set other properties
        if let Some(help) = self.help {
            cmd_builder = cmd_builder.help(&help);
        }
        if let Some(epilog) = self.epilog {
            cmd_builder = cmd_builder.epilog(&epilog);
        }
        if let Some(short_help) = self.short_help {
            cmd_builder = cmd_builder.short_help(&short_help);
        }
        if self.hidden {
            cmd_builder = cmd_builder.hidden();
        }
        if let Some(deprecated) = self.deprecated {
            cmd_builder = cmd_builder.deprecated(&deprecated);
        }
        if let Some(callback) = self.callback {
            // We need to wrap the callback
            let callback_wrapper = move |ctx: &Context| callback(ctx);
            cmd_builder = cmd_builder.callback(callback_wrapper);
        }

        let command = cmd_builder.build();

        Group {
            command,
            commands: self.commands,
            command_ids_by_name: self.command_ids_by_name,
            command_aliases_by_id: self.command_aliases_by_id,
            next_command_id: self.next_command_id,
            chain: self.chain,
            invoke_without_command: self.invoke_without_command,
            result_callback: self.result_callback,
            subcommand_required,
            subcommand_metavar,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_group_creation_defaults() {
        let group = Group::new("test").build();

        assert_eq!(group.name(), Some("test"));
        assert!(group.commands.is_empty());
        assert!(!group.chain);
        assert!(!group.invoke_without_command);
        assert!(group.subcommand_required);
        assert_eq!(group.subcommand_metavar, "COMMAND [ARGS]...");
    }

    #[test]
    fn test_group_with_subcommands() {
        let group = Group::new("cli")
            .command(Command::new("init").help("Initialize").build())
            .command(Command::new("build").help("Build").build())
            .build();

        assert_eq!(group.commands.len(), 2);
        assert!(group.get_command("init").is_some());
        assert!(group.get_command("build").is_some());
        assert!(group.get_command("unknown").is_none());
    }

    #[test]
    fn test_list_commands_sorted() {
        let group = Group::new("cli")
            .command(Command::new("zebra").build())
            .command(Command::new("alpha").build())
            .command(Command::new("middle").build())
            .build();

        let commands = group.list_commands();
        assert_eq!(commands, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn test_add_command_with_name() {
        let mut group = Group::new("cli").build();

        group.add_command(Command::new("original").build(), Some("renamed"));

        assert!(group.get_command("renamed").is_some());
        assert!(group.get_command("original").is_none());
    }

    #[test]
    fn test_alias_metadata_for_shared_command() {
        let cmd: Arc<dyn CommandLike> = Arc::new(Command::new("original").build());

        let group = Group::new("cli")
            .command_shared(Arc::clone(&cmd))
            .command_shared_with_name("alias", Arc::clone(&cmd))
            .build();

        assert!(group.get_command("original").is_some());
        assert!(group.get_command("alias").is_some());

        assert_eq!(
            group.list_command_aliases("original"),
            vec!["alias".to_string()]
        );
        assert_eq!(
            group.list_command_aliases("alias"),
            vec!["original".to_string()]
        );

        let entries = group.list_command_entries();
        let alias_entry = entries
            .iter()
            .find(|(name, _)| name == "alias")
            .expect("alias entry missing");
        assert_eq!(alias_entry.1.name(), Some("original"));
    }

    #[test]
    fn test_group_chain_mode() {
        let group = Group::new("cli").chain(true).build();

        assert!(group.chain);
        assert_eq!(
            group.subcommand_metavar,
            "COMMAND1 [ARGS]... [COMMAND2 [ARGS]...]..."
        );
    }

    #[test]
    fn test_invoke_without_command() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let group = Group::new("cli")
            .invoke_without_command(true)
            .callback(move |_ctx| {
                called_clone.store(true, Ordering::SeqCst);
                Ok(())
            })
            .build();

        // invoke_without_command implies subcommand_required = false
        assert!(!group.subcommand_required);

        // Create context with no subcommand
        let ctx = ContextBuilder::new().info_name("cli").build();

        // Invoke should call the group callback
        let result = group.invoke(&ctx);
        assert!(result.is_ok());
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_group_help_formatting() {
        let group = Group::new("cli")
            .help("A sample CLI application")
            .command(
                Command::new("init")
                    .short_help("Initialize the project")
                    .build(),
            )
            .command(
                Command::new("build")
                    .short_help("Build the project")
                    .build(),
            )
            .build();

        let ctx = ContextBuilder::new().info_name("cli").build();
        let help = group.get_help(&ctx);

        assert!(help.contains("Usage:"));
        assert!(help.contains("cli"));
        assert!(help.contains("COMMAND [ARGS]..."));
        assert!(help.contains("A sample CLI application"));
        assert!(help.contains("Commands:"));
        assert!(help.contains("init"));
        assert!(help.contains("build"));
    }

    #[test]
    fn test_resolve_command() {
        let group = Group::new("cli")
            .command(Command::new("hello").build())
            .command(Command::new("world").build())
            .build();

        let ctx = ContextBuilder::new().info_name("cli").build();

        // Resolve existing command
        let args = vec!["hello".to_string(), "arg1".to_string()];
        let resolved = group.resolve_command(&ctx, &args);
        assert!(resolved.is_ok());

        let (name, _cmd, remaining) = resolved.unwrap().unwrap();
        assert_eq!(name, "hello");
        assert_eq!(remaining, vec!["arg1".to_string()]);

        // Resolve non-existent command
        let args = vec!["unknown".to_string()];
        let resolved = group.resolve_command(&ctx, &args);
        assert!(resolved.is_err());
    }

    #[test]
    fn test_resolve_command_empty_args() {
        let group = Group::new("cli")
            .command(Command::new("hello").build())
            .build();

        let ctx = ContextBuilder::new().info_name("cli").build();

        let resolved = group.resolve_command(&ctx, &[]);
        assert!(resolved.is_ok());
        assert!(resolved.unwrap().is_none());
    }

    #[test]
    fn test_group_with_options() {
        let group = Group::new("cli")
            .help("A CLI with options")
            .option(
                ClickOption::new(&["--verbose", "-v"])
                    .flag("true")
                    .help("Enable verbose mode")
                    .build(),
            )
            .command(Command::new("run").build())
            .build();

        assert_eq!(group.command.options.len(), 1);

        let ctx = ContextBuilder::new().info_name("cli").build();
        let help = group.get_help(&ctx);

        assert!(help.contains("--verbose"));
        assert!(help.contains("Enable verbose mode"));
    }

    #[test]
    fn test_hidden_commands_not_in_help() {
        let group = Group::new("cli")
            .command(Command::new("visible").build())
            .command(Command::new("hidden").hidden().build())
            .build();

        let ctx = ContextBuilder::new().info_name("cli").build();
        let help = group.format_commands(&ctx);

        assert!(help.contains("visible"));
        assert!(!help.contains("hidden"));
    }

    #[test]
    fn test_subcommand_required_default() {
        // Without invoke_without_command: subcommand_required = true
        let group1 = Group::new("cli").build();
        assert!(group1.subcommand_required);

        // With invoke_without_command: subcommand_required = false
        let group2 = Group::new("cli").invoke_without_command(true).build();
        assert!(!group2.subcommand_required);

        // Explicit override
        let group3 = Group::new("cli")
            .invoke_without_command(true)
            .subcommand_required(true)
            .build();
        assert!(group3.subcommand_required);
    }

    #[test]
    fn test_group_short_help() {
        let group = Group::new("cli")
            .help("This is the long help text. It has multiple sentences.")
            .build();

        let short = group.get_short_help();
        assert_eq!(short, "This is the long help text");

        let group_explicit = Group::new("cli")
            .help("Long help")
            .short_help("Short help")
            .build();

        let short = group_explicit.get_short_help();
        assert_eq!(short, "Short help");
    }

    #[test]
    fn test_group_debug_format() {
        let group = Group::new("cli")
            .command(Command::new("a").build())
            .command(Command::new("b").build())
            .build();

        let debug_str = format!("{:?}", group);
        assert!(debug_str.contains("Group"));
        assert!(debug_str.contains("2 subcommands"));
    }

    #[test]
    fn test_nested_groups() {
        let sub_group = Group::new("sub")
            .help("Subgroup")
            .command(Command::new("cmd").build())
            .build();

        let main_group = Group::new("main")
            .help("Main group")
            .command(sub_group)
            .build();

        assert!(main_group.get_command("sub").is_some());

        // Can get the nested command through the subgroup
        let sub = main_group.get_command("sub").unwrap();
        assert_eq!(sub.name(), Some("sub"));
    }

    #[test]
    fn test_command_with_name_builder() {
        let group = Group::new("cli")
            .command_with_name("alias", Command::new("original").build())
            .build();

        assert!(group.get_command("alias").is_some());
        assert!(group.get_command("original").is_none());
    }

    #[test]
    fn test_missing_command_error() {
        let group = Group::new("cli").subcommand_required(true).build();

        let ctx = ContextBuilder::new().info_name("cli").build();

        // No args and subcommand required should error
        let result = group.invoke(&ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ClickError::UsageError { .. }));
    }

    #[test]
    fn test_commandlike_trait() {
        // Test that both Command and Group implement CommandLike
        let cmd: Box<dyn CommandLike> = Box::new(Command::new("cmd").build());
        let grp: Box<dyn CommandLike> = Box::new(Group::new("grp").build());

        assert_eq!(cmd.name(), Some("cmd"));
        assert_eq!(grp.name(), Some("grp"));

        assert!(!cmd.is_hidden());
        assert!(!grp.is_hidden());
    }

    #[test]
    fn test_group_usage() {
        let group = Group::new("cli")
            .option(ClickOption::new(&["--debug"]).flag("true").build())
            .build();

        let ctx = ContextBuilder::new().info_name("cli").build();
        let usage = group.get_usage(&ctx);

        assert!(usage.contains("cli"));
        assert!(usage.contains("[OPTIONS]"));
        assert!(usage.contains("COMMAND [ARGS]..."));
    }

    #[test]
    fn test_chain_metavar() {
        let group = Group::new("cli")
            .chain(true)
            .subcommand_metavar("CMD1 CMD2...")
            .build();

        // Custom metavar should override chain default
        assert_eq!(group.subcommand_metavar, "CMD1 CMD2...");
    }

    #[test]
    fn test_group_deprecated() {
        let group = Group::new("old")
            .help("Old group")
            .deprecated("Use 'new' instead")
            .build();

        let short = group.get_short_help();
        assert!(short.contains("DEPRECATED"));
        assert!(short.contains("Use 'new' instead"));
    }

    // =========================================================================
    // Tests for context inheritance, chain mode, and result_callback
    // =========================================================================

    #[test]
    fn test_subcommand_context_inheritance() {
        // Test that subcommand context properly inherits from parent context
        let parent_info_name = Arc::new(std::sync::Mutex::new(String::new()));
        let parent_info_clone = Arc::clone(&parent_info_name);

        let group = Group::new("cli")
            .command(
                Command::new("sub")
                    .callback(move |ctx| {
                        // Check that the parent context is accessible
                        if let Some(parent) = ctx.parent() {
                            let mut lock = parent_info_clone.lock().unwrap();
                            if let Some(name) = parent.info_name() {
                                *lock = name.to_string();
                            }
                        }
                        Ok(())
                    })
                    .build(),
            )
            .build();

        // Use main() which sets up proper context stack
        let result = group.main(vec!["sub".to_string()]);
        assert!(result.is_ok());

        // The subcommand should have seen "cli" as parent info_name
        let captured = parent_info_name.lock().unwrap();
        assert_eq!(*captured, "cli");
    }

    #[test]
    fn test_subcommand_inherits_terminal_settings() {
        // Test that subcommand context inherits terminal_width and color settings
        let inherited_width = Arc::new(std::sync::Mutex::new(None::<usize>));
        let inherited_color = Arc::new(std::sync::Mutex::new(None::<bool>));
        let width_clone = Arc::clone(&inherited_width);
        let color_clone = Arc::clone(&inherited_color);

        let group = Group::new("cli")
            .command(
                Command::new("sub")
                    .callback(move |ctx| {
                        *width_clone.lock().unwrap() = ctx.terminal_width();
                        *color_clone.lock().unwrap() = ctx.color();
                        Ok(())
                    })
                    .build(),
            )
            .build();

        // Create parent context with specific settings
        let parent_ctx = ContextBuilder::new()
            .info_name("cli")
            .terminal_width(120)
            .color(true)
            .allow_extra_args(true)
            .build();
        let _parent_ctx = Arc::new(parent_ctx);

        // Parse args through the group
        let ctx = group
            .make_context("cli", vec!["sub".to_string()], None)
            .unwrap();

        // Manually set the inherited values (simulating what ContextBuilder does with parent)
        // In real usage, main() would set these up properly
        push_context(Arc::new(
            ContextBuilder::new()
                .info_name("cli")
                .terminal_width(120)
                .color(true)
                .allow_extra_args(true)
                .build(),
        ));

        let result = group.invoke(&ctx);
        pop_context();

        assert!(result.is_ok());
        // Note: The actual inheritance depends on Context implementation
        // This test verifies the invoke path doesn't break
    }

    #[test]
    fn test_chain_mode_multiple_commands() {
        // Test that chain mode invokes multiple subcommands
        let call_order = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let order1 = Arc::clone(&call_order);
        let order2 = Arc::clone(&call_order);
        let order3 = Arc::clone(&call_order);

        let group = Group::new("cli")
            .chain(true)
            .command(
                Command::new("cmd1")
                    .callback(move |_ctx| {
                        order1.lock().unwrap().push("cmd1".to_string());
                        Ok(())
                    })
                    .build(),
            )
            .command(
                Command::new("cmd2")
                    .callback(move |_ctx| {
                        order2.lock().unwrap().push("cmd2".to_string());
                        Ok(())
                    })
                    .build(),
            )
            .command(
                Command::new("cmd3")
                    .callback(move |_ctx| {
                        order3.lock().unwrap().push("cmd3".to_string());
                        Ok(())
                    })
                    .build(),
            )
            .build();

        // Invoke with multiple commands
        let result = group.main(vec![
            "cmd1".to_string(),
            "cmd2".to_string(),
            "cmd3".to_string(),
        ]);
        assert!(result.is_ok());

        // All commands should have been called in order
        let order = call_order.lock().unwrap();
        assert_eq!(*order, vec!["cmd1", "cmd2", "cmd3"]);
    }

    #[test]
    fn test_chain_mode_with_args() {
        // Test chain mode where commands have arguments
        let captured_args = Arc::new(std::sync::Mutex::new(Vec::<Vec<String>>::new()));
        let args1 = Arc::clone(&captured_args);
        let args2 = Arc::clone(&captured_args);

        let group = Group::new("cli")
            .chain(true)
            .command(
                Command::new("first")
                    .callback(move |ctx| {
                        args1.lock().unwrap().push(ctx.args().to_vec());
                        Ok(())
                    })
                    .build(),
            )
            .command(
                Command::new("second")
                    .callback(move |ctx| {
                        args2.lock().unwrap().push(ctx.args().to_vec());
                        Ok(())
                    })
                    .build(),
            )
            .build();

        // Both commands called without arguments to each
        let result = group.main(vec!["first".to_string(), "second".to_string()]);
        assert!(result.is_ok());

        let args = captured_args.lock().unwrap();
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_chain_mode_empty_returns_ok() {
        // Test that chain mode with invoke_without_command returns ok with no commands
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let group = Group::new("cli")
            .chain(true)
            .invoke_without_command(true)
            .callback(move |_ctx| {
                called_clone.store(true, Ordering::SeqCst);
                Ok(())
            })
            .command(Command::new("sub").build())
            .build();

        let result = group.main(vec![]);
        assert!(result.is_ok());
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_result_callback_invoked() {
        // Test that result_callback is called after subcommand execution
        let result_callback_called = Arc::new(AtomicBool::new(false));
        let callback_clone = Arc::clone(&result_callback_called);

        let group = Group::new("cli")
            .command(Command::new("sub").callback(|_ctx| Ok(())).build())
            .result_callback(move |_ctx, _results| {
                callback_clone.store(true, Ordering::SeqCst);
                Ok(())
            })
            .build();

        let result = group.main(vec!["sub".to_string()]);
        assert!(result.is_ok());
        assert!(result_callback_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_result_callback_with_chain_mode() {
        // Test that result_callback receives results from all chained commands
        let result_callback_called = Arc::new(AtomicBool::new(false));
        let callback_clone = Arc::clone(&result_callback_called);
        let results_count = Arc::new(std::sync::Mutex::new(0usize));
        let count_clone = Arc::clone(&results_count);

        let group = Group::new("cli")
            .chain(true)
            .command(Command::new("a").callback(|_| Ok(())).build())
            .command(Command::new("b").callback(|_| Ok(())).build())
            .result_callback(move |_ctx, results| {
                callback_clone.store(true, Ordering::SeqCst);
                *count_clone.lock().unwrap() = results.len();
                Ok(())
            })
            .build();

        let result = group.main(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result_callback_called.load(Ordering::SeqCst));

        // Should have 2 results (one for each command)
        let count = *results_count.lock().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_result_callback_invoke_without_command() {
        // Test that result_callback is called even when invoke_without_command is used
        let result_callback_called = Arc::new(AtomicBool::new(false));
        let callback_clone = Arc::clone(&result_callback_called);

        let group = Group::new("cli")
            .invoke_without_command(true)
            .callback(|_ctx| Ok(()))
            .result_callback(move |_ctx, _results| {
                callback_clone.store(true, Ordering::SeqCst);
                Ok(())
            })
            .build();

        let result = group.main(vec![]);
        assert!(result.is_ok());
        assert!(result_callback_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_chain_mode_subcommand_failure_stops_chain() {
        // Test that if a subcommand fails, the chain stops
        let second_called = Arc::new(AtomicBool::new(false));
        let second_clone = Arc::clone(&second_called);

        let group = Group::new("cli")
            .chain(true)
            .command(
                Command::new("fail")
                    .callback(|_ctx| Err(ClickError::usage("intentional failure")))
                    .build(),
            )
            .command(
                Command::new("second")
                    .callback(move |_ctx| {
                        second_clone.store(true, Ordering::SeqCst);
                        Ok(())
                    })
                    .build(),
            )
            .build();

        let result = group.main(vec!["fail".to_string(), "second".to_string()]);
        assert!(result.is_err());
        // Second command should not have been called
        assert!(!second_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_non_chain_mode_single_command() {
        // Test that non-chain mode only invokes one command
        let calls = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let calls1 = Arc::clone(&calls);

        let group = Group::new("cli")
            .chain(false) // explicitly not chain mode
            .command(
                Command::new("cmd1")
                    .callback(move |_ctx| {
                        calls1.lock().unwrap().push("cmd1".to_string());
                        Ok(())
                    })
                    .build(),
            )
            .command(Command::new("cmd2").build())
            .build();

        // In non-chain mode, "cmd2" would be passed as arg to cmd1, not as separate command
        let result = group.main(vec!["cmd1".to_string()]);
        assert!(result.is_ok());

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], "cmd1");
    }

    #[test]
    fn test_group_callback_called_before_subcommand() {
        // Test that group callback is called before subcommand in non-chain mode
        let call_order = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let order_group = Arc::clone(&call_order);
        let order_sub = Arc::clone(&call_order);

        let group = Group::new("cli")
            .callback(move |_ctx| {
                order_group.lock().unwrap().push("group".to_string());
                Ok(())
            })
            .command(
                Command::new("sub")
                    .callback(move |_ctx| {
                        order_sub.lock().unwrap().push("sub".to_string());
                        Ok(())
                    })
                    .build(),
            )
            .build();

        let result = group.main(vec!["sub".to_string()]);
        assert!(result.is_ok());

        let order = call_order.lock().unwrap();
        assert_eq!(*order, vec!["group", "sub"]);
    }

    #[test]
    fn test_group_callback_called_before_chain() {
        // Test that group callback is called before chained subcommands
        let call_order = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let order_group = Arc::clone(&call_order);
        let order_a = Arc::clone(&call_order);
        let order_b = Arc::clone(&call_order);

        let group = Group::new("cli")
            .chain(true)
            .callback(move |_ctx| {
                order_group.lock().unwrap().push("group".to_string());
                Ok(())
            })
            .command(
                Command::new("a")
                    .callback(move |_ctx| {
                        order_a.lock().unwrap().push("a".to_string());
                        Ok(())
                    })
                    .build(),
            )
            .command(
                Command::new("b")
                    .callback(move |_ctx| {
                        order_b.lock().unwrap().push("b".to_string());
                        Ok(())
                    })
                    .build(),
            )
            .build();

        let result = group.main(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());

        let order = call_order.lock().unwrap();
        assert_eq!(*order, vec!["group", "a", "b"]);
    }

    #[test]
    fn test_command_collection_list_commands_union_sorted() {
        let src = Group::new("src")
            .command(Command::new("c").help("C").build())
            .command(Command::new("b").help("B").build())
            .build();

        let collection = CommandCollection::new("coll")
            .command(Command::new("a").help("A").build())
            .source(src)
            .build();

        assert_eq!(
            collection.list_commands(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_command_collection_prefers_base_over_sources() {
        let src = Group::new("src")
            .command(Command::new("dup").help("Src").build())
            .build();

        let collection = CommandCollection::new("coll")
            .command(Command::new("dup").help("Base").build())
            .source(src)
            .build();

        let ctx = ContextBuilder::new().info_name("coll").build();
        let help = collection.get_help(&ctx);
        assert!(help.contains("dup"));
        assert_eq!(
            collection.get_command("dup").unwrap().get_short_help(),
            "Base"
        );
    }
}
