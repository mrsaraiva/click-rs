//! Parameter types for click-rs.
//!
//! This module provides the `TypeConverter` trait and all built-in types for
//! converting and validating command-line arguments.

use std::fmt;
use std::fs::{self, File as StdFile, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path as StdPath, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime as ChronoDateTime, NaiveDate, NaiveDateTime, Utc};
use uuid::Uuid;

/// Format a float value like Python does (always show decimal point for whole numbers).
fn format_float(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{:.1}", value)
    } else {
        format!("{}", value)
    }
}

// =============================================================================
// TypeConverter Trait
// =============================================================================

/// Trait for parameter types that convert and validate command-line values.
///
/// Each type must define how to convert a string value from the command line
/// into the appropriate Rust type.
pub trait TypeConverter: fmt::Debug + Send + Sync {
    /// The Rust type that this parameter type converts to.
    type Value;

    /// Returns the descriptive name of this type (used in error messages).
    fn name(&self) -> &str;

    /// Convert a string value to the target type.
    ///
    /// Returns an error message if conversion fails.
    fn convert(&self, value: &str) -> Result<Self::Value, String>;

    /// Returns the metavar for this type (used in help text).
    ///
    /// For example, `INT` might return `"INTEGER"`.
    fn get_metavar(&self) -> Option<String> {
        None
    }

    /// Returns an optional message when a required value is missing.
    fn get_missing_message(&self) -> Option<String> {
        None
    }

    /// Split an environment variable value into multiple values.
    ///
    /// By default, splits on whitespace. Path-based types override this
    /// to split on the platform's path separator.
    fn split_envvar_value(&self, value: &str) -> Vec<String> {
        value.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Returns shell completion items for the given incomplete value.
    ///
    /// Most types return an empty list; types like `Choice` and `Path`
    /// can provide completions.
    fn shell_complete(&self, _incomplete: &str) -> Vec<CompletionItem> {
        Vec::new()
    }

    /// Whether this type is a composite type (like Tuple).
    fn is_composite(&self) -> bool {
        false
    }

    /// The arity (number of values consumed) for composite types.
    fn arity(&self) -> usize {
        1
    }
}

/// A shell completion item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// The completion value.
    pub value: String,
    /// The type of completion (e.g., "file", "dir", "plain").
    pub completion_type: String,
    /// Optional help text for the completion.
    pub help: Option<String>,
}

impl CompletionItem {
    /// Create a new completion item with the given value.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            completion_type: "plain".to_string(),
            help: None,
        }
    }

    /// Create a new completion item with a specific type.
    pub fn with_type(value: impl Into<String>, completion_type: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            completion_type: completion_type.into(),
            help: None,
        }
    }

    /// Add help text to this completion item.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

// =============================================================================
// STRING Type
// =============================================================================

/// A string parameter type (the default).
///
/// Passes through string values unchanged.
#[derive(Debug, Clone, Copy, Default)]
pub struct StringType;

impl TypeConverter for StringType {
    type Value = String;

    fn name(&self) -> &str {
        "TEXT"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        Ok(value.to_string())
    }

    fn get_metavar(&self) -> Option<String> {
        Some("TEXT".to_string())
    }
}

/// Singleton instance for string type.
pub const STRING: StringType = StringType;

// =============================================================================
// INT Type
// =============================================================================

/// An integer parameter type.
#[derive(Debug, Clone, Copy, Default)]
pub struct IntType;

impl TypeConverter for IntType {
    type Value = i64;

    fn name(&self) -> &str {
        "INTEGER"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        value
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("'{}' is not a valid integer.", value))
    }

    fn get_metavar(&self) -> Option<String> {
        Some("INTEGER".to_string())
    }
}

/// Singleton instance for integer type.
pub const INT: IntType = IntType;

// =============================================================================
// FLOAT Type
// =============================================================================

/// A floating-point parameter type.
#[derive(Debug, Clone, Copy, Default)]
pub struct FloatType;

impl TypeConverter for FloatType {
    type Value = f64;

    fn name(&self) -> &str {
        "FLOAT"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        value
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("'{}' is not a valid float.", value))
    }

    fn get_metavar(&self) -> Option<String> {
        Some("FLOAT".to_string())
    }
}

/// Singleton instance for float type.
pub const FLOAT: FloatType = FloatType;

// =============================================================================
// BOOL Type
// =============================================================================

/// A boolean parameter type.
///
/// Accepts various string representations of boolean values:
/// - True: "1", "true", "yes", "on", "t", "y"
/// - False: "0", "false", "no", "off", "f", "n", ""
#[derive(Debug, Clone, Copy, Default)]
pub struct BoolType;

impl BoolType {
    /// Convert a string to a boolean, returning None if not recognized.
    pub fn str_to_bool(value: &str) -> Option<bool> {
        match value.trim().to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" | "t" | "y" => Some(true),
            "0" | "false" | "no" | "off" | "f" | "n" | "" => Some(false),
            _ => None,
        }
    }

    /// List of recognized boolean string values (includes empty string at start).
    pub const BOOL_STATES: &'static [&'static str] = &[
        "", "0", "1", "f", "false", "n", "no", "off", "on", "t", "true", "y", "yes",
    ];
}

impl TypeConverter for BoolType {
    type Value = bool;

    fn name(&self) -> &str {
        "BOOLEAN"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        Self::str_to_bool(value).ok_or_else(|| {
            format!(
                "'{}' is not a valid boolean. Recognized values: {}",
                value,
                Self::BOOL_STATES.join(", ")
            )
        })
    }

    fn get_metavar(&self) -> Option<String> {
        Some("BOOLEAN".to_string())
    }
}

