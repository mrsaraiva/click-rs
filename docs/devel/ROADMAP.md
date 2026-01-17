# Click-rs Development Roadmap

A comprehensive task list for porting Python Click to Rust. Reference: `/home/msaraiva/dev/mark/Proj/Libs/click`

*Note: This is planning documentation. Implementation may diverge as architectural decisions are finalized.*

## Current Status

**Project State:** Phase 1 complete. Error types, parameter type system, and source tracking implemented with 37 unit tests passing.

## Milestones

| Milestone | Target | Criteria |
|-----------|--------|----------|
| **M1: Core Types** | Phase 1 complete | Error types, ParamType trait, all built-in types pass parity tests |
| **M2: Minimal CLI** | Phase 2-3 complete | Can parse args, execute commands, display help (no macros) |
| **M3: Derive Macros** | Phase 4 complete | `#[derive(Command)]` works, parity with Click's decorator API |
| **M4: Full Port** | Phase 5-6 complete | Terminal UI, shell completion, CliRunner all functional |
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

## Platform & Compatibility

| Aspect | Target |
|--------|--------|
| **MSRV** | Rust 1.70+ (edition 2021) |
| **Platforms** | Linux, macOS, Windows 10+ |
| **Terminal** | Any terminal supporting ANSI escape codes; fallback for Windows legacy console |
| **Click version** | Behavioral parity with Click 8.1.x |

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
| Todo | Python test scripts for types | `tests/parity/phase1/python/` | test_types.py |
| Todo | Python test scripts for errors | `tests/parity/phase1/python/` | test_errors.py |
| Todo | Rust parity binary crate | `tests/parity/phase1/rust/` | Matching output format |
| Todo | Parity test runner script | `tests/parity/run_parity.sh` | Runs both, shows diff |

---

## Phase 2: Context & Parameters

### 2.1 Context

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Context` struct | `core.py:Context` | Central execution context |
| Todo | `Context::new()` with command | `core.py:Context.__init__` | ~30 parameters in Python |
| Todo | `Context::scope()` push/pop | `core.py:Context.scope` | Thread-local stack |
| Todo | `Context::params` storage | `core.py:Context.params` | Parsed param values |
| Todo | `Context::obj` user object | `core.py:Context.obj` | Custom state storage |
| Todo | `Context::meta` shared dict | `core.py:Context.meta` | Shared across nesting |
| Todo | `Context::parent` chain | `core.py:Context.parent` | Parent context link |
| Todo | `Context::command_path` computed | `core.py:Context.command_path` | Full invocation path |
| Todo | `Context::invoked_subcommand` | `core.py:Context.invoked_subcommand` | Active subcommand |
| Todo | `Context::default_map` | `core.py:Context.default_map` | Override defaults |
| Todo | `Context::get_parameter_source()` | `core.py:Context.get_parameter_source` | Source tracking |
| Todo | `Context::invoke()` smart caller | `core.py:Context.invoke` | Call commands/functions |
| Todo | `Context::forward()` | `core.py:Context.forward` | Forward to other command |
| Todo | `Context::fail()` helper | `core.py:Context.fail` | Raise UsageError |
| Todo | `Context::abort()` helper | `core.py:Context.abort` | Raise Abort |
| Todo | `Context::exit()` helper | `core.py:Context.exit` | Raise Exit |
| Todo | `Context::with_resource()` | `core.py:Context.with_resource` | Register cleanup |
| Todo | `Context::call_on_close()` | `core.py:Context.call_on_close` | Cleanup callbacks |
| Todo | Thread-local context stack | `globals.py` | `get_current_context()` |

### 2.2 Parameter Base

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Parameter` trait | `core.py:Parameter` | Abstract base |
| Todo | `Parameter::name` | `core.py:Parameter.name` | Primary name |
| Todo | `Parameter::nargs` | `core.py:Parameter.nargs` | Argument count (-1 = variadic) |
| Todo | `Parameter::multiple` | `core.py:Parameter.multiple` | Repeatable |
| Todo | `Parameter::is_eager` | `core.py:Parameter.is_eager` | Process first |
| Todo | `Parameter::expose_value` | `core.py:Parameter.expose_value` | Pass to callback |
| Todo | `Parameter::consume_value()` | `core.py:Parameter.consume_value` | Get value from sources |
| Todo | `Parameter::handle_parse_result()` | `core.py:Parameter.handle_parse_result` | End-to-end processing |
| Todo | `Parameter::type_cast_value()` | `core.py:Parameter.type_cast_value` | Type conversion |
| Todo | `Parameter::resolve_envvar_value()` | `core.py:Parameter.resolve_envvar_value` | Env var lookup |
| Todo | `Parameter::value_from_envvar()` | `core.py:Parameter.value_from_envvar` | Parse env var |

