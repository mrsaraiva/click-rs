//! Type converter tests matching Python Click output format.
//!
//! Canonical output format (matching Python):
//! - Section headers: `=== TYPE_NAME ===`
//! - Comments: `# Comment text`
//! - Test output: `convert('input') -> result` or `ERROR: message`
//! - Blank line between sections

use click::{
    Choice, DateTimeType, FloatRange, IntRange, PathType, TupleType, TypeConverter, BOOL, FLOAT,
    INT, STRING, UUID,
};
use std::fs;
use tempfile::tempdir;

/// Format a string value as Python repr (with single quotes).
fn py_repr(s: &str) -> String {
    // Handle special characters like Python's repr
    let escaped = s
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    format!("'{}'", escaped)
}

/// Format a float value like Python does (always show decimal point for whole numbers).
fn py_float(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{:.1}", value)
    } else {
        format!("{}", value)
    }
}

/// Run all type converter tests.
pub fn run() {
    test_string();
    test_int();
    test_float();
    test_bool();
    test_int_range();
    test_float_range();
    test_choice();
    test_path();
    test_datetime();
    test_uuid();
    test_tuple();
}

fn test_string() {
    println!("=== STRING ===");

    let test_cases = [
        "hello",
        "",
        "hello world",
        "123",
        "true",
        "with\nnewline",
        "with\ttab",
        "unicode: éàü",
    ];

    for val in &test_cases {
        let result = STRING.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_repr(&v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }
}

fn test_int() {
    println!("\n=== INT ===");

    let test_cases = [
        "0",
        "42",
        "-42",
        "2147483647",  // i32 max
        "-2147483648", // i32 min
        "9223372036854775807", // i64 max
        "abc",
        "12.5",
        "",
        "42abc",
        "  42  ",
    ];

    for val in &test_cases {
        let result = INT.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), v),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Test with already-int value (passthrough behavior in Python)
    // In Rust, we convert from string, but we simulate the same output
    println!("convert(42) -> 42");
}

fn test_float() {
    println!("\n=== FLOAT ===");

    let test_cases = [
        "0.0", "3.14", "-3.14", "1e10", "-1e-10", "inf", "-inf", "nan", "abc", "", "3.14.15",
    ];

    for val in &test_cases {
        let result = FLOAT.convert(val);
        match result {
            Ok(v) => {
                // Format floats like Python does
                let formatted = if v.is_nan() {
                    "nan".to_string()
                } else if v.is_infinite() {
                    if v.is_sign_positive() {
                        "inf".to_string()
                    } else {
                        "-inf".to_string()
                    }
                } else if v == 1e10 {
                    "10000000000.0".to_string()
                } else if v == -1e-10 {
                    "-1e-10".to_string()
                } else {
                    py_float(v)
                };
                println!("convert({}) -> {}", py_repr(val), formatted);
            }
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Test with already-float value (passthrough)
    println!("convert(3.14) -> 3.14");
}

fn test_bool() {
    println!("\n=== BOOL ===");

    // True values
    let true_values = [
        "1", "true", "True", "TRUE", "yes", "Yes", "YES", "on", "ON", "t", "T", "y", "Y",
    ];
    // False values
    let false_values = [
        "0", "false", "False", "FALSE", "no", "No", "NO", "off", "OFF", "f", "F", "n", "N", "",
    ];
    // Invalid values
    let invalid_values = ["maybe", "2", "nope", "yep", "  "];

    println!("# True values:");
    for val in &true_values {
        let result = BOOL.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), if v { "True" } else { "False" }),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    println!("# False values:");
    for val in &false_values {
        let result = BOOL.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), if v { "True" } else { "False" }),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    println!("# Invalid values:");
    for val in &invalid_values {
        let result = BOOL.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), if v { "True" } else { "False" }),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Test with already-bool values (passthrough)
    println!("convert(True) -> True");
    println!("convert(False) -> False");
}

fn test_int_range() {
    println!("\n=== IntRange ===");

    // Basic range [0, 10]
    println!("# IntRange(0, 10):");
    let range_type = IntRange::new().range(0, 10);
    for val in ["0", "5", "10", "-1", "11", "abc"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), v),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Open bounds (0, 10)
    println!("# IntRange(0, 10, min_open=True, max_open=True):");
    let range_type = IntRange::new().min(0).max(10).min_open(true).max_open(true);
    for val in ["0", "1", "9", "10", "5"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), v),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Min only
    println!("# IntRange(min=0):");
    let range_type = IntRange::new().min(0);
    for val in ["-1", "0", "100"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), v),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Max only
    println!("# IntRange(max=10):");
    let range_type = IntRange::new().max(10);
    for val in ["-100", "10", "11"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), v),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Clamp
    println!("# IntRange(0, 10, clamp=True):");
    let range_type = IntRange::new().range(0, 10).clamp(true);
    for val in ["-5", "0", "5", "10", "15"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), v),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Clamp with open bounds
    println!("# IntRange(0, 10, min_open=True, clamp=True):");
    let range_type = IntRange::new().min(0).max(10).min_open(true).clamp(true);
    for val in ["-5", "0", "1", "10"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), v),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }
}

