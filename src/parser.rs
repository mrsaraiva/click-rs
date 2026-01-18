//! Low-level argument parsing for click-rs.
//!
//! This module provides the internal option parser that handles the actual
//! parsing of command-line arguments. It is modeled after Python Click's
//! `parser.py` and brings a similar but simplified API.
//!
//! # Reference
//!
//! Based on Python Click's `parser.py`.
//!
//! # Key Behaviors
//!
//! - `--` ends option parsing; remaining args are positional
//! - Short options can be grouped: `-abc` = `-a -b -c`
//! - Short option with value: `-nfoo` or `-n foo`
//! - Long option with value: `--name=foo` or `--name foo`

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::ClickError;

// =============================================================================
// Type Aliases
// =============================================================================

/// Type alias for token normalization function.
type TokenNormalizeFunc = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Result type for parsing: (options, remaining args, parameter order).
pub type ParseResult = Result<(HashMap<String, ParsedValue>, Vec<String>, Vec<String>), ClickError>;

/// Special nargs value indicating optional (? in Python Click).
/// This is used internally to distinguish optional from 0.
pub const NARGS_OPTIONAL: i32 = -2;

// =============================================================================
// OptionAction Enum
// =============================================================================

/// Action to perform when an option is encountered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionAction {
    /// Store a single value (default).
    #[default]
    Store,
    /// Store a constant value (for flags).
    StoreConst,
    /// Append value to a list.
    Append,
    /// Append a constant to a list.
    AppendConst,
    /// Count occurrences.
    Count,
}

// =============================================================================
// ParsedValue Enum
// =============================================================================

/// A parsed value from the command line.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedValue {
    /// A single string value.
    Single(String),
    /// Multiple string values (for append or multi-valued options).
    Multiple(Vec<String>),
    /// A count of occurrences.
    Count(usize),
    /// A boolean flag value.
    Flag(bool),
    /// Value was not set (placeholder for missing optional values).
    Unset,
    /// Option was used as a flag without providing a value.
    /// This happens when an optional-value option (like `--opt`) is used
    /// without a value and the next token looks like another option.
    /// The command layer should handle this by prompting or using a flag_value.
    FlagNeedsValue,
}

impl ParsedValue {
    /// Check if the value is unset.
    pub fn is_unset(&self) -> bool {
        matches!(self, ParsedValue::Unset)
    }

    /// Get as a single string, if applicable.
    pub fn as_single(&self) -> Option<&str> {
        match self {
            ParsedValue::Single(s) => Some(s),
            _ => None,
        }
    }

    /// Get as multiple strings, if applicable.
    pub fn as_multiple(&self) -> Option<&[String]> {
        match self {
            ParsedValue::Multiple(v) => Some(v),
            _ => None,
        }
    }

    /// Get as a count, if applicable.
    pub fn as_count(&self) -> Option<usize> {
        match self {
            ParsedValue::Count(n) => Some(*n),
            _ => None,
        }
    }

    /// Get as a flag, if applicable.
    pub fn as_flag(&self) -> Option<bool> {
        match self {
            ParsedValue::Flag(b) => Some(*b),
            _ => None,
        }
    }
}

// =============================================================================
// ParserOption Struct (Internal)
// =============================================================================

/// Internal representation of an option for the parser.
#[derive(Debug, Clone)]
struct ParserOption {
    /// The canonical name for this option.
    #[allow(dead_code)]
    name: String,
    /// Short option forms (e.g., ["-n", "-N"]).
    #[allow(dead_code)]
    short_opts: Vec<String>,
    /// Long option forms (e.g., ["--name"]).
    #[allow(dead_code)]
    long_opts: Vec<String>,
    /// Action to perform when this option is encountered.
    action: OptionAction,
    /// Number of arguments: 1 = single, -1 = variadic, 0 = flag, -2 = optional.
    nargs: i32,
    /// Constant value for StoreConst/AppendConst actions.
    const_value: Option<String>,
    /// Destination key in the parsed results.
    dest: String,
    /// Whether the option allows omitting the value (for optional options).
    /// When true and no value is provided (next arg looks like an option),
    /// the option uses a sentinel value instead of consuming the next arg.
    flag_needs_value: bool,
}

impl ParserOption {
    /// Check if this option takes a value.
    fn takes_value(&self) -> bool {
        matches!(self.action, OptionAction::Store | OptionAction::Append)
    }
}

// =============================================================================
// ParserArgument Struct (Internal)
// =============================================================================

/// Internal representation of a positional argument for the parser.
#[derive(Debug, Clone)]
struct ParserArgument {
    /// The name of the argument.
    #[allow(dead_code)]
    name: String,
    /// Number of values to consume: 1 = single, -1 = variadic.
    nargs: i32,
    /// Destination key in the parsed results.
    dest: String,
}

// =============================================================================
// ParsingState Struct
// =============================================================================

/// Mutable state during parsing.
#[derive(Debug)]
struct ParsingState {
    /// Parsed option values.
    opts: HashMap<String, ParsedValue>,
    /// Left args (positional arguments accumulated so far).
    largs: Vec<String>,
    /// Right args (remaining arguments to process).
    rargs: VecDeque<String>,
    /// Order in which parameters were seen (dest names).
    order: Vec<String>,
}

