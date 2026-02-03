# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project scaffolding
- `CLAUDE.md` - Claude Code guidance document
- `docs/devel/ROADMAP.md` - Development roadmap with phased task list
- Project planning documentation covering:
  - Architecture overview (Context, Command, Group, Parameter, ParamType)
  - Python → Rust mapping strategies
  - Milestones (M1-M4 + 1.0 Release)
  - Parity testing strategy
  - Platform compatibility targets (MSRV 1.70+, Linux/macOS/Windows)
  - Open design questions for Phase 1-2 resolution
- **Phase 1: Foundation** (complete)
  - `ClickError` enum with 10 variants mirroring Python Click's exception hierarchy
  - `ErrorContext` for attaching command path and "Try --help" hints
  - `TypeConverter` trait with `convert()`, `get_metavar()`, `split_envvar_value()`, `shell_complete()`
  - Built-in types: `STRING`, `INT`, `FLOAT`, `BOOL`, `UUID`
  - Range types: `IntRange`, `FloatRange` with min/max bounds and clamp support
  - `DateTime` type for ISO8601 parsing via chrono
  - `Choice` type with case-insensitive matching option
  - `PathType` with exists/readable/writable/executable validation
  - `FileType` with lazy opening and stdin/stdout ("-") support
  - `TupleType` for composite multi-value parameters
  - `UNPROCESSED` type for raw passthrough
  - `ParameterSource` enum for tracking value origin (CommandLine, Environment, Default, DefaultMap, Prompt)
  - `CompletionItem` struct for shell completion support
  - 37 unit tests + 2 doc tests
- **Phase 2: Context & Parameters** (complete)
  - `Context` struct with thread-local stack (`push_context`, `pop_context`, `get_current_context`)
  - `ContextBuilder` with parent inheritance for nested commands
  - Parameter source tracking and close callbacks
  - `Parameter` trait with `Nargs` enum (Count, Variadic, Optional)
  - `ParameterConfig` with builder pattern and deprecation support
  - `ClickOption` for named parameters (--flag, -f) with flag/bool/count modes
  - `OptionBuilder` with prompt, confirmation, and hidden input support
  - `Argument` for positional parameters with variadic support
  - `ArgumentBuilder` with automatic required/optional detection
  - 113 unit tests + 21 doc tests
- **Phase 3: Parser, Command & Group** (complete)
  - `OptionParser` for command-line argument parsing with Click-compatible behavior
  - Support for short (`-v`), long (`--verbose`), grouped (`-abc`), and equals (`--opt=val`) option syntax
  - `--` terminator for end-of-options handling
  - Levenshtein distance-based option suggestions for typos
  - `Command` struct with callback execution, help generation, and `main()` entry point
  - `CommandBuilder` for fluent command construction
  - `Group` struct for subcommand dispatch with `CommandLike` trait
  - Chain mode for executing multiple subcommands in sequence
  - Eager option processing (`--help` works before validating required params)
  - Optional positional argument handling with lookahead for required args
  - `FlagNeedsValue` support for optional option values
  - 229 unit tests + 36 doc tests
- **Phase 4: Derive Macros & Help Formatting** (complete)
  - `click-derive` proc-macro crate for declarative CLI definition
  - `#[derive(Command)]` macro for generating `Command` from structs
    - `#[command(name, help, hidden, no_args_is_help)]` container attributes
    - Automatic help text extraction from doc comments
    - `command()` and `command_with_run()` methods generated
    - `from_context()` for extracting typed values from Context
  - `#[derive(Group)]` macro for generating `Group` from structs
    - `#[group(name, chain, invoke_without_command)]` container attributes
    - Same field attributes as Command
  - Field attributes:
    - `#[option(short, long, default, required, hidden, flag, count, multiple, envvar)]`
    - `#[argument(required, hidden, multiple, default)]`
    - `#[subcommand]` (structure defined, implementation pending)
  - Rust type inference: `String`→STRING, `i32`→INT, `bool`→flag, `Vec<T>`→multiple, `Option<T>`→optional
  - `HelpFormatter` struct for terminal-aware help output
    - `write_usage()`, `write_heading()`, `write_paragraph()` methods
    - `write_definition_list()` for options/arguments formatting
    - Text wrapping with `wrap_text()` function
    - Terminal width detection via `detect_terminal_width()`
    - `truncate_text()` and `split_into_lines()` utilities
  - 244 unit tests + 37 doc tests

### Known Limitations (v1.0)
- Mixed flag/value append ordering (`--opt --opt val`) may not preserve order
- Repeated flag-as-optional (`--opt --opt`) collapses to single value in append mode
- Environment variable reading in derive macros requires parser enhancement

### Dependencies
- `thiserror` 2.0 - Error type derivation
- `uuid` 1.0 - UUID parsing
- `chrono` 0.4 - DateTime parsing
- `syn` 2.0 - Rust AST parsing (click-derive)
- `quote` 1.0 - Code generation (click-derive)
- `proc-macro2` 1.0 - Procedural macro utilities (click-derive)

### Planned (Phase 5)
- Terminal UI: `echo()`, `prompt()`, `confirm()`, `progressbar()`
- ANSI color and styling support

### Planned (Phase 6)
- Shell completion (Bash, Zsh, Fish)
- `CliRunner` for testing
- Utility functions
