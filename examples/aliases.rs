//! Port of Python Click's aliases example.
//!
//! This example demonstrates how to implement command aliases using a custom Group
//! subclass that reads aliases from a config file.
//!
//! Run with: cargo run --example aliases -- [--config aliases.ini] <command>
//!
//! Features demonstrated:
//! - Custom Group implementation with alias resolution
//! - Reading aliases from INI-style config files
//! - Automatic command abbreviation (e.g., "st" matches "status")
//! - pass_decorator pattern for sharing state

use click::{
    echo, make_pass_decorator, Argument, ClickError, ClickOption, Command, CommandLike, Context,
    ContextBuilder, Group, Result,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::sync::{Arc, Mutex};

/// The config in this example only holds aliases.
#[derive(Debug, Clone)]
struct Config {
    path: String,
    aliases: HashMap<String, String>,
}

impl Config {
    fn new() -> Self {
        Self {
            path: env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            aliases: HashMap::new(),
        }
    }

    fn add_alias(&mut self, alias: &str, cmd: &str) {
        self.aliases.insert(alias.to_string(), cmd.to_string());
    }

    /// Read aliases from an INI-style config file.
    /// Expected format:
    /// ```ini
    /// [aliases]
    /// st = status
    /// ci = commit
    /// ```
    fn read_config(&mut self, filename: &str) {
        if let Ok(content) = fs::read_to_string(filename) {
            let mut in_aliases_section = false;
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('[') && line.ends_with(']') {
                    in_aliases_section = line == "[aliases]";
                } else if in_aliases_section && line.contains('=') {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim();
                        let value = parts[1].trim();
                        self.aliases.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }
    }

    /// Write aliases to an INI-style config file.
    fn write_config(&self, filename: &str) -> std::io::Result<()> {
        let mut content = String::from("[aliases]\n");
        for (key, value) in &self.aliases {
            content.push_str(&format!("{} = {}\n", key, value));
        }
        fs::write(filename, content)
    }
}

/// Thread-safe wrapper for Config that can be stored in context.
#[derive(Debug, Clone)]
struct SharedConfig(Arc<Mutex<Config>>);

impl SharedConfig {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Config::new())))
    }

    fn with_config<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Config) -> R,
    {
        let guard = self.0.lock().unwrap();
        f(&guard)
    }

    fn with_config_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Config) -> R,
    {
        let mut guard = self.0.lock().unwrap();
        f(&mut guard)
    }
}

/// AliasedGroup: A group that supports looking up aliases in a config file.
///
/// This custom group implementation:
/// 1. First tries to find built-in commands as normal
/// 2. Then looks up explicit aliases from the config
/// 3. Finally, allows automatic abbreviation (e.g., "st" matches "status")
struct AliasedGroup {
    inner: Group,
    config: SharedConfig,
}

impl AliasedGroup {
    fn new(inner: Group, config: SharedConfig) -> Self {
        Self { inner, config }
    }

    /// Resolve a command name, handling aliases and abbreviations.
    fn resolve_alias(&self, cmd_name: &str) -> Option<String> {
        // Step 1: Check if it's a built-in command
        if self.inner.get_command(cmd_name).is_some() {
            return Some(cmd_name.to_string());
        }

        // Step 2: Look up an explicit alias in the config
        let alias_result = self
            .config
            .with_config(|cfg| cfg.aliases.get(cmd_name).cloned());
        if let Some(actual_cmd) = alias_result {
            if self.inner.get_command(&actual_cmd).is_some() {
                return Some(actual_cmd);
            }
        }

        // Step 3: Try automatic abbreviation
        let cmd_lower = cmd_name.to_lowercase();
        let commands = self.inner.list_commands();
        let matches: Vec<&str> = commands
            .iter()
            .filter(|name| name.to_lowercase().starts_with(&cmd_lower))
            .copied()
            .collect();

        if matches.len() == 1 {
            return Some(matches[0].to_string());
        }

        None
    }
}

impl CommandLike for AliasedGroup {
    fn name(&self) -> Option<&str> {
        self.inner.name()
    }

    fn make_context(
        &self,
        info_name: &str,
        args: Vec<String>,
        parent: Option<Arc<Context>>,
    ) -> Result<Context> {
        self.inner.make_context(info_name, args, parent)
    }

    fn invoke(&self, ctx: &Context) -> Result<()> {
        let args = ctx.args().to_vec();

        if args.is_empty() {
            // No subcommand - let the inner group handle it
            return self.inner.invoke(ctx);
        }

        // Resolve the command name (handling aliases)
        let cmd_name = &args[0];
        if let Some(resolved_name) = self.resolve_alias(cmd_name) {
            // Get the actual command
            if let Some(cmd) = self.inner.get_command(&resolved_name) {
                // Create context for subcommand
                let remaining = args[1..].to_vec();
                let parent_arc = click::get_current_context();
                let sub_ctx = cmd.make_context(&resolved_name, remaining, parent_arc)?;
                let sub_ctx = Arc::new(sub_ctx);

                click::push_context(Arc::clone(&sub_ctx));
                let result = cmd.invoke(&sub_ctx);
                click::pop_context();
                sub_ctx.close();

                return result;
            }
        }

        // Check if it looks like an unknown command
        if !args.is_empty() && !args[0].starts_with('-') {
            // Check for multiple matches (ambiguous abbreviation)
            let cmd_lower = args[0].to_lowercase();
            let commands = self.inner.list_commands();
            let matches: Vec<&str> = commands
                .iter()
                .filter(|name| name.to_lowercase().starts_with(&cmd_lower))
                .copied()
                .collect();

            if matches.len() > 1 {
                let mut sorted_matches: Vec<&str> = matches;
                sorted_matches.sort();
                return Err(ClickError::usage(format!(
                    "Too many matches: {}",
                    sorted_matches.join(", ")
                )));
            }
        }

        // Fall back to inner group (which will report "no such command")
        self.inner.invoke(ctx)
    }