### 2.3 Option

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Option` struct | `core.py:Option` | Named parameter |
| Todo | `Option::is_flag` | `core.py:Option.is_flag` | Boolean flag |
| Todo | `Option::is_bool_flag` | `core.py:Option.is_bool_flag` | --flag/--no-flag |
| Todo | `Option::flag_value` | `core.py:Option.flag_value` | Value when flag present |
| Todo | `Option::count` | `core.py:Option.count` | -v -v -v counting |
| Todo | `Option::prompt` | `core.py:Option.prompt` | Interactive prompt |
| Todo | `Option::confirmation_prompt` | `core.py:Option.confirmation_prompt` | Double entry |
| Todo | `Option::hide_input` | `core.py:Option.hide_input` | Password style |
| Todo | `Option::show_default` | `core.py:Option.show_default` | Display in help |
| Todo | `Option::show_envvar` | `core.py:Option.show_envvar` | Display envvar in help |
| Todo | Long/short name parsing | `core.py:Option` | --name, -n |
| Todo | `Option::get_help_record()` | `core.py:Option.get_help_record` | Help text entry |

### 2.4 Argument

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Argument` struct | `core.py:Argument` | Positional parameter |
| Todo | Required by default | `core.py:Argument` | Unless default provided |
| Todo | Single declaration name | `core.py:Argument` | No aliases |
| Todo | Variadic support (nargs=-1) | `core.py:Argument` | Consume remaining |
| Todo | `Argument::get_help_record()` | `core.py:Argument.get_help_record` | Help text entry |

### 2.5 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | Python test scripts for Context | `tests/parity/phase2/python/` | test_context.py |
| Todo | Python test scripts for Parameter | `tests/parity/phase2/python/` | test_parameter.py |
| Todo | Rust parity binary crate | `tests/parity/phase2/rust/` | Matching output format |

---

## Phase 3: Parser & Commands

### 3.1 Argument Parser

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `OptionParser` struct | `parser.py:_OptionParser` | Token-based parser |
| Todo | `parse_args()` main entry | `parser.py:_OptionParser.parse_args` | Returns (opts, args, order) |
| Todo | Short option parsing `-x` | `parser.py` | Single char |
| Todo | Grouped short `-xyz` | `parser.py` | Multiple flags |
| Todo | Long option `--name=value` | `parser.py` | With = separator |
| Todo | Long option `--name value` | `parser.py` | Space separator |
| Todo | `--` terminator | `parser.py` | End option parsing |
| Todo | Option/arg interspersing | `parser.py` | Mixed positions |
| Todo | Extra args handling | `parser.py` | After command |
| Todo | Unknown option error | `parser.py` | NoSuchOption |
| Todo | `_Argument` internal struct | `parser.py:_Argument` | Argument spec |
| Todo | `_Option` internal struct | `parser.py:_Option` | Option spec |

### 3.2 Command

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Command` struct | `core.py:Command` | Basic command |
| Todo | `Command::callback` | `core.py:Command.callback` | The function to call |
| Todo | `Command::params` | `core.py:Command.params` | Parameter definitions |
| Todo | `Command::main()` | `core.py:Command.main` | Entry point |
| Todo | `Command::make_context()` | `core.py:Command.make_context` | Create context |
| Todo | `Command::parse_args()` | `core.py:Command.parse_args` | Orchestrate parsing |
| Todo | `Command::invoke()` | `core.py:Command.invoke` | Call callback |
| Todo | `Command::get_help()` | `core.py:Command.get_help` | Generate help |
| Todo | `Command::get_usage()` | `core.py:Command.get_usage` | Usage line |
| Todo | `Command::format_help()` | `core.py:Command.format_help` | Full help text |
| Todo | `Command::format_usage()` | `core.py:Command.format_usage` | Full usage text |
| Todo | Eager parameter ordering | `core.py:_check_iter` | Process --help first |

### 3.3 Group

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Group` struct | `core.py:Group` | Command container |
| Todo | `Group::commands` map | `core.py:Group.commands` | Name → Command |
| Todo | `Group::add_command()` | `core.py:Group.add_command` | Register subcommand |
| Todo | `Group::get_command()` | `core.py:Group.get_command` | Lookup subcommand |
| Todo | `Group::list_commands()` | `core.py:Group.list_commands` | Get command names |
| Todo | `Group::resolve_command()` | `core.py:Group.resolve_command` | Parse and find |
| Todo | `Group::invoke()` dispatch | `core.py:Group.invoke` | Subcommand dispatch |
| Todo | Command chaining | `core.py:Group` | Execute multiple |
| Todo | `CommandCollection` | `core.py:CommandCollection` | Merged groups |

