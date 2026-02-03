//! Phase 6 parity tests: Completion / Testing

mod completion;
mod testing;
mod util;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <completion|testing|all>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "completion" => completion::run(),
        "testing" => testing::run(),
        "all" => {
            completion::run();
            println!("\n--- TESTING ---\n");
            testing::run();
        }
        other => {
            eprintln!("Unknown test module: {}", other);
            std::process::exit(1);
        }
    }
}

