//! Integration tests for terminal UI functions.
//!
//! These tests verify the public API of the termui module.

use click::termui::{
    get_terminal_size, strip_ansi_codes, style, Color, ProgressBar, BLACK, BLUE, BRIGHT_BLACK,
    BRIGHT_BLUE, BRIGHT_CYAN, BRIGHT_GREEN, BRIGHT_MAGENTA, BRIGHT_RED, BRIGHT_WHITE,
    BRIGHT_YELLOW, CYAN, GREEN, MAGENTA, RED, RESET, WHITE, YELLOW,
};

// ============================================================================
// Color Code Tests
// ============================================================================

#[test]
fn test_color_fg_codes() {
    assert_eq!(Color::Black.fg_code(), 30);
    assert_eq!(Color::Red.fg_code(), 31);
    assert_eq!(Color::Green.fg_code(), 32);
    assert_eq!(Color::Yellow.fg_code(), 33);
    assert_eq!(Color::Blue.fg_code(), 34);
    assert_eq!(Color::Magenta.fg_code(), 35);
    assert_eq!(Color::Cyan.fg_code(), 36);
    assert_eq!(Color::White.fg_code(), 37);
}

#[test]
fn test_color_bright_fg_codes() {
    assert_eq!(Color::BrightBlack.fg_code(), 90);
    assert_eq!(Color::BrightRed.fg_code(), 91);
    assert_eq!(Color::BrightGreen.fg_code(), 92);
    assert_eq!(Color::BrightYellow.fg_code(), 93);
    assert_eq!(Color::BrightBlue.fg_code(), 94);
    assert_eq!(Color::BrightMagenta.fg_code(), 95);
    assert_eq!(Color::BrightCyan.fg_code(), 96);
    assert_eq!(Color::BrightWhite.fg_code(), 97);
}

#[test]
fn test_color_bg_codes() {
    assert_eq!(Color::Black.bg_code(), 40);
    assert_eq!(Color::Red.bg_code(), 41);
    assert_eq!(Color::Green.bg_code(), 42);
    assert_eq!(Color::Yellow.bg_code(), 43);
    assert_eq!(Color::Blue.bg_code(), 44);
    assert_eq!(Color::Magenta.bg_code(), 45);
    assert_eq!(Color::Cyan.bg_code(), 46);
    assert_eq!(Color::White.bg_code(), 47);
}

#[test]
fn test_color_bright_bg_codes() {
    assert_eq!(Color::BrightBlack.bg_code(), 100);
    assert_eq!(Color::BrightRed.bg_code(), 101);
    assert_eq!(Color::BrightGreen.bg_code(), 102);
    assert_eq!(Color::BrightYellow.bg_code(), 103);
    assert_eq!(Color::BrightBlue.bg_code(), 104);
    assert_eq!(Color::BrightMagenta.bg_code(), 105);
    assert_eq!(Color::BrightCyan.bg_code(), 106);
    assert_eq!(Color::BrightWhite.bg_code(), 107);
}

#[test]
fn test_color_reset_codes() {
    assert_eq!(Color::Reset.fg_code(), 39);
    assert_eq!(Color::Reset.bg_code(), 49);
}

// ============================================================================
// Color Constant Tests
// ============================================================================

#[test]
fn test_color_constants() {
    assert_eq!(BLACK, Color::Black);
    assert_eq!(RED, Color::Red);
    assert_eq!(GREEN, Color::Green);
    assert_eq!(YELLOW, Color::Yellow);
    assert_eq!(BLUE, Color::Blue);
    assert_eq!(MAGENTA, Color::Magenta);
    assert_eq!(CYAN, Color::Cyan);
    assert_eq!(WHITE, Color::White);
    assert_eq!(BRIGHT_BLACK, Color::BrightBlack);
    assert_eq!(BRIGHT_RED, Color::BrightRed);
    assert_eq!(BRIGHT_GREEN, Color::BrightGreen);
    assert_eq!(BRIGHT_YELLOW, Color::BrightYellow);
    assert_eq!(BRIGHT_BLUE, Color::BrightBlue);
    assert_eq!(BRIGHT_MAGENTA, Color::BrightMagenta);
    assert_eq!(BRIGHT_CYAN, Color::BrightCyan);
    assert_eq!(BRIGHT_WHITE, Color::BrightWhite);
    assert_eq!(RESET, Color::Reset);
}

// ============================================================================
// Style String Generation Tests
// ============================================================================

#[test]
fn test_style_no_formatting() {
    let result = style(
        "hello", None, None, false, false, false, false, false, false, false, false,
    );
    assert_eq!(result, "hello");
}

#[test]
fn test_style_fg_color_only() {
    let result = style(
        "hello",
        Some(Color::Red),
        None,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
    );
    assert_eq!(result, "\x1b[31mhello\x1b[0m");
}