/// Singleton instance for boolean type.
pub const BOOL: BoolType = BoolType;

// =============================================================================
// UUID Type
// =============================================================================

/// A UUID parameter type.
#[derive(Debug, Clone, Copy, Default)]
pub struct UuidType;

impl TypeConverter for UuidType {
    type Value = Uuid;

    fn name(&self) -> &str {
        "UUID"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        Uuid::parse_str(value.trim()).map_err(|_| format!("'{}' is not a valid UUID", value))
    }

    fn get_metavar(&self) -> Option<String> {
        Some("UUID".to_string())
    }
}

/// Singleton instance for UUID type.
pub const UUID: UuidType = UuidType;

// =============================================================================
// UNPROCESSED Type
// =============================================================================

/// A type that passes through values without any processing.
///
/// This is useful when you want to defer processing to a callback
/// or when working with raw byte paths.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnprocessedType;

impl TypeConverter for UnprocessedType {
    type Value = String;

    fn name(&self) -> &str {
        "TEXT"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        Ok(value.to_string())
    }
}

/// Singleton instance for unprocessed type.
pub const UNPROCESSED: UnprocessedType = UnprocessedType;

// =============================================================================
// IntRange Type
// =============================================================================

/// An integer type restricted to a range of values.
///
/// If `min` or `max` are `None`, the range is unbounded in that direction.
/// If `clamp` is true, out-of-range values are clamped to the boundary
/// instead of producing an error.
#[derive(Debug, Clone, Copy)]
pub struct IntRange {
    /// Minimum allowed value (inclusive unless `min_open` is true).
    pub min: Option<i64>,
    /// Maximum allowed value (inclusive unless `max_open` is true).
    pub max: Option<i64>,
    /// If true, the minimum bound is exclusive (value must be > min).
    pub min_open: bool,
    /// If true, the maximum bound is exclusive (value must be < max).
    pub max_open: bool,
    /// If true, clamp out-of-range values instead of failing.
    pub clamp: bool,
}

impl Default for IntRange {
    fn default() -> Self {
        Self::new()
    }
}

impl IntRange {
    /// Create a new unbounded integer range.
    pub const fn new() -> Self {
        Self {
            min: None,
            max: None,
            min_open: false,
            max_open: false,
            clamp: false,
        }
    }

