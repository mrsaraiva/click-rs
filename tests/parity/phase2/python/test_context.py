#!/usr/bin/env python3
"""
Parity tests for Click Context.
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
from click import Context, Command
from click.globals import push_context, pop_context, get_current_context
from click.core import ParameterSource


def test_context_creation():
    """Test basic context creation and settings."""
    print("=== Context Creation ===")

    # Create a minimal command for context
    @click.command()
    def dummy():
        pass

    # Basic context with info_name
    print("# Context with info_name:")
    ctx = Context(dummy, info_name="myapp")
    print(f"info_name: {ctx.info_name}")
    print(f"parent: {ctx.parent}")
    print(f"allow_extra_args: {ctx.allow_extra_args}")
    print(f"allow_interspersed_args: {ctx.allow_interspersed_args}")
    print(f"ignore_unknown_options: {ctx.ignore_unknown_options}")
    print(f"resilient_parsing: {ctx.resilient_parsing}")
    print(f"help_option_names: {ctx.help_option_names}")

    # Context with custom settings
    print("# Context with custom settings:")
    ctx = Context(
        dummy,
        info_name="cli",
        terminal_width=120,
        max_content_width=100,
        color=True,
        show_default=False,
        resilient_parsing=True,
        allow_extra_args=True,
        allow_interspersed_args=False,
        ignore_unknown_options=True,
    )
    print(f"info_name: {ctx.info_name}")
    print(f"terminal_width: {ctx.terminal_width}")
    print(f"max_content_width: {ctx.max_content_width}")
    print(f"color: {ctx.color}")
    print(f"show_default: {ctx.show_default}")
    print(f"resilient_parsing: {ctx.resilient_parsing}")
    print(f"allow_extra_args: {ctx.allow_extra_args}")
    print(f"allow_interspersed_args: {ctx.allow_interspersed_args}")
    print(f"ignore_unknown_options: {ctx.ignore_unknown_options}")


def test_context_parent_chain():
    """Test parent-child context relationships."""
    print("\n=== Context Parent Chain ===")

    @click.command()
    def dummy():
        pass

    # Create parent-child chain
    print("# Parent-child chain:")
    parent = Context(dummy, info_name="cli")
    child = Context(dummy, info_name="subcommand", parent=parent)

    print(f"parent.info_name: {parent.info_name}")
    print(f"child.info_name: {child.info_name}")
    print(f"child.parent.info_name: {child.parent.info_name}")

    # Inheritance of settings
    print("# Inheritance from parent:")
    parent = Context(dummy, info_name="cli", terminal_width=100, color=True)
    child = Context(dummy, info_name="sub", parent=parent)

    print(f"parent.terminal_width: {parent.terminal_width}")
    print(f"child.terminal_width: {child.terminal_width}")
    print(f"parent.color: {parent.color}")
    print(f"child.color: {child.color}")

    # Child overrides parent
    print("# Child overrides parent:")
    child = Context(dummy, info_name="sub", parent=parent, terminal_width=80)
    print(f"parent.terminal_width: {parent.terminal_width}")
    print(f"child.terminal_width: {child.terminal_width}")


def test_command_path():
    """Test command path computation."""
    print("\n=== Command Path ===")

    @click.command()
    def dummy():
        pass

    # Single context
    print("# Single context:")
    ctx = Context(dummy, info_name="cli")
    print(f"command_path: '{ctx.command_path}'")

    # Two-level chain
    print("# Two-level chain:")
    parent = Context(dummy, info_name="cli")
    child = Context(dummy, info_name="subcommand", parent=parent)
    print(f"command_path: '{child.command_path}'")

    # Three-level chain
    print("# Three-level chain:")
    root = Context(dummy, info_name="cli")
    group = Context(dummy, info_name="group", parent=root)
    cmd = Context(dummy, info_name="command", parent=group)
    print(f"command_path: '{cmd.command_path}'")


def test_find_root():
    """Test find_root method."""
    print("\n=== Find Root ===")

    @click.command()
    def dummy():
        pass

    # Create a chain
    print("# Three-level chain:")
    root = Context(dummy, info_name="cli")
    group = Context(dummy, info_name="group", parent=root)
    cmd = Context(dummy, info_name="command", parent=group)

    print(f"root.find_root().info_name: {root.find_root().info_name}")
    print(f"group.find_root().info_name: {group.find_root().info_name}")
    print(f"cmd.find_root().info_name: {cmd.find_root().info_name}")


def test_context_stack():
    """Test thread-local context stack."""
    print("\n=== Context Stack ===")

    @click.command()
    def dummy():
        pass

    # Initially empty
    print("# Initially empty:")
    result = get_current_context(silent=True)
    print(f"get_current_context(silent=True): {result}")

    # Push and get
    print("# Push and get:")
    ctx1 = Context(dummy, info_name="ctx1")
    push_context(ctx1)
    current = get_current_context(silent=True)
    print(f"get_current_context().info_name: {current.info_name}")

    # Push another
    print("# Push another:")
    ctx2 = Context(dummy, info_name="ctx2")
    push_context(ctx2)
    current = get_current_context(silent=True)
    print(f"get_current_context().info_name: {current.info_name}")

    # Pop
    print("# Pop:")
    pop_context()
    current = get_current_context(silent=True)
    print(f"get_current_context().info_name: {current.info_name}")

    # Clean up
    pop_context()
    print("# After cleanup:")
    result = get_current_context(silent=True)
    print(f"get_current_context(silent=True): {result}")


def test_obj_storage():
    """Test obj storage and inheritance."""
    print("\n=== Obj Storage ===")

    @click.command()
    def dummy():
        pass

    # Set obj
    print("# Set obj:")
    ctx = Context(dummy, info_name="cli", obj={"key": "value"})
    print(f"obj: {ctx.obj}")

    # Inherit obj from parent
    print("# Inherit obj from parent:")
    parent = Context(dummy, info_name="cli", obj={"parent_key": "parent_value"})
    child = Context(dummy, info_name="sub", parent=parent)
    print(f"parent.obj: {parent.obj}")
    print(f"child.obj: {child.obj}")
    print(f"child.obj is parent.obj: {child.obj is parent.obj}")

    # Child can override obj
    print("# Child overrides obj:")
    child = Context(dummy, info_name="sub", parent=parent, obj={"child_key": "child_value"})
    print(f"parent.obj: {parent.obj}")
    print(f"child.obj: {child.obj}")


def test_meta_storage():
    """Test meta dictionary."""
    print("\n=== Meta Storage ===")

    @click.command()
    def dummy():
        pass

    # Set meta
    print("# Set and get meta:")
    ctx = Context(dummy, info_name="cli")
    ctx.meta["mymodule.key"] = "value"
    print(f"meta['mymodule.key']: {ctx.meta['mymodule.key']}")

    # Meta is shared between parent and child
    print("# Meta shared with child:")
    parent = Context(dummy, info_name="cli")
    parent.meta["shared.key"] = "shared_value"
    child = Context(dummy, info_name="sub", parent=parent)
    print(f"parent.meta['shared.key']: {parent.meta['shared.key']}")
    print(f"child.meta['shared.key']: {child.meta['shared.key']}")
    print(f"child.meta is parent.meta: {child.meta is parent.meta}")


def test_parameter_source():
    """Test parameter source tracking."""
    print("\n=== Parameter Source ===")

    @click.command()
    def dummy():
        pass

    ctx = Context(dummy, info_name="cli")

    # Initially no source
    print("# Initially no source:")
    source = ctx.get_parameter_source("name")
    print(f"get_parameter_source('name'): {source}")

    # Set sources
    print("# Set sources:")
    ctx._parameter_source["cli_param"] = ParameterSource.COMMANDLINE
    ctx._parameter_source["env_param"] = ParameterSource.ENVIRONMENT
    ctx._parameter_source["default_param"] = ParameterSource.DEFAULT
    ctx._parameter_source["default_map_param"] = ParameterSource.DEFAULT_MAP
    ctx._parameter_source["prompt_param"] = ParameterSource.PROMPT

    print(f"cli_param source: {ctx.get_parameter_source('cli_param').name}")
    print(f"env_param source: {ctx.get_parameter_source('env_param').name}")
    print(f"default_param source: {ctx.get_parameter_source('default_param').name}")
    print(f"default_map_param source: {ctx.get_parameter_source('default_map_param').name}")
    print(f"prompt_param source: {ctx.get_parameter_source('prompt_param').name}")


def test_auto_envvar_prefix():
    """Test auto_envvar_prefix computation."""
    print("\n=== Auto Envvar Prefix ===")

    @click.command()
    def dummy():
        pass

    # Direct prefix
    print("# Direct prefix:")
    ctx = Context(dummy, info_name="cli", auto_envvar_prefix="MYAPP")
    print(f"auto_envvar_prefix: {ctx.auto_envvar_prefix}")

    # Prefix with dashes (normalized)
    print("# Prefix with dashes:")
    ctx = Context(dummy, info_name="cli", auto_envvar_prefix="my-app")
    print(f"auto_envvar_prefix: {ctx.auto_envvar_prefix}")

    # Inherited and expanded
    print("# Inherited and expanded:")
    parent = Context(dummy, info_name="cli", auto_envvar_prefix="MYAPP")
    child = Context(dummy, info_name="sub-cmd", parent=parent)
    print(f"parent.auto_envvar_prefix: {parent.auto_envvar_prefix}")
    print(f"child.auto_envvar_prefix: {child.auto_envvar_prefix}")


def test_help_option_names():
    """Test help_option_names inheritance."""
    print("\n=== Help Option Names ===")

    @click.command()
    def dummy():
        pass

    # Default
    print("# Default:")
    ctx = Context(dummy, info_name="cli")
    print(f"help_option_names: {ctx.help_option_names}")

    # Custom
    print("# Custom:")
    ctx = Context(dummy, info_name="cli", help_option_names=["--help", "-h"])
    print(f"help_option_names: {ctx.help_option_names}")

    # Inherited
    print("# Inherited from parent:")
    parent = Context(dummy, info_name="cli", help_option_names=["--help", "-h", "-?"])
    child = Context(dummy, info_name="sub", parent=parent)
    print(f"parent.help_option_names: {parent.help_option_names}")
    print(f"child.help_option_names: {child.help_option_names}")


def main():
    """Run all context tests."""
    test_context_creation()
    test_context_parent_chain()
    test_command_path()
    test_find_root()
    test_context_stack()
    test_obj_storage()
    test_meta_storage()
    test_parameter_source()
    test_auto_envvar_prefix()
    test_help_option_names()


if __name__ == "__main__":
    main()
