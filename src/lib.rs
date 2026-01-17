//! click-rs: A Rust port of Python's Click library for creating command-line interfaces.
//!
//! This crate provides a declarative way to build command-line applications with
//! support for commands, options, arguments, and help generation.

pub mod argument;
pub mod context;
pub mod error;
pub mod option;
pub mod parameter;
mod source;
pub mod types;

pub use argument::{Argument, ArgumentBuilder};
pub use context::{get_current_context, pop_context, push_context, Context, ContextBuilder};
pub use error::{ClickError, ErrorContext, ParamType, Result};
pub use option::{parse_option_name, split_option_names, ClickOption, OptionBuilder};
pub use parameter::{DeprecationInfo, Nargs, Parameter, ParameterConfig};
pub use source::ParameterSource;

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