#[test]
fn test_style_bg_color_only() {
    let result = style(
        "hello",
        None,
        Some(Color::Blue),
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
    );
    assert_eq!(result, "\x1b[44mhello\x1b[0m");
}

#[test]
fn test_style_both_colors() {
    let result = style(
        "hello",
        Some(Color::White),
        Some(Color::Red),
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
    );
    assert!(result.starts_with("\x1b["));
    assert!(result.contains("37")); // white fg
    assert!(result.contains("41")); // red bg
    assert!(result.ends_with("\x1b[0m"));
}

#[test]
fn test_style_bold() {
    let result = style(
        "hello", None, None, true, false, false, false, false, false, false, true,
    );
    assert_eq!(result, "\x1b[1mhello\x1b[0m");
}

#[test]
fn test_style_dim() {
    let result = style(
        "hello", None, None, false, true, false, false, false, false, false, true,
    );
    assert_eq!(result, "\x1b[2mhello\x1b[0m");
}

#[test]
fn test_style_italic() {
    let result = style(
        "hello", None, None, false, false, false, false, true, false, false, true,
    );
    assert_eq!(result, "\x1b[3mhello\x1b[0m");
}

#[test]
fn test_style_underline() {
    let result = style(
        "hello", None, None, false, false, true, false, false, false, false, true,
    );
    assert_eq!(result, "\x1b[4mhello\x1b[0m");
}

#[test]
fn test_style_blink() {
    let result = style(
        "hello", None, None, false, false, false, false, false, true, false, true,
    );
    assert_eq!(result, "\x1b[5mhello\x1b[0m");
}

#[test]
fn test_style_overline() {
    let result = style(
        "hello", None, None, false, false, false, true, false, false, false, true,
    );
    assert_eq!(result, "\x1b[53mhello\x1b[0m");
}

#[test]
fn test_style_strikethrough() {
    let result = style(
        "hello", None, None, false, false, false, false, false, false, true, true,
    );
    assert_eq!(result, "\x1b[9mhello\x1b[0m");
}

#[test]
fn test_style_no_reset() {
    let result = style(
        "hello",
        Some(Color::Green),
        None,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
    );
    assert_eq!(result, "\x1b[32mhello");
    assert!(!result.contains("\x1b[0m"));
}

#[test]
fn test_style_multiple_attributes() {
    let result = style(
        "hello",
        Some(Color::Red),
        Some(Color::White),
        true, // bold
        false,
        true, // underline
        false,
        false,
        false,
        false,
        true, // reset
    );

    // Should contain bold (1), underline (4), red fg (31), white bg (47)
    assert!(result.starts_with("\x1b["));
    assert!(result.contains("1"));
    assert!(result.contains("4"));
    assert!(result.contains("31"));
    assert!(result.contains("47"));
    assert!(result.contains("hello"));
    assert!(result.ends_with("\x1b[0m"));
}

#[test]
fn test_style_all_attributes() {
    let result = style(
        "test",
        Some(Color::Blue),
        Some(Color::Yellow),
        true, // bold
        true, // dim
        true, // underline
        true, // overline
        true, // italic
        true, // blink
        true, // strikethrough
        true, // reset
    );

    assert!(result.starts_with("\x1b["));
    assert!(result.contains("1")); // bold
    assert!(result.contains("2")); // dim
    assert!(result.contains("3")); // italic
    assert!(result.contains("4")); // underline
    assert!(result.contains("5")); // blink
    assert!(result.contains("9")); // strikethrough
    assert!(result.contains("53")); // overline
    assert!(result.contains("34")); // blue fg
    assert!(result.contains("43")); // yellow bg
    assert!(result.ends_with("\x1b[0m"));
}

// ============================================================================
// Strip ANSI Codes Tests
// ============================================================================

#[test]
fn test_strip_ansi_plain_text() {
    let result = strip_ansi_codes("hello world");
    assert_eq!(result, "hello world");
}

#[test]
fn test_strip_ansi_simple_color() {
    let result = strip_ansi_codes("\x1b[31mred\x1b[0m");
    assert_eq!(result, "red");
}

#[test]
fn test_strip_ansi_multiple_codes() {
    let result = strip_ansi_codes("\x1b[1;31;40mbold red on black\x1b[0m");
    assert_eq!(result, "bold red on black");
}

#[test]
fn test_strip_ansi_mixed_content() {
    let result = strip_ansi_codes("before \x1b[32mgreen\x1b[0m after");
    assert_eq!(result, "before green after");
}

#[test]
fn test_strip_ansi_nested() {
    let result = strip_ansi_codes("\x1b[1m\x1b[31mred\x1b[0m\x1b[32mgreen\x1b[0m");
    assert_eq!(result, "redgreen");
}