    /// Set the minimum value (inclusive).
    pub const fn min(mut self, min: i64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set the maximum value (inclusive).
    pub const fn max(mut self, max: i64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set both minimum and maximum values.
    pub const fn range(mut self, min: i64, max: i64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Make the minimum bound exclusive (value must be > min).
    pub const fn min_open(mut self, open: bool) -> Self {
        self.min_open = open;
        self
    }

    /// Make the maximum bound exclusive (value must be < max).
    pub const fn max_open(mut self, open: bool) -> Self {
        self.max_open = open;
        self
    }

    /// Enable clamping of out-of-range values.
    pub const fn clamp(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }

    /// Describe the range for error messages.
    fn describe_range(&self) -> String {
        match (self.min, self.max) {
            (None, None) => "any integer".to_string(),
            (Some(min), None) => {
                let op = if self.min_open { ">" } else { ">=" };
                format!("x{}{}", op, min)
            }
            (None, Some(max)) => {
                let op = if self.max_open { "<" } else { "<=" };
                format!("x{}{}", op, max)
            }
            (Some(min), Some(max)) => {
                let lop = if self.min_open { "<" } else { "<=" };
                let rop = if self.max_open { "<" } else { "<=" };
                format!("{}{lop}x{rop}{}", min, max)
            }
        }
    }

    /// Clamp a value to the range bounds.
    fn clamp_value(&self, value: i64) -> i64 {
        let mut result = value;
        if let Some(min) = self.min {
            // Use saturating_add to avoid overflow when min == i64::MAX and min_open
            let effective_min = if self.min_open {
                min.saturating_add(1)
            } else {
                min
            };
            if result < effective_min {
                result = effective_min;
            }
        }
        if let Some(max) = self.max {
            // Use saturating_sub to avoid underflow when max == i64::MIN and max_open
            let effective_max = if self.max_open {
                max.saturating_sub(1)
            } else {
                max
            };
            if result > effective_max {
                result = effective_max;
            }
        }
        result
    }
}

impl TypeConverter for IntRange {
    type Value = i64;

    fn name(&self) -> &str {
        "INTEGER RANGE"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        let parsed: i64 = value
            .trim()
            .parse()
            .map_err(|_| format!("'{}' is not a valid integer range.", value))?;

        // Check if value is below minimum
        let lt_min = self.min.is_some_and(|min| {
            if self.min_open {
                parsed <= min
            } else {
                parsed < min
            }
        });

        // Check if value is above maximum
        let gt_max = self.max.is_some_and(|max| {
            if self.max_open {
                parsed >= max
            } else {
                parsed > max
            }
        });

        if self.clamp && (lt_min || gt_max) {
            return Ok(self.clamp_value(parsed));
        }

        if lt_min || gt_max {
            return Err(format!(
                "{} is not in the range {}.",
                parsed,
                self.describe_range()
            ));
        }

        Ok(parsed)
    }

    fn get_metavar(&self) -> Option<String> {
        Some(format!("INTEGER RANGE {}", self.describe_range()))
    }
}

// =============================================================================
// FloatRange Type
// =============================================================================

/// A floating-point type restricted to a range of values.
///
/// If `min` or `max` are `None`, the range is unbounded in that direction.
/// If `clamp` is true, out-of-range values are clamped to the boundary
/// instead of producing an error. Note: clamping is not supported with
/// open bounds.
#[derive(Debug, Clone, Copy)]
pub struct FloatRange {
    /// Minimum allowed value (inclusive unless `min_open` is true).
    pub min: Option<f64>,
    /// Maximum allowed value (inclusive unless `max_open` is true).
    pub max: Option<f64>,
    /// If true, the minimum bound is exclusive (value must be > min).
    pub min_open: bool,
    /// If true, the maximum bound is exclusive (value must be < max).
    pub max_open: bool,
    /// If true, clamp out-of-range values instead of failing.
    /// Not supported with open bounds.
    pub clamp: bool,
}

impl Default for FloatRange {
    fn default() -> Self {
        Self::new()
    }
}

impl FloatRange {
    /// Create a new unbounded float range.
    pub const fn new() -> Self {
        Self {
            min: None,
            max: None,
            min_open: false,
            max_open: false,
            clamp: false,
        }
    }

    /// Set the minimum value (inclusive).
    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set the maximum value (inclusive).
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set both minimum and maximum values.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Make the minimum bound exclusive (value must be > min).
    ///
    /// # Panics
    /// Panics if clamping is enabled, as clamping is not supported for open bounds.
    pub fn min_open(mut self, open: bool) -> Self {
        if open && self.clamp {
            panic!("Clamping is not supported for open bounds");
        }
        self.min_open = open;
        self
    }

    /// Make the maximum bound exclusive (value must be < max).
    ///
    /// # Panics
    /// Panics if clamping is enabled, as clamping is not supported for open bounds.
    pub fn max_open(mut self, open: bool) -> Self {
        if open && self.clamp {
            panic!("Clamping is not supported for open bounds");
        }
        self.max_open = open;
        self
    }

    /// Enable clamping of out-of-range values.
    ///
    /// # Panics
    /// Panics if either bound is open, as clamping is not supported for open bounds.
    pub fn clamp(mut self, clamp: bool) -> Self {
        if clamp && (self.min_open || self.max_open) {
            panic!("Clamping is not supported for open bounds");
        }
        self.clamp = clamp;
        self
    }

    /// Describe the range for error messages.
    fn describe_range(&self) -> String {
        match (self.min, self.max) {
            (None, None) => "any float".to_string(),
            (Some(min), None) => {
                let op = if self.min_open { ">" } else { ">=" };
                format!("x{}{}", op, format_float(min))
            }
            (None, Some(max)) => {
                let op = if self.max_open { "<" } else { "<=" };
                format!("x{}{}", op, format_float(max))
            }
            (Some(min), Some(max)) => {
                let lop = if self.min_open { "<" } else { "<=" };
                let rop = if self.max_open { "<" } else { "<=" };
                format!("{}{lop}x{rop}{}", format_float(min), format_float(max))
            }
        }
    }
}

impl TypeConverter for FloatRange {
    type Value = f64;

    fn name(&self) -> &str {
        "FLOAT RANGE"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        let parsed: f64 = value
            .trim()
            .parse()
            .map_err(|_| format!("'{}' is not a valid float range.", value))?;

        // Check if value is below minimum
        let lt_min = self.min.is_some_and(|min| {
            if self.min_open {
                parsed <= min
            } else {
                parsed < min
            }
        });

        // Check if value is above maximum
        let gt_max = self.max.is_some_and(|max| {
            if self.max_open {
                parsed >= max
            } else {
                parsed > max
            }
        });

        if self.clamp {
            if lt_min {
                return Ok(self.min.unwrap());
            }
            if gt_max {
                return Ok(self.max.unwrap());
            }
        }

        if lt_min || gt_max {
            return Err(format!(
                "{} is not in the range {}.",
                format_float(parsed),
                self.describe_range()
            ));
        }

        Ok(parsed)
    }

    fn get_metavar(&self) -> Option<String> {
        Some(format!("FLOAT RANGE {}", self.describe_range()))
    }
}

// =============================================================================
// DateTime Type
// =============================================================================

/// A datetime parameter type that parses ISO 8601 format strings.
///
/// By default, attempts to parse using these formats (in order):
/// - `%Y-%m-%d` (date only)
/// - `%Y-%m-%dT%H:%M:%S` (datetime with T separator)
/// - `%Y-%m-%d %H:%M:%S` (datetime with space separator)
///
/// Custom formats can be specified using the `formats` field.
#[derive(Debug, Clone)]
pub struct DateTimeType {
    /// The datetime formats to try, in order.
    pub formats: Vec<String>,
}

impl Default for DateTimeType {
    fn default() -> Self {
        Self::new()
    }
}

impl DateTimeType {
    /// Default datetime formats.
    pub const DEFAULT_FORMATS: &'static [&'static str] = &[
        "%Y-%m-%d",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%dT%H:%M:%S%.f%z",
    ];

    /// Create a new datetime type with default formats.
    pub fn new() -> Self {
        Self {
            formats: Self::DEFAULT_FORMATS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    /// Create a datetime type with custom formats.
    pub fn with_formats(formats: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            formats: formats.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl TypeConverter for DateTimeType {
    type Value = NaiveDateTime;

    fn name(&self) -> &str {
        "DATETIME"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        let value = value.trim();

        // Try each format in order
        for format in &self.formats {
            // Try parsing as NaiveDateTime first
            if let Ok(dt) = NaiveDateTime::parse_from_str(value, format) {
                return Ok(dt);
            }
            // Try parsing as NaiveDate (date only formats)
            if let Ok(date) = NaiveDate::parse_from_str(value, format) {
                return Ok(date.and_hms_opt(0, 0, 0).unwrap());
            }
            // Try parsing as DateTime<Utc> (for timezone-aware formats)
            if let Ok(dt) = ChronoDateTime::parse_from_str(value, format) {
                return Ok(dt.with_timezone(&Utc).naive_utc());
            }
        }

        Err(format!(
            "'{}' does not match the formats: {}",
            value,
            self.formats.join(", ")
        ))
    }

    fn get_metavar(&self) -> Option<String> {
        Some(format!("[{}]", self.formats.join("|")))
    }
}

// =============================================================================
// Choice Type
// =============================================================================

/// A parameter type that restricts values to a fixed set of choices.
///
/// By default, matching is case-sensitive. Set `case_sensitive` to false
/// to enable case-insensitive matching.
#[derive(Debug, Clone)]
pub struct Choice {
    /// The valid choices.
    pub choices: Vec<String>,
    /// Whether matching is case-sensitive.
    pub case_sensitive: bool,
}

impl Choice {
    /// Create a new choice type with the given choices.
    pub fn new<I, S>(choices: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            choices: choices.into_iter().map(|s| s.into()).collect(),
            case_sensitive: true,
        }
    }

    /// Set whether matching is case-sensitive.
    pub fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    /// Normalize a value for comparison.
    fn normalize(&self, value: &str) -> String {
        if self.case_sensitive {
            value.to_string()
        } else {
            value.to_lowercase()
        }
    }
}

impl TypeConverter for Choice {
    type Value = String;

    fn name(&self) -> &str {
        "CHOICE"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        let normalized_value = self.normalize(value);

        for choice in &self.choices {
            if self.normalize(choice) == normalized_value {
                // Return the original choice, not the input value
                return Ok(choice.clone());
            }
        }

        if self.choices.len() == 1 {
            Err(format!("'{}' is not '{}'.", value, self.choices[0]))
        } else {
            let choices_str = self.choices.iter()
                .map(|c| format!("'{}'", c))
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!("'{}' is not one of {}.", value, choices_str))
        }
    }

    fn get_metavar(&self) -> Option<String> {
        let choices_str = self.choices.join("|");
        Some(format!("[{}]", choices_str))
    }

    fn get_missing_message(&self) -> Option<String> {
        Some(format!("Choose from:\n\t{}", self.choices.join(",\n\t")))
    }

    fn shell_complete(&self, incomplete: &str) -> Vec<CompletionItem> {
        let normalized_incomplete = self.normalize(incomplete);
        self.choices
            .iter()
            .filter(|choice| {
                let normalized_choice = self.normalize(choice);
                normalized_choice.starts_with(&normalized_incomplete)
            })
            .map(|choice| CompletionItem::new(choice.clone()))
            .collect()
    }
}

// =============================================================================
// Path Type
// =============================================================================

/// A parameter type for file system paths with optional validation.
///
/// Unlike the `File` type, this returns the path as a `PathBuf` rather
/// than opening the file.
#[derive(Debug, Clone)]
pub struct PathType {
    /// Whether the path must exist.
    pub exists: bool,
    /// Whether files are allowed.
    pub file_okay: bool,
    /// Whether directories are allowed.
    pub dir_okay: bool,
    /// Whether the path must be readable.
    pub readable: bool,
    /// Whether the path must be writable.
    pub writable: bool,
    /// Whether the path must be executable.
    pub executable: bool,
    /// Whether to resolve the path to an absolute path.
    pub resolve_path: bool,
    /// Whether to allow "-" to indicate stdin/stdout.
    pub allow_dash: bool,
}

impl Default for PathType {
    fn default() -> Self {
        Self::new()
    }
}

impl PathType {
    /// Create a new path type with default settings.
    pub const fn new() -> Self {
        Self {
            exists: false,
            file_okay: true,
            dir_okay: true,
            readable: true,
            writable: false,
            executable: false,
            resolve_path: false,
            allow_dash: false,
        }
    }

    /// Require the path to exist.
    pub const fn exists(mut self, exists: bool) -> Self {
        self.exists = exists;
        self
    }

    /// Allow only files (not directories).
    pub const fn file_okay(mut self, file_okay: bool) -> Self {
        self.file_okay = file_okay;
        self
    }

    /// Allow only directories (not files).
    pub const fn dir_okay(mut self, dir_okay: bool) -> Self {
        self.dir_okay = dir_okay;
        self
    }

    /// Require the path to be readable.
    pub const fn readable(mut self, readable: bool) -> Self {
        self.readable = readable;
        self
    }

    /// Require the path to be writable.
    pub const fn writable(mut self, writable: bool) -> Self {
        self.writable = writable;
        self
    }

    /// Require the path to be executable.
    pub const fn executable(mut self, executable: bool) -> Self {
        self.executable = executable;
        self
    }

    /// Resolve the path to an absolute path.
    pub const fn resolve_path(mut self, resolve_path: bool) -> Self {
        self.resolve_path = resolve_path;
        self
    }

    /// Allow "-" to indicate stdin/stdout.
    pub const fn allow_dash(mut self, allow_dash: bool) -> Self {
        self.allow_dash = allow_dash;
        self
    }

    /// Get the type name based on configuration.
    fn type_name(&self) -> &str {
        if self.file_okay && !self.dir_okay {
            "File"
        } else if self.dir_okay && !self.file_okay {
            "Directory"
        } else {
            "Path"
        }
    }
}

impl TypeConverter for PathType {
    type Value = PathBuf;

    fn name(&self) -> &str {
        if self.file_okay && !self.dir_okay {
            "FILE"
        } else if self.dir_okay && !self.file_okay {
            "DIRECTORY"
        } else {
            "PATH"
        }
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        // If neither files nor directories are allowed, reject everything
        if !self.file_okay && !self.dir_okay {
            return Err("No path is valid (file_okay=false and dir_okay=false)".to_string());
        }

        // Handle dash for stdin/stdout
        if value == "-" {
            if self.file_okay && self.allow_dash {
                return Ok(PathBuf::from("-"));
            }
            return Err("'-' is not allowed".to_string());
        }

        let path = if self.resolve_path {
            match std::fs::canonicalize(value) {
                Ok(p) => p,
                Err(_) if !self.exists => {
                    // Path doesn't exist, but resolve_path still means make absolute
                    // Use current dir + relative path
                    std::env::current_dir()
                        .map(|cwd| cwd.join(value))
                        .unwrap_or_else(|_| PathBuf::from(value))
                }
                Err(_) => {
                    return Err(format!(
                        "{} '{}' does not exist",
                        self.type_name(),
                        value
                    ))
                }
            }
        } else {
            PathBuf::from(value)
        };

        // Check existence
        if self.exists && !path.exists() {
            return Err(format!(
                "{} '{}' does not exist",
                self.type_name(),
                value
            ));
        }

        // Only perform these checks if the path exists
        if path.exists() {
            let metadata = std::fs::metadata(&path)
                .map_err(|e| format!("Cannot access '{}': {}", value, e))?;

            // Check file/directory constraints
            if !self.file_okay && metadata.is_file() {
                return Err(format!("{} '{}' is a file", self.type_name(), value));
            }
            if !self.dir_okay && metadata.is_dir() {
                return Err(format!(
                    "{} '{}' is a directory",
                    self.type_name(),
                    value
                ));
            }

            // Check permissions (Unix-specific checks, simplified for cross-platform)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = metadata.permissions();
                let mode = perms.mode();

                if self.readable && (mode & 0o444) == 0 {
                    return Err(format!(
                        "{} '{}' is not readable",
                        self.type_name(),
                        value
                    ));
                }
                if self.writable && (mode & 0o222) == 0 {
                    return Err(format!(
                        "{} '{}' is not writable",
                        self.type_name(),
                        value
                    ));
                }
                if self.executable && (mode & 0o111) == 0 {
                    return Err(format!(
                        "{} '{}' is not executable",
                        self.type_name(),
                        value
                    ));
                }
            }
        }

        Ok(path)
    }

    fn split_envvar_value(&self, value: &str) -> Vec<String> {
        // Use OS-specific path list separator (: on Unix, ; on Windows)
        std::env::split_paths(value)
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    fn shell_complete(&self, incomplete: &str) -> Vec<CompletionItem> {
        let completion_type = if self.dir_okay && !self.file_okay {
            "dir"
        } else {
            "file"
        };
        vec![CompletionItem::with_type(incomplete, completion_type)]
    }
}

// =============================================================================
// File Type
// =============================================================================

/// The mode for opening a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileMode {
    /// Open for reading (file must exist).
    #[default]
    Read,
    /// Open for writing (creates or truncates).
    Write,
    /// Open for appending (creates if doesn't exist).
    Append,
    /// Open for reading and writing.
    ReadWrite,
}

impl FileMode {
    /// Parse a mode string (like Python's open modes).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "r" | "rb" => Some(FileMode::Read),
            "w" | "wb" => Some(FileMode::Write),
            "a" | "ab" => Some(FileMode::Append),
            "r+" | "rb+" | "r+b" => Some(FileMode::ReadWrite),
            "w+" | "wb+" | "w+b" => Some(FileMode::ReadWrite),
            "a+" | "ab+" | "a+b" => Some(FileMode::ReadWrite), // read+append
            _ => None,
        }
    }

    /// Returns true if this mode is for reading.
    pub fn is_read(&self) -> bool {
        matches!(self, FileMode::Read | FileMode::ReadWrite)
    }

    /// Returns true if this mode is for writing.
    pub fn is_write(&self) -> bool {
        matches!(self, FileMode::Write | FileMode::Append | FileMode::ReadWrite)
    }
}

/// The source of a file handle (regular file or stdio).
#[derive(Debug)]
enum FileSource {
    /// A regular file on disk.
    File(StdFile),
    /// Standard input.
    Stdin,
    /// Standard output.
    Stdout,
}

/// Counter for generating unique temp file names.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A wrapper around a file that supports lazy opening and stdin/stdout.
///
/// When `atomic` is enabled for write operations, writes go to a temporary file
/// in the same directory. The temp file is renamed to the final path when
/// `close()` is called or the `LazyFile` is dropped.
#[derive(Debug)]
pub struct LazyFile {
    path: PathBuf,
    mode: FileMode,
    source: Option<FileSource>,
    is_stdio: bool,
    atomic: bool,
    /// Path to the temporary file when using atomic writes.
    temp_path: Option<PathBuf>,
}

impl LazyFile {
    /// Create a new lazy file.
    pub fn new(path: PathBuf, mode: FileMode) -> Self {
        let is_stdio = path.as_os_str() == "-";
        Self {
            path,
            mode,
            source: None,
            is_stdio,
            atomic: false,
            temp_path: None,
        }
    }

