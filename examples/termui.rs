//! Terminal UI demo - Rust port of Python Click's termui.py example.
//!
//! This script showcases different terminal UI helpers in click-rs.
//!
//! Run with: cargo run --example termui -- <subcommand>
//! Available subcommands: colordemo, pager, progress, open, locate, edit, clear, pause, menu

use std::thread;
use std::time::Duration;

use click::launch;
use click::{
    clear, echo, edit_text, getchar, pause, style, Argument, ClickOption, Color, Command,
    CommandLike, Group, ProgressBar,
};

fn main() {
    let cli = Group::new("termui")
        .help("This script showcases different terminal UI helpers in click-rs.")
        .command(colordemo_cmd())
        .command(pager_cmd())
        .command(progress_cmd())
        .command(open_cmd())
        .command(locate_cmd())
        .command(edit_cmd())
        .command(clear_cmd())
        .command(pause_cmd())
        .command(menu_cmd())
        .build();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = CommandLike::main(&cli, args) {
        let code = e.exit_code();
        if code == 0 {
            // Help was requested - already printed by make_context flow
            // For groups we need to print help manually since Group::main doesn't do it
            let ctx = click::ContextBuilder::new().info_name("termui").build();
            println!("{}", cli.get_help(&ctx));
        } else {
            eprintln!("{}", e.format_full());
        }
        std::process::exit(code);
    }
}