### 3.4 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | Python test scripts for Parser | `tests/parity/phase3/python/` | test_parser.py |
| Todo | Python test scripts for Command | `tests/parity/phase3/python/` | test_command.py |
| Todo | Python test scripts for Group | `tests/parity/phase3/python/` | test_group.py |
| Todo | Rust parity binary crate | `tests/parity/phase3/rust/` | Matching output format |

---

## Phase 4: Derive Macros & Formatting

### 4.1 Derive Macros (Proc Macro Crate)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | Create `click-derive` proc-macro crate | N/A | Separate crate |
| Todo | `#[derive(Command)]` | `decorators.py:command` | Command from struct |
| Todo | `#[derive(Group)]` | `decorators.py:group` | Group from struct |
| Todo | `#[command(...)]` attributes | `decorators.py:command` | name, help, etc. |
| Todo | `#[option(...)]` field attribute | `decorators.py:option` | Named parameter |
| Todo | `#[argument(...)]` field attribute | `decorators.py:argument` | Positional parameter |
| Todo | Name extraction from function | `decorators.py` | Strip _cmd suffix |
| Todo | Help from doc comments | N/A | Rust convention |
| Todo | Type inference | N/A | Field type → ParamType |

### 4.2 Convenience Macros/Attributes

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `#[version_option]` | `decorators.py:version_option` | Pre-configured --version |
| Todo | `#[help_option]` | `decorators.py:help_option` | Pre-configured --help |
| Todo | `#[confirmation_option]` | `decorators.py:confirmation_option` | --yes confirmation |
| Todo | `#[password_option]` | `decorators.py:password_option` | Password with confirm |
| Todo | `#[pass_context]` | `decorators.py:pass_context` | Inject context |
| Todo | `#[pass_obj]` | `decorators.py:pass_obj` | Inject ctx.obj |
| Todo | `make_pass_decorator()` equivalent | `decorators.py:make_pass_decorator` | Custom passthrough |

### 4.3 Help Formatting

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `HelpFormatter` struct | `formatting.py:HelpFormatter` | Help text builder |
| Todo | `HelpFormatter::write_usage()` | `formatting.py:HelpFormatter.write_usage` | Usage line |
| Todo | `HelpFormatter::write_heading()` | `formatting.py:HelpFormatter.write_heading` | Section heading |
| Todo | `HelpFormatter::write_paragraph()` | `formatting.py:HelpFormatter.write_paragraph` | Wrapped text |
| Todo | `HelpFormatter::write_text()` | `formatting.py:HelpFormatter.write_text` | Raw text |
| Todo | `HelpFormatter::write_dl()` | `formatting.py:HelpFormatter.write_dl` | Definition list |
| Todo | `HelpFormatter::section()` | `formatting.py:HelpFormatter.section` | Indented section |
| Todo | `HelpFormatter::indent()` | `formatting.py:HelpFormatter.indent` | Increase indent |
| Todo | `HelpFormatter::dedent()` | `formatting.py:HelpFormatter.dedent` | Decrease indent |
| Todo | Terminal width detection | `formatting.py:wrap_text` | Auto-wrap |
| Todo | `wrap_text()` function | `formatting.py:wrap_text` | Text wrapping |
| Todo | Paragraph preservation | `formatting.py:wrap_text` | Double newlines |

### 4.4 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | Python test scripts for Decorators | `tests/parity/phase4/python/` | test_decorators.py |
| Todo | Python test scripts for Formatting | `tests/parity/phase4/python/` | test_formatting.py |
| Todo | Rust parity binary crate | `tests/parity/phase4/rust/` | Matching output format |

---

## Phase 5: Terminal UI

### 5.1 Output Functions

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `echo()` function | `utils.py:echo` | Print with color support |
| Todo | `echo!()` macro | N/A | Rust convenience |
| Todo | `secho()` function | `utils.py:secho` | Styled echo |
| Todo | `style()` function | `termui.py:style` | ANSI color/bold text |
| Todo | Color output control | `utils.py:_default_text_stdout` | Auto-detect TTY |
| Todo | `echo_via_pager()` | `utils.py:echo_via_pager` | Pipe to less/more |

