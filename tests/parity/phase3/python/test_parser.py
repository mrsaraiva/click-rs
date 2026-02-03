#!/usr/bin/env python3
"""
Parity tests for Click parser functionality.
Compares Python Click output with Rust click output.
"""
import sys
import os

# Insert Click library path before imports.
# Use CLICK_SRC environment variable, or fall back to common locations.
#
# Some environments have these directories present but unreadable; only prepend
# the path if the Click package can actually be read from it.
def _maybe_add_click_src(path: str) -> bool:
    init_py = os.path.join(path, "click", "__init__.py")
    if os.path.isfile(init_py) and os.access(init_py, os.R_OK):
        sys.path.insert(0, path)
        return True
    return False


click_src = os.environ.get("CLICK_SRC")
if not (click_src and _maybe_add_click_src(click_src)):
    for path in [
        os.path.expanduser("~/dev/mark/Proj/Libs/click/src"),
        "/home/msaraiva/dev/mark/Proj/Libs/click/src",
    ]:
        if _maybe_add_click_src(path):
            break

import click
from click.testing import CliRunner


def test_short_options():
    """Test short option parsing (-v, -xyz grouped)."""
    print("=== Short Options ===")

    # Single short option flag
    @click.command()
    @click.option("-v", "--verbose", is_flag=True)
    def cmd1(verbose):
        click.echo(f"verbose={verbose}")

    runner = CliRunner()
    result = runner.invoke(cmd1, ["-v"])
    print("# Single short flag (-v):")
    print(f"  args: ['-v']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Short option with value (attached)
    @click.command()
    @click.option("-n", "--name", type=str)
    def cmd2(name):
        click.echo(f"name={name}")

    result = runner.invoke(cmd2, ["-nAlice"])
    print("# Short option with attached value (-nAlice):")
    print(f"  args: ['-nAlice']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Short option with value (separate)
    result = runner.invoke(cmd2, ["-n", "Bob"])
    print("# Short option with separate value (-n Bob):")
    print(f"  args: ['-n', 'Bob']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Grouped short flags (-abc)
    @click.command()
    @click.option("-a", is_flag=True)
    @click.option("-b", is_flag=True)
    @click.option("-c", is_flag=True)
    def cmd3(a, b, c):
        click.echo(f"a={a} b={b} c={c}")

    result = runner.invoke(cmd3, ["-abc"])
    print("# Grouped short flags (-abc):")
    print(f"  args: ['-abc']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Partial grouped flags
    result = runner.invoke(cmd3, ["-ab"])
    print("# Partial grouped flags (-ab):")
    print(f"  args: ['-ab']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def test_long_options():
    """Test long option parsing (--name=value, --name value)."""
    print("\n=== Long Options ===")

    @click.command()
    @click.option("--name", type=str)
    def cmd(name):
        click.echo(f"name={name}")

    runner = CliRunner()

    # Long option with = separator
    result = runner.invoke(cmd, ["--name=Alice"])
    print("# Long option with = (--name=Alice):")
    print(f"  args: ['--name=Alice']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Long option with space separator
    result = runner.invoke(cmd, ["--name", "Bob"])
    print("# Long option with space (--name Bob):")
    print(f"  args: ['--name', 'Bob']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Long flag
    @click.command()
    @click.option("--verbose", is_flag=True)
    def cmd2(verbose):
        click.echo(f"verbose={verbose}")

    result = runner.invoke(cmd2, ["--verbose"])
    print("# Long flag (--verbose):")
    print(f"  args: ['--verbose']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Empty value
    result = runner.invoke(cmd, ["--name="])
    print("# Long option with empty value (--name=):")
    print(f"  args: ['--name=']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def test_double_dash_terminator():
    """Test double-dash (--) terminator."""
    print("\n=== Double-Dash Terminator ===")

    @click.command()
    @click.option("--verbose", "-v", is_flag=True)
    @click.argument("args", nargs=-1)
    def cmd(verbose, args):
        click.echo(f"verbose={verbose}")
        click.echo(f"args={list(args)}")

    runner = CliRunner()

    # -- terminates options
    result = runner.invoke(cmd, ["--", "--verbose"])
    print("# Double-dash before option-like arg (-- --verbose):")
    print(f"  args: ['--', '--verbose']")
    for line in result.output.strip().split('\n'):
        print(f"  output: {line!r}")
    print(f"  exit_code: {result.exit_code}")

    # Options before --
    result = runner.invoke(cmd, ["-v", "--", "-flag", "--opt"])
    print("# Options before --, args after (-v -- -flag --opt):")
    print(f"  args: ['-v', '--', '-flag', '--opt']")
    for line in result.output.strip().split('\n'):
        print(f"  output: {line!r}")
    print(f"  exit_code: {result.exit_code}")

    # Just --
    result = runner.invoke(cmd, ["--"])
    print("# Just double-dash (--):")
    print(f"  args: ['--']")
    for line in result.output.strip().split('\n'):
        print(f"  output: {line!r}")
    print(f"  exit_code: {result.exit_code}")


def test_mixed_options_and_arguments():
    """Test mixed options and arguments."""
    print("\n=== Mixed Options and Arguments ===")

    @click.command()
    @click.option("--count", "-c", type=int, default=1)
    @click.argument("name")
    def cmd(count, name):
        click.echo(f"count={count} name={name}")

    runner = CliRunner()

    # Options before argument
    result = runner.invoke(cmd, ["-c", "5", "Alice"])
    print("# Options before argument (-c 5 Alice):")
    print(f"  args: ['-c', '5', 'Alice']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Argument before options (interspersed)
    result = runner.invoke(cmd, ["Alice", "-c", "5"])
    print("# Argument before options - interspersed (Alice -c 5):")
    print(f"  args: ['Alice', '-c', '5']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Multiple arguments with options interspersed
    @click.command()
    @click.option("--flag", "-f", is_flag=True)
    @click.argument("files", nargs=-1)
    def cmd2(flag, files):
        click.echo(f"flag={flag} files={list(files)}")

    result = runner.invoke(cmd2, ["a.txt", "-f", "b.txt"])
    print("# Files with flag interspersed (a.txt -f b.txt):")
    print(f"  args: ['a.txt', '-f', 'b.txt']")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def test_unknown_option_errors():
    """Test unknown option errors."""
    print("\n=== Unknown Option Errors ===")

    @click.command()
    @click.option("--help-me", is_flag=True)
    @click.option("--verbose", "-v", is_flag=True)
    def cmd(help_me, verbose):
        click.echo("ok")

    runner = CliRunner()

    # Unknown long option
    result = runner.invoke(cmd, ["--unknown"])
    print("# Unknown long option (--unknown):")
    print(f"  args: ['--unknown']")
    # Normalize error message format
    err_msg = result.output.strip()
    if "No such option:" in err_msg or "no such option:" in err_msg:
        print(f"  error: 'No such option: --unknown'")
    else:
        print(f"  error: {err_msg!r}")
    print(f"  exit_code: {result.exit_code}")

    # Unknown short option
    result = runner.invoke(cmd, ["-x"])
    print("# Unknown short option (-x):")
    print(f"  args: ['-x']")
    if "No such option:" in result.output or "no such option:" in result.output:
        print(f"  error: 'No such option: -x'")
    else:
        print(f"  error: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    # Typo with suggestion (--hlep similar to --help-me)
    result = runner.invoke(cmd, ["--hlep"])
    print("# Typo option (--hlep):")
    print(f"  args: ['--hlep']")
    # Normalize - just check if it reports unknown
    if "No such option:" in result.output or "no such option:" in result.output:
        print(f"  error_type: 'NoSuchOption'")
    print(f"  exit_code: {result.exit_code}")


def main():
    """Run all parser tests."""
    test_short_options()
    test_long_options()
    test_double_dash_terminator()
    test_mixed_options_and_arguments()
    test_unknown_option_errors()


if __name__ == "__main__":
    main()
