# Click-rs Development Roadmap

A comprehensive task list for porting Python Click to Rust. Reference: `/home/msaraiva/dev/mark/Proj/Libs/click`

*Note: This is planning documentation. Implementation may diverge as architectural decisions are finalized.*

## Current Status

**Last Updated:** 2026-02-03

**Project State:** Phases 1–6 complete (Milestone M4 achieved). Parity suites for phases 1–6 pass via `tests/parity/run_parity.sh`.

**Next Milestone:** M5: Cross-platform `CliRunner` output capture (must-capture on Linux/macOS/Windows) + behavior gap hardening.

**Notes:** Some roadmap items are implemented with slightly different Rust APIs than the Python references. See individual phase tables for remaining gaps.

## Milestones

| Milestone | Target | Criteria |
|-----------|--------|----------|
| **M1: Core Types** | Phase 1 complete | Error types, ParamType trait, all built-in types pass parity tests |
| **M2: Minimal CLI** | Phase 2-3 complete | Can parse args, execute commands, display help (no macros) |
| **M3: Derive Macros** | Phase 4 complete | `#[derive(Command)]` works, parity with Click's decorator API |
| **M4: Full Port** | Phase 5-6 complete | Terminal UI, shell completion, CliRunner all functional |
| **M5: Cross-Platform Runner** | Phase 7 complete | `CliRunner` must-capture stdout/stderr on Linux/macOS/Windows + CI matrix |
| **1.0 Release** | All phases + docs | API stable, comprehensive tests, published to crates.io |

## Parity Testing Strategy

**Approach:** Mirror the rich-rs parity testing model. Each module has equivalent Python and Rust test programs producing identical deterministic output.

**Structure:**
```
tests/parity/
├── phase1/
│   ├── python/     # Python test scripts
│   └── rust/       # Rust binary crate
├── phase2/
│   └── ...
└── run_parity.sh   # Diff-based comparison
```

**Pass/Fail Criteria:**
- Output must match exactly (no floating-point tolerance, deterministic ordering)
- Edge cases from Click's test suite must be covered
- Each ParamType must handle: valid input, invalid input, missing value, envvar
- Commands must handle: help generation, error messages, exit codes

**Reference:** See `/home/msaraiva/dev/mark/Proj/Libs/rich-rs/tests/parity/` for the parity testing model. Key pattern: Python and Rust programs print identical formatted output, compared via `diff`.

**Python Click version pin:** Parity tests are pinned to **Click 8.3.1** via `tests/parity/requirements.txt`, and `tests/parity/run_parity.sh` runs them inside a dedicated venv at `tests/parity/.venv`.

## Platform & Compatibility

| Aspect | Target |
|--------|--------|
| **MSRV** | Rust 1.70+ (edition 2021) |
| **Platforms** | Linux, macOS, Windows 10+ |
| **Terminal** | Any terminal supporting ANSI escape codes; fallback for Windows legacy console |
| **Click version** | Behavioral parity with Click 8.3.1 (pinned for parity tests) |

## Open Design Questions

These will be resolved during Phase 1-2 implementation. Decisions will be documented in `docs/devel/DECISIONS.md` as they are made.

1. **Context ownership:** Should `Context` own params or borrow them? Likely owned for simplicity.
2. **Param lifetimes:** Should command structs require `'static` bounds? Probably yes for derive macro ergonomics.
3. **Callback return types:** Support `Result<T>` where T is arbitrary, or just `Result<()>`?
4. **Subcommand storage:** `HashMap<String, Box<dyn CommandLike>>` vs enum dispatch?

---

## Phase 1: Foundation

