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

### Planned (Phase 1)
- `ClickError` enum with full exception hierarchy
- `ParamType` trait and built-in types (STRING, INT, FLOAT, BOOL, Choice, Path, etc.)
- `ParameterSource` enum for value origin tracking

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
