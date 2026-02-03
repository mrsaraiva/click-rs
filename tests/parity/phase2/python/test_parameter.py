#!/usr/bin/env python3
"""
Parity tests for Click Option and Argument parameters.
Compares Python Click output with Rust click output.
"""
import sys
import os

# Optional: use a local Click source tree by setting CLICK_SRC to its `src/` dir.
# Parity runs default to the pinned `click` wheel installed by `tests/parity/run_parity.sh`.
def _maybe_add_click_src(path: str) -> bool:
    init_py = os.path.join(path, "click", "__init__.py")
    if os.path.isfile(init_py) and os.access(init_py, os.R_OK):
        sys.path.insert(0, path)
        return True
    return False

click_src = os.environ.get("CLICK_SRC")
if click_src and not _maybe_add_click_src(click_src):
    raise RuntimeError(f"CLICK_SRC is set but not readable: {click_src!r}")

import click
from click import Option, Argument, Context, Command


def make_context():
    """Create a dummy context for testing."""
    @click.command()
    def dummy():
        pass
    return Context(dummy, info_name="test")


def test_option_creation():
    """Test basic option creation."""
    print("=== Option Creation ===")

    # Basic option with long name
    print("# Basic option --name:")
    opt = Option(["--name"])
    print(f"name: {opt.name}")
    print(f"opts: {opt.opts}")
    print(f"is_flag: {opt.is_flag}")
    print(f"required: {opt.required}")
    print(f"multiple: {opt.multiple}")
    print(f"count: {opt.count}")
    print(f"param_type_name: {opt.param_type_name}")

    # Option with short and long
    print("# Option -n/--name:")
    opt = Option(["-n", "--name"])
    print(f"name: {opt.name}")
    print(f"opts: {opt.opts}")
    print(f"secondary_opts: {opt.secondary_opts}")

    # Option with explicit name
    print("# Option with explicit name:")
    opt = Option(["-n", "--name"], "username")
    print(f"name: {opt.name}")


def test_option_flags():
    """Test flag options."""
    print("\n=== Option Flags ===")

    # Simple flag
    print("# Simple flag --verbose:")
    opt = Option(["--verbose"], is_flag=True)
    print(f"name: {opt.name}")
    print(f"is_flag: {opt.is_flag}")
    print(f"is_bool_flag: {opt.is_bool_flag}")
    print(f"flag_value: {opt.flag_value}")

    # Boolean flag with secondary
    print("# Boolean flag --flag/--no-flag:")
    opt = Option(["--flag/--no-flag"])
    print(f"name: {opt.name}")
    print(f"is_flag: {opt.is_flag}")
    print(f"is_bool_flag: {opt.is_bool_flag}")
    print(f"opts: {opt.opts}")
    print(f"secondary_opts: {opt.secondary_opts}")


def test_option_count():
    """Test count option."""
    print("\n=== Option Count ===")

    # Count option
    print("# Count option -v/--verbose:")
    opt = Option(["-v", "--verbose"], count=True)
    print(f"name: {opt.name}")
    print(f"count: {opt.count}")
    print(f"is_flag: {opt.is_flag}")
    print(f"default: {opt.default}")


def test_option_multiple():
    """Test multiple option."""
    print("\n=== Option Multiple ===")

    # Multiple option
    print("# Multiple option -f/--file:")
    opt = Option(["-f", "--file"], multiple=True)
    print(f"name: {opt.name}")
    print(f"multiple: {opt.multiple}")


def test_option_required():
    """Test required option."""
    print("\n=== Option Required ===")

    # Required option
    print("# Required option --name:")
    opt = Option(["--name"], required=True)
    print(f"name: {opt.name}")
    print(f"required: {opt.required}")


def test_option_help_record():
    """Test option help record generation."""
    print("\n=== Option Help Record ===")

    ctx = make_context()

    # Basic option
    print("# Basic option --name:")
    opt = Option(["--name"], help="Your name")
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")
        print(f"help: {record[1]}")

    # Option with short
    print("# Option -n/--name:")
    opt = Option(["-n", "--name"], help="Your name")
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")
        print(f"help: {record[1]}")

    # Flag option
    print("# Flag option --verbose:")
    opt = Option(["--verbose", "-v"], is_flag=True, help="Enable verbose mode")
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")
        print(f"help: {record[1]}")

    # Count option
    print("# Count option -v/--verbose:")
    opt = Option(["-v", "--verbose"], count=True, help="Increase verbosity")
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")
        print(f"help: {record[1]}")

    # Required option
    print("# Required option --name:")
    opt = Option(["--name"], required=True, help="Your name")
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")
        print(f"help: {record[1]}")

    # Option with default (show_default)
    print("# Option with default:")
    opt = Option(["--count"], default=10, show_default=True, help="Item count")
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")
        print(f"help: {record[1]}")

    # Option with envvar (show_envvar)
    print("# Option with envvar:")
    opt = Option(["--name"], envvar="MY_NAME", show_envvar=True, help="Your name")
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")
        print(f"help: {record[1]}")

    # Hidden option
    print("# Hidden option:")
    opt = Option(["--secret"], hidden=True, help="Secret option")
    record = opt.get_help_record(ctx)
    print(f"help_record: {record}")