/// Demonstrates ANSI color support.
fn colordemo_cmd() -> Command {
    Command::new("colordemo")
        .help("Demonstrates ANSI color support.")
        .callback(|_ctx| {
            for color in ["red", "green", "blue"] {
                let fg_color = match color {
                    "red" => Color::Red,
                    "green" => Color::Green,
                    "blue" => Color::Blue,
                    _ => unreachable!(),
                };

                // Foreground colored
                let styled_fg = style(
                    &format!("I am colored {}", color),
                    Some(fg_color),
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
                echo(&styled_fg, true, false, None);

                // Background colored
                let styled_bg = style(
                    &format!("I am background colored {}", color),
                    None,
                    Some(fg_color),
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    true,
                );
                echo(&styled_bg, true, false, None);
            }
            Ok(())
        })
        .build()
}

/// Demonstrates using the pager.
fn pager_cmd() -> Command {
    Command::new("pager")
        .help("Demonstrates using the pager.")
        .callback(|_ctx| {
            let mut lines = Vec::new();
            for x in 0..200 {
                let num_styled = style(
                    &x.to_string(),
                    Some(Color::Green),
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
                lines.push(format!("{}. Hello World!", num_styled));
            }
            click::termui::echo_via_pager(&lines.join("\n"), None);
            Ok(())
        })
        .build()
}

/// Demonstrates the progress bar.
fn progress_cmd() -> Command {
    Command::new("progress")
        .help("Demonstrates the progress bar.")
        .option(
            ClickOption::new(&["--count"])
                .default("8000")
                .help("The number of items to process.")
                .build(),
        )
        .callback(|ctx| {
            let count_str = ctx
                .get_param::<String>("count")
                .map(|s| s.as_str())
                .unwrap_or("8000");
            let count: usize = count_str.parse().unwrap_or(8000);
            let count = count.clamp(1, 100000);

            // Simulated slow processing
            fn process_slowly() {
                let delay_ms = (rand_simple() * 2.0) as u64;
                thread::sleep(Duration::from_millis(delay_ms));
            }

            // Simple pseudo-random number generator (0.0 to 1.0)
            fn rand_simple() -> f64 {
                use std::time::SystemTime;
                let nanos = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();
                (nanos % 1000) as f64 / 1000.0
            }

            // Progress bar 1: Processing accounts with custom characters
            {
                echo(
                    &format!("Processing {} accounts...", count),
                    true,
                    false,
                    None,
                );
                let mut bar =
                    ProgressBar::new(count, Some("Processing accounts"), true, true, true, 30)
                        .fill_char('█')
                        .empty_char('░');
                for _ in 0..count {
                    process_slowly();
                    bar.update(1);
                }
                bar.finish();
            }

            // Progress bar 2: Committing transaction (with item display)
            {
                let filtered_count = (count as f64 * 0.7) as usize; // ~70% pass filter
                echo(
                    &format!("\nCommitting ~{} transactions...", filtered_count),
                    true,
                    false,
                    None,
                );
                let mut bar = ProgressBar::new(
                    filtered_count,
                    Some("Committing transaction"),
                    true,
                    true,
                    true,
                    30,
                );
                let mut processed = 0;
                for _i in 0..count {
                    if rand_simple() > 0.3 {
                        process_slowly();
                        processed += 1;
                        bar.update(1);
                        if processed >= filtered_count {
                            break;
                        }
                    }
                }
                bar.finish();
            }

            // Progress bar 3: Counting with custom characters
            {
                echo(
                    &format!("\nCounting {} items with custom characters...", count),
                    true,
                    false,
                    None,
                );
                let mut bar = ProgressBar::new(count, Some("Counting"), true, true, true, 30)
                    .fill_char('=')
                    .empty_char(' ');
                for _ in 0..count {
                    process_slowly();
                    bar.update(1);
                }
                bar.finish();
            }

            // Progress bar 4: Minimal (no percent, no ETA)
            {
                echo(
                    &format!("\nMinimal progress bar ({} items)...", count),
                    true,
                    false,
                    None,
                );
                let mut bar = ProgressBar::new(count, None, false, false, false, 30);
                for _ in 0..count {
                    process_slowly();
                    bar.update(1);
                }
                bar.finish();
            }

            // Progress bar 5: Non-linear progress
            {
                let steps: Vec<f64> = (0..20)
                    .map(|x| (x as f64 * 1.0 / 20.0).exp() - 1.0)
                    .collect();
                let total: f64 = steps.iter().sum();
                let total_count = total as usize;

                echo(
                    &format!("\nSlowing progress bar ({} steps)...", total_count),
                    true,
                    false,
                    None,
                );
                let mut bar = ProgressBar::new(
                    total_count,
                    Some("Slowing progress bar"),
                    true,
                    false,
                    true,
                    30,
                );

                for step in steps {
                    let sleep_ms = (step * 1000.0) as u64;
                    thread::sleep(Duration::from_millis(sleep_ms));
                    bar.update(step as usize);
                }
                bar.finish();
            }

            Ok(())
        })
        .build()
}

/// Opens a file or URL in the default application.
fn open_cmd() -> Command {
    Command::new("open")
        .help("Opens a file or URL in the default application.")
        .argument(
            Argument::new("url")
                .help("URL or file path to open")
                .build(),
        )
        .callback(|ctx| {
            let url = ctx
                .get_param::<String>("url")
                .ok_or_else(|| click::ClickError::usage("URL argument is required"))?;
            launch(url, false, false)?;
            Ok(())
        })
        .build()
}

/// Opens a file location in the file manager.
fn locate_cmd() -> Command {
    Command::new("locate")
        .help("Opens the file location in a file manager.")
        .argument(Argument::new("url").help("File path to locate").build())
        .callback(|ctx| {
            let url = ctx
                .get_param::<String>("url")
                .ok_or_else(|| click::ClickError::usage("URL argument is required"))?;
            launch(url, false, true)?;
            Ok(())
        })
        .build()
}

/// Opens an editor with some text in it.
fn edit_cmd() -> Command {
    Command::new("edit")
        .help("Opens an editor with some text in it.")
        .callback(|_ctx| {
            let marker = "# Everything below is ignored\n";
            let initial_text = format!("\n\n{}", marker);

            match edit_text(Some(&initial_text), None, "txt", true)? {
                Some(message) => {
                    // Split at marker and take content before it
                    let msg = message
                        .split(marker)
                        .next()
                        .unwrap_or("")
                        .trim_end_matches('\n');

                    if msg.is_empty() {
                        echo("Empty message!", true, false, None);
                    } else {
                        echo(&format!("Message:\n{}", msg), true, false, None);
                    }
                }
                None => {
                    echo("You did not enter anything!", true, false, None);
                }
            }
            Ok(())
        })
        .build()
}

/// Clears the entire screen.
fn clear_cmd() -> Command {
    Command::new("clear")
        .help("Clears the entire screen.")
        .callback(|_ctx| {
            clear();
            Ok(())
        })
        .build()
}

/// Waits for the user to press a button.
fn pause_cmd() -> Command {
    Command::new("pause")
        .help("Waits for the user to press a button.")
        .callback(|_ctx| {
            pause(None);
            Ok(())
        })
        .build()
}

/// Shows a simple menu.
fn menu_cmd() -> Command {
    Command::new("menu")
        .help("Shows a simple menu.")
        .callback(|_ctx| {
            let mut menu = "main";

            loop {
                match menu {
                    "main" => {
                        echo("Main menu:", true, false, None);
                        echo("  d: debug menu", true, false, None);
                        echo("  q: quit", true, false, None);

                        let c = getchar(false)?;
                        match c {
                            'd' => menu = "debug",
                            'q' => menu = "quit",
                            _ => echo("Invalid input", true, false, None),
                        }
                    }
                    "debug" => {
                        echo("Debug menu", true, false, None);
                        echo("  b: back", true, false, None);

                        let c = getchar(false)?;
                        if c == 'b' {
                            menu = "main";
                        } else {
                            echo("Invalid input", true, false, None);
                        }
                    }
                    "quit" => return Ok(()),
                    _ => unreachable!(),
                }
            }
        })
        .build()
}
