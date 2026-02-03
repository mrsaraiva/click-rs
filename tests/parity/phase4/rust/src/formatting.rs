//! Formatting parity tests matching Python Click output format.
//!
//! These mirror `tests/parity/phase4/python/test_formatting.py`.

use crate::util::py_repr;

pub fn run() {
    test_wrap_text();
}

fn test_wrap_text() {
    println!("=== Wrap Text ===");

    let text = "This is a test of the text wrapping functionality";
    let wrapped = click::wrap_text(text, 20);
    println!("# basic:");
    println!("  wrapped: {}", py_repr(&wrapped));

    let text2 = "Line one\nLine two\nLine three";
    let wrapped2 = click::wrap_text(text2, 80);
    println!("# preserves newlines:");
    println!("  wrapped: {}", py_repr(&wrapped2));
}

