//! Phase 4 parity tests: Decorators / Formatting
//!
//! These tests produce output matching the Python Click equivalents
//! for diff-based parity verification.

mod decorators;
mod formatting;
mod util;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <decorators|formatting|all>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "decorators" => decorators::run(),
        "formatting" => formatting::run(),
        "all" => {
            decorators::run();
            println!("\n--- FORMATTING TESTS ---\n");
            formatting::run();
        }
        other => {
            eprintln!("Unknown test module: {}", other);
            std::process::exit(1);
        }
    }
}

