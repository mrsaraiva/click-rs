//! Colors example - demonstrates terminal color output.
//!
//! This example shows styled terminal output that automatically strips
//! ANSI codes when output is piped to a file.
//!
//! Equivalent to Python Click's examples/colors/colors.py

use click::{echo, style, Color, Command, Context, Result};

/// All available colors to demonstrate.
const ALL_COLORS: &[(&str, Color)] = &[
    ("black", Color::Black),
    ("red", Color::Red),
    ("green", Color::Green),
    ("yellow", Color::Yellow),
    ("blue", Color::Blue),
    ("magenta", Color::Magenta),
    ("cyan", Color::Cyan),
    ("white", Color::White),
    ("bright_black", Color::BrightBlack),
    ("bright_red", Color::BrightRed),
    ("bright_green", Color::BrightGreen),
    ("bright_yellow", Color::BrightYellow),
    ("bright_blue", Color::BrightBlue),
    ("bright_magenta", Color::BrightMagenta),
    ("bright_cyan", Color::BrightCyan),
    ("bright_white", Color::BrightWhite),
];

/// Build the colors command.
fn build_command() -> Command {
    Command::new("colors")
        .help(
            "This script prints some colors. It will also automatically remove \
            all ANSI styles if data is piped into a file.\n\n\
            Give it a try!",
        )
        .callback(cli_callback)
        .build()
}

/// The callback that prints colored text.
fn cli_callback(_ctx: &Context) -> Result<()> {
    // Print each color with foreground only
    for (name, color) in ALL_COLORS {
        let text = format!("I am colored {}", name);
        let styled = style(
            &text,
            Some(*color),
            None,  // bg
            false, // bold
            false, // dim
            false, // underline
            false, // overline
            false, // italic
            false, // blink
            false, // strikethrough
            true,  // reset
        );
        echo(&styled, true, false, None);
    }

    // Print each color bold
    for (name, color) in ALL_COLORS {
        let text = format!("I am colored {} and bold", name);
        let styled = style(
            &text,
            Some(*color),
            None,  // bg
            true,  // bold
            false, // dim
            false, // underline
            false, // overline
            false, // italic
            false, // blink
            false, // strikethrough
            true,  // reset
        );
        echo(&styled, true, false, None);
    }

    // Print each color in reverse
    for (name, color) in ALL_COLORS {
        let text = format!("I am reverse colored {}", name);
        // Reverse is simulated by swapping fg and bg
        // Since the original Python uses reverse=True, we approximate with bg color
        let styled = style(
            &text,
            None,         // fg (default)
            Some(*color), // bg
            false,        // bold
            false,        // dim
            false,        // underline
            false,        // overline
            false,        // italic
            false,        // blink
            false,        // strikethrough
            true,         // reset
        );
        echo(&styled, true, false, None);
    }

    // Print blinking text
    let blinking = style(
        "I am blinking",
        None,
        None,
        false, // bold
        false, // dim
        false, // underline
        false, // overline
        false, // italic
        true,  // blink
        false, // strikethrough
        true,  // reset
    );
    echo(&blinking, true, false, None);

    // Print underlined text
    let underlined = style(
        "I am underlined",
        None,
        None,
        false, // bold
        false, // dim
        true,  // underline
        false, // overline
        false, // italic
        false, // blink
        false, // strikethrough
        true,  // reset
    );
    echo(&underlined, true, false, None);

    Ok(())
}

fn main() {
    let cmd = build_command();
    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Err(e) = cmd.main(args) {
        eprintln!("{}", e.format_full());
        std::process::exit(e.exit_code());
    }
}
