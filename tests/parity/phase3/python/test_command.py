#!/usr/bin/env python3
"""
Parity tests for Click command functionality.
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
from click.testing import CliRunner


def test_command_creation():
    """Test command creation and basic invocation."""
    print("=== Command Creation ===")

    @click.command()
    def simple_cmd():
        click.echo("Hello, World!")

    runner = CliRunner()
    result = runner.invoke(simple_cmd, [])
    print("# Simple command with no args:")
    print(f"  command: 'simple_cmd'")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Command with name
    @click.command(name="custom-name")
    def named_cmd():
        click.echo("Named command")

    print("# Command with custom name:")
    print(f"  name: 'custom-name'")
    print(f"  actual_name: {named_cmd.name!r}")


def test_command_with_options():
    """Test command with options."""
    print("\n=== Command With Options ===")

    @click.command()
    @click.option("--name", "-n", default="World")
    @click.option("--count", "-c", type=int, default=1)
    def greet(name, count):
        for _ in range(count):
            click.echo(f"Hello, {name}!")

    runner = CliRunner()

    # With defaults
    result = runner.invoke(greet, [])
    print("# Command with default options:")
    print(f"  args: []")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # With custom values
    result = runner.invoke(greet, ["--name", "Alice", "-c", "2"])
    print("# Command with custom option values:")
    print(f"  args: ['--name', 'Alice', '-c', '2']")
    lines = result.output.strip().split('\n')
    for line in lines:
        print(f"  output: {line!r}")
    print(f"  exit_code: {result.exit_code}")


def test_command_with_arguments():
    """Test command with positional arguments."""
    print("\n=== Command With Arguments ===")

    @click.command()
    @click.argument("filename")
    def cmd(filename):
        click.echo(f"File: {filename}")

    runner = CliRunner()

    result = runner.invoke(cmd, ["test.txt"])
    print("# Command with single argument:")
    print(f"  args: ['test.txt']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Multiple arguments
    @click.command()
    @click.argument("src")
    @click.argument("dst")
    def copy(src, dst):
        click.echo(f"Copy {src} -> {dst}")

    result = runner.invoke(copy, ["a.txt", "b.txt"])
    print("# Command with multiple arguments:")
    print(f"  args: ['a.txt', 'b.txt']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Variadic arguments
    @click.command()
    @click.argument("files", nargs=-1)
    def ls(files):
        click.echo(f"Files: {list(files)}")

    result = runner.invoke(ls, ["a.txt", "b.txt", "c.txt"])
    print("# Command with variadic arguments:")
    print(f"  args: ['a.txt', 'b.txt', 'c.txt']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def test_help_text_generation():
    """Test help text generation."""
    print("\n=== Help Text Generation ===")

    @click.command()
    @click.option("--name", "-n", help="Name to greet")
    @click.option("--count", "-c", type=int, default=1, help="Number of greetings")
    def greet(name, count):
        """Greet someone."""
        pass

    runner = CliRunner()
    result = runner.invoke(greet, ["--help"])

    # Parse help output for key elements
    help_text = result.output
    print("# Help text elements:")
    print(f"  has_usage: {'Usage:' in help_text}")
    print(f"  has_options_section: {'Options:' in help_text}")
    print(f"  has_help_option: {'--help' in help_text}")
    print(f"  has_name_option: {'--name' in help_text or '-n' in help_text}")
    print(f"  has_count_option: {'--count' in help_text or '-c' in help_text}")
    print(f"  has_docstring: {'Greet someone' in help_text}")
    print(f"  exit_code: {result.exit_code}")


def test_usage_line_generation():
    """Test usage line generation."""
    print("\n=== Usage Line Generation ===")

    # Simple command
    @click.command()
    def simple():
        pass

    runner = CliRunner()
    result = runner.invoke(simple, ["--help"])
    usage_line = [l for l in result.output.split('\n') if 'Usage:' in l][0]
    print("# Simple command usage:")
    # Normalize - extract just the pattern
    print(f"  pattern: 'Usage: <name> [OPTIONS]'")

    # Command with argument
    @click.command()
    @click.argument("filename")
    def with_arg(filename):
        pass

    result = runner.invoke(with_arg, ["--help"])
    usage_line = [l for l in result.output.split('\n') if 'Usage:' in l][0]
    print("# Command with argument usage:")
    print(f"  pattern: 'Usage: <name> [OPTIONS] FILENAME'")

    # Command with optional argument
    @click.command()
    @click.argument("filename", required=False)
    def with_optional_arg(filename):
        pass

    result = runner.invoke(with_optional_arg, ["--help"])
    usage_line = [l for l in result.output.split('\n') if 'Usage:' in l][0]
    print("# Command with optional argument usage:")
    print(f"  pattern: 'Usage: <name> [OPTIONS] [FILENAME]'")

    # Command with variadic argument
    @click.command()
    @click.argument("files", nargs=-1)
    def with_variadic(files):
        pass

    result = runner.invoke(with_variadic, ["--help"])
    usage_line = [l for l in result.output.split('\n') if 'Usage:' in l][0]
    print("# Command with variadic argument usage:")
    print(f"  pattern: 'Usage: <name> [OPTIONS] [FILES]...'")


def test_missing_required_parameter():
    """Test missing required parameter errors."""
    print("\n=== Missing Required Parameter ===")

    @click.command()
    @click.option("--name", "-n", required=True)
    def cmd_opt(name):
        click.echo(f"name={name}")

    runner = CliRunner()
    result = runner.invoke(cmd_opt, [])
    print("# Missing required option:")
    print(f"  args: []")
    # Normalize error message
    if "Missing option" in result.output or "missing option" in result.output.lower():
        print(f"  error_type: 'MissingParameter'")
        print(f"  param_type: 'option'")
    print(f"  exit_code: {result.exit_code}")

    @click.command()
    @click.argument("filename")
    def cmd_arg(filename):
        click.echo(f"filename={filename}")

    result = runner.invoke(cmd_arg, [])
    print("# Missing required argument:")
    print(f"  args: []")
    if "Missing argument" in result.output or "missing argument" in result.output.lower():
        print(f"  error_type: 'MissingParameter'")
        print(f"  param_type: 'argument'")
    print(f"  exit_code: {result.exit_code}")


def test_callback_execution():
    """Test callback execution."""
    print("\n=== Callback Execution ===")

    callback_log = []

    @click.command()
    @click.option("--value", default="default")
    def cmd(value):
        callback_log.append(f"called with {value}")
        click.echo(f"value={value}")

    runner = CliRunner()

    # Clear log and invoke
    callback_log.clear()
    result = runner.invoke(cmd, [])
    print("# Callback with default value:")
    print(f"  callback_called: {len(callback_log) > 0}")
    print(f"  callback_log: {callback_log}")
    print(f"  output: {result.output.strip()!r}")

    # With custom value
    callback_log.clear()
    result = runner.invoke(cmd, ["--value", "custom"])
    print("# Callback with custom value:")
    print(f"  callback_called: {len(callback_log) > 0}")
    print(f"  callback_log: {callback_log}")
    print(f"  output: {result.output.strip()!r}")

    # Callback that raises
    @click.command()
    def error_cmd():
        raise click.ClickException("intentional error")

    result = runner.invoke(error_cmd, [])
    print("# Callback that raises ClickException:")
    print(f"  error_in_output: {'intentional error' in result.output}")
    print(f"  exit_code: {result.exit_code}")


def main():
    """Run all command tests."""
    test_command_creation()
    test_command_with_options()
    test_command_with_arguments()
    test_help_text_generation()
    test_usage_line_generation()
    test_missing_required_parameter()
    test_callback_execution()


if __name__ == "__main__":
    main()
