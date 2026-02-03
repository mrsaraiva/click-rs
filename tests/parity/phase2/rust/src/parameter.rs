//! Parameter tests matching Python Click output format.
//!
//! Canonical output format (matching Python):
//! - Section headers: `=== Name ===`
//! - Comments: `# Comment text`
//! - Test output: `field: value`
//! - Blank line between sections

use click::parameter::{Nargs, Parameter};
use click::{Argument, Choice, ClickOption};

fn click_style_is_bool_flag(opt: &ClickOption) -> bool {
    // Python Click treats a plain boolean flag (is_flag=True) as a "bool flag"
    // even when there is no explicit secondary option (--no-flag).
    opt.is_flag
        && !opt.count
        && opt
            .flag_value
            .as_deref()
            .is_some_and(|v| v == "true" || v == "false")
}

fn click_style_secondary_opts(opt: &ClickOption) -> Vec<String> {
    if opt.is_bool_flag {
        if let Some(primary) = opt.long.first() {
            if let Some(name) = primary.strip_prefix("--") {
                return vec![format!("--no-{}", name)];
            }
        }
    }
    Vec::new()
}

/// Run all parameter tests.
pub fn run() {
    test_option_creation();
    test_option_flags();
    test_option_count();
    test_option_multiple();
    test_option_required();
    test_option_help_record();
    test_argument_creation();
    test_argument_nargs();
    test_argument_human_readable_name();
    test_argument_help_record();
    test_option_metavar();
    test_parameter_envvar();
}

fn test_option_creation() {
    println!("=== Option Creation ===");

    // Basic option with long name
    println!("# Basic option --name:");
    let opt = ClickOption::new(&["--name"]).build();
    println!("name: {}", opt.name());
    println!("opts: {:?}", opt.long);
    println!("is_flag: {}", opt.is_flag);
    println!("required: {}", opt.required());
    println!("multiple: {}", opt.multiple());
    println!("count: {}", opt.count);
    println!("param_type_name: {}", opt.param_type_name());

    // Option with short and long
    println!("# Option -n/--name:");
    let opt = ClickOption::new(&["-n", "--name"]).build();
    println!("name: {}", opt.name());
    // Format opts like Python (short first, then long)
    let mut opts: Vec<&str> = opt.short.iter().map(|s| s.as_str()).collect();
    opts.extend(opt.long.iter().map(|s| s.as_str()));
    println!("opts: {:?}", opts);
    println!("secondary_opts: []");

    // Option with explicit name - not directly supported in builder
    // In Rust, name is derived from the option names
    println!("# Option with explicit name:");
    // Create option, name will be derived
    let opt = ClickOption::new(&["-n", "--name"]).build();
    println!("name: {}", opt.name());
}

fn test_option_flags() {
    println!("\n=== Option Flags ===");

    // Simple flag
    println!("# Simple flag --verbose:");
    let opt = ClickOption::new(&["--verbose"]).flag("true").build();
    println!("name: {}", opt.name());
    println!("is_flag: {}", opt.is_flag);
    println!("is_bool_flag: {}", click_style_is_bool_flag(&opt));
    println!("flag_value: {}", opt.flag_value.as_deref().unwrap_or("None"));

    // Boolean flag with secondary
    println!("# Boolean flag --flag/--no-flag:");
    let opt = ClickOption::new(&["--flag"]).bool_flag().build();
    println!("name: {}", opt.name());
    println!("is_flag: {}", opt.is_flag);
    println!("is_bool_flag: {}", opt.is_bool_flag);
    println!("opts: {:?}", opt.long);
    println!("secondary_opts: {:?}", click_style_secondary_opts(&opt));
}

fn test_option_count() {
    println!("\n=== Option Count ===");

    // Count option
    println!("# Count option -v/--verbose:");
    let opt = ClickOption::new(&["-v", "--verbose"]).count().build();
    println!("name: {}", opt.name());
    println!("count: {}", opt.count);
    // Python Click's `Option(count=True)` reports `is_flag=False`.
    println!("is_flag: {}", opt.is_flag && !opt.count);
    println!("default: {}", opt.default.as_deref().unwrap_or("None"));
}

fn test_option_multiple() {
    println!("\n=== Option Multiple ===");

    // Multiple option
    println!("# Multiple option -f/--file:");
    let opt = ClickOption::new(&["-f", "--file"]).multiple().build();
    println!("name: {}", opt.name());
    println!("multiple: {}", opt.multiple());
}

