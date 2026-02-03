#!/usr/bin/env python3
"""
Parity tests for Click formatting utilities.
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
if click_src and not _maybe_add_click_src(click_src):
    raise RuntimeError(f"CLICK_SRC is set but not readable: {click_src!r}")

from click.formatting import wrap_text


def test_wrap_text():
    print("=== Wrap Text ===")

    text = "This is a test of the text wrapping functionality"
    wrapped = wrap_text(text, width=20)
    print("# basic:")
    print(f"  wrapped: {wrapped!r}")

    text2 = "Line one\nLine two\nLine three"
    wrapped2 = wrap_text(text2, width=80)
    print("# preserves newlines:")
    print(f"  wrapped: {wrapped2!r}")


def main():
    test_wrap_text()


if __name__ == "__main__":
    main()