### 1.0 Error Handling

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `ClickError` enum in `src/error.rs` | `exceptions.py` | Base error type |
| Done | `UsageError` variant | `exceptions.py:UsageError` | Command usage errors |
| Done | `BadParameter` variant | `exceptions.py:BadParameter` | Parameter validation errors |
| Done | `MissingParameter` variant | `exceptions.py:MissingParameter` | Required param missing |
| Done | `NoSuchOption` variant | `exceptions.py:NoSuchOption` | Unknown option |
| Done | `BadOptionUsage` variant | `exceptions.py:BadOptionUsage` | Invalid option usage |
| Done | `BadArgumentUsage` variant | `exceptions.py:BadArgumentUsage` | Invalid argument usage |
| Done | `FileError` variant | `exceptions.py:FileError` | File operation errors |
| Done | `Abort` variant | `exceptions.py:Abort` | User abort signal |
| Done | `Exit` variant with code | `exceptions.py:Exit` | Exit with code |
| Done | Error formatting with context | `exceptions.py:ClickException.format_message` | "Try --help" hints |

### 1.1 Parameter Types System

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `TypeConverter` trait definition | `types.py:ParamType` | Base trait for all types (renamed to avoid conflict) |
| Done | `TypeConverter::convert()` method | `types.py:ParamType.convert` | String → typed value |
| Done | `TypeConverter::get_metavar()` method | `types.py:ParamType.get_metavar` | Help text display |
| Done | `TypeConverter::get_missing_message()` | `types.py:ParamType.get_missing_message` | Custom missing error |
| Done | `TypeConverter::split_envvar_value()` | `types.py:ParamType.split_envvar_value` | Split env vars (uses OS path separator) |
| Done | `TypeConverter::shell_complete()` | `types.py:ParamType.shell_complete` | Tab completion |
| Done | `STRING` type | `types.py:STRING` | Default text type |
| Done | `INT` type | `types.py:INT` | Integer conversion |
| Done | `FLOAT` type | `types.py:FLOAT` | Float conversion |
| Done | `BOOL` type | `types.py:BOOL` | Boolean parsing |
| Done | `UUID` type | `types.py:UUID` | UUID parsing |
| Done | `IntRange` type with min/max | `types.py:IntRange` | Bounded integer with clamp support |
| Done | `FloatRange` type with min/max | `types.py:FloatRange` | Bounded float with clamp support |
| Done | `DateTime` type | `types.py:DateTime` | ISO8601 parsing via chrono |
| Done | `Choice` type | `types.py:Choice` | Enumerated values with case-insensitive option |
| Done | `PathType` with validation | `types.py:Path` | File path, exists/readable/writable checks |
| Done | `FileType` (lazy open) | `types.py:File` | File handle with stdin/stdout "-" support |
| Done | `TupleType` composite type | `types.py:Tuple` | Multiple value types with convert_values() |
| Done | `UNPROCESSED` type | `types.py:UNPROCESSED` | Raw passthrough |
| Todo | `convert_type()` auto-detection | `types.py:convert_type` | Rust type → TypeConverter (deferred) |

### 1.2 Parameter Source Tracking

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `ParameterSource` enum | `core.py:ParameterSource` | Track value origin |
| Done | `CommandLine` variant | `core.py:ParameterSource.COMMANDLINE` | From CLI args |
| Done | `Environment` variant | `core.py:ParameterSource.ENVIRONMENT` | From env var |
| Done | `Default` variant | `core.py:ParameterSource.DEFAULT` | From parameter default |
| Done | `DefaultMap` variant | `core.py:ParameterSource.DEFAULT_MAP` | From context default_map |
| Done | `Prompt` variant | `core.py:ParameterSource.PROMPT` | From interactive prompt |

### 1.3 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Python test scripts for types | `tests/parity/phase1/python/` | test_types.py |
| Done | Python test scripts for errors | `tests/parity/phase1/python/` | test_errors.py |
| Done | Rust parity binary crate | `tests/parity/phase1/rust/` | Compiles and runs |
| Done | Parity test runner script | `tests/parity/run_parity.sh` | Runs both, shows diff |
| Done | Output format alignment | N/A | All library-level message differences resolved |

---

## Phase 2: Context & Parameters

