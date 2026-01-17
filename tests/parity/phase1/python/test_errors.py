#!/usr/bin/env python3
"""
Parity tests for Click error formatting.
Compares Python Click output with Rust click output.
"""
import sys
import os
import io

# Insert Click library path before imports
# Use CLICK_SRC environment variable, or fall back to common locations
click_src = os.environ.get("CLICK_SRC")
if click_src:
    sys.path.insert(0, click_src)
else:
    # Try common locations
    for path in [
        os.path.expanduser("~/dev/mark/Proj/Libs/click/src"),
        "/home/msaraiva/dev/mark/Proj/Libs/click/src",
    ]:
        if os.path.isdir(path):
            sys.path.insert(0, path)
            break

from click.exceptions import (
    ClickException,
    UsageError,
    BadParameter,
    MissingParameter,
    NoSuchOption,
    FileError,
    BadOptionUsage,
    BadArgumentUsage,
)


def capture_show(exception):
    """Capture the output of exception.show() to a string."""
    buffer = io.StringIO()
    try:
        exception.show(file=buffer)
        return buffer.getvalue().rstrip()
    except Exception as e:
        return f"SHOW_ERROR: {type(e).__name__}: {e}"


def test_click_exception():
    """Test base ClickException."""
    print("=== ClickException ===")

    test_cases = [
        "Something went wrong",
        "File not found",
        "",
        "Error with 'quotes' and \"double quotes\"",
    ]

    for msg in test_cases:
        exc = ClickException(msg)
        print(f'ClickException({msg!r}):')
        print(f'  format_message() -> {exc.format_message()!r}')
        print(f'  show() -> {capture_show(exc)!r}')
        print(f'  exit_code -> {exc.exit_code}')


def test_usage_error():
    """Test UsageError."""
    print("\n=== UsageError ===")

    test_cases = [
        "Invalid command usage",
        "Missing required argument",
        "Too many arguments",
    ]

    for msg in test_cases:
        exc = UsageError(msg)
        print(f'UsageError({msg!r}):')
        print(f'  format_message() -> {exc.format_message()!r}')
        print(f'  show() -> {capture_show(exc)!r}')
        print(f'  exit_code -> {exc.exit_code}')


