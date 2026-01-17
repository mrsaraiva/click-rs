//! Parameter source tracking for click-rs.
//!
//! This module provides the [`ParameterSource`] enum which indicates where a
//! parameter's value originated from. This is useful for debugging and for
//! implementing precedence rules when values can come from multiple sources.

use std::cmp::Ordering;
use std::fmt;

/// Indicates the source of a parameter's value.
///
/// Use [`Context::get_parameter_source`] to get the source for a parameter by name.
///
/// # Precedence
///
/// Sources have a defined precedence order (highest to lowest):
/// 1. [`CommandLine`](ParameterSource::CommandLine) - Explicit user input
/// 2. [`Environment`](ParameterSource::Environment) - Environment variables
/// 3. [`DefaultMap`](ParameterSource::DefaultMap) - Context default map
/// 4. [`Default`](ParameterSource::Default) - Parameter default value
///
/// [`Prompt`](ParameterSource::Prompt) is considered equal to `CommandLine` for precedence
/// since both represent direct user input.
///
/// # Example
///
/// ```
/// use click::ParameterSource;
///
/// let source = ParameterSource::CommandLine;
/// assert!(source.is_from_user());
///
/// let env_source = ParameterSource::Environment;
/// assert!(!env_source.is_from_user());
///
/// // CommandLine has higher precedence than Environment
/// assert!(ParameterSource::CommandLine > ParameterSource::Environment);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParameterSource {
    /// The value was provided by command line arguments.
    CommandLine,

    /// The value was provided via an environment variable.
    Environment,

    /// The value used the default specified by the parameter.
    Default,

    /// The value used a default provided by the context's default_map.
    DefaultMap,

    /// The value was provided via an interactive prompt.
    Prompt,
}

impl ParameterSource {
    /// Returns `true` if the value came from direct user input.
    ///
    /// This returns `true` for [`CommandLine`](ParameterSource::CommandLine)
    /// and [`Prompt`](ParameterSource::Prompt), as both represent values
    /// explicitly provided by the user during execution.
    ///
    /// # Example
    ///
    /// ```
    /// use click::ParameterSource;
    ///
    /// assert!(ParameterSource::CommandLine.is_from_user());
    /// assert!(ParameterSource::Prompt.is_from_user());
    /// assert!(!ParameterSource::Environment.is_from_user());
    /// assert!(!ParameterSource::Default.is_from_user());
    /// assert!(!ParameterSource::DefaultMap.is_from_user());
    /// ```
    #[inline]
    pub fn is_from_user(&self) -> bool {
        matches!(self, Self::CommandLine | Self::Prompt)
    }

    /// Returns the precedence value for ordering.
    ///
    /// Higher values indicate higher precedence (more authoritative sources).
    #[inline]
    fn precedence(&self) -> u8 {
        match self {
            // User input has highest precedence
            Self::CommandLine | Self::Prompt => 4,
            // Environment variables are next
            Self::Environment => 3,
            // Context default_map overrides parameter defaults
            Self::DefaultMap => 2,
            // Parameter defaults have lowest precedence
            Self::Default => 1,
        }
    }
}

impl fmt::Display for ParameterSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandLine => write!(f, "command line"),
            Self::Environment => write!(f, "environment variable"),
            Self::Default => write!(f, "default value"),
            Self::DefaultMap => write!(f, "default map"),
            Self::Prompt => write!(f, "prompt"),
        }
    }
}

impl PartialOrd for ParameterSource {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ParameterSource {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.precedence().cmp(&other.precedence())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_from_user() {
        assert!(ParameterSource::CommandLine.is_from_user());
        assert!(ParameterSource::Prompt.is_from_user());
        assert!(!ParameterSource::Environment.is_from_user());
        assert!(!ParameterSource::Default.is_from_user());
        assert!(!ParameterSource::DefaultMap.is_from_user());
    }

    #[test]
    fn test_display() {
        assert_eq!(ParameterSource::CommandLine.to_string(), "command line");
        assert_eq!(
            ParameterSource::Environment.to_string(),
            "environment variable"
        );
        assert_eq!(ParameterSource::Default.to_string(), "default value");
        assert_eq!(ParameterSource::DefaultMap.to_string(), "default map");
        assert_eq!(ParameterSource::Prompt.to_string(), "prompt");
    }

    #[test]
    fn test_precedence_ordering() {
        // CommandLine > Environment > DefaultMap > Default
        assert!(ParameterSource::CommandLine > ParameterSource::Environment);
        assert!(ParameterSource::Environment > ParameterSource::DefaultMap);
        assert!(ParameterSource::DefaultMap > ParameterSource::Default);

        // Prompt equals CommandLine in precedence
        assert_eq!(
            ParameterSource::CommandLine.cmp(&ParameterSource::Prompt),
            Ordering::Equal
        );
        assert!(ParameterSource::Prompt > ParameterSource::Environment);
    }

    #[test]
    fn test_equality() {
        assert_eq!(ParameterSource::CommandLine, ParameterSource::CommandLine);
        assert_ne!(ParameterSource::CommandLine, ParameterSource::Environment);
    }

    #[test]
    fn test_copy_clone() {
        let source = ParameterSource::CommandLine;
        let copied = source; // Copy
        let cloned = source.clone(); // Clone
        assert_eq!(source, copied);
        assert_eq!(source, cloned);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ParameterSource::CommandLine);
        set.insert(ParameterSource::Environment);
        set.insert(ParameterSource::CommandLine); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ParameterSource::CommandLine));
        assert!(set.contains(&ParameterSource::Environment));
    }

    #[test]
    fn test_debug() {
        let debug_str = format!("{:?}", ParameterSource::CommandLine);
        assert_eq!(debug_str, "CommandLine");
    }
}