    /// Create a lazy file for stdin.
    pub fn stdin() -> Self {
        Self {
            path: PathBuf::from("-"),
            mode: FileMode::Read,
            source: None,
            is_stdio: true,
            atomic: false,
            temp_path: None,
        }
    }

    /// Create a lazy file for stdout.
    pub fn stdout() -> Self {
        Self {
            path: PathBuf::from("-"),
            mode: FileMode::Write,
            source: None,
            is_stdio: true,
            atomic: false,
            temp_path: None,
        }
    }

    /// Set whether to use atomic writes.
    pub fn atomic(mut self, atomic: bool) -> Self {
        self.atomic = atomic;
        self
    }

    /// Get the path to this file.
    pub fn path(&self) -> &StdPath {
        &self.path
    }

    /// Returns true if this is stdin or stdout.
    pub fn is_stdio(&self) -> bool {
        self.is_stdio
    }

    /// Open the file (lazily).
    fn open(&mut self) -> io::Result<()> {
        if self.source.is_some() {
            return Ok(());
        }

        let source = if self.is_stdio {
            if self.mode.is_read() {
                FileSource::Stdin
            } else {
                FileSource::Stdout
            }
        } else if self.atomic && self.mode == FileMode::Write {
            // For atomic writes, create a temp file in the same directory
            let parent = self.path.parent().unwrap_or(StdPath::new("."));
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
            let temp_name = format!(
                ".{}.tmp.{}",
                self.path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default(),
                counter
            );
            let temp_path = parent.join(&temp_name);
            let file = StdFile::create(&temp_path)?;
            self.temp_path = Some(temp_path);
            FileSource::File(file)
        } else {
            let file = match self.mode {
                FileMode::Read => StdFile::open(&self.path),
                FileMode::Write => StdFile::create(&self.path),
                FileMode::Append => OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&self.path),
                FileMode::ReadWrite => OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&self.path),
            }?;
            FileSource::File(file)
        };
        self.source = Some(source);
        Ok(())
    }

    /// Close the file and finalize atomic writes.
    ///
    /// For atomic writes, this renames the temp file to the final path.
    /// Returns an error if the rename fails.
    ///
    /// Note: This is called automatically on drop, but errors during drop
    /// are silently ignored. Call this explicitly if you need to handle errors.
    pub fn close(&mut self) -> io::Result<()> {
        // Flush and drop the file handle first
        if let Some(FileSource::File(mut f)) = self.source.take() {
            f.flush()?;
            // File is dropped here, releasing the handle
        }

        // Now rename the temp file to the final path
        if let Some(temp_path) = self.temp_path.take() {
            fs::rename(&temp_path, &self.path)?;
        }

        Ok(())
    }
}

