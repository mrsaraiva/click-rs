//! Phase 5 parity tests: Terminal UI

mod termui;
mod util;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <termui|all>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "termui" => termui::run(),
        "all" => termui::run(),
        other => {
            eprintln!("Unknown test module: {}", other);
            std::process::exit(1);
        }
    }
}