### 5.2 Input Functions

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `prompt()` function | `termui.py:prompt` | Interactive input |
| Todo | Type conversion in prompt | `termui.py:prompt` | With ParamType |
| Todo | Default value display | `termui.py:prompt` | Show default |
| Todo | Value processing callback | `termui.py:prompt` | Transform input |
| Todo | `confirm()` function | `termui.py:confirm` | Yes/no prompt |
| Todo | Abort on no | `termui.py:confirm` | Optional abort |
| Todo | `getchar()` function | `termui.py:getchar` | Single char input |
| Todo | `pause()` function | `termui.py:pause` | Press any key |
| Todo | Hidden input | `termui.py:prompt` | Password style |

### 5.3 Progress & Interactive

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `progressbar()` context | `termui.py:progressbar` | Progress indicator |
| Todo | `ProgressBar` struct | `termui.py:ProgressBar` | ~400 lines in Python |
| Todo | Bar rendering | `termui.py:ProgressBar` | Character-based |
| Todo | ETA calculation | `termui.py:ProgressBar` | Time estimation |
| Todo | `clear()` function | `termui.py:clear` | Clear screen |
| Todo | `edit()` function | `termui.py:edit` | Launch editor |
| Todo | `launch()` function | `termui.py:launch` | Open URL/file |

### 5.4 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | Python test scripts for termui | `tests/parity/phase5/python/` | test_termui.py |
| Todo | Rust parity binary crate | `tests/parity/phase5/rust/` | Matching output format |

---

## Phase 6: Advanced Features

### 6.1 Shell Completion

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `ShellComplete` base | `shell_completion.py:ShellComplete` | Abstract base |
| Todo | `BashComplete` | `shell_completion.py:BashComplete` | Bash completion |
| Todo | `ZshComplete` | `shell_completion.py:ZshComplete` | Zsh completion |
| Todo | `FishComplete` | `shell_completion.py:FishComplete` | Fish completion |
| Todo | `CompletionItem` struct | `shell_completion.py:CompletionItem` | Completion entry |
| Todo | `get_completion()` | `shell_completion.py:_resolve_context` | Context resolution |
| Todo | Custom completers | `shell_completion.py` | Parameter.shell_complete |

### 6.2 Testing Utilities

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `CliRunner` struct | `testing.py:CliRunner` | Test harness |
| Todo | `CliRunner::invoke()` | `testing.py:CliRunner.invoke` | Run command |
| Todo | `CliRunner::isolated_filesystem()` | `testing.py:CliRunner.isolated_filesystem` | Temp directory |
| Todo | `Result` struct | `testing.py:Result` | Invocation result |
| Todo | `Result::output` | `testing.py:Result.output` | Captured stdout |
| Todo | `Result::exit_code` | `testing.py:Result.exit_code` | Exit code |
| Todo | `Result::exception` | `testing.py:Result.exception` | Any error |
| Todo | Input simulation | `testing.py:CliRunner` | Simulate stdin |
| Todo | Environment isolation | `testing.py:CliRunner` | Custom env vars |

### 6.3 Utilities

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `get_binary_stream()` | `utils.py:_default_text_stdout` | Binary stdout/stderr |
| Todo | `get_text_stream()` | `utils.py:_default_text_stdout` | Text stdout/stderr |
| Todo | `open_file()` | `utils.py:LazyFile` | Lazy file open |
| Todo | `format_filename()` | `utils.py:format_filename` | Path formatting |
| Todo | `get_app_dir()` | `utils.py:get_app_dir` | Platform app dir |
| Todo | `get_os_args()` | `utils.py` | Get sys.argv |
| Todo | Environment variable handling | `utils.py` | Case normalization |

### 6.4 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | Python test scripts for Completion | `tests/parity/phase6/python/` | test_completion.py |
| Todo | Python test scripts for Testing | `tests/parity/phase6/python/` | test_testing.py |
| Todo | Rust parity binary crate | `tests/parity/phase6/rust/` | Matching output format |

---

## Utilities & Helpers

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `safecall()` wrapper | `utils.py:safecall` | Safe callback invocation |
| Todo | `make_str()` conversion | `utils.py:make_str` | Bytes → String |
| Todo | `_expand_args()` | `utils.py:_expand_args` | Glob expansion |
| Todo | `split_arg_string()` | `shell_completion.py:split_arg_string` | Shell-like splitting |
| Todo | `_posixify()` name normalization | `core.py:_posixify` | Name cleanup |
| Todo | `_bashcomplete` environment var check | `shell_completion.py` | Completion mode detection |

---

## Legend

| Status | Meaning |
|--------|---------|
| Todo | Not started |
| In Progress | Currently being implemented |
| Done | Implemented and tested |

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