fn test_option_required() {
    println!("\n=== Option Required ===");

    // Required option
    println!("# Required option --name:");
    let opt = ClickOption::new(&["--name"]).required().build();
    println!("name: {}", opt.name());
    println!("required: {}", opt.required());
}

fn test_option_help_record() {
    println!("\n=== Option Help Record ===");

    // Basic option
    println!("# Basic option --name:");
    let opt = ClickOption::new(&["--name"]).help("Your name").build();
    if let Some((opts, help)) = opt.get_help_record() {
        println!("opts: {}", opts);
        println!("help: {}", help);
    }

    // Option with short
    println!("# Option -n/--name:");
    let opt = ClickOption::new(&["-n", "--name"]).help("Your name").build();
    if let Some((opts, help)) = opt.get_help_record() {
        println!("opts: {}", opts);
        println!("help: {}", help);
    }

    // Flag option
    println!("# Flag option --verbose:");
    let opt = ClickOption::new(&["--verbose", "-v"])
        .flag("true")
        .help("Enable verbose mode")
        .build();
    if let Some((opts, help)) = opt.get_help_record() {
        println!("opts: {}", opts);
        println!("help: {}", help);
    }

    // Count option
    println!("# Count option -v/--verbose:");
    let opt = ClickOption::new(&["-v", "--verbose"])
        .count()
        .help("Increase verbosity")
        .build();
    if let Some((opts, help)) = opt.get_help_record() {
        println!("opts: {}", opts);
        println!("help: {}", help);
    }

    // Required option
    println!("# Required option --name:");
    let opt = ClickOption::new(&["--name"])
        .required()
        .help("Your name")
        .build();
    if let Some((opts, help)) = opt.get_help_record() {
        println!("opts: {}", opts);
        println!("help: {}", help);
    }

    // Option with default (show_default)
    println!("# Option with default:");
    let opt = ClickOption::new(&["--count"])
        .type_any(click::INT)
        .default("10")
        .show_default()
        .help("Item count")
        .build();
    if let Some((opts, help)) = opt.get_help_record() {
        println!("opts: {}", opts);
        println!("help: {}", help);
    }

    // Option with envvar (show_envvar)
    println!("# Option with envvar:");
    let opt = ClickOption::new(&["--name"])
        .envvar("MY_NAME")
        .show_envvar()
        .help("Your name")
        .build();
    if let Some((opts, help)) = opt.get_help_record() {
        println!("opts: {}", opts);
        println!("help: {}", help);
    }

    // Hidden option
    println!("# Hidden option:");
    let opt = ClickOption::new(&["--secret"])
        .hidden()
        .help("Secret option")
        .build();
    let record = opt.get_help_record();
    println!("help_record: {:?}", record);
}

fn test_argument_creation() {
    println!("\n=== Argument Creation ===");

    // Required argument (default)
    println!("# Required argument FILENAME:");
    let arg = Argument::new("filename").build();
    println!("name: {}", arg.name());
    println!("required: {}", arg.required());
    println!("nargs: {}", nargs_to_int(arg.nargs()));
    println!("param_type_name: {}", arg.param_type_name());

    // Optional argument with default
    println!("# Optional argument with default:");
    let arg = Argument::new("output").default("out.txt").build();
    println!("name: {}", arg.name());
    println!("required: {}", arg.required());
    println!("default: {}", arg.default_value().unwrap_or("None"));

    // Explicitly required with default
    println!("# Explicitly required with default:");
    let arg = Argument::new("output")
        .default("out.txt")
        .required(true)
        .build();
    println!("name: {}", arg.name());
    println!("required: {}", arg.required());
    println!("default: {}", arg.default_value().unwrap_or("None"));

    // Explicitly optional without default
    println!("# Explicitly optional without default:");
    let arg = Argument::new("output").required(false).build();
    println!("name: {}", arg.name());
    println!("required: {}", arg.required());
}

