//! Phase 3 parity tests: Parser, Command, Group
//!
//! These tests produce output matching the Python Click equivalents
//! for diff-based parity verification.

mod command;
mod group;
mod parser;
mod util;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <parser|command|group|all>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "parser" => parser::run(),
        "command" => command::run(),
        "group" => group::run(),
        "all" => {
            parser::run();
            println!("\n--- COMMAND TESTS ---\n");
            command::run();
            println!("\n--- GROUP TESTS ---\n");
            group::run();
        }
        other => {
            eprintln!("Unknown test module: {}", other);
            std::process::exit(1);
        }
    }
}
