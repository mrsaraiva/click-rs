//! Validation example - demonstrates declarative parameter validation.
//!
//! Equivalent to Python Click's examples/validation/validation.py

use click::{echo, run, Result, TypeConverter};

/// A URL parameter type that validates HTTP/HTTPS URLs.
#[derive(Debug, Clone)]
struct UrlType;

/// Parsed URL components (simplified version of Python's urlparse result).
#[derive(Debug, Clone)]
struct ParsedUrl {
    scheme: String,
    netloc: String,
    path: String,
}

impl ParsedUrl {
    fn parse(url: &str) -> Option<Self> {
        let mut parts = url.splitn(2, "://");
        let scheme = parts.next()?.to_lowercase();
        let rest = parts.next()?;
        let (netloc, path) = if let Some(slash_pos) = rest.find('/') {
            (&rest[..slash_pos], &rest[slash_pos..])
        } else {
            (rest, "")
        };
        Some(Self {
            scheme,
            netloc: netloc.to_string(),
            path: path.to_string(),
        })
    }
}

impl std::fmt::Display for ParsedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ParsedUrl(scheme='{}', netloc='{}', path='{}')",
            self.scheme, self.netloc, self.path
        )
    }
}

impl TypeConverter for UrlType {
    type Value = ParsedUrl;

    fn name(&self) -> &str {
        "URL"
    }

    fn convert(&self, value: &str) -> std::result::Result<Self::Value, String> {
        let parsed = ParsedUrl::parse(value).ok_or_else(|| format!("invalid URL: {}", value))?;
        if parsed.scheme != "http" && parsed.scheme != "https" {
            return Err(format!(
                "invalid URL scheme ({}). Only HTTP URLs are allowed",
                parsed.scheme
            ));
        }
        Ok(parsed)
    }

    fn get_metavar(&self) -> Option<String> {
        Some("URL".to_string())
    }
}

fn validate_count(value: &String) -> std::result::Result<(), String> {
    let count: i32 = value
        .parse()
        .map_err(|_| format!("'{}' is not a valid integer", value))?;
    if count < 0 || count % 2 != 0 {
        return Err("Should be a positive, even integer.".to_string());
    }
    Ok(())
}

fn validate_foo(value: &String) -> std::result::Result<(), String> {
    if value == "wat" {
        Ok(())
    } else {
        Err("If a value is provided it needs to be the value \"wat\".".to_string())
    }
}

#[click::command(name = "validation")]
fn validation(
    #[option(
        long,
        default = "2".to_string(),
        help = "A positive even number.",
        validate = validate_count
    )]
    count: String,
    #[option(long, help = "A mysterious parameter.", validate = validate_foo)]
    foo: Option<String>,
    #[option(long, help = "A URL", type = UrlType)]
    url: Option<ParsedUrl>,
) -> Result<()> {
    echo(&format!("count: {}", count), true, false, None);
    echo(
        &format!("foo: {}", foo.as_deref().unwrap_or("None")),
        true,
        false,
        None,
    );
    echo(
        &format!(
            "url: {}",
            url.map(|parsed| parsed.to_string())
                .unwrap_or_else(|| "None".to_string())
        ),
        true,
        false,
        None,
    );
    Ok(())
}

fn main() {
    run(&validation_command());
}
