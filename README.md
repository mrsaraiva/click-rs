# click-rs

[![Crates.io](https://img.shields.io/crates/v/click-rs.svg)](https://crates.io/crates/click-rs)
[![Documentation](https://docs.rs/click-rs/badge.svg)](https://docs.rs/click-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust port of Python's [Click](https://github.com/pallets/click) library for creating
beautiful command-line interfaces in a composable, declarative way.

click-rs lets you build CLIs from commands, groups, options and arguments — with automatic
help generation, prompts, colored output, shell completion, and a testing harness — using
either a derive macro or a builder API.

> **Attribution.** click-rs is a derivative work: a Rust port of the excellent
> [Click](https://github.com/pallets/click) library by the
> [Pallets](https://palletsprojects.com/) team. All credit for the original design and API
> goes to them. This project aims to bring that ergonomics to the Rust ecosystem.

## Compatibility

Works on Linux, macOS, and Windows.

**Minimum Supported Rust Version:** 1.70+

## Installing

The crate is published as `click-rs`, but its library is imported as `click`:

```toml
[dependencies]
click-rs = "1.0"
```

```rust
use click::Command;
```

The `derive` feature is enabled by default. Disable it with `default-features = false` to use
the builder API only.

## Quick Start (derive)

```rust,no_run
use click::Command;

#[derive(Command)]
#[command(name = "greet")]
/// A friendly greeter
struct Greet {
    /// Name to greet
    #[argument]
    name: String,

    /// Number of times to greet
    #[option(short, long, default = 1)]
    count: i32,
}

impl Greet {
    fn run(&self) {
        for _ in 0..self.count {
            println!("Hello, {}!", self.name);
        }
    }
}

fn main() {
    Greet::main_with(std::env::args().skip(1).collect(), |greet, _ctx| {
        greet.run();
        Ok(())
    }).unwrap();
}
```

## Features

- **Commands & groups** — compose nested subcommands with shared context
- **Options & arguments** — typed parameters with defaults, flags, multiple values
- **Automatic help** — generated usage and `--help` output
- **Prompts & I/O** — interactive prompts, confirmation, and piped input/output
- **Colored output** — styled terminal text
- **Shell completion** — generate completion scripts
- **Validation** — custom parameter validation and error reporting
- **Testing** — a `CliRunner` harness for invoking commands in tests

## Examples

The [`examples/`](examples) directory mirrors classic Click examples:

| Example | Shows |
|---------|-------|
| `naval` | The canonical Click "naval fate" multi-command app |
| `repo` | Groups, context objects, and shared state |
| `complex` | Plugin-style command loading |
| `aliases` | Command aliases |
| `colors` | Colored terminal output |
| `completion` | Shell completion generation |
| `validation` | Custom parameter validation |
| `inout` | Reading from and writing to files / stdin / stdout |
| `imagepipe` | Piping data between commands |
| `termui` | Terminal UI helpers (prompts, progress) |

Run one with:

```sh
cargo run --example naval -- --help
```

## License

Licensed under the [MIT License](LICENSE).