impl Drop for LazyFile {
    fn drop(&mut self) {
        // Best-effort close on drop; errors are silently ignored
        let _ = self.close();
    }
}

impl Read for LazyFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.open()?;
        match self.source.as_mut() {
            Some(FileSource::File(f)) => f.read(buf),
            Some(FileSource::Stdin) => io::stdin().read(buf),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Cannot read from write-only file",
            )),
        }
    }
}

impl Write for LazyFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.open()?;
        match self.source.as_mut() {
            Some(FileSource::File(f)) => f.write(buf),
            Some(FileSource::Stdout) => io::stdout().write(buf),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Cannot write to read-only file",
            )),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.source.as_mut() {
            Some(FileSource::File(f)) => f.flush(),
            Some(FileSource::Stdout) => io::stdout().flush(),
            _ => Ok(()),
        }
    }
}

/// A parameter type for files.
///
/// Unlike `PathType`, this opens the file and returns a handle.
/// The special value "-" indicates stdin (for reading) or stdout (for writing).
#[derive(Debug, Clone)]
pub struct FileType {
    /// The mode to open the file in.
    pub mode: FileMode,
    /// Whether to open the file lazily.
    pub lazy: Option<bool>,
    /// Whether to use atomic writes.
    pub atomic: bool,
}

impl Default for FileType {
    fn default() -> Self {
        Self::new()
    }
}