impl ParsingState {
    /// Create a new parsing state from the argument list.
    fn new(args: Vec<String>) -> Self {
        Self {
            opts: HashMap::new(),
            largs: Vec::new(),
            rargs: VecDeque::from(args),
            order: Vec::new(),
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Split an option string into its prefix and name.
///
/// # Examples
///
/// ```
/// use click::parser::split_opt;
///
/// assert_eq!(split_opt("--name"), Some(("--", "name")));
/// assert_eq!(split_opt("-n"), Some(("-", "n")));
/// assert_eq!(split_opt("name"), None);
/// assert_eq!(split_opt("-"), None);
/// ```
pub fn split_opt(opt: &str) -> Option<(&str, &str)> {
    if opt.is_empty() {
        return None;
    }

    let first = opt.chars().next().unwrap();
    if first.is_alphanumeric() {
        // No prefix - not an option
        return None;
    }

    // Check for double-dash prefix
    if opt.len() >= 2 && &opt[0..2] == "--" {
        let name = &opt[2..];
        if name.is_empty() {
            return None; // Just "--" is not a valid option split
        }
        return Some(("--", name));
    }

    // Single-dash prefix
    if opt.len() >= 2 && opt.starts_with('-') {
        return Some(("-", &opt[1..]));
    }

    // Single character prefix (non-alphanumeric)
    if opt.len() >= 2 {
        let prefix_end = first.len_utf8();
        Some((&opt[..prefix_end], &opt[prefix_end..]))
    } else {
        None
    }
}

/// Compute the Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Use two rows for space efficiency
    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];

    // Initialize first row
    for (j, item) in prev.iter_mut().enumerate().take(n + 1) {
        *item = j;
    }

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1) // deletion
                .min(curr[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Find close matches to a string from a list of possibilities.
///
/// Returns options with edit distance <= 2, sorted by distance.
fn get_close_matches(word: &str, possibilities: &[&str], max_matches: usize) -> Vec<String> {
    let mut scored: Vec<(usize, &str)> = possibilities
        .iter()
        .map(|&p| (edit_distance(word, p), p))
        .filter(|(dist, _)| *dist <= 2)
        .collect();

    scored.sort_by_key(|(dist, _)| *dist);
    scored
        .into_iter()
        .take(max_matches)
        .map(|(_, s)| s.to_string())
        .collect()
}

/// Unpack arguments according to nargs specifications.
///
/// Given an iterable of arguments and nargs specs, returns (unpacked_values, remaining_args).
/// Missing items are represented as ParsedValue::Unset.
///
/// # Nargs values:
/// - `1` or `n > 1`: consume exactly that many values
/// - `-1`: variadic, consume all remaining
/// - `-2` (NARGS_OPTIONAL): optional, consume one if present
/// - `0`: skip (no values consumed)
///
/// # Errors
///
/// Returns an error if there are two variadic specs (nargs < 0 and != NARGS_OPTIONAL).
fn unpack_args(
    args: &[String],
    nargs_spec: &[i32],
) -> Result<(Vec<ParsedValue>, Vec<String>), ClickError> {
    let mut args_deque: VecDeque<String> = VecDeque::from(args.to_vec());
    let mut nargs_deque: VecDeque<i32> = VecDeque::from(nargs_spec.to_vec());
    let mut result: Vec<ParsedValue> = Vec::new();
    let mut star_pos: Option<usize> = None;

    // Helper to fetch from front or back depending on whether we've seen a star
    fn fetch(deque: &mut VecDeque<String>, from_back: bool) -> Option<String> {
        if from_back {
            deque.pop_back()
        } else {
            deque.pop_front()
        }
    }

    // Count required args remaining in the nargs_deque (positive values, excluding optional)
    fn count_required_remaining(deque: &VecDeque<i32>) -> usize {
        deque
            .iter()
            .filter(|&&n| n > 0) // Positive = required (1 or more)
            .map(|&n| n.max(1) as usize)
            .sum()
    }

    while let Some(nargs) = if star_pos.is_none() {
        nargs_deque.pop_front()
    } else {
        nargs_deque.pop_back()
    } {
        if nargs == 1 {
            // Single value
            let value = fetch(&mut args_deque, star_pos.is_some());
            result.push(match value {
                Some(v) => ParsedValue::Single(v),
                None => ParsedValue::Unset,
            });
        } else if nargs == NARGS_OPTIONAL {
            // Optional: consume one value if available AND we have enough for remaining required
            let required_remaining = count_required_remaining(&nargs_deque);
            let available = args_deque.len();

            if available > required_remaining {
                // We have more args than required, so optional can take one
                let value = fetch(&mut args_deque, star_pos.is_some());
                result.push(match value {
                    Some(v) => ParsedValue::Single(v),
                    None => ParsedValue::Unset,
                });
            } else {
                // Not enough args for required, skip optional
                result.push(ParsedValue::Unset);
            }
        } else if nargs > 1 {
            // Multiple values - must get all of them
            let mut values = Vec::new();
            let mut missing_count = 0;
            for _ in 0..nargs {
                match fetch(&mut args_deque, star_pos.is_some()) {
                    Some(v) => {
                        values.push(v);
                    }
                    None => {
                        missing_count += 1;
                    }
                }
            }
            // If we're fetching from back, reverse to restore order
            if star_pos.is_some() {
                values.reverse();
            }
            // All missing = completely unset
            if missing_count == nargs as usize {
                result.push(ParsedValue::Unset);
            } else if missing_count > 0 {
                // Partial missing - mark with special value to indicate incomplete
                // This will be caught later by process_args_for_args
                result.push(ParsedValue::Multiple(values));
            } else {
                result.push(ParsedValue::Multiple(values));
            }
        } else if nargs == -1 {
            // Variadic (star) - consumes all remaining
            if star_pos.is_some() {
                // Can't have two variadic specs
                return Err(ClickError::usage(
                    "Cannot have more than one variadic argument.".to_string()
                ));
            }
            star_pos = Some(result.len());
            result.push(ParsedValue::Unset); // Placeholder, will be filled later
        }
        // nargs == 0 means no arguments, skip
    }

    // If we had a star position, fill it with remaining args
    if let Some(pos) = star_pos {
        let remaining: Vec<String> = args_deque.drain(..).collect();
        if remaining.is_empty() {
            result[pos] = ParsedValue::Multiple(Vec::new());
        } else {
            result[pos] = ParsedValue::Multiple(remaining);
        }
        // Reverse items after star position
        let after_star: Vec<_> = result.drain(pos + 1..).rev().collect();
        result.extend(after_star);
    }

    Ok((result, args_deque.into_iter().collect()))
}

// =============================================================================
// OptionParser Struct
// =============================================================================

/// The main option parser.
///
/// This parser handles the low-level parsing of command-line arguments.
/// It supports both short and long options, with various actions like
/// store, append, and count.
///
/// # Example
///
/// ```
/// use click::parser::{OptionParser, OptionAction, ParsedValue};
///
/// let mut parser = OptionParser::new();
/// parser.add_option(&["-n", "--name"], "name", OptionAction::Store, 1, None);
/// parser.add_option(&["-v", "--verbose"], "verbose", OptionAction::Count, 0, None);
/// parser.add_argument("file", 1);
///
/// let args = vec!["-v", "-v", "--name", "test", "input.txt"]
///     .into_iter().map(String::from).collect();
/// let (opts, remaining, order) = parser.parse_args(args).unwrap();
///
/// assert_eq!(opts.get("verbose"), Some(&ParsedValue::Count(2)));
/// assert_eq!(opts.get("name"), Some(&ParsedValue::Single("test".to_string())));
/// ```
pub struct OptionParser {
    /// Whether to allow positional args between options.
    allow_interspersed_args: bool,
    /// Whether to ignore unknown options (pass them through).
    ignore_unknown_options: bool,
    /// Map of short option strings to their definitions.
    short_opt: HashMap<String, ParserOption>,
    /// Map of long option strings to their definitions.
    long_opt: HashMap<String, ParserOption>,
    /// Set of valid option prefixes.
    opt_prefixes: HashSet<String>,
    /// List of positional argument definitions.
    args: Vec<ParserArgument>,
    /// Token normalize function (for case-insensitive matching, etc.).
    token_normalize_func: Option<TokenNormalizeFunc>,
}

impl std::fmt::Debug for OptionParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptionParser")
            .field("allow_interspersed_args", &self.allow_interspersed_args)
            .field("ignore_unknown_options", &self.ignore_unknown_options)
            .field("short_opt", &self.short_opt)
            .field("long_opt", &self.long_opt)
            .field("opt_prefixes", &self.opt_prefixes)
            .field("args", &self.args)
            .field(
                "token_normalize_func",
                &self.token_normalize_func.as_ref().map(|_| "<function>"),
            )
            .finish()
    }
}

impl Default for OptionParser {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionParser {
    /// Create a new option parser with default settings.
    pub fn new() -> Self {
        let mut opt_prefixes = HashSet::new();
        opt_prefixes.insert("-".to_string());
        opt_prefixes.insert("--".to_string());

        Self {
            allow_interspersed_args: true,
            ignore_unknown_options: false,
            short_opt: HashMap::new(),
            long_opt: HashMap::new(),
            opt_prefixes,
            args: Vec::new(),
            token_normalize_func: None,
        }
    }

