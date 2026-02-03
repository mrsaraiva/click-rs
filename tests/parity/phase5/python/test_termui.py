#!/usr/bin/env python3
"""
Phase 5 parity tests for terminal UI helpers.
Compares Python Click output with Rust click-rs output.
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


def test_style_and_strip():
    print("=== Style / Strip ANSI ===")
    styled = click.style("Hello", fg="red", bold=True)
    stripped = click.termui.strip_ansi(styled)
    print(f"  has_ansi: {str('\\x1b[' in styled or '\x1b[' in styled).lower()}")
    print(f"  stripped: {stripped!r}")


def test_echo_and_pager():
    print("\n=== Echo / Pager ===")

    @click.command()
    def cli():
        click.echo("out")
        click.echo("err", err=True)
        click.echo_via_pager("paged")

    runner = CliRunner()
    result = runner.invoke(cli, [], env={"CI": "1"})
    print(f"  exit_code: {result.exit_code}")
    print(f"  stdout: {result.stdout!r}")
    print(f"  stderr: {result.stderr!r}")


def main():
    test_style_and_strip()
    test_echo_and_pager()


if __name__ == "__main__":
    main()