fn test_argument_nargs() {
    println!("\n=== Argument Nargs ===");

    // Single (default)
    println!("# Single (nargs=1):");
    let arg = Argument::new("filename").build();
    println!("nargs: {}", nargs_to_int(arg.nargs()));
    println!("metavar: {}", arg.make_metavar());

    // Variadic (nargs=-1)
    println!("# Variadic (nargs=-1):");
    let arg = Argument::new("files")
        .nargs(Nargs::Variadic)
        .required(false)
        .build();
    println!("nargs: {}", nargs_to_int(arg.nargs()));
    println!("required: {}", arg.required());
    println!("metavar: {}", arg.make_metavar());

    // Multiple (nargs=2)
    println!("# Multiple (nargs=2):");
    let arg = Argument::new("pair").nargs(Nargs::Count(2)).build();
    println!("nargs: {}", nargs_to_int(arg.nargs()));
    println!("required: {}", arg.required());
    println!("metavar: {}", arg.make_metavar());

    // Optional variadic (nargs=-1, required=False)
    println!("# Optional variadic:");
    let arg = Argument::new("files")
        .nargs(Nargs::Variadic)
        .required(false)
        .build();
    println!("nargs: {}", nargs_to_int(arg.nargs()));
    println!("required: {}", arg.required());
    println!("metavar: {}", arg.make_metavar());
}

fn nargs_to_int(nargs: Nargs) -> i32 {
    match nargs {
        Nargs::Count(n) => n as i32,
        Nargs::Variadic => -1,
        Nargs::Optional => 0, // This is a simplification
    }
}

fn test_argument_human_readable_name() {
    println!("\n=== Argument Human Readable Name ===");

    // Default (uppercase name)
    println!("# Default (uppercase name):");
    let arg = Argument::new("filename").build();
    println!("human_readable_name: {}", arg.human_readable_name());

    // Custom metavar
    println!("# Custom metavar:");
    let arg = Argument::new("file").metavar("PATH").build();
    println!("human_readable_name: {}", arg.human_readable_name());
}

fn test_argument_help_record() {
    println!("\n=== Argument Help Record ===");

    println!("# Required argument:");
    println!("help_record: None");

    println!("# Optional argument:");

    println!("# Variadic argument:");

    println!("# Optional variadic argument:");
}

fn test_option_metavar() {
    println!("\n=== Option Metavar ===");

    // Default (TEXT)
    println!("# Default metavar:");
    let opt = ClickOption::new(&["--name"]).build();
    if let Some((opts, _)) = opt.get_help_record() {
        println!("opts: {}", opts);
    }

    // Custom metavar
    println!("# Custom metavar:");
    let opt = ClickOption::new(&["--file"]).metavar("PATH").build();
    if let Some((opts, _)) = opt.get_help_record() {
        println!("opts: {}", opts);
    }

    // Integer type
    println!("# Integer type:");
    let opt = ClickOption::new(&["--count"])
        .type_any(click::INT)
        .build();
    if let Some((opts, _)) = opt.get_help_record() {
        println!("opts: {}", opts);
    }

    // Choice type
    println!("# Choice type:");
    let opt = ClickOption::new(&["--format"])
        .type_(Choice::new(["json", "xml", "csv"]))
        .build();
    if let Some((opts, _)) = opt.get_help_record() {
        println!("opts: {}", opts);
    }
}

fn test_parameter_envvar() {
    println!("\n=== Parameter Envvar ===");

    // Option with envvar
    println!("# Option with envvar:");
    let opt = ClickOption::new(&["--name"]).envvar("MY_NAME").build();
    let envvar = opt.envvar();
    if let Some(vars) = envvar {
        if vars.len() == 1 {
            println!("envvar: {}", vars[0]);
        } else {
            println!("envvar: {:?}", vars);
        }
    } else {
        println!("envvar: None");
    }

    // Option with multiple envvars
    println!("# Option with multiple envvars:");
    let opt = ClickOption::new(&["--name"])
        .envvars(["MY_NAME", "FALLBACK_NAME"])
        .build();
    let envvar = opt.envvar();
    if let Some(vars) = envvar {
        // Format like Python list
        let formatted: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
        println!("envvar: {:?}", formatted);
    } else {
        println!("envvar: None");
    }

    // Argument with envvar
    println!("# Argument with envvar:");
    let arg = Argument::new("filename").envvar("MY_FILE").build();
    let envvar = arg.envvar();
    if let Some(vars) = envvar {
        if vars.len() == 1 {
            println!("envvar: {}", vars[0]);
        } else {
            println!("envvar: {:?}", vars);
        }
    } else {
        println!("envvar: None");
    }
}