    /// Set whether to allow interspersed positional arguments.
    ///
    /// When true (default), positional args can appear between options.
    /// When false, the parser stops on the first non-option.
    pub fn allow_interspersed_args(mut self, allow: bool) -> Self {
        self.allow_interspersed_args = allow;
        self
    }

    /// Set whether to ignore unknown options.
    ///
    /// When true, unknown options are passed through as positional args.
    /// When false (default), unknown options cause an error.
    pub fn ignore_unknown_options(mut self, ignore: bool) -> Self {
        self.ignore_unknown_options = ignore;
        self
    }

    /// Set a token normalization function.
    ///
    /// This function is applied to option names for matching.
    /// Useful for case-insensitive option matching.
    pub fn token_normalize_func<F>(mut self, func: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.token_normalize_func = Some(Box::new(func));
        self
    }

    /// Normalize an option string using the token_normalize_func if set.
    fn normalize_opt(&self, opt: &str) -> String {
        if let Some(ref func) = self.token_normalize_func {
            if let Some((prefix, name)) = split_opt(opt) {
                format!("{}{}", prefix, func(name))
            } else {
                opt.to_string()
            }
        } else {
            opt.to_string()
        }
    }

    /// Add an option to the parser.
    ///
    /// # Arguments
    ///
    /// * `opts` - Option strings (e.g., `["-n", "--name"]`)
    /// * `dest` - Destination key in the results
    /// * `action` - Action to perform when option is encountered
    /// * `nargs` - Number of arguments (1 = single, -1 = variadic, 0 = flag, -2 = optional)
    /// * `const_value` - Constant value for StoreConst/AppendConst actions
    pub fn add_option(
        &mut self,
        opts: &[&str],
        dest: &str,
        action: OptionAction,
        nargs: i32,
        const_value: Option<&str>,
    ) {
        self.add_option_ex(opts, dest, action, nargs, const_value, false);
    }

    /// Add an option to the parser with extended options.
    ///
    /// # Arguments
    ///
    /// * `opts` - Option strings (e.g., `["-n", "--name"]`)
    /// * `dest` - Destination key in the results
    /// * `action` - Action to perform when option is encountered
    /// * `nargs` - Number of arguments (1 = single, -1 = variadic, 0 = flag, -2 = optional)
    /// * `const_value` - Constant value for StoreConst/AppendConst actions
    /// * `flag_needs_value` - If true, value is optional: if next arg looks like an option,
    ///   returns FlagNeedsValue instead of consuming it
    pub fn add_option_ex(
        &mut self,
        opts: &[&str],
        dest: &str,
        action: OptionAction,
        nargs: i32,
        const_value: Option<&str>,
        flag_needs_value: bool,
    ) {
        let opts: Vec<String> = opts.iter().map(|o| self.normalize_opt(o)).collect();

        let mut short_opts = Vec::new();
        let mut long_opts = Vec::new();

        for opt in &opts {
            if let Some((prefix, value)) = split_opt(opt) {
                // Track the prefix
                self.opt_prefixes.insert(prefix.chars().next().unwrap().to_string());
                if prefix.len() == 2 {
                    self.opt_prefixes.insert(prefix.to_string());
                }

                // Categorize as short or long
                if prefix.len() == 1 && value.len() == 1 {
                    short_opts.push(opt.clone());
                } else {
                    long_opts.push(opt.clone());
                }
            }
        }

        let name = long_opts
            .first()
            .or(short_opts.first())
            .cloned()
            .unwrap_or_else(|| dest.to_string());

        let parser_opt = ParserOption {
            name,
            short_opts: short_opts.clone(),
            long_opts: long_opts.clone(),
            action,
            nargs,
            const_value: const_value.map(String::from),
            dest: dest.to_string(),
            flag_needs_value,
        };

        // Register in lookup maps
        for opt in &short_opts {
            self.short_opt.insert(opt.clone(), parser_opt.clone());
        }
        for opt in &long_opts {
            self.long_opt.insert(opt.clone(), parser_opt.clone());
        }
    }

    /// Add a positional argument to the parser.
    ///
    /// # Arguments
    ///
    /// * `dest` - Destination key in the results
    /// * `nargs` - Number of values (1 = single, -1 = variadic)
    pub fn add_argument(&mut self, dest: &str, nargs: i32) {
        self.args.push(ParserArgument {
            name: dest.to_string(),
            nargs,
            dest: dest.to_string(),
        });
    }

    /// Parse command-line arguments.
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// - Parsed option values (dest -> value)
    /// - Remaining unparsed arguments
    /// - Order of parameters as they were seen
    ///
    /// # Errors
    ///
    /// Returns a `ClickError` if parsing fails (e.g., unknown option,
    /// missing value).
    pub fn parse_args(&self, args: Vec<String>) -> ParseResult {
        let mut state = ParsingState::new(args);

        self.process_args_for_options(&mut state)?;
        self.process_args_for_args(&mut state)?;

        Ok((state.opts, state.largs, state.order))
    }

