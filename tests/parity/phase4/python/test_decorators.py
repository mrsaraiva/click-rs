#!/usr/bin/env python3
"""
Parity tests for Click decorators / convenience helpers.
Compares Python Click output with Rust click output.
"""
import os
import sys


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


def test_version_option():
    print("=== Version Option ===")

    @click.command()
    @click.version_option(
        "1.2.3",
        prog_name="myapp",
        message="%(prog)s, version %(version)s",
    )
    def cli():
        pass

    runner = CliRunner()
    result = runner.invoke(cli, ["--version"])
    print("# default:")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    @click.command()
    @click.version_option(
        "1.2.3",
        "--ver",
        prog_name="myapp",
        message="%(prog)s, version %(version)s",
    )
    def cli2():
        pass

    result = runner.invoke(cli2, ["--ver"])
    print("# custom names:")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def test_help_option():
    print("\n=== Help Option ===")

    @click.command()
    @click.help_option("--assist", "-h", help="Show this message and exit.")
    def cli():
        pass

    runner = CliRunner()
    result = runner.invoke(cli, ["--assist"])
    help_text = result.output

    print("# custom names:")
    print(f"  has_assist: {str('--assist' in help_text).lower()}")
    print(f"  has_default_help: {str('--help' in help_text).lower()}")
    print(f"  exit_code: {result.exit_code}")


def test_pass_context_and_obj():
    print("\n=== Pass Context / Obj ===")

    @click.command()
    @click.pass_context
    def pc(ctx):
        click.echo(f"info_name={ctx.info_name}")

    runner = CliRunner()
    result = runner.invoke(pc, [])
    print("# pass_context:")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")

    @click.command()
    @click.pass_obj
    def po(obj):
        click.echo(f"obj={obj['value']}")

    result = runner.invoke(po, [], obj={"value": 7})
    print("# pass_obj:")
    print(f"  output: {result.output.strip()!r}")
    print(f"  exit_code: {result.exit_code}")


def main():
    test_version_option()
    test_help_option()
    test_pass_context_and_obj()


if __name__ == "__main__":
    main()
