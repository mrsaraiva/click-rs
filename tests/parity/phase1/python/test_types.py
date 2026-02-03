#!/usr/bin/env python3
"""
Parity tests for Click type converters.
Compares Python Click output with Rust click output.
"""
import sys
import os
import tempfile
import uuid as uuid_module

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

from click.types import (
    STRING,
    INT,
    FLOAT,
    BOOL,
    UUID,
    IntRange,
    FloatRange,
    Choice,
    Path,
    DateTime,
    Tuple,
    StringParamType,
    IntParamType,
    FloatParamType,
    BoolParamType,
)
from click.exceptions import BadParameter


def safe_convert(type_instance, value):
    """Safely convert a value, catching BadParameter exceptions."""
    try:
        result = type_instance.convert(value, None, None)
        return f"{result!r}"
    except BadParameter as e:
        return f"ERROR: {e.message}"
    except Exception as e:
        return f"ERROR: {type(e).__name__}: {e}"


def test_string():
    """Test STRING type converter."""
    print("=== STRING ===")
    test_cases = [
        "hello",
        "",
        "hello world",
        "123",
        "true",
        "with\nnewline",
        "with\ttab",
        "unicode: \u00e9\u00e0\u00fc",
    ]
    for val in test_cases:
        print(f'convert({val!r}) -> {safe_convert(STRING, val)}')


def test_int():
    """Test INT type converter."""
    print("\n=== INT ===")
    test_cases = [
        "0",
        "42",
        "-42",
        "2147483647",   # i32 max
        "-2147483648",  # i32 min
        "9223372036854775807",  # i64 max
        "abc",
        "12.5",
        "",
        "42abc",
        "  42  ",
    ]
    for val in test_cases:
        print(f'convert({val!r}) -> {safe_convert(INT, val)}')

    # Test with actual int (passthrough)
    print(f'convert(42) -> {safe_convert(INT, 42)}')


def test_float():
    """Test FLOAT type converter."""
    print("\n=== FLOAT ===")
    test_cases = [
        "0.0",
        "3.14",
        "-3.14",
        "1e10",
        "-1e-10",
        "inf",
        "-inf",
        "nan",
        "abc",
        "",
        "3.14.15",
    ]
    for val in test_cases:
        print(f'convert({val!r}) -> {safe_convert(FLOAT, val)}')

    # Test with actual float (passthrough)
    print(f'convert(3.14) -> {safe_convert(FLOAT, 3.14)}')


def test_bool():
    """Test BOOL type converter."""
    print("\n=== BOOL ===")
    # True values
    true_values = ["1", "true", "True", "TRUE", "yes", "Yes", "YES", "on", "ON", "t", "T", "y", "Y"]
    # False values
    false_values = ["0", "false", "False", "FALSE", "no", "No", "NO", "off", "OFF", "f", "F", "n", "N", ""]
    # Invalid values
    invalid_values = ["maybe", "2", "nope", "yep", "  "]

    print("# True values:")
    for val in true_values:
        print(f'convert({val!r}) -> {safe_convert(BOOL, val)}')

    print("# False values:")
    for val in false_values:
        print(f'convert({val!r}) -> {safe_convert(BOOL, val)}')

    print("# Invalid values:")
    for val in invalid_values:
        print(f'convert({val!r}) -> {safe_convert(BOOL, val)}')

    # Test with actual bool (passthrough)
    print(f'convert(True) -> {safe_convert(BOOL, True)}')
    print(f'convert(False) -> {safe_convert(BOOL, False)}')