    /// Process arguments for options.
    fn process_args_for_options(&self, state: &mut ParsingState) -> Result<(), ClickError> {
        while let Some(arg) = state.rargs.pop_front() {
            let arglen = arg.len();

            // Double dashes always end option parsing
            if arg == "--" {
                return Ok(());
            }

            // Check if this looks like an option
            let first_char = arg.chars().next().unwrap_or(' ');
            let is_option_like = self.opt_prefixes.contains(&first_char.to_string()) && arglen > 1;

            if is_option_like {
                self.process_opts(&arg, state)?;
            } else if self.allow_interspersed_args {
                state.largs.push(arg);
            } else {
                // Stop processing options, put arg back
                state.rargs.push_front(arg);
                return Ok(());
            }
        }
        Ok(())
    }

    /// Process arguments for positional arguments.
    fn process_args_for_args(&self, state: &mut ParsingState) -> Result<(), ClickError> {
        // Combine left args and remaining right args
        let all_args: Vec<String> = state
            .largs
            .drain(..)
            .chain(state.rargs.drain(..))
            .collect();

        // Get nargs specs for all arguments
        let nargs_spec: Vec<i32> = self.args.iter().map(|a| a.nargs).collect();

        // Unpack arguments
        let (parsed, remaining) = unpack_args(&all_args, &nargs_spec)?;

        // Store results and validate multi-value arguments
        for (idx, arg_def) in self.args.iter().enumerate() {
            if idx < parsed.len() {
                let value = &parsed[idx];

                // Check for incomplete multi-value arguments
                if arg_def.nargs > 1 {
                    if let ParsedValue::Multiple(ref values) = value {
                        if values.len() < arg_def.nargs as usize {
                            return Err(ClickError::bad_argument_usage(
                                format!(
                                    "Argument '{}' takes {} values.",
                                    arg_def.dest, arg_def.nargs
                                ),
                            ));
                        }
                    }
                }

                state.opts.insert(arg_def.dest.clone(), value.clone());
                if !value.is_unset() {
                    state.order.push(arg_def.dest.clone());
                }
            } else {
                state.opts.insert(arg_def.dest.clone(), ParsedValue::Unset);
            }
        }

        state.largs = remaining;
        Ok(())
    }

    /// Process a single option argument.
    fn process_opts(&self, arg: &str, state: &mut ParsingState) -> Result<(), ClickError> {
        let mut explicit_value = None;
        let long_opt;

        // Check for explicit value (--name=value)
        if let Some(eq_pos) = arg.find('=') {
            long_opt = arg[..eq_pos].to_string();
            explicit_value = Some(arg[eq_pos + 1..].to_string());
        } else {
            long_opt = arg.to_string();
        }

        let norm_long_opt = self.normalize_opt(&long_opt);

        // Try long option match first
        match self.match_long_opt(&norm_long_opt, explicit_value.clone(), state) {
            Ok(()) => Ok(()),
            Err(ClickError::NoSuchOption { .. }) => {
                // Long option not found - try short option if prefix is single char
                if let Some((prefix, _)) = split_opt(arg) {
                    if prefix.len() == 1 {
                        return self.match_short_opt(arg, state);
                    }
                }

                // Unknown long option
                if self.ignore_unknown_options {
                    state.largs.push(arg.to_string());
                    return Ok(());
                }

                // Re-raise the error with suggestions
                self.match_long_opt(&norm_long_opt, explicit_value, state)
            }
            Err(e) => Err(e),
        }
    }

    /// Match a long option.
    fn match_long_opt(
        &self,
        opt: &str,
        explicit_value: Option<String>,
        state: &mut ParsingState,
    ) -> Result<(), ClickError> {
        let option = match self.long_opt.get(opt) {
            Some(o) => o.clone(),
            None => {
                // Find close matches for suggestions
                let all_opts: Vec<&str> = self.long_opt.keys().map(|s| s.as_str()).collect();
                let possibilities = get_close_matches(opt, &all_opts, 3);

                return Err(if possibilities.is_empty() {
                    ClickError::no_such_option(opt)
                } else {
                    ClickError::no_such_option_with_suggestions(opt, possibilities)
                });
            }
        };

        if option.takes_value() {
            // Inject explicit value back into rargs if present
            if let Some(val) = explicit_value {
                state.rargs.push_front(val);
            }

            let value = self.get_value_from_state(opt, &option, state)?;
            self.process_option(&option, value, state);
        } else if explicit_value.is_some() {
            return Err(ClickError::bad_option_usage(
                opt,
                format!("Option '{}' does not take a value.", opt),
            ));
        } else {
            self.process_option(&option, ParsedValue::Unset, state);
        }

        Ok(())
    }

    /// Match short options (potentially grouped).
    fn match_short_opt(&self, arg: &str, state: &mut ParsingState) -> Result<(), ClickError> {
        let prefix = arg.chars().next().unwrap();
        let mut i = 1;
        let chars: Vec<char> = arg.chars().collect();
        let mut unknown_options = Vec::new();

        while i < chars.len() {
            let ch = chars[i];
            let opt = self.normalize_opt(&format!("{}{}", prefix, ch));
            i += 1;

            let option = match self.short_opt.get(&opt) {
                Some(o) => o.clone(),
                None => {
                    if self.ignore_unknown_options {
                        unknown_options.push(ch);
                        continue;
                    }
                    return Err(ClickError::no_such_option(&opt));
                }
            };

            if option.takes_value() {
                // Remaining characters are the value
                if i < chars.len() {
                    let value: String = chars[i..].iter().collect();
                    state.rargs.push_front(value);
                }

                let value = self.get_value_from_state(&opt, &option, state)?;
                self.process_option(&option, value, state);

                // Stop processing this arg
                break;
            } else {
                self.process_option(&option, ParsedValue::Unset, state);
            }
        }

        // Re-combine unknown options
        if self.ignore_unknown_options && !unknown_options.is_empty() {
            let combined: String =
                std::iter::once(prefix).chain(unknown_options).collect();
            state.largs.push(combined);
        }

        Ok(())
    }

