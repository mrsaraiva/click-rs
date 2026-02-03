#!/usr/bin/env python3
"""
Phase 6 parity tests for testing utilities (CliRunner basics).
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
if click_src and not _maybe_add_click_src(click_src):
    raise RuntimeError(f"CLICK_SRC is set but not readable: {click_src!r}")

import click
from click.testing import CliRunner


@click.command()
def cli():
    click.echo("out")
    click.echo("err", err=True)


def main():
    print("=== CliRunner ===")
    runner = CliRunner()
    result = runner.invoke(cli, [])
    print(f"  exit_code: {result.exit_code}")
    print(f"  stdout: {result.stdout!r}")
    print(f"  stderr: {result.stderr!r}")


if __name__ == "__main__":
    main()