#[test]
fn test_strip_ansi_empty_string() {
    let result = strip_ansi_codes("");
    assert_eq!(result, "");
}

#[test]
fn test_strip_ansi_only_codes() {
    let result = strip_ansi_codes("\x1b[31m\x1b[0m");
    assert_eq!(result, "");
}

// ============================================================================
// Progress Bar Tests
// ============================================================================

#[test]
fn test_progress_bar_render_initial() {
    let bar = ProgressBar::new(100, Some("Processing"), false, true, true, 20);
    let output = bar.render();

    assert!(output.contains("Processing"));
    assert!(output.contains("["));
    assert!(output.contains("]"));
    assert!(output.contains("0%"));
    assert!(output.contains("0/100"));
}

#[test]
fn test_progress_bar_render_half() {
    let mut bar = ProgressBar::new(100, None, false, true, true, 20);
    bar.set_position(50);
    let output = bar.render();

    assert!(output.contains("50%"));
    assert!(output.contains("50/100"));
}

#[test]
fn test_progress_bar_render_complete() {
    let mut bar = ProgressBar::new(100, None, false, true, true, 20);
    bar.finish();
    let output = bar.render();

    assert!(output.contains("100%"));
    assert!(output.contains("100/100"));
}

#[test]
fn test_progress_bar_update_incremental() {
    let mut bar = ProgressBar::new(100, None, false, true, false, 20);

    bar.update(25);
    let output1 = bar.render();
    assert!(output1.contains("25%"));

    bar.update(25);
    let output2 = bar.render();
    assert!(output2.contains("50%"));

    bar.update(50);
    let output3 = bar.render();
    assert!(output3.contains("100%"));
}

#[test]
fn test_progress_bar_zero_length() {
    let bar = ProgressBar::new(0, None, false, true, false, 20);
    let output = bar.render();

    // Should not panic and should show 0%
    assert!(output.contains("0%"));
}

#[test]
fn test_progress_bar_no_label() {
    let bar = ProgressBar::new(100, None, false, true, true, 20);
    let output = bar.render();

    // Should start with the bar
    assert!(output.starts_with("["));
}

#[test]
fn test_progress_bar_bar_chars() {
    let mut bar = ProgressBar::new(10, None, false, false, false, 10);
    bar.set_position(5);
    let output = bar.render();

    // Bar should have 5 filled and 5 empty
    assert!(output.contains("#####-----"));
}

#[test]
fn test_progress_bar_width() {
    let bar = ProgressBar::new(100, None, false, false, false, 30);
    let output = bar.render();

    // Count the dashes (should be 30 for 0% progress)
    let bar_content: String = output
        .chars()
        .skip_while(|c| *c != '[')
        .skip(1)
        .take_while(|c| *c != ']')
        .collect();

    assert_eq!(bar_content.len(), 30);
}

#[test]
fn test_progress_bar_overflow_protection() {
    let mut bar = ProgressBar::new(100, None, false, true, true, 20);

    // Try to update beyond the length
    bar.update(150);
    let output = bar.render();

    // Should cap at 100%
    assert!(output.contains("100%"));
    assert!(output.contains("100/100"));
}

#[test]
fn test_progress_bar_finish_idempotent() {
    let mut bar = ProgressBar::new(100, None, false, true, false, 20);

    bar.finish();
    let output1 = bar.render();

    bar.finish(); // Second call should be no-op
    let output2 = bar.render();

    assert_eq!(output1, output2);
}

#[test]
fn test_progress_bar_update_after_finish() {
    let mut bar = ProgressBar::new(100, None, false, true, true, 20);

    bar.finish();
    bar.update(10); // Should be ignored

    let output = bar.render();
    assert!(output.contains("100%"));
    assert!(output.contains("100/100"));
}

// ============================================================================
// Terminal Size Tests
// ============================================================================

#[test]
fn test_get_terminal_size_returns_valid() {
    let (width, height) = get_terminal_size();

    // Should return positive values
    assert!(width > 0, "Terminal width should be positive");
    assert!(height > 0, "Terminal height should be positive");

    // Should return reasonable values (not absurdly large)
    assert!(width < 10000, "Terminal width should be reasonable");
    assert!(height < 10000, "Terminal height should be reasonable");
}

// ============================================================================
// Color Equality Tests
// ============================================================================

#[test]
fn test_color_equality() {
    assert_eq!(Color::Red, Color::Red);
    assert_ne!(Color::Red, Color::Blue);
    assert_eq!(Color::BrightRed, Color::BrightRed);
    assert_ne!(Color::Red, Color::BrightRed);
}

#[test]
fn test_color_clone() {
    let color = Color::Green;
    let cloned = color;
    assert_eq!(color, cloned);
}

#[test]
fn test_color_debug() {
    let debug_str = format!("{:?}", Color::Red);
    assert_eq!(debug_str, "Red");
}
