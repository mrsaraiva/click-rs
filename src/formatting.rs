//! Help text formatting utilities.
//!
//! This module provides the `HelpFormatter` struct for creating well-formatted
//! help output for CLI commands, including proper text wrapping, indentation,
//! and definition list formatting.

/// Default terminal width when detection fails
const DEFAULT_WIDTH: usize = 80;

/// Minimum terminal width
const MIN_WIDTH: usize = 40;

/// Default indentation for help text
#[allow(dead_code)]
const DEFAULT_INDENT: usize = 2;

/// Default indentation for definition lists
const DEFINITION_INDENT: usize = 2;

/// Column spacing between definition term and description
const DEFINITION_SPACING: usize = 2;

/// Maximum width for definition terms before wrapping description below
const MAX_TERM_WIDTH: usize = 24;

/// Help text formatter with terminal-aware wrapping.
///
/// This struct provides methods for formatting help text with proper
/// indentation, text wrapping, and special formatting for definition
/// lists (options/arguments).
///
/// # Example
///
/// ```rust
/// use click::HelpFormatter;
///
/// let mut formatter = HelpFormatter::new(80);
///
/// formatter.write_usage("mycli", "[OPTIONS] COMMAND");
/// formatter.write_paragraph("A helpful CLI tool.");
/// formatter.write_heading("Options");
/// formatter.write_definition_list(&[
///     ("--help, -h", "Show this help message"),
///     ("--verbose, -v", "Enable verbose output"),
/// ]);
///
/// println!("{}", formatter.get_help());
/// ```
#[derive(Debug)]
pub struct HelpFormatter {
    /// Maximum width for output
    width: usize,
    /// Current indentation level
    indent: usize,
    /// Buffer for accumulated output
    buffer: String,
    /// Current column position
    current_col: usize,
}

impl HelpFormatter {
    /// Create a new HelpFormatter with the specified terminal width.
    ///
    /// If width is 0 or less than minimum, uses default width.
    pub fn new(width: usize) -> Self {
        let width = if width < MIN_WIDTH {
            DEFAULT_WIDTH
        } else {
            width
        };
        Self {
            width,
            indent: 0,
            buffer: String::new(),
            current_col: 0,
        }
    }

    /// Create a new HelpFormatter that detects terminal width.
    pub fn detect_width() -> Self {
        let width = detect_terminal_width().unwrap_or(DEFAULT_WIDTH);
        Self::new(width)
    }

    /// Get the accumulated help text.
    pub fn get_help(&self) -> &str {
        &self.buffer
    }

    /// Consume the formatter and return the help text.
    pub fn into_help(self) -> String {
        self.buffer
    }

    /// Get the current terminal width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Set the current indentation level.
    pub fn set_indent(&mut self, indent: usize) {
        self.indent = indent;
    }

    /// Increase indentation by the given amount.
    pub fn indent(&mut self, amount: usize) {
        self.indent += amount;
    }

    /// Decrease indentation by the given amount.
    pub fn dedent(&mut self, amount: usize) {
        self.indent = self.indent.saturating_sub(amount);
    }

    /// Write a blank line.
    pub fn write_blank(&mut self) {
        self.buffer.push('\n');
        self.current_col = 0;
    }

    /// Write raw text without any processing.
    pub fn write_raw(&mut self, text: &str) {
        self.buffer.push_str(text);
        // Update column position based on last line
        if let Some(last_newline) = text.rfind('\n') {
            self.current_col = text.len() - last_newline - 1;
        } else {
            self.current_col += text.len();
        }
    }

    /// Write text with the current indentation.
    pub fn write(&mut self, text: &str) {
        let indent_str = " ".repeat(self.indent);
        for line in text.lines() {
            self.buffer.push_str(&indent_str);
            self.buffer.push_str(line);
            self.buffer.push('\n');
        }
        self.current_col = 0;
    }

