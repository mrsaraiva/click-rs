//! Shell completion support for click-rs.
//!
//! This module provides shell completion functionality for Bash, Zsh, and Fish shells.
//! It generates completion scripts that can be sourced in each shell to provide
//! tab completion for CLI applications built with click-rs.
//!
//! # Reference
//!
//! Based on Python Click's `shell_completion.py`.
//!
//! # Example
//!
//! ```rust,ignore
//! use click::completion::{get_completion_class, shell_complete};
//! use click::command::Command;
//!
//! let cmd = Command::new("myapp").build();
//!
//! // Generate completion script for bash
//! if let Some(completer) = get_completion_class("bash") {
//!     let script = completer.source_template();
//!     println!("{}", script.replace("%(prog_name)s", "myapp"));
//! }
//! ```

use std::env;
use std::io::{self, Write};

use crate::command::Command;
use crate::context::ContextBuilder;
use crate::group::{CommandLike, Group};
use crate::parameter::Parameter;
use crate::types::CompletionItem;

// =============================================================================
// ShellComplete Trait
// =============================================================================

/// Trait for shell-specific completion implementations.
///
/// Each shell has different completion mechanisms and script formats.
/// Implementations of this trait provide the necessary integration for each shell.
pub trait ShellComplete: Send + Sync {
    /// Returns the name of the shell (e.g., "bash", "zsh", "fish").
    fn name(&self) -> &str;

    /// Returns the shell script template for enabling completions.
    ///
    /// The template can contain placeholders:
    /// - `%(prog_name)s` - The program name
    /// - `%(complete_func)s` - The completion function name
    /// - `%(complete_var)s` - The completion environment variable
    fn source_template(&self) -> &str;

    /// Get completion arguments from the shell environment.
    ///
    /// Parses the completion environment variables set by the shell's completion
    /// system and returns the arguments to complete.
    fn get_completion_args(&self) -> CompletionArgs;

    /// Format a completion item for output to the shell.
    ///
    /// Each shell expects completions in a different format.
    fn format_completion(&self, item: &CompletionItem) -> String;

    /// Get the source script with placeholders replaced.
    fn get_source(&self, prog_name: &str, complete_var: &str) -> String {
        let complete_func = format!("_{}_completion", prog_name.replace('-', "_"));
        self.source_template()
            .replace("%(prog_name)s", prog_name)
            .replace("%(complete_func)s", &complete_func)
            .replace("%(complete_var)s", complete_var)
    }
}

/// Arguments parsed from the shell completion environment.
#[derive(Debug, Clone)]
pub struct CompletionArgs {
    /// The arguments up to the cursor position.
    pub args: Vec<String>,
    /// The incomplete word being typed.
    pub incomplete: String,
}

impl Default for CompletionArgs {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            incomplete: String::new(),
        }
    }
}

// =============================================================================
// BashComplete
// =============================================================================

/// Bash shell completion implementation.
///
/// Uses `COMP_WORDS` and `COMP_CWORD` environment variables set by bash's
/// completion system.
#[derive(Debug, Clone, Default)]
pub struct BashComplete;

impl BashComplete {
    /// The bash completion script template.
    const SOURCE_TEMPLATE: &'static str = r#"
%(complete_func)s() {
    local IFS=$'\n'
    COMPREPLY=( $( env COMP_WORDS="${COMP_WORDS[*]}" \
                   COMP_CWORD=$COMP_CWORD \
                   %(complete_var)s=bash_complete \
                   %(prog_name)s ) )
    return 0
}

%(complete_func)s_setup() {
    complete -o default -F %(complete_func)s %(prog_name)s
}

%(complete_func)s_setup
"#;
}

impl ShellComplete for BashComplete {
    fn name(&self) -> &str {
        "bash"
    }

    fn source_template(&self) -> &str {
        Self::SOURCE_TEMPLATE
    }

