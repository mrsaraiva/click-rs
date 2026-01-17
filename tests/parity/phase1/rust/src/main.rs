mod errors;
mod types;

fn main() {
    let module = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    match module.as_str() {
        "types" => types::run(),
        "errors" => errors::run(),
        "all" => {
            types::run();
            errors::run();
        }
        _ => {
            eprintln!("Unknown module: {}", module);
            std::process::exit(1);
        }
    }
}