def test_bad_parameter():
    """Test BadParameter exception."""
    print("\n=== BadParameter ===")

    # Basic message only
    print("# BadParameter with message only:")
    exc = BadParameter("invalid value")
    print(f'BadParameter("invalid value"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # With param_hint string
    print("# BadParameter with param_hint (string):")
    exc = BadParameter("must be positive", param_hint="--count")
    print(f'BadParameter("must be positive", param_hint="--count"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # With param_hint list
    print("# BadParameter with param_hint (list):")
    exc = BadParameter("invalid choice", param_hint=["--color", "-c"])
    print(f'BadParameter("invalid choice", param_hint=["--color", "-c"]):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')


def test_missing_parameter():
    """Test MissingParameter exception."""
    print("\n=== MissingParameter ===")

    # For option
    print("# MissingParameter for option:")
    exc = MissingParameter(param_hint="--name", param_type="option")
    print(f'MissingParameter(param_hint="--name", param_type="option"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # For argument
    print("# MissingParameter for argument:")
    exc = MissingParameter(param_hint="FILENAME", param_type="argument")
    print(f'MissingParameter(param_hint="FILENAME", param_type="argument"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # With message
    print("# MissingParameter with message:")
    exc = MissingParameter(message="value is required", param_hint="--config", param_type="option")
    print(f'MissingParameter(message="value is required", param_hint="--config", param_type="option"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # Generic parameter
    print("# MissingParameter for generic parameter:")
    exc = MissingParameter(param_hint="VALUE", param_type="parameter")
    print(f'MissingParameter(param_hint="VALUE", param_type="parameter"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # No param_hint
    print("# MissingParameter with no param_hint:")
    exc = MissingParameter(param_type="option")
    print(f'MissingParameter(param_type="option"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')


def test_no_such_option():
    """Test NoSuchOption exception."""
    print("\n=== NoSuchOption ===")

    # Basic
    print("# NoSuchOption basic:")
    exc = NoSuchOption("--hlep")
    print(f'NoSuchOption("--hlep"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # With single suggestion
    print("# NoSuchOption with one suggestion:")
    exc = NoSuchOption("--hlep", possibilities=["--help"])
    print(f'NoSuchOption("--hlep", possibilities=["--help"]):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # With multiple suggestions
    print("# NoSuchOption with multiple suggestions:")
    exc = NoSuchOption("--colr", possibilities=["--color", "--colour"])
    print(f'NoSuchOption("--colr", possibilities=["--color", "--colour"]):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # With custom message
    print("# NoSuchOption with custom message:")
    exc = NoSuchOption("--foo", message="Unknown flag: --foo")
    print(f'NoSuchOption("--foo", message="Unknown flag: --foo"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')


def test_file_error():
    """Test FileError exception."""
    print("\n=== FileError ===")

    # Basic
    print("# FileError basic:")
    exc = FileError("/path/to/file.txt")
    print(f'FileError("/path/to/file.txt"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # With hint
    print("# FileError with hint:")
    exc = FileError("/path/to/file.txt", hint="Permission denied")
    print(f'FileError("/path/to/file.txt", hint="Permission denied"):')
    print(f'  format_message() -> {exc.format_message()!r}')
    print(f'  show() -> {capture_show(exc)!r}')

    # Different paths
    print("# FileError with different paths:")
    paths = [
        "/tmp/nonexistent.txt",
        "relative/path.txt",
        "/path with spaces/file.txt",
    ]
    for path in paths:
        exc = FileError(path, hint="No such file or directory")
        print(f'FileError({path!r}, hint="No such file or directory"):')
        print(f'  format_message() -> {exc.format_message()!r}')


def test_bad_option_usage():
    """Test BadOptionUsage exception."""
    print("\n=== BadOptionUsage ===")

    test_cases = [
        ("--count", "Option --count requires an argument"),
        ("--file", "Option --file takes 2 arguments but 1 was given"),
    ]

    for option_name, msg in test_cases:
        exc = BadOptionUsage(option_name, msg)
        print(f'BadOptionUsage({option_name!r}, {msg!r}):')
        print(f'  format_message() -> {exc.format_message()!r}')
        print(f'  show() -> {capture_show(exc)!r}')


def test_bad_argument_usage():
    """Test BadArgumentUsage exception."""
    print("\n=== BadArgumentUsage ===")

    test_cases = [
        "Got unexpected extra argument (bar)",
        "Argument 'src' takes 2 values but 1 was given",
    ]

    for msg in test_cases:
        exc = BadArgumentUsage(msg)
        print(f'BadArgumentUsage({msg!r}):')
        print(f'  format_message() -> {exc.format_message()!r}')
        print(f'  show() -> {capture_show(exc)!r}')


def test_exit_codes():
    """Test exit codes for different exception types."""
    print("\n=== Exit Codes ===")

    exceptions = [
        ("ClickException", ClickException("test")),
        ("UsageError", UsageError("test")),
        ("BadParameter", BadParameter("test")),
        ("MissingParameter", MissingParameter(param_type="option")),
        ("NoSuchOption", NoSuchOption("--test")),
        ("FileError", FileError("/test")),
        ("BadOptionUsage", BadOptionUsage("--test", "test")),
        ("BadArgumentUsage", BadArgumentUsage("test")),
    ]

    for name, exc in exceptions:
        print(f'{name}.exit_code -> {exc.exit_code}')


def main():
    """Run all error tests."""
    test_click_exception()
    test_usage_error()
    test_bad_parameter()
    test_missing_parameter()
    test_no_such_option()
    test_file_error()
    test_bad_option_usage()
    test_bad_argument_usage()
    test_exit_codes()


if __name__ == "__main__":
    main()