    fn get_completion_args(&self) -> CompletionArgs {
        // COMP_WORDS is a space-separated list of all words
        // COMP_CWORD is the index of the current word
        let comp_words = env::var("COMP_WORDS").unwrap_or_default();
        let comp_cword: usize = env::var("COMP_CWORD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let words: Vec<&str> = comp_words.split_whitespace().collect();

        // Words before the current position are complete args
        // The current word (at comp_cword) is the incomplete part
        let args: Vec<String> = words
            .iter()
            .take(comp_cword.saturating_sub(1))
            .skip(1) // Skip program name
            .map(|s| s.to_string())
            .collect();

        let incomplete = if comp_cword > 0 && comp_cword <= words.len() {
            words.get(comp_cword).map(|s| s.to_string()).unwrap_or_default()
        } else {
            String::new()
        };

        CompletionArgs { args, incomplete }
    }

    fn format_completion(&self, item: &CompletionItem) -> String {
        // Bash expects plain completion values, one per line
        item.value.clone()
    }
}

// =============================================================================
// ZshComplete
// =============================================================================

/// Zsh shell completion implementation.
///
/// Uses the `_arguments` style completion with `COMP_WORDS` and `COMP_CWORD`.
#[derive(Debug, Clone, Default)]
pub struct ZshComplete;

impl ZshComplete {
    /// The zsh completion script template.
    const SOURCE_TEMPLATE: &'static str = r#"
#compdef %(prog_name)s

%(complete_func)s() {
    local -a completions
    local -a completions_with_descriptions
    local -a response
    (( ! $+commands[%(prog_name)s] )) && return 1

    response=("${(@f)$(env COMP_WORDS="${words[*]}" COMP_CWORD=$((CURRENT-1)) %(complete_var)s=zsh_complete %(prog_name)s)}")

    for key descr in ${(kv)response}; do
        if [[ "$descr" == "_" ]]; then
            completions+=("$key")
        else
            completions_with_descriptions+=("$key":"$descr")
        fi
    done

    if [ -n "$completions_with_descriptions" ]; then
        _describe -V unsorted completions_with_descriptions -U
    fi

    if [ -n "$completions" ]; then
        compadd -U -V unsorted -a completions
    fi
}

if [[ $zsh_eval_context[-1] == loadautofun ]]; then
    %(complete_func)s "$@"
else
    compdef %(complete_func)s %(prog_name)s
fi
"#;
}

impl ShellComplete for ZshComplete {
    fn name(&self) -> &str {
        "zsh"
    }

    fn source_template(&self) -> &str {
        Self::SOURCE_TEMPLATE
    }

    fn get_completion_args(&self) -> CompletionArgs {
        // Zsh uses the same env vars as bash when invoked through our script
        let comp_words = env::var("COMP_WORDS").unwrap_or_default();
        let comp_cword: usize = env::var("COMP_CWORD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let words: Vec<&str> = comp_words.split_whitespace().collect();

        let args: Vec<String> = words
            .iter()
            .take(comp_cword)
            .skip(1)
            .map(|s| s.to_string())
            .collect();

        let incomplete = if comp_cword > 0 && comp_cword <= words.len() {
            words.get(comp_cword).map(|s| s.to_string()).unwrap_or_default()
        } else {
            String::new()
        };

        CompletionArgs { args, incomplete }
    }

    fn format_completion(&self, item: &CompletionItem) -> String {
        // Zsh format: value:description (or just value if no description)
        match &item.help {
            Some(help) if !help.is_empty() => format!("{}:{}", item.value, help),
            _ => format!("{}:_", item.value),
        }
    }
}

// =============================================================================
// FishComplete
// =============================================================================

/// Fish shell completion implementation.
///
/// Uses Fish's native completion system with `complete` command.
#[derive(Debug, Clone, Default)]
pub struct FishComplete;

impl FishComplete {
    /// The fish completion script template.
    const SOURCE_TEMPLATE: &'static str = r#"
function %(complete_func)s
    set -l response (env %(complete_var)s=fish_complete COMP_WORDS=(commandline -cp) COMP_CWORD=(commandline -t) %(prog_name)s)

    for completion in $response
        set -l metadata (string split "," -- $completion)

        if [ $metadata[1] = "dir" ]
            __fish_complete_directories $metadata[2]
        else if [ $metadata[1] = "file" ]
            __fish_complete_path $metadata[2]
        else if [ $metadata[1] = "plain" ]
            echo $metadata[2]
        end
    end
end

complete -c %(prog_name)s -f -a "(%(complete_func)s)"
"#;
}

impl ShellComplete for FishComplete {
    fn name(&self) -> &str {
        "fish"
    }

    fn source_template(&self) -> &str {
        Self::SOURCE_TEMPLATE
    }

    fn get_completion_args(&self) -> CompletionArgs {
        // Fish passes COMP_WORDS as space-separated and COMP_CWORD as the current token
        let comp_words = env::var("COMP_WORDS").unwrap_or_default();
        let incomplete = env::var("COMP_CWORD").unwrap_or_default();

        let words: Vec<&str> = comp_words.split_whitespace().collect();

        // All words except the program name are args
        let args: Vec<String> = words.iter().skip(1).map(|s| s.to_string()).collect();

        CompletionArgs { args, incomplete }
    }