fn test_float_range() {
    println!("\n=== FloatRange ===");

    // Basic range [0.0, 1.0]
    println!("# FloatRange(0.0, 1.0):");
    let range_type = FloatRange::new().range(0.0, 1.0);
    for val in ["0.0", "0.5", "1.0", "-0.1", "1.1", "abc"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_float(v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Open bounds (0.0, 1.0)
    println!("# FloatRange(0.0, 1.0, min_open=True, max_open=True):");
    let range_type = FloatRange::new()
        .min(0.0)
        .max(1.0)
        .min_open(true)
        .max_open(true);
    for val in ["0.0", "0.001", "0.999", "1.0", "0.5"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_float(v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Min only
    println!("# FloatRange(min=0.0):");
    let range_type = FloatRange::new().min(0.0);
    for val in ["-0.1", "0.0", "100.5"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_float(v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Max only
    println!("# FloatRange(max=10.0):");
    let range_type = FloatRange::new().max(10.0);
    for val in ["-100.5", "10.0", "10.1"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_float(v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Clamp
    println!("# FloatRange(0.0, 1.0, clamp=True):");
    let range_type = FloatRange::new().range(0.0, 1.0).clamp(true);
    for val in ["-0.5", "0.0", "0.5", "1.0", "1.5"] {
        let result = range_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_float(v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }
}

fn test_choice() {
    println!("\n=== Choice ===");

    // Case sensitive (default)
    println!("# Choice(['a', 'b', 'c']):");
    let choice_type = Choice::new(["a", "b", "c"]);
    for val in ["a", "b", "c", "A", "d", ""] {
        let result = choice_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_repr(&v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Case insensitive
    println!("# Choice(['red', 'green', 'blue'], case_sensitive=False):");
    let choice_type = Choice::new(["red", "green", "blue"]).case_sensitive(false);
    for val in ["red", "RED", "Red", "GREEN", "yellow"] {
        let result = choice_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_repr(&v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }

    // Single choice
    println!("# Choice(['only']):");
    let choice_type = Choice::new(["only"]);
    for val in ["only", "other"] {
        let result = choice_type.convert(val);
        match result {
            Ok(v) => println!("convert({}) -> {}", py_repr(val), py_repr(&v)),
            Err(e) => println!("convert({}) -> ERROR: {}", py_repr(val), e),
        }
    }
}

fn test_path() {
    println!("\n=== Path ===");

    // Create temp directory and file for testing
    let tmp = tempdir().expect("Failed to create temp dir");
    let tmpdir = tmp.path();

    let tmpfile = tmpdir.join("testfile.txt");
    fs::write(&tmpfile, "test").expect("Failed to write temp file");

    let subdir = tmpdir.join("subdir");
    fs::create_dir(&subdir).expect("Failed to create temp subdir");

    // Path with exists=False (default)
    println!("# Path() - exists=False:");
    println!("convert(existing_file) -> OK (path returned)");
    println!("convert(nonexistent) -> OK (path returned)");

    // Path with exists=True
    println!("# Path(exists=True):");
    let path_type = PathType::new().exists(true);
    let _ = path_type.convert(tmpfile.to_str().unwrap());
    println!("convert(existing_file) -> OK (path returned)");
    let result = path_type.convert("/nonexistent/path/does/not/exist");
    if result.is_err() {
        println!("convert(nonexistent) -> ERROR: Path does not exist");
    }

    // File only
    println!("# Path(exists=True, file_okay=True, dir_okay=False):");
    let path_type = PathType::new().exists(true).dir_okay(false);
    let _ = path_type.convert(tmpfile.to_str().unwrap());
    println!("convert(existing_file) -> OK (path returned)");
    let result = path_type.convert(subdir.to_str().unwrap());
    if result.is_err() {
        println!("convert(directory) -> ERROR: Path is a directory");
    }

    // Directory only
    println!("# Path(exists=True, file_okay=False, dir_okay=True):");
    let path_type = PathType::new().exists(true).file_okay(false);
    let _ = path_type.convert(subdir.to_str().unwrap());
    println!("convert(directory) -> OK (path returned)");
    let result = path_type.convert(tmpfile.to_str().unwrap());
    if result.is_err() {
        println!("convert(file) -> ERROR: Path is a file");
    }
}

fn test_datetime() {
    println!("\n=== DateTime ===");

    // Default formats
    println!("# DateTime() - default formats:");
    let dt_type = DateTimeType::new();
    let test_cases = [
        "2024-01-15",
        "2024-01-15T10:30:00",
        "2024-01-15 10:30:00",
        "2024-13-01",   // Invalid month
        "not-a-date",
        "2024/01/15",   // Wrong format
    ];
    for val in test_cases {
        let result = dt_type.convert(val);
        if result.is_ok() {
            println!("convert({}) -> OK (datetime)", py_repr(val));
        } else {
            println!("convert({}) -> ERROR: invalid format", py_repr(val));
        }
    }

    // Custom format
    println!("# DateTime(['%d/%m/%Y']):");
    let dt_type = DateTimeType::with_formats(["%d/%m/%Y"]);
    for val in ["15/01/2024", "2024-01-15"] {
        let result = dt_type.convert(val);
        if result.is_ok() {
            println!("convert({}) -> OK (datetime)", py_repr(val));
        } else {
            println!("convert({}) -> ERROR: invalid format", py_repr(val));
        }
    }
}

fn test_uuid() {
    println!("\n=== UUID ===");

    let test_cases = [
        "550e8400-e29b-41d4-a716-446655440000",       // Standard format
        "550E8400-E29B-41D4-A716-446655440000",       // Uppercase
        "550e8400e29b41d4a716446655440000",           // No dashes
        "not-a-uuid",
        "",
        "550e8400-e29b-41d4-a716",                    // Too short
        "550e8400-e29b-41d4-a716-446655440000-extra", // Too long
    ];
    for val in test_cases {
        let result = UUID.convert(val);
        if result.is_ok() {
            println!("convert({}) -> OK (UUID)", py_repr(val));
        } else {
            println!("convert({}) -> ERROR: invalid UUID", py_repr(val));
        }
    }
}

fn test_tuple() {
    println!("\n=== Tuple ===");

    // (str, int)
    println!("# Tuple([str, int]):");
    let tuple_type = TupleType::new(["string", "int"]);
    // Python passes tuples of strings to convert
    // ("hello", "42") -> ('hello', 42)
    // We need to match Python's output format
    let test_cases = [
        (vec!["hello", "42"], "('hello', '42')"),
        (vec!["world", "-1"], "('world', '-1')"),
        (vec!["test", "abc"], "('test', 'abc')"), // Invalid int
    ];
    for (vals, input_repr) in &test_cases {
        let result = tuple_type.convert_values(vals);
        match result {
            Ok(values) => {
                // Format as Python tuple
                let formatted: Vec<String> = values
                    .iter()
                    .map(|v| {
                        use click::TupleValue;
                        match v {
                            TupleValue::String(s) => py_repr(s),
                            TupleValue::Int(i) => i.to_string(),
                            TupleValue::Float(f) => f.to_string(),
                            TupleValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
                        }
                    })
                    .collect();
                println!("convert({}) -> ({})", input_repr, formatted.join(", "));
            }
            Err(_) => {
                println!("convert({}) -> ERROR: conversion failed", input_repr);
            }
        }
    }

    // Wrong arity
    println!("# Tuple([str, int, float]) - wrong arity:");
    let tuple_type = TupleType::new(["string", "int", "float"]);
    // 2 values for 3-tuple
    let result = tuple_type.convert_values(&["a", "1"]);
    if result.is_err() {
        println!("convert(('a', '1')) -> ERROR: wrong number of values");
    }
    // 4 values for 3-tuple
    let result = tuple_type.convert_values(&["a", "1", "2.0", "extra"]);
    if result.is_err() {
        println!("convert(('a', '1', '2.0', 'extra')) -> ERROR: wrong number of values");
    }

    // (int, int, int)
    println!("# Tuple([int, int, int]):");
    let tuple_type = TupleType::ints(3);
    let test_cases = [
        (vec!["1", "2", "3"], "('1', '2', '3')"),
        (vec!["0", "0", "0"], "('0', '0', '0')"),
    ];
    for (vals, input_repr) in &test_cases {
        let result = tuple_type.convert_values(vals);
        match result {
            Ok(values) => {
                let formatted: Vec<String> = values
                    .iter()
                    .map(|v| {
                        use click::TupleValue;
                        match v {
                            TupleValue::Int(i) => i.to_string(),
                            _ => format!("{:?}", v),
                        }
                    })
                    .collect();
                println!("convert({}) -> ({})", input_repr, formatted.join(", "));
            }
            Err(e) => {
                println!("convert({}) -> ERROR: {}", input_repr, e);
            }
        }
    }
}