impl FileType {
    /// Create a new file type for reading.
    pub const fn new() -> Self {
        Self {
            mode: FileMode::Read,
            lazy: None,
            atomic: false,
        }
    }

    /// Set the file mode.
    pub const fn mode(mut self, mode: FileMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set whether to open lazily.
    pub const fn lazy(mut self, lazy: bool) -> Self {
        self.lazy = Some(lazy);
        self
    }

    /// Set whether to use atomic writes.
    pub const fn atomic(mut self, atomic: bool) -> Self {
        self.atomic = atomic;
        self
    }

    /// Determine if the file should be opened lazily.
    fn resolve_lazy(&self, path: &str) -> bool {
        if let Some(lazy) = self.lazy {
            return lazy;
        }
        // Default: non-lazy for stdin/stdout and reading, lazy for writing
        if path == "-" {
            return false;
        }
        matches!(self.mode, FileMode::Write | FileMode::Append)
    }
}

impl TypeConverter for FileType {
    type Value = LazyFile;

    fn name(&self) -> &str {
        "FILENAME"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        // Handle stdin/stdout via "-"
        if value == "-" {
            // ReadWrite mode is not supported for stdin/stdout
            if matches!(self.mode, FileMode::ReadWrite) {
                return Err("'-' (stdin/stdout) cannot be used with read+write mode".to_string());
            }
            let lazy_file = if self.mode.is_read() {
                LazyFile::stdin()
            } else {
                LazyFile::stdout()
            };
            return Ok(lazy_file);
        }

        let path = PathBuf::from(value);

        // Validate for read mode (file must exist)
        if self.mode.is_read() && !self.resolve_lazy(value) && !path.exists() {
            return Err(format!("'{}': No such file or directory", value));
        }

        let mut lazy_file = LazyFile::new(path, self.mode);
        if self.atomic {
            lazy_file = lazy_file.atomic(true);
        }

        // If not lazy, open immediately to catch errors
        if !self.resolve_lazy(value) {
            lazy_file
                .open()
                .map_err(|e| format!("'{}': {}", value, e))?;
        }

        Ok(lazy_file)
    }

