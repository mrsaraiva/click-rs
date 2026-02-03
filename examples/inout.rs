//! InOut example - cat-like file concatenation tool.
//!
//! This example demonstrates file handling with stdin/stdout support.
//!
//! Equivalent to Python Click's examples/inout/inout.py

use std::io::{Read, Write};

use click::{
    Argument, ClickError, Command, Context, FileMode, LazyFile, Nargs, Result, TypeConverter,
};

/// Build the inout command.
fn build_command() -> Command {
    Command::new("inout")
        .help(
            "This script works similar to the Unix `cat` command but it writes \
            into a specific file (which could be the standard output as denoted by \
            the `-` sign).\n\n\
            Copy stdin to stdout:\n    inout - -\n\n\
            Copy foo.txt and bar.txt to stdout:\n    inout foo.txt bar.txt -\n\n\
            Write stdin into the file foo.txt:\n    inout - foo.txt",
        )
        .argument(
            Argument::new("input")
                .nargs(Nargs::Variadic)
                .help("Input files to read from (use - for stdin)")
                .build(),
        )
        .argument(
            Argument::new("output")
                .help("Output file to write to (use - for stdout)")
                .build(),
        )
        .callback(cli_callback)
        .build()
}

/// Open a file for reading (supports "-" for stdin).
fn open_input(path: &str) -> Result<LazyFile> {
    let file_type = click::FileType::new().mode(FileMode::Read);
    file_type.convert(path).map_err(|e| ClickError::usage(e))
}

/// Open a file for writing (supports "-" for stdout).
fn open_output(path: &str) -> Result<LazyFile> {
    let file_type = click::FileType::new().mode(FileMode::Write);
    file_type.convert(path).map_err(|e| ClickError::usage(e))
}

/// The callback that performs the file copying.
fn cli_callback(ctx: &Context) -> Result<()> {
    // Get input files as paths
    let input_paths = ctx
        .get_param::<Vec<String>>("input")
        .cloned()
        .unwrap_or_default();

    let output_path = ctx
        .get_param::<String>("output")
        .cloned()
        .ok_or_else(|| ClickError::missing_argument("OUTPUT"))?;

    // Open output file
    let mut output = open_output(&output_path)?;

    // Process each input file
    for input_path in &input_paths {
        let mut input = open_input(input_path)?;

        // Read and write in chunks
        let mut buffer = [0u8; 1024];
        loop {
            let bytes_read = input
                .read(&mut buffer)
                .map_err(|e| ClickError::usage(format!("Error reading '{}': {}", input_path, e)))?;

            if bytes_read == 0 {
                break;
            }

            output.write_all(&buffer[..bytes_read]).map_err(|e| {
                ClickError::usage(format!("Error writing to '{}': {}", output_path, e))
            })?;

            output
                .flush()
                .map_err(|e| ClickError::usage(format!("Error flushing output: {}", e)))?;
        }
    }

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
