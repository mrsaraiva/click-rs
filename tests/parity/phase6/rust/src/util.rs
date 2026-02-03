/// Python-like `repr()` for a string, using single quotes when possible.
pub fn py_repr(value: &str) -> String {
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