def test_argument_creation():
    """Test basic argument creation."""
    print("\n=== Argument Creation ===")

    # Required argument (default)
    print("# Required argument FILENAME:")
    arg = Argument(["filename"])
    print(f"name: {arg.name}")
    print(f"required: {arg.required}")
    print(f"nargs: {arg.nargs}")
    print(f"param_type_name: {arg.param_type_name}")

    # Optional argument with default
    print("# Optional argument with default:")
    arg = Argument(["output"], default="out.txt")
    print(f"name: {arg.name}")
    print(f"required: {arg.required}")
    print(f"default: {arg.default}")

    # Explicitly required with default
    print("# Explicitly required with default:")
    arg = Argument(["output"], default="out.txt", required=True)
    print(f"name: {arg.name}")
    print(f"required: {arg.required}")
    print(f"default: {arg.default}")

    # Explicitly optional without default
    print("# Explicitly optional without default:")
    arg = Argument(["output"], required=False)
    print(f"name: {arg.name}")
    print(f"required: {arg.required}")


def test_argument_nargs():
    """Test argument nargs handling."""
    print("\n=== Argument Nargs ===")

    ctx = make_context()

    # Single (default)
    print("# Single (nargs=1):")
    arg = Argument(["filename"])
    print(f"nargs: {arg.nargs}")
    print(f"metavar: {arg.make_metavar(ctx)}")

    # Optional (nargs=0 or -1 with required=False depends on Click version)
    print("# Variadic (nargs=-1):")
    arg = Argument(["files"], nargs=-1)
    print(f"nargs: {arg.nargs}")
    print(f"required: {arg.required}")
    print(f"metavar: {arg.make_metavar(ctx)}")

    # Multiple (nargs=2)
    print("# Multiple (nargs=2):")
    arg = Argument(["pair"], nargs=2)
    print(f"nargs: {arg.nargs}")
    print(f"required: {arg.required}")
    print(f"metavar: {arg.make_metavar(ctx)}")

    # Optional variadic (nargs=-1, required=False)
    print("# Optional variadic:")
    arg = Argument(["files"], nargs=-1, required=False)
    print(f"nargs: {arg.nargs}")
    print(f"required: {arg.required}")
    print(f"metavar: {arg.make_metavar(ctx)}")


def test_argument_human_readable_name():
    """Test argument human_readable_name."""
    print("\n=== Argument Human Readable Name ===")

    # Default (uppercase name)
    print("# Default (uppercase name):")
    arg = Argument(["filename"])
    print(f"human_readable_name: {arg.human_readable_name}")

    # Custom metavar
    print("# Custom metavar:")
    arg = Argument(["file"], metavar="PATH")
    print(f"human_readable_name: {arg.human_readable_name}")


def test_argument_help_record():
    """Test argument help record generation."""
    print("\n=== Argument Help Record ===")

    ctx = make_context()

    # Required argument
    print("# Required argument:")
    arg = Argument(["filename"])
    record = arg.get_help_record(ctx)
    if record:
        print(f"metavar: {record[0]}")
        print(f"help: '{record[1]}'")
    else:
        print(f"help_record: {record}")

    # Optional argument
    print("# Optional argument:")
    arg = Argument(["filename"], required=False)
    record = arg.get_help_record(ctx)
    if record:
        print(f"metavar: {record[0]}")
        print(f"help: '{record[1]}'")

    # Variadic argument
    print("# Variadic argument:")
    arg = Argument(["files"], nargs=-1)
    record = arg.get_help_record(ctx)
    if record:
        print(f"metavar: {record[0]}")
        print(f"help: '{record[1]}'")

    # Optional variadic argument
    print("# Optional variadic argument:")
    arg = Argument(["files"], nargs=-1, required=False)
    record = arg.get_help_record(ctx)
    if record:
        print(f"metavar: {record[0]}")
        print(f"help: '{record[1]}'")


def test_option_metavar():
    """Test option metavar generation."""
    print("\n=== Option Metavar ===")

    ctx = make_context()

    # Default (TEXT)
    print("# Default metavar:")
    opt = Option(["--name"])
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")

    # Custom metavar
    print("# Custom metavar:")
    opt = Option(["--file"], metavar="PATH")
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")

    # Integer type
    print("# Integer type:")
    opt = Option(["--count"], type=int)
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")

    # Choice type
    print("# Choice type:")
    opt = Option(["--format"], type=click.Choice(["json", "xml", "csv"]))
    record = opt.get_help_record(ctx)
    if record:
        print(f"opts: {record[0]}")


def test_parameter_envvar():
    """Test parameter environment variable handling."""
    print("\n=== Parameter Envvar ===")

    # Option with envvar
    print("# Option with envvar:")
    opt = Option(["--name"], envvar="MY_NAME")
    print(f"envvar: {opt.envvar}")

    # Option with multiple envvars
    print("# Option with multiple envvars:")
    opt = Option(["--name"], envvar=["MY_NAME", "FALLBACK_NAME"])
    print(f"envvar: {opt.envvar}")

    # Argument with envvar
    print("# Argument with envvar:")
    arg = Argument(["filename"], envvar="MY_FILE")
    print(f"envvar: {arg.envvar}")


def main():
    """Run all parameter tests."""
    test_option_creation()
    test_option_flags()
    test_option_count()
    test_option_multiple()
    test_option_required()
    test_option_help_record()
    test_argument_creation()
    test_argument_nargs()
    test_argument_human_readable_name()
    test_argument_help_record()
    test_option_metavar()
    test_parameter_envvar()


if __name__ == "__main__":
    main()