    fn split_envvar_value(&self, value: &str) -> Vec<String> {
        // Use OS-specific path list separator (: on Unix, ; on Windows)
        std::env::split_paths(value)
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    fn shell_complete(&self, incomplete: &str) -> Vec<CompletionItem> {
        vec![CompletionItem::with_type(incomplete, "file")]
    }
}

// =============================================================================
// Tuple Type
// =============================================================================

/// A boxed parameter type for runtime polymorphism.
pub type BoxedTypeConverter<T> = Box<dyn TypeConverter<Value = T> + Send + Sync>;

/// A composite parameter type that collects multiple values with different types.
///
/// Unlike regular types with `nargs`, each position in a tuple can have
/// a different type.
#[derive(Debug)]
pub struct TupleType {
    /// The types for each position in the tuple.
    types: Vec<TupleElementType>,
}

/// An element type within a tuple (erased to String for simplicity).
#[derive(Debug, Clone)]
enum TupleElementType {
    String,
    Int,
    Float,
    Bool,
}

impl TupleType {
    /// Create a new tuple type from a list of type specifiers.
    ///
    /// Each specifier should be one of: "string", "int", "float", "bool"
    pub fn new<I, S>(types: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let types = types
            .into_iter()
            .map(|s| match s.as_ref().to_lowercase().as_str() {
                "string" | "str" | "text" => TupleElementType::String,
                "int" | "integer" => TupleElementType::Int,
                "float" => TupleElementType::Float,
                "bool" | "boolean" => TupleElementType::Bool,
                _ => TupleElementType::String,
            })
            .collect();
        Self { types }
    }

    /// Create a tuple of strings.
    pub fn strings(count: usize) -> Self {
        Self {
            types: vec![TupleElementType::String; count],
        }
    }

    /// Create a tuple of integers.
    pub fn ints(count: usize) -> Self {
        Self {
            types: vec![TupleElementType::Int; count],
        }
    }
}

/// A converted tuple value with dynamic types.
#[derive(Debug, Clone, PartialEq)]
pub enum TupleValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl TupleValue {
    /// Get as string, if this is a string value.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            TupleValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as integer, if this is an integer value.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            TupleValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as float, if this is a float value.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            TupleValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Get as bool, if this is a bool value.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            TupleValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl TupleType {
    /// Convert a single element at the given index.
    ///
    /// This is the preferred way to convert tuple elements - the parser
    /// should call this once for each argument consumed.
    pub fn convert_element(&self, index: usize, value: &str) -> Result<TupleValue, String> {
        let element_type = self.types.get(index).ok_or_else(|| {
            format!("tuple index {} out of bounds (arity {})", index, self.types.len())
        })?;

        match element_type {
            TupleElementType::String => Ok(TupleValue::String(value.to_string())),
            TupleElementType::Int => {
                let i: i64 = value
                    .parse()
                    .map_err(|_| format!("'{}' is not a valid integer", value))?;
                Ok(TupleValue::Int(i))
            }
            TupleElementType::Float => {
                let f: f64 = value
                    .parse()
                    .map_err(|_| format!("'{}' is not a valid float", value))?;
                Ok(TupleValue::Float(f))
            }
            TupleElementType::Bool => {
                let b = BoolType::str_to_bool(value)
                    .ok_or_else(|| format!("'{}' is not a valid boolean", value))?;
                Ok(TupleValue::Bool(b))
            }
        }
    }

    /// Convert a slice of pre-split values into a tuple.
    ///
    /// This is the standard Click-compatible conversion - the parser provides
    /// already-split argument values.
    pub fn convert_values(&self, values: &[&str]) -> Result<Vec<TupleValue>, String> {
        if values.len() != self.types.len() {
            return Err(format!(
                "{} values are required, but {} were given",
                self.types.len(),
                values.len()
            ));
        }

        values
            .iter()
            .enumerate()
            .map(|(i, v)| self.convert_element(i, v))
            .collect()
    }
}

impl TypeConverter for TupleType {
    type Value = Vec<TupleValue>;

    fn name(&self) -> &str {
        "TUPLE"
    }

    fn convert(&self, value: &str) -> Result<Self::Value, String> {
        // When called with a single value, this is typically for a single-element
        // tuple or when the entire tuple is provided as one string (e.g., from envvar).
        // For envvar compatibility, split on whitespace. For proper CLI parsing,
        // use convert_values() with pre-split arguments.
        let parts: Vec<&str> = value.split_whitespace().collect();
        self.convert_values(&parts)
    }

    fn get_metavar(&self) -> Option<String> {
        let names: Vec<&str> = self
            .types
            .iter()
            .map(|t| match t {
                TupleElementType::String => "TEXT",
                TupleElementType::Int => "INTEGER",
                TupleElementType::Float => "FLOAT",
                TupleElementType::Bool => "BOOLEAN",
            })
            .collect();
        Some(format!("<{}>", names.join(" ")))
    }

    fn is_composite(&self) -> bool {
        true
    }