    fn format_completion(&self, item: &CompletionItem) -> String {
        // Fish format: type,value (type is used for special completions like files)
        format!("{},{}", item.completion_type, item.value)
    }
}

// =============================================================================
// Shell Registry
// =============================================================================

/// Get a shell completion implementation by name.
///
/// # Arguments
///
/// * `shell` - The shell name ("bash", "zsh", or "fish")
///
/// # Returns
///
/// Returns `Some(Box<dyn ShellComplete>)` if the shell is supported, `None` otherwise.
///
/// # Example
///
/// ```rust
/// use click::completion::get_completion_class;
///
/// if let Some(completer) = get_completion_class("bash") {
///     println!("Shell: {}", completer.name());
/// }
/// ```
pub fn get_completion_class(shell: &str) -> Option<Box<dyn ShellComplete>> {
    match shell.to_lowercase().as_str() {
        "bash" => Some(Box::new(BashComplete)),
        "zsh" => Some(Box::new(ZshComplete)),
        "fish" => Some(Box::new(FishComplete)),
        _ => None,
    }
}

/// Detect the current shell from environment.
///
/// Checks `SHELL` environment variable and tries to determine the shell type.
///
/// # Returns
///
/// Returns the shell name if detected, or `None` if unknown.
pub fn detect_shell() -> Option<String> {
    env::var("SHELL").ok().and_then(|shell| {
        let shell_name = shell.rsplit('/').next()?;
        match shell_name {
            "bash" => Some("bash".to_string()),
            "zsh" => Some("zsh".to_string()),
            "fish" => Some("fish".to_string()),
            _ => None,
        }
    })
}

/// List all supported shell names.
pub fn list_shells() -> Vec<&'static str> {
    vec!["bash", "zsh", "fish"]
}

// =============================================================================
// Completion Functions
// =============================================================================

/// Main shell completion entry point.
///
/// This function should be called when the completion environment variable is set.
/// It parses the completion arguments, generates completions, and outputs them
/// in the appropriate format for the shell.
///
/// # Arguments
///
/// * `cmd` - The root command to complete for
/// * `prog_name` - The program name
/// * `complete_var` - The environment variable name that triggers completion
///
/// # Example
///
/// ```rust,ignore
/// use click::completion::shell_complete;
/// use click::command::Command;
/// use std::env;
///
/// let cmd = Command::new("myapp").build();
///
/// // In your main(), check if completion is requested
/// if let Ok(shell) = env::var("_MYAPP_COMPLETE") {
///     shell_complete(&cmd, "myapp", "_MYAPP_COMPLETE");
///     return;
/// }
/// ```
pub fn shell_complete(cmd: &dyn CommandLike, prog_name: &str, complete_var: &str) {
    // Get the shell type from the completion variable
    let shell_type = match env::var(complete_var) {
        Ok(val) => {
            // The value is typically "bash_complete", "zsh_complete", etc.
            val.split('_').next().unwrap_or("bash").to_string()
        }
        Err(_) => return,
    };

    let completer = match get_completion_class(&shell_type) {
        Some(c) => c,
        None => return,
    };

    // Get completion arguments from environment
    let comp_args = completer.get_completion_args();

    // Generate completions
    let completions = get_completions(cmd, prog_name, &comp_args.args, &comp_args.incomplete);

    // Output completions in shell-specific format
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for item in completions {
        let _ = writeln!(handle, "{}", completer.format_completion(&item));
    }
}

/// Get completions for a command.
///
/// This is the internal completion generation function that can be used
/// for testing or custom completion implementations.
///
/// # Arguments
///
/// * `cmd` - The command to complete for
/// * `prog_name` - The program name
/// * `args` - The arguments entered so far
/// * `incomplete` - The incomplete word being typed
///
/// # Returns
///
/// A vector of completion items.
pub fn get_completions(
    cmd: &dyn CommandLike,
    prog_name: &str,
    args: &[String],
    incomplete: &str,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    // Create a resilient parsing context
    let ctx = ContextBuilder::new()
        .info_name(prog_name)
        .resilient_parsing(true)
        .build();

    // Check if we're completing a subcommand
    if let Some(group) = cmd.as_any().downcast_ref::<Group>() {
        // First check if the first arg is a valid subcommand - if so, recurse into it
        if !args.is_empty() {
            if let Some(subcmd) = group.get_command(&args[0]) {
                let remaining_args: Vec<String> = args[1..].to_vec();
                return get_completions(subcmd, &args[0], &remaining_args, incomplete);
            }
        }

        // Otherwise, list subcommand completions
        for name in group.list_commands() {
            if name.starts_with(incomplete) {
                let subcmd = group.get_command(name);
                let help = subcmd.map(|c| c.get_short_help());
                let mut item = CompletionItem::new(name);
                if let Some(h) = help {
                    if !h.is_empty() {
                        item = item.with_help(h);
                    }
                }
                completions.push(item);
            }
        }
    }

    // Complete options
    if incomplete.starts_with('-') || completions.is_empty() {
        if let Some(command) = cmd.as_any().downcast_ref::<Command>() {
            completions.extend(get_option_completions(command, &ctx, incomplete));
        } else if let Some(group) = cmd.as_any().downcast_ref::<Group>() {
            completions.extend(get_option_completions(&group.command, &ctx, incomplete));
        }
    }

    completions
}

/// Get option completions for a command.
fn get_option_completions(
    cmd: &Command,
    _ctx: &crate::context::Context,
    incomplete: &str,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    for opt in &cmd.options {
        // Add long options
        for long in &opt.long {
            if long.starts_with(incomplete) {
                let mut item = CompletionItem::new(long);
                if let Some(help) = opt.help() {
                    item = item.with_help(help.to_string());
                }
                completions.push(item);
            }
        }

        // Add short options
        for short in &opt.short {
            if short.starts_with(incomplete) {
                let mut item = CompletionItem::new(short);
                if let Some(help) = opt.help() {
                    item = item.with_help(help.to_string());
                }
                completions.push(item);
            }
        }
    }

    // Add help option
    if "--help".starts_with(incomplete) {
        completions.push(CompletionItem::new("--help").with_help("Show this message and exit."));
    }

    completions
}

/// Add the shell completion option to a command.
///
/// This returns options that can be added to a command to enable
/// shell completion script generation.
///
/// # Example
///
/// ```rust,ignore
/// use click::command::Command;
/// use click::completion::completion_option;
///
/// let cmd = Command::new("myapp")
///     .option(completion_option("_MYAPP_COMPLETE"))
///     .build();
/// ```
pub fn make_completion_option(complete_var: &str) -> CompletionOption {
    CompletionOption {
        complete_var: complete_var.to_string(),
    }
}

/// Configuration for the shell completion option.
#[derive(Debug, Clone)]
pub struct CompletionOption {
    /// The environment variable that triggers completion.
    pub complete_var: String,
}

impl CompletionOption {
    /// Check if completion is requested via environment variable.
    pub fn is_completion_requested(&self) -> bool {
        env::var(&self.complete_var).is_ok()
    }

