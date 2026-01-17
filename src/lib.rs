//! click-rs: A Rust port of Python's Click library for creating command-line interfaces.
//!
//! This crate provides a declarative way to build command-line applications with
//! support for commands, options, arguments, and help generation.

pub mod error;
mod source;
pub mod types;

pub use error::{ClickError, ErrorContext, ParamType, Result};
pub use source::ParameterSource;

// Re-export type converter trait and common types
pub use types::{
    // Trait
    TypeConverter,
    // Singleton types
    BOOL, FLOAT, INT, STRING, UNPROCESSED, UUID,
    // Type structs
    BoolType, Choice, DateTimeType, FileMode, FileType, FloatRange, FloatType,
    IntRange, IntType, LazyFile, PathType, StringType, TupleType, TupleValue,
    UuidType, UnprocessedType,
    // Completion
    CompletionItem,
};
