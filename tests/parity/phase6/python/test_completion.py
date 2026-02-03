#!/usr/bin/env python3
"""
Phase 6 parity tests for shell completion.
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


@click.group()
def cli():
    pass


@cli.command(help="Build")
@click.option("--count", "-c", help="count")
def build(count):
    pass


@cli.command(help="Init")
def init():
    pass


def invoke_completion(env):
    runner = CliRunner()
    result = runner.invoke(cli, [], env=env)
    return result.exit_code, result.output


def main():
    print("=== Completion ===")

    # Group command completion.
    code, out = invoke_completion(
        {"_CLI_COMPLETE": "bash_complete", "COMP_WORDS": "cli b", "COMP_CWORD": "1"}
    )
    print("# bash group:")
    print(f"  exit_code: {code}")
    print(f"  output: {out!r}")

    code, out = invoke_completion(
        {"_CLI_COMPLETE": "zsh_complete", "COMP_WORDS": "cli b", "COMP_CWORD": "1"}
    )
    print("# zsh group:")
    print(f"  exit_code: {code}")
    print(f"  output: {out!r}")

    code, out = invoke_completion(
        {"_CLI_COMPLETE": "fish_complete", "COMP_WORDS": "cli b", "COMP_CWORD": "b"}
    )
    print("# fish group:")
    print(f"  exit_code: {code}")
    print(f"  output: {out!r}")

    # Option completion inside subcommand.
    code, out = invoke_completion(
        {"_CLI_COMPLETE": "bash_complete", "COMP_WORDS": "cli build --", "COMP_CWORD": "2"}
    )
    print("# bash options:")
    print(f"  exit_code: {code}")
    print(f"  output: {out!r}")

    code, out = invoke_completion(
        {"_CLI_COMPLETE": "zsh_complete", "COMP_WORDS": "cli build --", "COMP_CWORD": "2"}
    )
    print("# zsh options:")
    print(f"  exit_code: {code}")
    print(f"  output: {out!r}")

    code, out = invoke_completion(
        {"_CLI_COMPLETE": "fish_complete", "COMP_WORDS": "cli build --", "COMP_CWORD": "--"}
    )
    print("# fish options:")
    print(f"  exit_code: {code}")
    print(f"  output: {out!r}")


if __name__ == "__main__":
    main()