### 2.1 Context

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Context` struct | `core.py:Context` | Central execution context |
| Done | Context construction | `core.py:Context.__init__` | Implemented via `ContextBuilder` + `Command::make_context()` |
| Done | Context stack push/pop | `core.py:Context.scope` | Implemented via `push_context`/`pop_context` + `get_current_context` |
| Done | `Context::params` storage | `core.py:Context.params` | Parsed param values |
| Done | `Context::obj` user object | `core.py:Context.obj` | Custom state storage |
| Done | `Context::meta` shared dict | `core.py:Context.meta` | Shared across nesting |
| Done | `Context::parent` chain | `core.py:Context.parent` | Parent context link |
| Done | `Context::command_path` computed | `core.py:Context.command_path` | Full invocation path |
| Done | `Context::invoked_subcommand` | `core.py:Context.invoked_subcommand` | Active subcommand |
| Done | `Context::default_map` | `core.py:Context.default_map` | Override defaults |
| Done | `Context::get_parameter_source()` | `core.py:Context.get_parameter_source` | Source tracking |
| Done | `Context::invoke()` smart caller | `core.py:Context.invoke` | Implemented as `Context::invoke()` |
| Done | `Context::forward()` | `core.py:Context.forward` | Implemented as `Context::forward()` |
| Done | `Context::fail()` helper | `core.py:Context.fail` | Usage error helper |
| Done | `Context::abort()` helper | `core.py:Context.abort` | Abort helper |
| Done | `Context::exit()` helper | `core.py:Context.exit` | Exit helper |
| Done | `Context::with_resource()` | `core.py:Context.with_resource` | Implemented as `Context::with_resource()` |
| Done | `Context::call_on_close()` | `core.py:Context.call_on_close` | Cleanup callbacks |
| Done | Thread-local context stack | `globals.py` | `get_current_context()` |

### 2.2 Parameter Base

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Parameter` trait | `core.py:Parameter` | Abstract base |
| Done | `Parameter::name` | `core.py:Parameter.name` | Primary name |
| Done | `Parameter::nargs` | `core.py:Parameter.nargs` | Argument count (-1 = variadic) |
| Done | `Parameter::multiple` | `core.py:Parameter.multiple` | Repeatable |
| Done | `Parameter::is_eager` | `core.py:Parameter.is_eager` | Process first |
| Done | `Parameter::expose_value` | `core.py:Parameter.expose_value` | Pass to callback |
| Done | Value consumption pipeline | `core.py:Parameter.consume_value` | Implemented via `Command`/`OptionParser` rather than per-parameter methods |
| Done | Parse result handling | `core.py:Parameter.handle_parse_result` | Implemented via `Command::make_context` + parser integration |
| Done | Type casting | `core.py:Parameter.type_cast_value` | Implemented via type converters + parser integration |
| Done | Envvar resolution | `core.py:Parameter.resolve_envvar_value` | Implemented via envvar support on parameters |
| Done | Envvar parsing | `core.py:Parameter.value_from_envvar` | Implemented via `TypeConverter::split_envvar_value` |

### 2.3 Option

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `ClickOption` struct | `core.py:Option` | Named parameter |
| Done | `Option::is_flag` | `core.py:Option.is_flag` | Boolean flag |
| Done | `Option::is_bool_flag` | `core.py:Option.is_bool_flag` | --flag/--no-flag |
| Done | `Option::flag_value` | `core.py:Option.flag_value` | Value when flag present |
| Done | `Option::count` | `core.py:Option.count` | -v -v -v counting |
| Done | `Option::prompt` | `core.py:Option.prompt` | Interactive prompt |
| Done | `Option::confirmation_prompt` | `core.py:Option.confirmation_prompt` | Double entry |
| Done | `Option::hide_input` | `core.py:Option.hide_input` | Password style |
| Done | `Option::show_default` | `core.py:Option.show_default` | Display in help |
| Done | `Option::show_envvar` | `core.py:Option.show_envvar` | Display envvar in help |
| Done | Long/short name parsing | `core.py:Option` | --name, -n |
| Done | `Option::get_help_record()` | `core.py:Option.get_help_record` | Help text entry |

