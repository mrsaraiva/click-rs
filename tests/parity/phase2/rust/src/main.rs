mod context;
mod parameter;

fn main() {
    let module = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    match module.as_str() {
        "context" => context::run(),
        "parameter" => parameter::run(),
        "all" => {
            context::run();
            parameter::run();
        }
        _ => {
            eprintln!("Unknown module: {}", module);
            std::process::exit(1);
        }
    }
}
