//! Derive macros for click-rs CLI library.
//!
//! This crate provides procedural macros for automatically generating
//! `CommandLike` implementations from Rust structs.
//!
//! # Example
//!
//! ```ignore
//! use click_derive::Command;
//!
//! #[derive(Command)]
//! #[command(name = "greet")]
//! /// A friendly greeter
//! struct Greet {
//!     /// Name to greet
//!     #[argument]
//!     name: String,
//!
//!     /// Number of times to greet
//!     #[option(short, long, default = 1)]
//!     count: i32,
//! }
//!
//! impl Greet {
//!     fn run(&self, _ctx: &click::Context) -> click::Result<()> {
//!         for _ in 0..self.count {
//!             println!("Hello, {}!", self.name);
//!         }
//!         Ok(())
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod attrs;
mod command;
mod group;

use command::expand_command;
use group::expand_group;

/// Derive macro for creating CLI commands from structs.
///
/// # Container Attributes
///
/// - `#[command(name = "...")]` - Set the command name (defaults to struct name in kebab-case)
/// - `#[command(help = "...")]` - Set the help text (defaults to doc comment)
/// - `#[command(hidden)]` - Hide the command from help
/// - `#[command(no_args_is_help)]` - Show help when no arguments provided
///
/// # Field Attributes
///
/// ## Options
///
/// - `#[option(short, long)]` - Create option with short (-n) and long (--name) flags
/// - `#[option(short = 'n')]` - Specify custom short flag
/// - `#[option(long = "name")]` - Specify custom long flag name
/// - `#[option(help = "...")]` - Set help text
/// - `#[option(default = value)]` - Set default value
/// - `#[option(required)]` - Mark as required
/// - `#[option(count)]` - Count occurrences (-v -v -v = 3)
/// - `#[option(flag)]` - Boolean flag (no value)
/// - `#[option(envvar = "VAR")]` - Read from environment variable
///
/// ## Arguments
///
/// - `#[argument]` - Positional argument
/// - `#[argument(help = "...")]` - Set help text
/// - `#[argument(required = false)]` - Optional argument
/// - `#[argument(multiple)]` - Accept multiple values
///
/// # Example
///
/// ```ignore
/// #[derive(Command)]
/// #[command(name = "greet")]
/// struct Greet {
///     #[option(short, long)]
///     name: String,
///
///     #[option(short, long, default = 1)]
///     count: i32,
///
///     #[argument]
///     target: String,
/// }
/// ```
#[proc_macro_derive(
    Command,
    attributes(
        command,
        option,
        argument,
        pass_context,
        pass_obj,
        version_option,
        help_option,
        confirmation_option,
        password_option
    )
)]
pub fn derive_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_command(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for creating CLI command groups from structs.
///
/// # Container Attributes
///
/// - `#[group(name = "...")]` - Set the group name
/// - `#[group(help = "...")]` - Set the help text
/// - `#[group(chain)]` - Enable command chaining
/// - `#[group(invoke_without_command)]` - Run callback even without subcommand
///
/// # Field Attributes
///
/// - `#[subcommand]` - Mark field as a subcommand
///
/// # Example
///
/// ```ignore
/// #[derive(Group)]
/// #[group(name = "cli")]
/// struct Cli {
///     #[option(short, long)]
///     verbose: bool,
///
///     #[subcommand]
///     command: Commands,
/// }
///
/// enum Commands {
///     Add(AddCmd),
///     Remove(RemoveCmd),
/// }
/// ```
#[proc_macro_derive(
    Group,
    attributes(
        group,
        option,
        argument,
        subcommand,
        pass_context,
        pass_obj,
        help_option
    )
)]
pub fn derive_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_group(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