    /// Get a value from the parsing state for an option.
    fn get_value_from_state(
        &self,
        option_name: &str,
        option: &ParserOption,
        state: &mut ParsingState,
    ) -> Result<ParsedValue, ClickError> {
        let nargs = option.nargs;

        // Handle optional nargs (-2 / NARGS_OPTIONAL)
        if nargs == NARGS_OPTIONAL {
            // Check if there's a value available
            if let Some(next_arg) = state.rargs.front() {
                // If the next arg looks like an option, don't consume it
                let first_char = next_arg.chars().next().unwrap_or(' ');
                let looks_like_option = self.opt_prefixes.contains(&first_char.to_string())
                    && next_arg.len() > 1;

                if looks_like_option && option.flag_needs_value {
                    // Option was used as a flag without a value
                    return Ok(ParsedValue::FlagNeedsValue);
                }
            } else if option.flag_needs_value {
                // No more args, option was used as flag
                return Ok(ParsedValue::FlagNeedsValue);
            }

            // Consume the value if available
            if let Some(value) = state.rargs.pop_front() {
                return Ok(ParsedValue::Single(value));
            } else {
                return Ok(ParsedValue::Unset);
            }
        }

        if nargs <= 0 {
            // Flag or count - no value needed
            return Ok(ParsedValue::Unset);
        }

        let rargs_len = state.rargs.len() as i32;

        // Check if we have enough arguments
        if rargs_len < nargs {
            // If flag_needs_value is set, allow omitting the value
            if option.flag_needs_value {
                return Ok(ParsedValue::FlagNeedsValue);
            }
            return Err(ClickError::bad_option_usage(
                option_name,
                if nargs == 1 {
                    format!("Option '{}' requires an argument.", option_name)
                } else {
                    format!("Option '{}' requires {} arguments.", option_name, nargs)
                },
            ));
        }

        if nargs == 1 {
            // Check if next arg looks like an option when flag_needs_value is set
            if option.flag_needs_value {
                if let Some(next_arg) = state.rargs.front() {
                    let first_char = next_arg.chars().next().unwrap_or(' ');
                    let looks_like_option = self.opt_prefixes.contains(&first_char.to_string())
                        && next_arg.len() > 1;

                    if looks_like_option {
                        return Ok(ParsedValue::FlagNeedsValue);
                    }
                }
            }

            let value = state.rargs.pop_front().unwrap();
            Ok(ParsedValue::Single(value))
        } else {
            let values: Vec<String> = (0..nargs)
                .filter_map(|_| state.rargs.pop_front())
                .collect();
            Ok(ParsedValue::Multiple(values))
        }
    }

    /// Process an option with its value.
    fn process_option(&self, option: &ParserOption, value: ParsedValue, state: &mut ParsingState) {
        match option.action {
            OptionAction::Store => {
                state.opts.insert(option.dest.clone(), value);
            }
            OptionAction::StoreConst => {
                let const_val = option
                    .const_value
                    .clone()
                    .map(ParsedValue::Single)
                    .unwrap_or(ParsedValue::Flag(true));
                state.opts.insert(option.dest.clone(), const_val);
            }
            OptionAction::Append => {
                let entry = state
                    .opts
                    .entry(option.dest.clone())
                    .or_insert_with(|| ParsedValue::Multiple(Vec::new()));
                if let ParsedValue::Multiple(ref mut vec) = entry {
                    match value {
                        ParsedValue::Single(s) => vec.push(s),
                        ParsedValue::Multiple(v) => vec.extend(v),
                        ParsedValue::FlagNeedsValue => {
                            // Option used without value in append mode
                            // Mark as FlagNeedsValue to let command layer handle it
                            // Use internal namespace prefix to avoid collision with user options
                            state.opts.insert(
                                format!("__click_internal_flag_needs_value_{}", option.dest),
                                ParsedValue::Flag(true),
                            );
                        }
                        _ => {}
                    }
                }
            }
            OptionAction::AppendConst => {
                let entry = state
                    .opts
                    .entry(option.dest.clone())
                    .or_insert_with(|| ParsedValue::Multiple(Vec::new()));
                if let ParsedValue::Multiple(ref mut vec) = entry {
                    if let Some(ref const_val) = option.const_value {
                        vec.push(const_val.clone());
                    }
                }
            }
            OptionAction::Count => {
                let entry = state
                    .opts
                    .entry(option.dest.clone())
                    .or_insert(ParsedValue::Count(0));
                if let ParsedValue::Count(ref mut n) = entry {
                    *n += 1;
                }
            }
        }

        state.order.push(option.dest.clone());
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // split_opt tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_split_opt_long() {
        assert_eq!(split_opt("--name"), Some(("--", "name")));
        assert_eq!(split_opt("--full-name"), Some(("--", "full-name")));
    }

    #[test]
    fn test_split_opt_short() {
        assert_eq!(split_opt("-n"), Some(("-", "n")));
        assert_eq!(split_opt("-abc"), Some(("-", "abc")));
    }

    #[test]
    fn test_split_opt_no_prefix() {
        assert_eq!(split_opt("name"), None);
        assert_eq!(split_opt(""), None);
    }

    #[test]
    fn test_split_opt_just_dashes() {
        assert_eq!(split_opt("-"), None);
        assert_eq!(split_opt("--"), None);
    }

    // -------------------------------------------------------------------------
    // edit_distance tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("a", ""), 1);
        assert_eq!(edit_distance("", "b"), 1);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("abc", "abd"), 1);
        assert_eq!(edit_distance("help", "hlep"), 2);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_get_close_matches() {
        let opts = vec!["--help", "--hello", "--version", "--verbose"];
        let matches = get_close_matches("--hlep", &opts, 3);
        assert!(matches.contains(&"--help".to_string()));
    }

    // -------------------------------------------------------------------------
    // Short option parsing tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_short_option_with_space() {
        let mut parser = OptionParser::new();
        parser.add_option(&["-n"], "name", OptionAction::Store, 1, None);