### 2.4 Argument

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Argument` struct | `core.py:Argument` | Positional parameter |
| Done | Required by default | `core.py:Argument` | Unless default provided |
| Done | Single declaration name | `core.py:Argument` | No aliases |
| Done | Variadic support (nargs=-1) | `core.py:Argument` | Consume remaining |
| Done | `Argument::get_help_record()` | `core.py:Argument.get_help_record` | Help text entry |

### 2.5 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Python test scripts for Context | `tests/parity/phase2/python/` | test_context.py |
| Done | Python test scripts for Parameter | `tests/parity/phase2/python/` | test_parameter.py |
| Done | Rust parity binary crate | `tests/parity/phase2/rust/` | Matching output format |

---

## Phase 3: Parser & Commands

### 3.1 Argument Parser

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `OptionParser` struct | `parser.py:_OptionParser` | Token-based parser |
| Done | `parse_args()` main entry | `parser.py:_OptionParser.parse_args` | Returns parsed values + leftovers |
| Done | Short option parsing `-x` | `parser.py` | Single char |
| Done | Grouped short `-xyz` | `parser.py` | Multiple flags |
| Done | Long option `--name=value` | `parser.py` | With = separator |
| Done | Long option `--name value` | `parser.py` | Space separator |
| Done | `--` terminator | `parser.py` | End option parsing |
| Done | Option/arg interspersing | `parser.py` | Mixed positions |
| Done | Extra args handling | `parser.py` | After command |
| Done | Unknown option error | `parser.py` | NoSuchOption |
| Done | `_Argument` internal struct | `parser.py:_Argument` | Argument spec |
| Done | `_Option` internal struct | `parser.py:_Option` | Option spec |

### 3.2 Command

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Command` struct | `core.py:Command` | Basic command |
| Done | `Command::callback` | `core.py:Command.callback` | The function to call |
| Done | `Command::params` | `core.py:Command.params` | Parameter definitions |
| Done | `Command::main()` | `core.py:Command.main` | Entry point |
| Done | `Command::make_context()` | `core.py:Command.make_context` | Create context |
| Done | `Command::parse_args()` | `core.py:Command.parse_args` | Orchestrate parsing |
| Done | `Command::invoke()` | `core.py:Command.invoke` | Call callback |
| Done | `Command::get_help()` | `core.py:Command.get_help` | Generate help |
| Done | `Command::get_usage()` | `core.py:Command.get_usage` | Usage line |
| Done | `Command::format_help()` | `core.py:Command.format_help` | Full help text |
| Done | `Command::format_usage()` | `core.py:Command.format_usage` | Full usage text |
| Done | Eager parameter ordering | `core.py:_check_iter` | Process --help first |

### 3.3 Group

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Group` struct | `core.py:Group` | Command container |
| Done | `Group::commands` map | `core.py:Group.commands` | Name → Command |
| Done | `Group::add_command()` | `core.py:Group.add_command` | Register subcommand |
| Done | `Group::get_command()` | `core.py:Group.get_command` | Lookup subcommand |
| Done | `Group::list_commands()` | `core.py:Group.list_commands` | Get command names |
| Done | `Group::resolve_command()` | `core.py:Group.resolve_command` | Parse and find |
| Done | `Group::invoke()` dispatch | `core.py:Group.invoke` | Subcommand dispatch |
| Done | Command chaining | `core.py:Group` | Execute multiple |
| Done | `CommandCollection` | `core.py:CommandCollection` | Merged groups |

### 3.4 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Python test scripts for Parser | `tests/parity/phase3/python/` | test_parser.py |
| Done | Python test scripts for Command | `tests/parity/phase3/python/` | test_command.py |
| Done | Python test scripts for Group | `tests/parity/phase3/python/` | test_group.py |
| Done | Rust parity binary crate | `tests/parity/phase3/rust/` | Matching output format |

---

## Phase 4: Derive Macros & Formatting

### 4.1 Derive Macros (Proc Macro Crate)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Create `click-derive` proc-macro crate | N/A | Separate crate |
| Done | `#[derive(Command)]` | `decorators.py:command` | Command from struct |
| Done | `#[derive(Group)]` | `decorators.py:group` | Group from struct |
| Done | `#[command(...)]` attributes | `decorators.py:command` | name, help, etc. |
| Done | `#[option(...)]` field attribute | `decorators.py:option` | Named parameter |
| Done | `#[argument(...)]` field attribute | `decorators.py:argument` | Positional parameter |
| Done | Name extraction from function | `decorators.py` | Uses struct name, converts to kebab-case |
| Done | Help from doc comments | N/A | Rust convention |
| Done | Type inference | N/A | Field type → ParamType |

