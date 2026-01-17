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

### Dependencies
- `thiserror` 2.0 - Error type derivation
- `uuid` 1.0 - UUID parsing
- `chrono` 0.4 - DateTime parsing

### Planned (Phase 2)
- `Context` struct with thread-local storage
- `Parameter` trait with `Option` and `Argument` implementations

### Planned (Phase 3)
- `OptionParser` for command-line argument parsing
- `Command` and `Group` structs

### Planned (Phase 4)
- `click-derive` proc-macro crate
- `#[derive(Command)]`, `#[derive(Group)]` macros
- `HelpFormatter` for help text generation

### Planned (Phase 5)
- Terminal UI: `echo()`, `prompt()`, `confirm()`, `progressbar()`
- ANSI color and styling support

### Planned (Phase 6)
- Shell completion (Bash, Zsh, Fish)
- `CliRunner` for testing
- Utility functions