        let args = vec!["-n".to_string(), "value".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("value".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_short_option_with_equals() {
        // For short options, the `=` is part of the value (not a separator like long options).
        // This matches POSIX behavior: -n=value means -n with value "=value".
        let mut parser = OptionParser::new();
        parser.add_option(&["-n"], "name", OptionAction::Store, 1, None);

        let args = vec!["-n=value".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        // The `=` is included in the value because short options take the rest of the arg as value
        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("=value".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_short_option_attached_value() {
        let mut parser = OptionParser::new();
        parser.add_option(&["-n"], "name", OptionAction::Store, 1, None);

        let args = vec!["-nvalue".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("value".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_grouped_short_flags() {
        let mut parser = OptionParser::new();
        parser.add_option(&["-a"], "a", OptionAction::StoreConst, 0, Some("true"));
        parser.add_option(&["-b"], "b", OptionAction::StoreConst, 0, Some("true"));
        parser.add_option(&["-c"], "c", OptionAction::StoreConst, 0, Some("true"));

        let args = vec!["-abc".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("a"),
            Some(&ParsedValue::Single("true".to_string()))
        );
        assert_eq!(
            opts.get("b"),
            Some(&ParsedValue::Single("true".to_string()))
        );
        assert_eq!(
            opts.get("c"),
            Some(&ParsedValue::Single("true".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_grouped_short_with_value() {
        let mut parser = OptionParser::new();
        parser.add_option(&["-a"], "a", OptionAction::StoreConst, 0, Some("true"));
        parser.add_option(&["-b"], "b", OptionAction::StoreConst, 0, Some("true"));
        parser.add_option(&["-n"], "name", OptionAction::Store, 1, None);

        // -abn means -a -b -n, where -n takes "value" as argument
        let args = vec!["-abn".to_string(), "value".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("a"),
            Some(&ParsedValue::Single("true".to_string()))
        );
        assert_eq!(
            opts.get("b"),
            Some(&ParsedValue::Single("true".to_string()))
        );
        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("value".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_grouped_short_with_attached_value() {
        let mut parser = OptionParser::new();
        parser.add_option(&["-a"], "a", OptionAction::StoreConst, 0, Some("true"));
        parser.add_option(&["-n"], "name", OptionAction::Store, 1, None);

        // -anVALUE means -a -nVALUE
        let args = vec!["-anVALUE".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("a"),
            Some(&ParsedValue::Single("true".to_string()))
        );
        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("VALUE".to_string()))
        );
        assert!(remaining.is_empty());
    }

    // -------------------------------------------------------------------------
    // Long option parsing tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_long_option_with_space() {
        let mut parser = OptionParser::new();
        parser.add_option(&["--name"], "name", OptionAction::Store, 1, None);

        let args = vec!["--name".to_string(), "value".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("value".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_long_option_with_equals() {
        let mut parser = OptionParser::new();
        parser.add_option(&["--name"], "name", OptionAction::Store, 1, None);

        let args = vec!["--name=value".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("value".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_long_option_multiple_args() {
        let mut parser = OptionParser::new();
        parser.add_option(&["--point"], "point", OptionAction::Store, 2, None);

        let args = vec!["--point".to_string(), "1".to_string(), "2".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("point"),
            Some(&ParsedValue::Multiple(vec!["1".to_string(), "2".to_string()]))
        );
        assert!(remaining.is_empty());
    }

    // -------------------------------------------------------------------------
    // Double-dash terminator tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_double_dash_terminator() {
        let mut parser = OptionParser::new();
        parser.add_option(&["--name"], "name", OptionAction::Store, 1, None);
        parser.add_argument("files", -1);

        let args = vec![
            "--name".to_string(),
            "test".to_string(),
            "--".to_string(),
            "--not-an-option".to_string(),
            "file.txt".to_string(),
        ];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("test".to_string()))
        );
        assert_eq!(
            opts.get("files"),
            Some(&ParsedValue::Multiple(vec![
                "--not-an-option".to_string(),
                "file.txt".to_string()
            ]))
        );
        assert!(remaining.is_empty());
    }

    // -------------------------------------------------------------------------
    // Interspersed args tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_interspersed_args_allowed() {
        let mut parser = OptionParser::new().allow_interspersed_args(true);
        parser.add_option(&["--name"], "name", OptionAction::Store, 1, None);
        parser.add_argument("files", -1);

        let args = vec![
            "file1.txt".to_string(),
            "--name".to_string(),
            "test".to_string(),
            "file2.txt".to_string(),
        ];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("test".to_string()))
        );
        assert_eq!(
            opts.get("files"),
            Some(&ParsedValue::Multiple(vec![
                "file1.txt".to_string(),
                "file2.txt".to_string()
            ]))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_interspersed_args_not_allowed() {
        let mut parser = OptionParser::new().allow_interspersed_args(false);
        parser.add_option(&["--name"], "name", OptionAction::Store, 1, None);
        parser.add_argument("files", -1);

        let args = vec![
            "file1.txt".to_string(),
            "--name".to_string(),
            "test".to_string(),
            "file2.txt".to_string(),
        ];
        let (opts, _remaining, _) = parser.parse_args(args).unwrap();

        // --name should NOT be parsed as an option since we stopped at file1.txt
        assert!(opts.get("name").is_none() || opts.get("name") == Some(&ParsedValue::Unset));
        assert_eq!(
            opts.get("files"),
            Some(&ParsedValue::Multiple(vec![
                "file1.txt".to_string(),
                "--name".to_string(),
                "test".to_string(),
                "file2.txt".to_string()
            ]))
        );
    }

    // -------------------------------------------------------------------------
    // Unknown option handling tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_unknown_option_error() {
        let parser = OptionParser::new();

        let args = vec!["--unknown".to_string()];
        let result = parser.parse_args(args);

        assert!(result.is_err());
        match result.unwrap_err() {
            ClickError::NoSuchOption { option_name, .. } => {
                assert_eq!(option_name, "--unknown");
            }
            _ => panic!("Expected NoSuchOption error"),
        }
    }

    #[test]
    fn test_unknown_option_with_suggestion() {
        let mut parser = OptionParser::new();
        parser.add_option(&["--help"], "help", OptionAction::StoreConst, 0, Some("true"));

        let args = vec!["--hlep".to_string()];
        let result = parser.parse_args(args);

        assert!(result.is_err());
        match result.unwrap_err() {
            ClickError::NoSuchOption {
                option_name,
                possibilities,
                ..
            } => {
                assert_eq!(option_name, "--hlep");
                assert!(possibilities.is_some());
                assert!(possibilities.unwrap().contains(&"--help".to_string()));
            }
            _ => panic!("Expected NoSuchOption error with suggestions"),
        }
    }

    #[test]
    fn test_ignore_unknown_options() {
        let mut parser = OptionParser::new().ignore_unknown_options(true);
        parser.add_option(&["--name"], "name", OptionAction::Store, 1, None);
        parser.add_argument("files", -1);

        let args = vec![
            "--unknown".to_string(),
            "--name".to_string(),
            "test".to_string(),
        ];
        let (opts, _, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("test".to_string()))
        );
        // --unknown should be in files
        if let Some(ParsedValue::Multiple(files)) = opts.get("files") {
            assert!(files.contains(&"--unknown".to_string()));
        }
    }

    // -------------------------------------------------------------------------
    // Action tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_count_action() {
        let mut parser = OptionParser::new();
        parser.add_option(&["-v", "--verbose"], "verbose", OptionAction::Count, 0, None);

        let args = vec![
            "-v".to_string(),
            "-v".to_string(),
            "--verbose".to_string(),
        ];
        let (opts, _, _) = parser.parse_args(args).unwrap();

        assert_eq!(opts.get("verbose"), Some(&ParsedValue::Count(3)));
    }

    #[test]
    fn test_append_action() {
        let mut parser = OptionParser::new();
        parser.add_option(&["-f", "--file"], "files", OptionAction::Append, 1, None);

        let args = vec![
            "-f".to_string(),
            "a.txt".to_string(),
            "--file".to_string(),
            "b.txt".to_string(),
            "-f".to_string(),
            "c.txt".to_string(),
        ];
        let (opts, _, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("files"),
            Some(&ParsedValue::Multiple(vec![
                "a.txt".to_string(),
                "b.txt".to_string(),
                "c.txt".to_string()
            ]))
        );
    }

    #[test]
    fn test_append_const_action() {
        let mut parser = OptionParser::new();
        parser.add_option(
            &["--debug"],
            "flags",
            OptionAction::AppendConst,
            0,
            Some("debug"),
        );
        parser.add_option(
            &["--trace"],
            "flags",
            OptionAction::AppendConst,
            0,
            Some("trace"),
        );

        let args = vec!["--debug".to_string(), "--trace".to_string()];
        let (opts, _, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("flags"),
            Some(&ParsedValue::Multiple(vec![
                "debug".to_string(),
                "trace".to_string()
            ]))
        );
    }

    #[test]
    fn test_store_const_action() {
        let mut parser = OptionParser::new();
        parser.add_option(
            &["--debug"],
            "debug",
            OptionAction::StoreConst,
            0,
            Some("true"),
        );

        let args = vec!["--debug".to_string()];
        let (opts, _, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("debug"),
            Some(&ParsedValue::Single("true".to_string()))
        );
    }

    // -------------------------------------------------------------------------
    // Positional argument tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_single_positional_argument() {
        let mut parser = OptionParser::new();
        parser.add_argument("file", 1);

        let args = vec!["input.txt".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("file"),
            Some(&ParsedValue::Single("input.txt".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_variadic_positional_argument() {
        let mut parser = OptionParser::new();
        parser.add_argument("files", -1);

        let args = vec!["a.txt".to_string(), "b.txt".to_string(), "c.txt".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("files"),
            Some(&ParsedValue::Multiple(vec![
                "a.txt".to_string(),
                "b.txt".to_string(),
                "c.txt".to_string()
            ]))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_multiple_positional_arguments() {
        let mut parser = OptionParser::new();
        parser.add_argument("source", 1);
        parser.add_argument("dest", 1);

        let args = vec!["input.txt".to_string(), "output.txt".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("source"),
            Some(&ParsedValue::Single("input.txt".to_string()))
        );
        assert_eq!(
            opts.get("dest"),
            Some(&ParsedValue::Single("output.txt".to_string()))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_positional_with_variadic() {
        let mut parser = OptionParser::new();
        parser.add_argument("dest", 1);
        parser.add_argument("sources", -1);

        let args = vec!["out.txt".to_string(), "a.txt".to_string(), "b.txt".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("dest"),
            Some(&ParsedValue::Single("out.txt".to_string()))
        );
        assert_eq!(
            opts.get("sources"),
            Some(&ParsedValue::Multiple(vec![
                "a.txt".to_string(),
                "b.txt".to_string()
            ]))
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_missing_required_positional() {
        let mut parser = OptionParser::new();
        parser.add_argument("file", 1);

        let args: Vec<String> = vec![];
        let (opts, _, _) = parser.parse_args(args).unwrap();

        // Missing argument results in Unset
        assert_eq!(opts.get("file"), Some(&ParsedValue::Unset));
    }

    // -------------------------------------------------------------------------
    // Combined option and argument tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_options_and_arguments() {
        let mut parser = OptionParser::new();
        parser.add_option(&["-n", "--name"], "name", OptionAction::Store, 1, None);
        parser.add_option(&["-v", "--verbose"], "verbose", OptionAction::Count, 0, None);
        parser.add_argument("file", 1);

        let args = vec![
            "-v".to_string(),
            "--name".to_string(),
            "test".to_string(),
            "-v".to_string(),
            "input.txt".to_string(),
        ];
        let (opts, remaining, order) = parser.parse_args(args).unwrap();

        assert_eq!(opts.get("verbose"), Some(&ParsedValue::Count(2)));
        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("test".to_string()))
        );
        assert_eq!(
            opts.get("file"),
            Some(&ParsedValue::Single("input.txt".to_string()))
        );
        assert!(remaining.is_empty());

        // Check order
        assert!(order.contains(&"verbose".to_string()));
        assert!(order.contains(&"name".to_string()));
        assert!(order.contains(&"file".to_string()));
    }

    // -------------------------------------------------------------------------
    // Error case tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_missing_option_value() {
        let mut parser = OptionParser::new();
        parser.add_option(&["--name"], "name", OptionAction::Store, 1, None);

        let args = vec!["--name".to_string()];
        let result = parser.parse_args(args);

        assert!(result.is_err());
        match result.unwrap_err() {
            ClickError::BadOptionUsage { option_name, .. } => {
                assert_eq!(option_name, "--name");
            }
            _ => panic!("Expected BadOptionUsage error"),
        }
    }

    #[test]
    fn test_option_takes_no_value() {
        let mut parser = OptionParser::new();
        parser.add_option(
            &["--debug"],
            "debug",
            OptionAction::StoreConst,
            0,
            Some("true"),
        );

        let args = vec!["--debug=value".to_string()];
        let result = parser.parse_args(args);

        assert!(result.is_err());
        match result.unwrap_err() {
            ClickError::BadOptionUsage { option_name, .. } => {
                assert_eq!(option_name, "--debug");
            }
            _ => panic!("Expected BadOptionUsage error"),
        }
    }

    // -------------------------------------------------------------------------
    // Token normalization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_token_normalize_func() {
        let mut parser =
            OptionParser::new().token_normalize_func(|s| s.to_lowercase());
        parser.add_option(&["--name"], "name", OptionAction::Store, 1, None);

        let args = vec!["--NAME".to_string(), "value".to_string()];
        let (opts, _, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("name"),
            Some(&ParsedValue::Single("value".to_string()))
        );
    }

    // -------------------------------------------------------------------------
    // ParsedValue tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parsed_value_accessors() {
        let single = ParsedValue::Single("test".to_string());
        assert_eq!(single.as_single(), Some("test"));
        assert!(single.as_multiple().is_none());
        assert!(single.as_count().is_none());
        assert!(single.as_flag().is_none());
        assert!(!single.is_unset());

        let multiple = ParsedValue::Multiple(vec!["a".to_string(), "b".to_string()]);
        assert!(multiple.as_single().is_none());
        assert_eq!(
            multiple.as_multiple(),
            Some(&["a".to_string(), "b".to_string()][..])
        );

        let count = ParsedValue::Count(5);
        assert_eq!(count.as_count(), Some(5));

        let flag = ParsedValue::Flag(true);
        assert_eq!(flag.as_flag(), Some(true));

        let unset = ParsedValue::Unset;
        assert!(unset.is_unset());
    }

    // -------------------------------------------------------------------------
    // unpack_args tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_unpack_args_simple() {
        let args = vec!["a".to_string(), "b".to_string()];
        let specs = vec![1, 1];
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ParsedValue::Single("a".to_string()));
        assert_eq!(result[1], ParsedValue::Single("b".to_string()));
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_variadic() {
        let args = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let specs = vec![-1]; // variadic
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            ParsedValue::Multiple(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string()
            ])
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_with_variadic_in_middle() {
        let args = vec![
            "dest".to_string(),
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ];
        let specs = vec![1, -1]; // dest, then variadic sources
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ParsedValue::Single("dest".to_string()));
        assert_eq!(
            result[1],
            ParsedValue::Multiple(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string()
            ])
        );
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_missing() {
        let args = vec!["a".to_string()];
        let specs = vec![1, 1];
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ParsedValue::Single("a".to_string()));
        assert_eq!(result[1], ParsedValue::Unset);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_empty_variadic() {
        let args: Vec<String> = vec![];
        let specs = vec![-1];
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ParsedValue::Multiple(vec![]));
        assert!(remaining.is_empty());
    }

    // -------------------------------------------------------------------------
    // Optional positional argument tests (Nargs::Optional -> NARGS_OPTIONAL)
    // -------------------------------------------------------------------------

    #[test]
    fn test_unpack_args_optional_with_value() {
        let args = vec!["value".to_string()];
        let specs = vec![NARGS_OPTIONAL]; // optional
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ParsedValue::Single("value".to_string()));
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_optional_without_value() {
        let args: Vec<String> = vec![];
        let specs = vec![NARGS_OPTIONAL]; // optional
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ParsedValue::Unset);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_optional_after_required() {
        let args = vec!["required".to_string(), "optional".to_string()];
        let specs = vec![1, NARGS_OPTIONAL]; // required, then optional
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ParsedValue::Single("required".to_string()));
        assert_eq!(result[1], ParsedValue::Single("optional".to_string()));
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_optional_missing_after_required() {
        let args = vec!["required".to_string()];
        let specs = vec![1, NARGS_OPTIONAL]; // required, then optional
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ParsedValue::Single("required".to_string()));
        assert_eq!(result[1], ParsedValue::Unset);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_optional_before_required_preserves_required() {
        // Optional before required: with only one arg, required gets it
        let args = vec!["x".to_string()];
        let specs = vec![NARGS_OPTIONAL, 1]; // optional, then required
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 2);
        // Optional should remain Unset since required needs the value
        assert_eq!(result[0], ParsedValue::Unset);
        // Required should get the value
        assert_eq!(result[1], ParsedValue::Single("x".to_string()));
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_unpack_args_optional_before_required_with_both() {
        // Optional before required: with two args, both get values
        let args = vec!["opt".to_string(), "req".to_string()];
        let specs = vec![NARGS_OPTIONAL, 1]; // optional, then required
        let (result, remaining) = unpack_args(&args, &specs).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ParsedValue::Single("opt".to_string()));
        assert_eq!(result[1], ParsedValue::Single("req".to_string()));
        assert!(remaining.is_empty());
    }

    // -------------------------------------------------------------------------
    // Two variadic specs error test
    // -------------------------------------------------------------------------

    #[test]
    fn test_unpack_args_two_variadic_error() {
        let args = vec!["a".to_string(), "b".to_string()];
        let specs = vec![-1, -1]; // two variadic - error
        let result = unpack_args(&args, &specs);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("variadic"));
    }

    // -------------------------------------------------------------------------
    // Optional option value tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_option_with_optional_value_explicit() {
        let mut parser = OptionParser::new();
        parser.add_option_ex(
            &["--opt"],
            "opt",
            OptionAction::Store,
            1,
            None,
            true, // flag_needs_value
        );

        // --opt=value should work normally
        let args = vec!["--opt=value".to_string()];
        let (opts, _, _) = parser.parse_args(args).unwrap();
        assert_eq!(
            opts.get("opt"),
            Some(&ParsedValue::Single("value".to_string()))
        );
    }

    #[test]
    fn test_option_with_optional_value_followed_by_option() {
        let mut parser = OptionParser::new();
        parser.add_option_ex(
            &["--opt"],
            "opt",
            OptionAction::Store,
            1,
            None,
            true, // flag_needs_value
        );
        parser.add_option(&["--other"], "other", OptionAction::StoreConst, 0, Some("true"));

        // --opt followed by another option should not consume the next option
        let args = vec!["--opt".to_string(), "--other".to_string()];
        let (opts, _, _) = parser.parse_args(args).unwrap();
        assert_eq!(opts.get("opt"), Some(&ParsedValue::FlagNeedsValue));
        assert_eq!(
            opts.get("other"),
            Some(&ParsedValue::Single("true".to_string()))
        );
    }

    #[test]
    fn test_option_with_optional_value_with_value() {
        let mut parser = OptionParser::new();
        parser.add_option_ex(
            &["--opt"],
            "opt",
            OptionAction::Store,
            1,
            None,
            true, // flag_needs_value
        );

        // --opt followed by a value should consume it
        let args = vec!["--opt".to_string(), "value".to_string()];
        let (opts, _, _) = parser.parse_args(args).unwrap();
        assert_eq!(
            opts.get("opt"),
            Some(&ParsedValue::Single("value".to_string()))
        );
    }

    #[test]
    fn test_option_with_optional_value_at_end() {
        let mut parser = OptionParser::new();
        parser.add_option_ex(
            &["--opt"],
            "opt",
            OptionAction::Store,
            1,
            None,
            true, // flag_needs_value
        );

        // --opt at end of args should return FlagNeedsValue
        let args = vec!["--opt".to_string()];
        let (opts, _, _) = parser.parse_args(args).unwrap();
        assert_eq!(opts.get("opt"), Some(&ParsedValue::FlagNeedsValue));
    }

    // -------------------------------------------------------------------------
    // Multi-value argument incomplete error tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_multi_value_argument_incomplete() {
        let mut parser = OptionParser::new();
        parser.add_argument("pair", 2); // requires 2 values

        // Only providing 1 value should error
        let args = vec!["first".to_string()];
        let result = parser.parse_args(args);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("takes 2 values"));
    }

    #[test]
    fn test_multi_value_argument_complete() {
        let mut parser = OptionParser::new();
        parser.add_argument("pair", 2); // requires 2 values

        // Providing 2 values should work
        let args = vec!["first".to_string(), "second".to_string()];
        let (opts, remaining, _) = parser.parse_args(args).unwrap();

        assert_eq!(
            opts.get("pair"),
            Some(&ParsedValue::Multiple(vec![
                "first".to_string(),
                "second".to_string()
            ]))
        );
        assert!(remaining.is_empty());
    }
}