### 4.2 Convenience Macros/Attributes

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `#[version_option]` | `decorators.py:version_option` | Pre-configured --version |
| Done | `#[help_option]` | `decorators.py:help_option` | Pre-configured --help |
| Done | `#[confirmation_option]` | `decorators.py:confirmation_option` | --yes confirmation |
| Done | `#[password_option]` | `decorators.py:password_option` | Password with confirm |
| Done | `#[pass_context]` | `decorators.py:pass_context` | Inject context |
| Done | `#[pass_obj]` | `decorators.py:pass_obj` | Inject ctx.obj |
| Done | `make_pass_decorator()` equivalent | `decorators.py:make_pass_decorator` | Custom passthrough |

### 4.3 Help Formatting

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `HelpFormatter` struct | `formatting.py:HelpFormatter` | Help text builder |
| Done | `HelpFormatter::write_usage()` | `formatting.py:HelpFormatter.write_usage` | Usage line |
| Done | `HelpFormatter::write_heading()` | `formatting.py:HelpFormatter.write_heading` | Section heading |
| Done | `HelpFormatter::write_paragraph()` | `formatting.py:HelpFormatter.write_paragraph` | Wrapped text |
| Done | `HelpFormatter::write_text()` | `formatting.py:HelpFormatter.write_text` | Raw text |
| Done | `HelpFormatter::write_dl()` | `formatting.py:HelpFormatter.write_dl` | Definition list (write_definition_list) |
| Done | `HelpFormatter::section()` | `formatting.py:HelpFormatter.section` | Indented section |
| Done | `HelpFormatter::indent()` | `formatting.py:HelpFormatter.indent` | Increase indent |
| Done | `HelpFormatter::dedent()` | `formatting.py:HelpFormatter.dedent` | Decrease indent |
| Done | Terminal width detection | `formatting.py:wrap_text` | detect_terminal_width() |
| Done | `wrap_text()` function | `formatting.py:wrap_text` | Text wrapping |
| Done | Paragraph preservation | `formatting.py:wrap_text` | Double newlines |

### 4.4 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Python test scripts for Decorators | `tests/parity/phase4/python/` | test_decorators.py |
| Done | Python test scripts for Formatting | `tests/parity/phase4/python/` | test_formatting.py |
| Done | Rust parity binary crate | `tests/parity/phase4/rust/` | Matching output format |

---

## Phase 5: Terminal UI

### 5.1 Output Functions

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `echo()` function | `utils.py:echo` | Print with color support |
| Done | `echo!()` macro | N/A | Rust convenience |
| Done | `secho()` function | `utils.py:secho` | Styled echo |
| Done | `style()` function | `termui.py:style` | ANSI color/bold text |
| Done | Color output control | `utils.py:_default_text_stdout` | Auto-detect TTY via isatty() |
| Done | `echo_via_pager()` | `utils.py:echo_via_pager` | Pipe to less/more |

### 5.2 Input Functions

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `prompt()` function | `termui.py:prompt` | Interactive input |
| Done | Type conversion in prompt | `termui.py:prompt` | With TypeConverter |
| Done | Default value display | `termui.py:prompt` | Show default |
| Done | Value processing callback | `termui.py:prompt` | Transform input |
| Done | `confirm()` function | `termui.py:confirm` | Yes/no prompt |
| Done | Abort on no | `termui.py:confirm` | Optional abort |
| Done | `getchar()` function | `termui.py:getchar` | Single char input |
| Done | `pause()` function | `termui.py:pause` | Press any key |
| Done | Hidden input | `termui.py:prompt` | Password style (hide_input param) |