    fn main(&self, args: Vec<String>) -> Result<()> {
        let prog_name = self.inner.name().unwrap_or("aliases").to_string();
        let ctx = self.make_context(&prog_name, args, None)?;
        let ctx = Arc::new(ctx);

        click::push_context(Arc::clone(&ctx));
        let result = self.invoke(&ctx);
        click::pop_context();
        ctx.close();

        result
    }

    fn get_help(&self, ctx: &Context) -> String {
        self.inner.get_help(ctx)
    }

    fn get_short_help(&self) -> String {
        self.inner.get_short_help()
    }

    fn is_hidden(&self) -> bool {
        self.inner.is_hidden()
    }

    fn get_usage(&self, ctx: &Context) -> String {
        self.inner.get_usage(ctx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn main() {
    // Create shared config
    let config = SharedConfig::new();

    // Read default config file if it exists
    let default_config = env::current_dir()
        .map(|p| p.join("aliases.ini"))
        .ok()
        .filter(|p| p.exists())
        .map(|p| p.display().to_string());

    if let Some(ref path) = default_config {
        config.with_config_mut(|cfg| cfg.read_config(path));
    }

    // Create pass_config decorator
    let pass_config = make_pass_decorator::<SharedConfig>();

    // Build the subcommands
    let push_cmd = Command::new("push")
        .help("Pushes changes.")
        .callback(|_ctx| {
            echo("Push", true, false, None);
            Ok(())
        })
        .build();

    let pull_cmd = Command::new("pull")
        .help("Pulls changes.")
        .callback(|_ctx| {
            echo("Pull", true, false, None);
            Ok(())
        })
        .build();

    let clone_cmd = Command::new("clone")
        .help("Clones a repository.")
        .callback(|_ctx| {
            echo("Clone", true, false, None);
            Ok(())
        })
        .build();

    let commit_cmd = Command::new("commit")
        .help("Commits pending changes.")
        .callback(|_ctx| {
            echo("Commit", true, false, None);
            Ok(())
        })
        .build();

    // Status command uses pass_config
    let status_cmd = Command::new("status")
        .help("Shows the status.")
        .callback(pass_config.decorate(move |cfg: &SharedConfig, _ctx| {
            let path = cfg.with_config(|c| c.path.clone());
            echo(&format!("Status for {}", path), true, false, None);
            Ok(())
        }))
        .build();

    // Alias command to add new aliases
    let alias_cmd = Command::new("alias")
        .help("Adds an alias to the specified configuration file.")
        .argument(Argument::new("alias_").help("The alias name").build())
        .argument(Argument::new("cmd").help("The command to alias").build())
        .option(
            ClickOption::new(&["--config-file"])
                .help("Config file to write to")
                .default("aliases.ini")
                .build(),
        )
        .callback(
            make_pass_decorator::<SharedConfig>().decorate(move |cfg: &SharedConfig, ctx| {
                let alias_ = ctx
                    .get_param::<String>("alias_")
                    .ok_or_else(|| ClickError::missing_argument("ALIAS"))?;
                let cmd = ctx
                    .get_param::<String>("cmd")
                    .ok_or_else(|| ClickError::missing_argument("CMD"))?;
                let config_file = ctx
                    .get_param::<String>("config_file")
                    .map(|s| s.as_str())
                    .unwrap_or("aliases.ini");

                cfg.with_config_mut(|c| {
                    c.add_alias(alias_, cmd);
                    if let Err(e) = c.write_config(config_file) {
                        eprintln!("Warning: Could not write config: {}", e);
                    }
                });

                echo(
                    &format!("Added '{}' as alias for '{}'", alias_, cmd),
                    true,
                    false,
                    None,
                );
                Ok(())
            }),
        )
        .build();

    // Build the inner group with --config option
    let inner_group = Group::new("aliases")
        .help("An example application that supports aliases.")
        .option(
            ClickOption::new(&["--config"])
                .help("The config file to use instead of the default.")
                .build(),
        )
        .invoke_without_command(false)
        .command(push_cmd)
        .command(pull_cmd)
        .command(clone_cmd)
        .command(commit_cmd)
        .command(status_cmd)
        .command(alias_cmd)
        .build();

    // Wrap in AliasedGroup
    let cli = AliasedGroup::new(inner_group, config.clone());

    // Get CLI args
    let args: Vec<String> = env::args().skip(1).collect();

    // Handle --config option early to load config before alias resolution
    let mut config_path = default_config;
    let mut filtered_args = Vec::new();
    let mut skip_next = false;

    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--config" {
            if let Some(path) = args.get(i + 1) {
                config_path = Some(path.clone());
                skip_next = true;
                continue;
            }
        } else if arg.starts_with("--config=") {
            config_path = Some(arg.trim_start_matches("--config=").to_string());
            continue;
        }
        filtered_args.push(arg.clone());
    }

    // Load config file
    if let Some(ref path) = config_path {
        config.with_config_mut(|cfg| cfg.read_config(path));
    }

    // Create context with config object
    let mut ctx = ContextBuilder::new()
        .info_name("aliases")
        .obj(config.clone())
        .allow_extra_args(true)
        .allow_interspersed_args(false)
        .build();

    // Store args
    *ctx.args_mut() = filtered_args;

    let ctx = Arc::new(ctx);
    click::push_context(Arc::clone(&ctx));

    if let Err(e) = cli.invoke(&ctx) {
        eprintln!("{}", e.format_full());
        std::process::exit(e.exit_code());
    }

    click::pop_context();
}