    /// Get the completion shell type if completion is requested.
    pub fn get_completion_shell(&self) -> Option<String> {
        env::var(&self.complete_var).ok().and_then(|val| {
            // Values are like "bash_complete", "bash_source", etc.
            let parts: Vec<&str> = val.split('_').collect();
            if parts.len() >= 2 {
                Some(parts[0].to_string())
            } else {
                None
            }
        })
    }

    /// Check if this is a "source" request (print the completion script).
    pub fn is_source_request(&self) -> bool {
        env::var(&self.complete_var)
            .map(|v| v.ends_with("_source"))
            .unwrap_or(false)
    }

    /// Handle completion if requested.
    ///
    /// Returns `true` if completion was handled and the program should exit.
    pub fn handle_completion(
        &self,
        cmd: &dyn CommandLike,
        prog_name: &str,
    ) -> bool {
        if !self.is_completion_requested() {
            return false;
        }

        if let Some(shell) = self.get_completion_shell() {
            if self.is_source_request() {
                // Print the completion script
                if let Some(completer) = get_completion_class(&shell) {
                    println!("{}", completer.get_source(prog_name, &self.complete_var));
                }
            } else {
                // Generate completions
                shell_complete(cmd, prog_name, &self.complete_var);
            }
            return true;
        }

        false
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_completion_class() {
        assert!(get_completion_class("bash").is_some());
        assert!(get_completion_class("zsh").is_some());
        assert!(get_completion_class("fish").is_some());
        assert!(get_completion_class("unknown").is_none());

        // Case insensitive
        assert!(get_completion_class("BASH").is_some());
        assert!(get_completion_class("Zsh").is_some());
    }

    #[test]
    fn test_shell_names() {
        let bash = BashComplete;
        assert_eq!(bash.name(), "bash");

        let zsh = ZshComplete;
        assert_eq!(zsh.name(), "zsh");

        let fish = FishComplete;
        assert_eq!(fish.name(), "fish");
    }

    #[test]
    fn test_list_shells() {
        let shells = list_shells();
        assert!(shells.contains(&"bash"));
        assert!(shells.contains(&"zsh"));
        assert!(shells.contains(&"fish"));
    }

    #[test]
    fn test_bash_source_template() {
        let bash = BashComplete;
        let source = bash.get_source("myapp", "_MYAPP_COMPLETE");

        assert!(source.contains("myapp"));
        assert!(source.contains("_MYAPP_COMPLETE"));
        assert!(source.contains("_myapp_completion"));
        assert!(source.contains("COMP_WORDS"));
        assert!(source.contains("COMP_CWORD"));
    }

    #[test]
    fn test_zsh_source_template() {
        let zsh = ZshComplete;
        let source = zsh.get_source("myapp", "_MYAPP_COMPLETE");

        assert!(source.contains("#compdef myapp"));
        assert!(source.contains("_MYAPP_COMPLETE"));
        assert!(source.contains("_myapp_completion"));
    }

    #[test]
    fn test_fish_source_template() {
        let fish = FishComplete;
        let source = fish.get_source("myapp", "_MYAPP_COMPLETE");

        assert!(source.contains("function _myapp_completion"));
        assert!(source.contains("_MYAPP_COMPLETE"));
        assert!(source.contains("complete -c myapp"));
    }

    #[test]
    fn test_bash_format_completion() {
        let bash = BashComplete;

        let item = CompletionItem::new("--help");
        assert_eq!(bash.format_completion(&item), "--help");

        let item_with_help = CompletionItem::new("--name").with_help("Specify name");
        assert_eq!(bash.format_completion(&item_with_help), "--name");
    }

    #[test]
    fn test_zsh_format_completion() {
        let zsh = ZshComplete;

        let item = CompletionItem::new("--help");
        assert_eq!(zsh.format_completion(&item), "--help:_");

        let item_with_help = CompletionItem::new("--name").with_help("Specify name");
        assert_eq!(zsh.format_completion(&item_with_help), "--name:Specify name");
    }

    #[test]
    fn test_fish_format_completion() {
        let fish = FishComplete;

        let item = CompletionItem::new("--help");
        assert_eq!(fish.format_completion(&item), "plain,--help");

        let item_file = CompletionItem::with_type("path", "file");
        assert_eq!(fish.format_completion(&item_file), "file,path");
    }

    #[test]
    fn test_completion_args_default() {
        let args = CompletionArgs::default();
        assert!(args.args.is_empty());
        assert!(args.incomplete.is_empty());
    }

    #[test]
    fn test_get_completions_empty() {
        let cmd = Command::new("test").build();
        let completions = get_completions(&cmd, "test", &[], "");

        // Should at least have --help
        assert!(completions.iter().any(|c| c.value == "--help"));
    }

    #[test]
    fn test_get_completions_options() {
        let cmd = Command::new("test")
            .option(
                crate::option::ClickOption::new(&["--name", "-n"])
                    .help("The name")
                    .build(),
            )
            .build();

        let completions = get_completions(&cmd, "test", &[], "--");

        assert!(completions.iter().any(|c| c.value == "--name"));
        assert!(completions.iter().any(|c| c.value == "--help"));
    }

    #[test]
    fn test_get_completions_subcommands() {
        let group = Group::new("cli")
            .command(Command::new("init").help("Initialize").build())
            .command(Command::new("build").help("Build").build())
            .build();

        let completions = get_completions(&group, "cli", &[], "");

        assert!(completions.iter().any(|c| c.value == "init"));
        assert!(completions.iter().any(|c| c.value == "build"));
    }

    #[test]
    fn test_get_completions_subcommand_prefix() {
        let group = Group::new("cli")
            .command(Command::new("init").build())
            .command(Command::new("install").build())
            .command(Command::new("build").build())
            .build();

        let completions = get_completions(&group, "cli", &[], "in");

        assert!(completions.iter().any(|c| c.value == "init"));
        assert!(completions.iter().any(|c| c.value == "install"));
        assert!(!completions.iter().any(|c| c.value == "build"));
    }

    #[test]
    fn test_completion_option() {
        let opt = make_completion_option("_TEST_COMPLETE");
        assert_eq!(opt.complete_var, "_TEST_COMPLETE");
    }

    #[test]
    fn test_completion_option_not_requested() {
        // Clear any existing env var
        env::remove_var("_TEST_COMPLETE");

        let opt = make_completion_option("_TEST_COMPLETE");
        assert!(!opt.is_completion_requested());
        assert!(opt.get_completion_shell().is_none());
        assert!(!opt.is_source_request());
    }

    #[test]
    fn test_prog_name_with_dash() {
        let bash = BashComplete;
        let source = bash.get_source("my-app", "_MY_APP_COMPLETE");

        // Function name should have underscore, not dash
        assert!(source.contains("_my_app_completion"));
    }
}