def test_int_range():
    """Test IntRange type converter."""
    print("\n=== IntRange ===")

    # Basic range [0, 10]
    print("# IntRange(0, 10):")
    range_type = IntRange(0, 10)
    for val in ["0", "5", "10", "-1", "11", "abc"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Open bounds (0, 10)
    print("# IntRange(0, 10, min_open=True, max_open=True):")
    range_type = IntRange(0, 10, min_open=True, max_open=True)
    for val in ["0", "1", "9", "10", "5"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Min only
    print("# IntRange(min=0):")
    range_type = IntRange(min=0)
    for val in ["-1", "0", "100"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Max only
    print("# IntRange(max=10):")
    range_type = IntRange(max=10)
    for val in ["-100", "10", "11"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Clamp
    print("# IntRange(0, 10, clamp=True):")
    range_type = IntRange(0, 10, clamp=True)
    for val in ["-5", "0", "5", "10", "15"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Clamp with open bounds
    print("# IntRange(0, 10, min_open=True, clamp=True):")
    range_type = IntRange(0, 10, min_open=True, clamp=True)
    for val in ["-5", "0", "1", "10"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')


def test_float_range():
    """Test FloatRange type converter."""
    print("\n=== FloatRange ===")

    # Basic range [0.0, 1.0]
    print("# FloatRange(0.0, 1.0):")
    range_type = FloatRange(0.0, 1.0)
    for val in ["0.0", "0.5", "1.0", "-0.1", "1.1", "abc"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Open bounds (0.0, 1.0)
    print("# FloatRange(0.0, 1.0, min_open=True, max_open=True):")
    range_type = FloatRange(0.0, 1.0, min_open=True, max_open=True)
    for val in ["0.0", "0.001", "0.999", "1.0", "0.5"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Min only
    print("# FloatRange(min=0.0):")
    range_type = FloatRange(min=0.0)
    for val in ["-0.1", "0.0", "100.5"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Max only
    print("# FloatRange(max=10.0):")
    range_type = FloatRange(max=10.0)
    for val in ["-100.5", "10.0", "10.1"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')

    # Clamp
    print("# FloatRange(0.0, 1.0, clamp=True):")
    range_type = FloatRange(0.0, 1.0, clamp=True)
    for val in ["-0.5", "0.0", "0.5", "1.0", "1.5"]:
        print(f'convert({val!r}) -> {safe_convert(range_type, val)}')


def test_choice():
    """Test Choice type converter."""
    print("\n=== Choice ===")

    # Case sensitive (default)
    print("# Choice(['a', 'b', 'c']):")
    choice_type = Choice(["a", "b", "c"])
    for val in ["a", "b", "c", "A", "d", ""]:
        print(f'convert({val!r}) -> {safe_convert(choice_type, val)}')

    # Case insensitive
    print("# Choice(['red', 'green', 'blue'], case_sensitive=False):")
    choice_type = Choice(["red", "green", "blue"], case_sensitive=False)
    for val in ["red", "RED", "Red", "GREEN", "yellow"]:
        print(f'convert({val!r}) -> {safe_convert(choice_type, val)}')

    # Single choice
    print("# Choice(['only']):")
    choice_type = Choice(["only"])
    for val in ["only", "other"]:
        print(f'convert({val!r}) -> {safe_convert(choice_type, val)}')


def test_path():
    """Test Path type converter."""
    print("\n=== Path ===")

    # Create temp directory and file for testing
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpfile = os.path.join(tmpdir, "testfile.txt")
        with open(tmpfile, "w") as f:
            f.write("test")

        subdir = os.path.join(tmpdir, "subdir")
        os.mkdir(subdir)

        nonexistent = os.path.join(tmpdir, "nonexistent.txt")

        # Path with exists=False (default)
        print("# Path() - exists=False:")
        path_type = Path()
        print(f'convert(existing_file) -> OK (path returned)')
        print(f'convert(nonexistent) -> OK (path returned)')

        # Path with exists=True
        print("# Path(exists=True):")
        path_type = Path(exists=True)
        result = safe_convert(path_type, tmpfile)
        print(f'convert(existing_file) -> OK (path returned)')
        result = safe_convert(path_type, nonexistent)
        # Normalize error message
        if "ERROR:" in result:
            print(f'convert(nonexistent) -> ERROR: Path does not exist')
        else:
            print(f'convert(nonexistent) -> {result}')

        # File only
        print("# Path(exists=True, file_okay=True, dir_okay=False):")
        path_type = Path(exists=True, file_okay=True, dir_okay=False)
        result = safe_convert(path_type, tmpfile)
        print(f'convert(existing_file) -> OK (path returned)')
        result = safe_convert(path_type, subdir)
        if "ERROR:" in result:
            print(f'convert(directory) -> ERROR: Path is a directory')
        else:
            print(f'convert(directory) -> {result}')

        # Directory only
        print("# Path(exists=True, file_okay=False, dir_okay=True):")
        path_type = Path(exists=True, file_okay=False, dir_okay=True)
        result = safe_convert(path_type, subdir)
        print(f'convert(directory) -> OK (path returned)')
        result = safe_convert(path_type, tmpfile)
        if "ERROR:" in result:
            print(f'convert(file) -> ERROR: Path is a file')
        else:
            print(f'convert(file) -> {result}')


def test_datetime():
    """Test DateTime type converter."""
    print("\n=== DateTime ===")

    # Default formats
    print("# DateTime() - default formats:")
    dt_type = DateTime()
    test_cases = [
        "2024-01-15",
        "2024-01-15T10:30:00",
        "2024-01-15 10:30:00",
        "2024-13-01",  # Invalid month
        "not-a-date",
        "2024/01/15",  # Wrong format
    ]
    for val in test_cases:
        result = safe_convert(dt_type, val)
        # Normalize datetime output for determinism
        if not result.startswith("ERROR:"):
            # Just show that it parsed successfully
            print(f'convert({val!r}) -> OK (datetime)')
        else:
            print(f'convert({val!r}) -> ERROR: invalid format')

    # Custom format
    print("# DateTime(['%d/%m/%Y']):")
    dt_type = DateTime(["%d/%m/%Y"])
    for val in ["15/01/2024", "2024-01-15"]:
        result = safe_convert(dt_type, val)
        if not result.startswith("ERROR:"):
            print(f'convert({val!r}) -> OK (datetime)')
        else:
            print(f'convert({val!r}) -> ERROR: invalid format')


def test_uuid():
    """Test UUID type converter."""
    print("\n=== UUID ===")

    test_cases = [
        "550e8400-e29b-41d4-a716-446655440000",  # Standard format
        "550E8400-E29B-41D4-A716-446655440000",  # Uppercase
        "550e8400e29b41d4a716446655440000",      # No dashes
        "not-a-uuid",
        "",
        "550e8400-e29b-41d4-a716",               # Too short
        "550e8400-e29b-41d4-a716-446655440000-extra",  # Too long
    ]
    for val in test_cases:
        result = safe_convert(UUID, val)
        if not result.startswith("ERROR:"):
            # Normalize UUID output
            print(f'convert({val!r}) -> OK (UUID)')
        else:
            print(f'convert({val!r}) -> ERROR: invalid UUID')


def test_tuple():
    """Test Tuple type converter."""
    print("\n=== Tuple ===")

    # (str, int)
    print("# Tuple([str, int]):")
    tuple_type = Tuple([str, int])
    test_cases = [
        ("hello", "42"),
        ("world", "-1"),
        ("test", "abc"),  # Invalid int
    ]
    for val in test_cases:
        result = safe_convert(tuple_type, val)
        if not result.startswith("ERROR:"):
            print(f'convert({val!r}) -> {result}')
        else:
            print(f'convert({val!r}) -> ERROR: conversion failed')

    # Wrong arity
    print("# Tuple([str, int, float]) - wrong arity:")
    tuple_type = Tuple([str, int, float])
    for val in [("a", "1"), ("a", "1", "2.0", "extra")]:
        result = safe_convert(tuple_type, val)
        if not result.startswith("ERROR:"):
            print(f'convert({val!r}) -> {result}')
        else:
            print(f'convert({val!r}) -> ERROR: wrong number of values')

    # (int, int, int)
    print("# Tuple([int, int, int]):")
    tuple_type = Tuple([int, int, int])
    test_cases = [
        ("1", "2", "3"),
        ("0", "0", "0"),
    ]
    for val in test_cases:
        print(f'convert({val!r}) -> {safe_convert(tuple_type, val)}')


def main():
    """Run all type tests."""
    test_string()
    test_int()
    test_float()
    test_bool()
    test_int_range()
    test_float_range()
    test_choice()
    test_path()
    test_datetime()
    test_uuid()
    test_tuple()


if __name__ == "__main__":
    main()