    /// Write a usage line.
    ///
    /// Format: `Usage: prog [OPTIONS] ARGS`
    pub fn write_usage(&mut self, prog: &str, args: &str) {
        self.buffer.push_str("Usage: ");
        self.buffer.push_str(prog);
        if !args.is_empty() {
            self.buffer.push(' ');
            self.buffer.push_str(args);
        }
        self.buffer.push('\n');
        self.current_col = 0;
    }

    /// Write a section heading.
    ///
    /// The heading is followed by a blank line.
    pub fn write_heading(&mut self, heading: &str) {
        if !self.buffer.is_empty() && !self.buffer.ends_with("\n\n") {
            self.buffer.push('\n');
        }
        self.buffer.push_str(heading);
        self.buffer.push_str(":\n");
        self.current_col = 0;
    }

    /// Write a paragraph of text with word wrapping.
    ///
    /// Preserves paragraph breaks (double newlines) and wraps text
    /// to fit within the terminal width.
    pub fn write_paragraph(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let max_width = self.width.saturating_sub(self.indent);
        let indent_str = " ".repeat(self.indent);

        // Split into paragraphs (separated by blank lines)
        for (i, paragraph) in text.split("\n\n").enumerate() {
            if i > 0 {
                self.buffer.push('\n');
            }

            // Wrap the paragraph
            let wrapped = wrap_text(paragraph, max_width);
            for line in wrapped.lines() {
                self.buffer.push_str(&indent_str);
                self.buffer.push_str(line);
                self.buffer.push('\n');
            }
        }

        self.current_col = 0;
    }

    /// Write a definition list (term + description pairs).
    ///
    /// Used for formatting options and arguments. Each item is a tuple
    /// of (term, description).
    ///
    /// Format depends on term length:
    /// - Short terms: description on same line
    /// - Long terms: description on next line, indented
    pub fn write_definition_list(&mut self, items: &[(&str, &str)]) {
        let base_indent = " ".repeat(DEFINITION_INDENT);
        let desc_indent = " ".repeat(DEFINITION_INDENT + MAX_TERM_WIDTH + DEFINITION_SPACING);
        let desc_width = self.width.saturating_sub(desc_indent.len());

        for (term, description) in items {
            // Write the term
            self.buffer.push_str(&base_indent);
            self.buffer.push_str(term);

            if term.len() <= MAX_TERM_WIDTH && !description.is_empty() {
                // Term fits - description on same line
                let padding = MAX_TERM_WIDTH - term.len() + DEFINITION_SPACING;
                self.buffer.push_str(&" ".repeat(padding));

                // Wrap description
                let wrapped = wrap_text(description, desc_width);
                let mut lines = wrapped.lines();

                // First line on same line as term
                if let Some(first) = lines.next() {
                    self.buffer.push_str(first);
                    self.buffer.push('\n');
                }

                // Remaining lines indented
                for line in lines {
                    self.buffer.push_str(&desc_indent);
                    self.buffer.push_str(line);
                    self.buffer.push('\n');
                }
            } else if !description.is_empty() {
                // Term too long - description on next line
                self.buffer.push('\n');

                let wrapped = wrap_text(description, desc_width);
                for line in wrapped.lines() {
                    self.buffer.push_str(&desc_indent);
                    self.buffer.push_str(line);
                    self.buffer.push('\n');
                }
            } else {
                self.buffer.push('\n');
            }
        }

        self.current_col = 0;
    }

    /// Write a definition list with string tuples.
    pub fn write_definition_list_strings(&mut self, items: &[(String, String)]) {
        let refs: Vec<(&str, &str)> = items
            .iter()
            .map(|(t, d)| (t.as_str(), d.as_str()))
            .collect();
        self.write_definition_list(&refs);
    }
}

impl Default for HelpFormatter {
    fn default() -> Self {
        Self::detect_width()
    }
}

