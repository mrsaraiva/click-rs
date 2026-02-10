#![warn(unsafe_code)]
//! click-rs: A Rust port of Python's Click library for creating command-line interfaces.
//!
//! This crate provides a declarative way to build command-line applications with
//! support for commands, options, arguments, and help generation.
//!
//! # Quick Start with Derive Macros
//!
//! The easiest way to use click-rs is with derive macros (enabled by default):
//!
//! ```no_run
//! use click::Command;
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
//!     fn run(&self) {
//!         for _ in 0..self.count {
//!             println!("Hello, {}!", self.name);
//!         }
//!     }
//! }
//!
//! fn main() {
//!     Greet::main_with(std::env::args().skip(1).collect(), |greet, _ctx| {
//!         greet.run();
//!         Ok(())
//!     }).unwrap();
//! }
//! ```
//!
//! # Builder API
//!
//! For more control, use the builder API directly:
//!
//! ```no_run
//! use click::{Command, ClickOption, Argument};
//!
//! let cmd = Command::new("greet")
//!     .help("A friendly greeter")
//!     .argument(Argument::new("name").build())
//!     .option(ClickOption::new(&["--count", "-c"]).default("1").build())
//!     .callback(|ctx| {
//!         let name = ctx.get_param::<String>("name").unwrap();
//!         let count: i32 = ctx.get_param::<String>("count")
//!             .map(|s| s.parse().unwrap_or(1))
//!             .unwrap_or(1);
//!         for _ in 0..count {
//!             println!("Hello, {}!", name);
//!         }
//!         Ok(())
//!     })
//!     .build();
//!
//! cmd.main(vec!["World".to_string()]).unwrap();
//! ```

pub mod argument;
pub mod command;
pub mod completion;
pub mod context;
pub mod decorators;
pub mod error;
pub mod formatting;
pub mod group;
pub mod option;
pub mod parameter;
pub mod parser;
mod source;
pub mod termui;
pub mod testing;
pub mod types;
pub mod utils;

pub use argument::{AnyTypeConverter, Argument, ArgumentBuilder, ShellCompleteCallback};
pub use command::{Command, CommandBuilder, CommandCallback};
pub use context::{get_current_context, pop_context, push_context, Context, ContextBuilder};
pub use decorators::{make_pass_decorator, PassDecorator};
pub use group::{CommandCollection, CommandCollectionBuilder, CommandLike, Group, GroupBuilder, ResultCallback};
pub use error::{ClickError, ErrorContext, ParamType, Result};
pub use option::{parse_option_name, split_option_names, ClickOption, OptionBuilder};
pub use parameter::{DeprecationInfo, Nargs, Parameter, ParameterConfig};
pub use parser::{split_opt, OptionAction, OptionParser, ParseResult, ParsedValue};
pub use source::ParameterSource;

// Re-export formatting utilities
pub use formatting::{
    detect_terminal_width, get_terminal_width, make_rule, split_into_lines, truncate_text,
    wrap_text, HelpFormatter,
};

// Re-export derive macros when "derive" feature is enabled
#[cfg(feature = "derive")]
pub use click_derive::{Command, Group};

// Re-export type converter trait and common types
pub use types::{
    // Type structs
    BoolType,
    Choice,
    // Completion
    CompletionItem,
    DateTimeType,
    FileMode,
    FileType,
    FloatRange,
    FloatType,
    IntRange,
    IntType,
    LazyFile,
    PathType,
    StringType,
    TupleType,
    TupleValue,
    // Trait
    TypeConverter,
    UnprocessedType,
    UuidType,
    // Singleton types
    BOOL,
    FLOAT,
    INT,
    STRING,
    UNPROCESSED,
    UUID,
};

// Re-export terminal UI functions
pub use termui::{
    clear, confirm, echo, echo_via_pager, edit_text, get_terminal_size, getchar, isatty, launch,
    pause, progressbar, prompt, secho, stderr_isatty, stdin_isatty, stdout_isatty,
    strip_ansi_codes, style, Color, ProgressBar,
};

// Re-export shell completion
pub use completion::{
    shell_complete, BashComplete, FishComplete, ShellComplete, ZshComplete,
};

// Re-export testing utilities
pub use testing::{CliRunner, InvokeResult, IsolatedFilesystem};

// Re-export general utilities
pub use utils::{
    expand_path, format_filename, get_app_dir, get_binary_stdout, get_os_args, get_text_stderr,
    get_text_stdout, safecall, should_strip_ansi,
};