    fn arity(&self) -> usize {
        self.types.len()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_type() {
        assert_eq!(STRING.convert("hello").unwrap(), "hello");
        assert_eq!(STRING.convert("  spaces  ").unwrap(), "  spaces  ");
        assert_eq!(STRING.name(), "TEXT");
    }

    #[test]
    fn test_int_type() {
        assert_eq!(INT.convert("42").unwrap(), 42);
        assert_eq!(INT.convert("-123").unwrap(), -123);
        assert_eq!(INT.convert("  456  ").unwrap(), 456);
        assert!(INT.convert("not a number").is_err());
        assert!(INT.convert("3.14").is_err());
    }

    #[test]
    fn test_float_type() {
        assert_eq!(FLOAT.convert("3.14").unwrap(), 3.14);
        assert_eq!(FLOAT.convert("-2.5").unwrap(), -2.5);
        assert_eq!(FLOAT.convert("42").unwrap(), 42.0);
        assert!(FLOAT.convert("not a number").is_err());
    }

    #[test]
    fn test_bool_type() {
        assert!(BOOL.convert("true").unwrap());
        assert!(BOOL.convert("True").unwrap());
        assert!(BOOL.convert("TRUE").unwrap());
        assert!(BOOL.convert("yes").unwrap());
        assert!(BOOL.convert("1").unwrap());
        assert!(BOOL.convert("on").unwrap());
        assert!(!BOOL.convert("false").unwrap());
        assert!(!BOOL.convert("no").unwrap());
        assert!(!BOOL.convert("0").unwrap());
        assert!(!BOOL.convert("off").unwrap());
        assert!(BOOL.convert("maybe").is_err());
    }

    #[test]
    fn test_uuid_type() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let result = UUID.convert(uuid_str).unwrap();
        assert_eq!(result.to_string(), uuid_str);
        assert!(UUID.convert("not-a-uuid").is_err());
    }

    #[test]
    fn test_int_range() {
        let range = IntRange::new().range(0, 100);
        assert_eq!(range.convert("50").unwrap(), 50);
        assert_eq!(range.convert("0").unwrap(), 0);
        assert_eq!(range.convert("100").unwrap(), 100);
        assert!(range.convert("-1").is_err());
        assert!(range.convert("101").is_err());
    }

    #[test]
    fn test_int_range_open() {
        let range = IntRange::new().min(0).max(10).min_open(true).max_open(true);
        assert!(range.convert("0").is_err()); // min is open
        assert!(range.convert("10").is_err()); // max is open
        assert_eq!(range.convert("1").unwrap(), 1);
        assert_eq!(range.convert("9").unwrap(), 9);
    }

    #[test]
    fn test_int_range_clamp() {
        let range = IntRange::new().range(0, 100).clamp(true);
        assert_eq!(range.convert("-50").unwrap(), 0);
        assert_eq!(range.convert("150").unwrap(), 100);
        assert_eq!(range.convert("50").unwrap(), 50);
    }

    #[test]
    fn test_float_range() {
        let range = FloatRange::new().range(0.0, 1.0);
        assert_eq!(range.convert("0.5").unwrap(), 0.5);
        assert_eq!(range.convert("0.0").unwrap(), 0.0);
        assert_eq!(range.convert("1.0").unwrap(), 1.0);
        assert!(range.convert("-0.1").is_err());
        assert!(range.convert("1.1").is_err());
    }

    #[test]
    fn test_datetime_type() {
        let dt = DateTimeType::new();

        // Date only
        let result = dt.convert("2024-01-15").unwrap();
        assert_eq!(result.date().to_string(), "2024-01-15");

        // Datetime with T
        let result = dt.convert("2024-01-15T10:30:00").unwrap();
        assert_eq!(result.to_string(), "2024-01-15 10:30:00");

        // Datetime with space
        let result = dt.convert("2024-01-15 10:30:00").unwrap();
        assert_eq!(result.to_string(), "2024-01-15 10:30:00");

        assert!(dt.convert("not a date").is_err());
    }

    #[test]
    fn test_choice() {
        let choice = Choice::new(["one", "two", "three"]);
        assert_eq!(choice.convert("one").unwrap(), "one");
        assert_eq!(choice.convert("two").unwrap(), "two");
        assert!(choice.convert("four").is_err());
    }

    #[test]
    fn test_choice_case_insensitive() {
        let choice = Choice::new(["One", "Two", "Three"]).case_sensitive(false);
        assert_eq!(choice.convert("one").unwrap(), "One");
        assert_eq!(choice.convert("ONE").unwrap(), "One");
        assert_eq!(choice.convert("oNe").unwrap(), "One");
    }

    #[test]
    fn test_choice_shell_complete() {
        let choice = Choice::new(["apple", "apricot", "banana"]);
        let completions = choice.shell_complete("ap");
        assert_eq!(completions.len(), 2);
        assert!(completions.iter().any(|c| c.value == "apple"));
        assert!(completions.iter().any(|c| c.value == "apricot"));
    }

    #[test]
    fn test_path_type() {
        let path = PathType::new();
        // Basic path conversion (doesn't require existence by default)
        let result = path.convert("/some/path").unwrap();
        assert_eq!(result, PathBuf::from("/some/path"));
    }

    #[test]
    fn test_tuple_type() {
        let tuple = TupleType::new(["string", "int", "bool"]);
        let result = tuple.convert("hello 42 true").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].as_string(), Some("hello"));
        assert_eq!(result[1].as_int(), Some(42));
        assert_eq!(result[2].as_bool(), Some(true));
    }

    #[test]
    fn test_tuple_type_wrong_count() {
        let tuple = TupleType::new(["string", "int"]);
        assert!(tuple.convert("hello").is_err());
        assert!(tuple.convert("hello 42 extra").is_err());
    }

    #[test]
    fn test_unprocessed() {
        assert_eq!(UNPROCESSED.convert("raw value").unwrap(), "raw value");
        assert_eq!(UNPROCESSED.name(), "TEXT");
    }

    #[test]
    fn test_completion_item() {
        let item = CompletionItem::new("test");
        assert_eq!(item.value, "test");
        assert_eq!(item.completion_type, "plain");
        assert!(item.help.is_none());

        let item = CompletionItem::with_type("path", "file").with_help("A file path");
        assert_eq!(item.value, "path");
        assert_eq!(item.completion_type, "file");
        assert_eq!(item.help, Some("A file path".to_string()));
    }
}