/// Wrap text to fit within the specified width.
///
/// Preserves existing line breaks and word boundaries.
pub fn wrap_text(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_string();
    }

    let mut result = String::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for line in text.lines() {
        // Handle explicit line breaks
        if !result.is_empty() || !current_line.is_empty() {
            if !current_line.is_empty() {
                result.push_str(&current_line);
                current_line.clear();
                current_width = 0;
            }
            result.push('\n');
        }

        // Process words in this line
        for word in line.split_whitespace() {
            let word_width = word.len();

            if current_width == 0 {
                // First word on line
                current_line.push_str(word);
                current_width = word_width;
            } else if current_width + 1 + word_width <= width {
                // Word fits on current line
                current_line.push(' ');
                current_line.push_str(word);
                current_width += 1 + word_width;
            } else {
                // Start new line
                result.push_str(&current_line);
                result.push('\n');
                current_line.clear();
                current_line.push_str(word);
                current_width = word_width;
            }
        }
    }

    // Add remaining content
    if !current_line.is_empty() {
        result.push_str(&current_line);
    }

    result
}

/// Detect the terminal width.
///
/// Returns None if detection fails. Currently only checks the COLUMNS
/// environment variable. For more robust terminal detection, consider
/// using the `crossterm` or `terminal_size` crate.
pub fn detect_terminal_width() -> Option<usize> {
    // Try environment variable first
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(width) = cols.parse::<usize>() {
            if width >= MIN_WIDTH {
                return Some(width);
            }
        }
    }

    // Try TERM_PROGRAM for common terminals
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        // Most modern terminals default to 80 or wider
        match term.as_str() {
            "vscode" | "iTerm.app" | "Apple_Terminal" | "Hyper" => {
                return Some(DEFAULT_WIDTH);
            }
            _ => {}
        }
    }

    None
}

/// Get terminal width with a fallback default.
pub fn get_terminal_width() -> usize {
    detect_terminal_width().unwrap_or(DEFAULT_WIDTH)
}

/// Create a horizontal rule of the specified character.
pub fn make_rule(char: char, width: usize) -> String {
    std::iter::repeat(char).take(width).collect()
}

/// Truncate text to fit within width, adding ellipsis if needed.
pub fn truncate_text(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        return text.to_string();
    }

    if max_width <= 3 {
        return "...".to_string();
    }

    let mut result = text[..max_width - 3].to_string();
    result.push_str("...");
    result
}

/// Split text into lines that fit within the specified width.
pub fn split_into_lines(text: &str, width: usize) -> Vec<String> {
    wrap_text(text, width).lines().map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text() {
        let text = "This is a test of the text wrapping functionality";
        let wrapped = wrap_text(text, 20);
        assert!(wrapped.lines().all(|l| l.len() <= 20));
    }

    #[test]
    fn test_wrap_preserves_newlines() {
        let text = "Line one\nLine two\nLine three";
        let wrapped = wrap_text(text, 80);
        assert_eq!(wrapped.lines().count(), 3);
    }

    #[test]
    fn test_help_formatter_usage() {
        let mut fmt = HelpFormatter::new(80);
        fmt.write_usage("mycli", "[OPTIONS] COMMAND");
        assert!(fmt.get_help().contains("Usage: mycli [OPTIONS] COMMAND"));
    }

    #[test]
    fn test_help_formatter_heading() {
        let mut fmt = HelpFormatter::new(80);
        fmt.write_heading("Options");
        assert!(fmt.get_help().contains("Options:\n"));
    }

    #[test]
    fn test_help_formatter_definition_list() {
        let mut fmt = HelpFormatter::new(80);
        fmt.write_definition_list(&[("--help, -h", "Show help"), ("--version", "Show version")]);
        let help = fmt.get_help();
        assert!(help.contains("--help, -h"));
        assert!(help.contains("Show help"));
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world", 8), "hello...");
        // Text that fits within max_width is not truncated
        assert_eq!(truncate_text("hi", 3), "hi");
        // Text that's too long for even "..." gets truncated to "..."
        assert_eq!(truncate_text("hello", 3), "...");
    }
}