### 5.3 Progress & Interactive

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `progressbar()` context | `termui.py:progressbar` | Progress indicator |
| Done | `ProgressBar` struct | `termui.py:ProgressBar` | Character-based rendering |
| Done | Bar rendering | `termui.py:ProgressBar` | Character-based |
| Done | ETA calculation | `termui.py:ProgressBar` | Time estimation |
| Done | `clear()` function | `termui.py:clear` | Clear screen |
| Done | `edit_text()` function | `termui.py:edit` | Launch editor |
| Done | `launch()` function | `termui.py:launch` | Open URL/file |

### 5.4 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Python test scripts for termui | `tests/parity/phase5/python/` | test_termui.py |
| Done | Rust parity binary crate | `tests/parity/phase5/rust/` | Matching output format |

---

## Phase 6: Advanced Features

### 6.1 Shell Completion

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `ShellComplete` base | `shell_completion.py:ShellComplete` | Abstract trait |
| Done | `BashComplete` | `shell_completion.py:BashComplete` | Bash completion |
| Done | `ZshComplete` | `shell_completion.py:ZshComplete` | Zsh completion |
| Done | `FishComplete` | `shell_completion.py:FishComplete` | Fish completion |
| Done | `CompletionItem` struct | `shell_completion.py:CompletionItem` | Completion entry (in types.rs) |
| Done | `get_completions()` | `shell_completion.py:_resolve_context` | Context resolution |
| Done | Custom completers | `shell_completion.py` | TypeConverter::shell_complete |

### 6.2 Testing Utilities

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `CliRunner` struct | `testing.py:CliRunner` | Test harness |
| Done | `CliRunner::invoke()` | `testing.py:CliRunner.invoke` | Run command |
| Done | `IsolatedFilesystem` | `testing.py:CliRunner.isolated_filesystem` | Temp directory |
| Done | `InvokeResult` struct | `testing.py:Result` | Invocation result |
| Done | `InvokeResult::output` | `testing.py:Result.output` | Captured stdout |
| Done | `InvokeResult::exit_code` | `testing.py:Result.exit_code` | Exit code |
| Done | `InvokeResult::exception_message` | `testing.py:Result.exception` | Error message |
| Done | Input simulation | `testing.py:CliRunner` | invoke_with_input() |
| Done | Environment isolation | `testing.py:CliRunner` | env(), env_unset(), clear_env() |

### 6.3 Utilities

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `get_binary_stdout()` | `utils.py:_default_text_stdout` | Binary stdout |
| Done | `get_text_stdout/stderr()` | `utils.py:_default_text_stdout` | Text streams |
| Done | `LazyFile` type | `utils.py:LazyFile` | Lazy file open (in types.rs) |
| Done | `format_filename()` | `utils.py:format_filename` | Path formatting |
| Done | `get_app_dir()` | `utils.py:get_app_dir` | Platform app dir |
| Done | `get_os_args()` | `utils.py` | Get sys.argv |
| Done | `expand_path()` | `utils.py` | Tilde and envvar expansion |
| Done | `safecall()` | `utils.py:safecall` | Safe callback invocation |
| Done | `should_strip_ansi()` | `utils.py` | ANSI output detection |

### 6.4 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Python test scripts for Completion | `tests/parity/phase6/python/` | test_completion.py |
| Done | Python test scripts for Testing | `tests/parity/phase6/python/` | test_testing.py |
| Done | Rust parity binary crate | `tests/parity/phase6/rust/` | Matching output format |

---

## Utilities & Helpers

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `safecall()` wrapper | `utils.py:safecall` | Safe callback invocation |
| N/A | `make_str()` conversion | `utils.py:make_str` | Not needed in Rust (String is UTF-8) |
| Done | `_expand_args()` | `utils.py:_expand_args` | Implemented as `expand_args()` |
| Done | `split_arg_string()` | `shell_completion.py:split_arg_string` | Shell-like splitting |
| Done | Name normalization | `core.py:_posixify` | to_kebab_case in derive macros |
| Done | Completion mode detection | `shell_completion.py` | CompletionOption struct |

---

## Legend

