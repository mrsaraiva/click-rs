#!/usr/bin/env python3
"""
Parity tests for Click group functionality.
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


def test_group_with_subcommands():
    """Test group with subcommands."""
    print("=== Group With Subcommands ===")

    @click.group()
    def cli():
        """Main CLI application."""
        pass

    @cli.command()
    def init():
        """Initialize the project."""
        click.echo("Initializing...")

    @cli.command()
    def build():
        """Build the project."""
        click.echo("Building...")

    runner = CliRunner()

    # List subcommands
    print("# Group subcommand listing:")
    # Get command names from group
    cmd_names = sorted(cli.commands.keys())
    print(f"  commands: {cmd_names}")

    # Invoke init
    result = runner.invoke(cli, ["init"])
    print("# Invoke 'init' subcommand:")
    print(f"  args: ['init']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Invoke build
    result = runner.invoke(cli, ["build"])
    print("# Invoke 'build' subcommand:")
    print(f"  args: ['build']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def test_subcommand_dispatch():
    """Test subcommand dispatch with arguments."""
    print("\n=== Subcommand Dispatch ===")

    @click.group()
    @click.option("--verbose", "-v", is_flag=True)
    def cli(verbose):
        """CLI with verbose option."""
        if verbose:
            click.echo("Verbose mode enabled")

    @cli.command()
    @click.argument("name")
    def greet(name):
        """Greet someone."""
        click.echo(f"Hello, {name}!")

    @cli.command()
    @click.option("--count", "-c", type=int, default=1)
    def repeat(count):
        """Repeat action."""
        click.echo(f"Repeating {count} times")

    runner = CliRunner()

    # Subcommand with argument
    result = runner.invoke(cli, ["greet", "Alice"])
    print("# Subcommand with argument:")
    print(f"  args: ['greet', 'Alice']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Group option + subcommand
    result = runner.invoke(cli, ["-v", "greet", "Bob"])
    print("# Group option + subcommand:")
    print(f"  args: ['-v', 'greet', 'Bob']")
    lines = result.output.strip().split('\n')
    for line in lines:
        print(f"  output: {line!r}")
    print(f"  exit_code: {result.exit_code}")

    # Subcommand with option
    result = runner.invoke(cli, ["repeat", "-c", "3"])
    print("# Subcommand with option:")
    print(f"  args: ['repeat', '-c', '3']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def test_missing_command_errors():
    """Test missing command errors."""
    print("\n=== Missing Command Errors ===")

    @click.group()
    def cli():
        pass

    @cli.command()
    def hello():
        click.echo("Hello!")

    runner = CliRunner()

    # No subcommand provided
    result = runner.invoke(cli, [])
    print("# No subcommand provided:")
    print(f"  args: []")
    # Check if help is shown or error
    has_usage = "Usage:" in result.output
    print(f"  shows_usage: {has_usage}")
    print(f"  exit_code: {result.exit_code}")

    # Unknown subcommand
    result = runner.invoke(cli, ["unknown"])
    print("# Unknown subcommand:")
    print(f"  args: ['unknown']")
    if "No such command" in result.output or "no such command" in result.output.lower():
        print(f"  error_type: 'UsageError'")
        print(f"  error_contains: 'No such command'")
    print(f"  exit_code: {result.exit_code}")


def test_help_with_subcommand_listing():
    """Test help with subcommand listing."""
    print("\n=== Help With Subcommand Listing ===")

    @click.group()
    def cli():
        """A sample CLI application."""
        pass

    @cli.command()
    def init():
        """Initialize the project."""
        pass

    @cli.command()
    def build():
        """Build the project."""
        pass

    @cli.command()
    def deploy():
        """Deploy the project."""
        pass

    runner = CliRunner()
    result = runner.invoke(cli, ["--help"])

    help_text = result.output
    print("# Group help text elements:")
    print(f"  has_usage: {'Usage:' in help_text}")
    print(f"  has_docstring: {'sample CLI' in help_text}")
    print(f"  has_options_section: {'Options:' in help_text}")
    print(f"  has_commands_section: {'Commands:' in help_text}")

    # Check for subcommands in help
    print(f"  lists_init: {'init' in help_text}")
    print(f"  lists_build: {'build' in help_text}")
    print(f"  lists_deploy: {'deploy' in help_text}")

    # Check for subcommand descriptions
    print(f"  init_has_help: {'Initialize' in help_text}")
    print(f"  build_has_help: {'Build' in help_text}")
    print(f"  deploy_has_help: {'Deploy' in help_text}")

    print(f"  exit_code: {result.exit_code}")

    # Help for specific subcommand
    result = runner.invoke(cli, ["init", "--help"])
    help_text = result.output
    print("# Subcommand help:")
    print(f"  has_usage: {'Usage:' in help_text}")
    print(f"  has_init_in_usage: {'init' in help_text}")
    print(f"  has_docstring: {'Initialize' in help_text}")
    print(f"  exit_code: {result.exit_code}")


def test_nested_groups():
    """Test nested groups."""
    print("\n=== Nested Groups ===")

    @click.group()
    def cli():
        """Main CLI."""
        pass

    @cli.group()
    def db():
        """Database commands."""
        pass

    @db.command()
    def init():
        """Initialize database."""
        click.echo("DB initialized")

    @db.command()
    def migrate():
        """Run migrations."""
        click.echo("Migrations run")

    runner = CliRunner()

    # Invoke nested command
    result = runner.invoke(cli, ["db", "init"])
    print("# Nested group command:")
    print(f"  args: ['db', 'init']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Help for nested group
    result = runner.invoke(cli, ["db", "--help"])
    help_text = result.output
    print("# Nested group help:")
    print(f"  has_commands: {'Commands:' in help_text}")
    print(f"  lists_init: {'init' in help_text}")
    print(f"  lists_migrate: {'migrate' in help_text}")
    print(f"  exit_code: {result.exit_code}")


def test_invoke_without_command():
    """Test invoke_without_command."""
    print("\n=== Invoke Without Command ===")

    @click.group(invoke_without_command=True)
    @click.pass_context
    def cli(ctx):
        """CLI that can run without subcommand."""
        if ctx.invoked_subcommand is None:
            click.echo("No subcommand invoked")

    @cli.command()
    def sub():
        click.echo("Subcommand executed")

    runner = CliRunner()

    # No subcommand
    result = runner.invoke(cli, [])
    print("# Group invoked without subcommand:")
    print(f"  args: []")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # With subcommand
    result = runner.invoke(cli, ["sub"])
    print("# Group invoked with subcommand:")
    print(f"  args: ['sub']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def test_chain_mode():
    """Test chain mode."""
    print("\n=== Chain Mode ===")

    @click.group(chain=True)
    def cli():
        """Chained CLI."""
        pass

    @cli.command()
    def cmd1():
        click.echo("cmd1 executed")

    @cli.command()
    def cmd2():
        click.echo("cmd2 executed")

    @cli.command()
    def cmd3():
        click.echo("cmd3 executed")

    runner = CliRunner()

    # Single command in chain
    result = runner.invoke(cli, ["cmd1"])
    print("# Chain with single command:")
    print(f"  args: ['cmd1']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Multiple commands in chain
    result = runner.invoke(cli, ["cmd1", "cmd2"])
    print("# Chain with two commands:")
    print(f"  args: ['cmd1', 'cmd2']")
    lines = result.output.strip().split('\n')
    for line in lines:
        print(f"  output: {line!r}")
    print(f"  exit_code: {result.exit_code}")

    # All three commands
    result = runner.invoke(cli, ["cmd1", "cmd2", "cmd3"])
    print("# Chain with three commands:")
    print(f"  args: ['cmd1', 'cmd2', 'cmd3']")
    lines = result.output.strip().split('\n')
    for line in lines:
        print(f"  output: {line!r}")
    print(f"  exit_code: {result.exit_code}")


def main():
    """Run all group tests."""
    test_group_with_subcommands()
    test_subcommand_dispatch()
    test_missing_command_errors()
    test_help_with_subcommand_listing()
    test_nested_groups()
    test_invoke_without_command()
    test_chain_mode()


if __name__ == "__main__":
    main()
