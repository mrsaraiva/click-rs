use std::sync::{Arc, Mutex};

use click::ClickError;

#[derive(Clone, Default)]
pub struct Output {
    inner: Arc<Mutex<Vec<String>>>,
}

impl Output {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, line: impl Into<String>) {
        self.inner.lock().unwrap().push(line.into());
    }

    pub fn lines(&self) -> Vec<String> {
        self.inner.lock().unwrap().clone()
    }
}

pub fn exit_code(result: &Result<(), ClickError>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(e) => e.exit_code(),
    }
}

pub fn py_bool(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

/// Python-like `repr()` for a string, using single quotes.
pub fn py_repr(value: &str) -> String {
    // Python's repr() chooses quote style to minimize escaping.
    // It prefers single quotes, but uses double quotes when the string contains
    // single quotes and no double quotes.
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };

    let mut out = String::with_capacity(value.len() + 2);
    out.push(quote);

    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let code = c as u32;
                if code <= 0xFF {
                    out.push_str(&format!("\\x{:02x}", code));
                } else if code <= 0xFFFF {
                    out.push_str(&format!("\\u{:04x}", code));
                } else {
                    out.push_str(&format!("\\U{:08x}", code));
                }
            }
            c => out.push(c),
        }
    }

    out.push(quote);
    out
}

pub fn py_list(items: &[String]) -> String {
    let mut out = String::from("[");
    for (i, item) in items.iter().enumerate() {
        if i != 0 {
            out.push_str(", ");
        }
        out.push_str(&py_repr(item));
    }
    out.push(']');
    out
}