| Status | Meaning |
|--------|---------|
| Todo | Not started |
| In Progress | Currently being implemented |
| Done | Implemented and tested |

---

## Phase 7: Cross-Platform Capture & Hardening (Post-M4)

This phase is required for a real-world 1.0: the test runner must be able to capture
stdout/stderr reliably on all major OSes, and we need a structured process for closing
behavior/API gaps beyond the Phase 1–6 parity suite coverage.

### 7.1 `CliRunner` Capture Backends (Must-Capture)

| Status | Task | Notes |
|--------|------|-------|
| Done | Define capture semantics | `InvokeResult.output` mirrors Click: `stdout + stderr` when mixed; stderr always captured separately |
| Done | Refactor capture into per-platform backend | Internal `run_with_capture` with Unix + Windows implementations |
| Done | Windows backend | Redirect `STD_*` handles via Win32 `SetStdHandle` and capture pipes |
| In Progress | macOS/Linux verification | Linux verified locally; macOS/Windows runtime verified via CI |
| Done | Panic-safety | Stdio/env restored even if invocation panics |

### 7.2 CI Matrix (Linux/macOS/Windows)

| Status | Task | Notes |
|--------|------|-------|
| Done | Add GitHub Actions matrix | `cargo test` on all 3 OSes (stable) |
| Done | Gate on `CliRunner` capture tests | Existing test suite exercises `CliRunner` capture on Windows |
| Todo | Optional parity checks | Parity runner depends on Python Click source; may be a separate job |

### 7.3 Behavior/API Gap Mitigation

| Status | Task | Notes |
|--------|------|-------|
| Done | Create “gap inventory” doc | `docs/devel/GAP_INVENTORY.md` |
| Todo | Expand parity suites incrementally | Add deterministic tests for parsing/completion edge cases |
| Todo | `make_pass_decorator(ensure=...)` decision | Requires `Context` interior mutability or an alternative API |
| Todo | Public API stabilization | Audit re-exports + builder surface for 1.0 |

---

## Quick Reference: Python → Rust Patterns

| Python | Rust |
|--------|------|
| `@decorator` | `#[derive(Macro)]` or `#[attribute]` |
| `__click_params__` list | Compile-time derive macro collection |
| `Protocol` | `trait` |
| `NamedTuple` | `#[derive(Clone, Copy)] struct` |
| `@dataclass` | `#[derive(Clone)] struct` |
| `Optional[T]` | `Option<T>` |
| `Union[A, B]` | `enum` or `impl Into<T>` |
| `List[T]` | `Vec<T>` |
| `Dict[K, V]` | `HashMap<K, V>` or `BTreeMap<K, V>` |
| `Callable` | `Fn` trait or function pointer |
| `contextmanager` | `impl Drop` or RAII guard |
| Thread-local storage | `thread_local!` macro |
| `functools.lru_cache` | `once_cell::sync::Lazy` |
| Exception hierarchy | Error enum with variants |
| `try/except` | `Result<T, E>` and `?` |

---

## Architecture Notes

### Click's Core Design Principles

1. **Composable Decorators** → Rust derive macros build the same composition
2. **Context Propagation** → Thread-local Context with parent chain
3. **Type Safety** → ParamType trait for conversion/validation
4. **Help Generation** → Auto-formatted from metadata and doc comments
5. **Testability** → CliRunner captures output and simulates input

### Key Differences from Python

1. **No runtime reflection** → Derive macros at compile time
2. **Ownership semantics** → Context owns its resources
3. **Error handling** → Result<T, E> instead of exceptions
4. **Callback types** → Trait objects or generics instead of callables
5. **No monkey patching** → Explicit extension points via traits

### Recommended Implementation Order

1. **Phase 1** (Foundation) - Error types, ParamType system
2. **Phase 2** (Context) - Context and Parameter traits
3. **Phase 3** (Parser) - Argument parsing and Command/Group
4. **Phase 4** (Derive) - Proc macros for ergonomic API
5. **Phase 5** (UI) - Terminal interaction
6. **Phase 6** (Advanced) - Completion, testing, utilities

Each phase should have working tests before moving to the next.
