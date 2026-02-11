//! InOut example - cat-like file concatenation tool.
//!
//! Equivalent to Python Click's examples/inout/inout.py

use std::io::{Read, Write};

use click::{run, ClickError, FileMode, LazyFile, Result, TypeConverter};

fn open_input(path: &str) -> Result<LazyFile> {
    let file_type = click::FileType::new().mode(FileMode::Read);
    file_type.convert(path).map_err(ClickError::usage)
}

fn open_output(path: &str) -> Result<LazyFile> {
    let file_type = click::FileType::new().mode(FileMode::Write);
    file_type.convert(path).map_err(ClickError::usage)
}

#[click::command(name = "inout")]
fn inout(
    #[argument(
        help = "Input files to read from (use - for stdin)",
        nargs = -1
    )]
    input: Vec<String>,
    #[argument(help = "Output file to write to (use - for stdout)")] output: String,
) -> Result<()> {
    let output_path = output;
    let mut output = open_output(&output_path)?;
    for input_path in &input {
        let mut source = open_input(input_path)?;
        let mut buffer = [0u8; 1024];
        loop {
            let bytes_read = source
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
    run(&inout_command());
}
