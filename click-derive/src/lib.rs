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
use syn::{parse_macro_input, DeriveInput, ItemFn};

mod attrs;
mod command;
mod function;
mod group;

use command::expand_command;
use function::expand_command_fn;
use group::expand_group;

/// Derive macro for creating CLI commands from structs.
///
/// # Container Attributes
///
/// - `#[command(name = "...")]` - Set the command name (defaults to struct name in kebab-case)
/// - `#[command(help = "...")]` - Set the help text (defaults to doc comment)
/// - `#[command(run)]` - Wire `Self::run(&self|self, &Context) -> Result<()>` as callback
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
/// - `#[option(nargs = -1)]` - Variadic option values
/// - `#[option(type = click::PathType::new())]` - Explicit converter expression
/// - `#[option(validate = my_validator)]` - Declarative validation (`fn(&T) -> Result<(), String>`)
/// - `#[option(envvar = "VAR")]` - Read from environment variable
/// - `#[option(dest = "name")]` - Override destination parameter name
/// - `#[option(shell_complete = my_completer)]` - Custom value completion callback
///
/// ## Arguments
///
/// - `#[argument]` - Positional argument
/// - `#[argument(help = "...")]` - Set help text
/// - `#[argument(required = false)]` - Optional argument
/// - `#[argument(multiple)]` - Accept multiple values
/// - `#[argument(nargs = -1)]` - Variadic argument values
/// - `#[argument(type = click::FileType::new())]` - Explicit converter expression
/// - `#[argument(validate = my_validator)]` - Declarative validation (`fn(&T) -> Result<(), String>`)
/// - `#[argument(shell_complete = my_completer)]` - Custom completion callback
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
/// - `#[group(run)]` - Wire `Self::run(&self|self, &Context) -> Result<()>` as callback
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

/// Attribute macro for function-first command definitions.
///
/// The annotated function parameters must use click parameter attributes such as
/// `#[option(...)]` or `#[argument(...)]`. The macro generates:
/// - a hidden derive-backed struct that carries the parameter metadata
/// - a `<function_name>_command()` helper that builds a `click::Command`
///
/// # Example
///
/// ```ignore
/// #[click::command(name = "hello")]
/// fn hello(#[argument] name: String, #[option(short, long)] loud: bool) -> click::Result<()> {
///     if loud { println!("HELLO, {}!", name); } else { println!("Hello, {}!", name); }
///     Ok(())
/// }
///
/// let cmd = hello_command();
/// ```
#[proc_macro_attribute]
pub fn command(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let input = parse_macro_input!(input as ItemFn);
    expand_command_fn(args, input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
