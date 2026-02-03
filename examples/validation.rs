//! Validation example - demonstrates parameter validation techniques.
//!
//! This example shows validation through:
//! - Callbacks (validate_count)
//! - Custom types (URL type)
//! - Manual validation in the callback
//!
//! Equivalent to Python Click's examples/validation/validation.py

use click::{echo, ClickError, ClickOption, Command, Context, Result, TypeConverter};

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
        // Simple URL parsing - split on ://
        let mut parts = url.splitn(2, "://");
        let scheme = parts.next()?.to_lowercase();
        let rest = parts.next()?;

        // Split path from netloc
        let (netloc, path) = if let Some(slash_pos) = rest.find('/') {
            (&rest[..slash_pos], &rest[slash_pos..])
        } else {
            (rest, "")
        };

        Some(ParsedUrl {
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

/// Validate that count is a positive, even integer.
fn validate_count(value: &str) -> std::result::Result<i32, String> {
    let count: i32 = value
        .parse()
        .map_err(|_| format!("'{}' is not a valid integer", value))?;

    if count < 0 || count % 2 != 0 {
        return Err("Should be a positive, even integer.".to_string());
    }

    Ok(count)
}

/// Build the validation command.
fn build_command() -> Command {
    Command::new("validation")
        .help(
            "Validation.\n\n\
            This example validates parameters in different ways. It does it \
            through callbacks, through a custom type as well as by validating \
            manually in the function.",
        )
        .option(
            ClickOption::new(&["--count"])
                .default("2")
                .help("A positive even number.")
                .build(),
        )
        .option(
            ClickOption::new(&["--foo"])
                .help("A mysterious parameter.")
                .build(),
        )
        .option(
            ClickOption::new(&["--url"])
                .help("A URL")
                .build(),
        )
        .option(
            ClickOption::new(&["--version", "-V"])
                .flag("true")
                .eager()
                .help("Show the version and exit.")
                .build(),
        )
        .callback(cli_callback)
        .build()
}

/// The callback that performs validation and prints results.
fn cli_callback(ctx: &Context) -> Result<()> {
    // Check for version flag
    if let Some(version) = ctx.get_param::<String>("version") {
        if version == "true" {
            echo("validation, version 1.0", true, false, None);
            return Ok(());
        }
    }

    // Get and validate count
    let count_str = ctx
        .get_param::<String>("count")
        .cloned()
        .unwrap_or_else(|| "2".to_string());

    let count = validate_count(&count_str)
        .map_err(|e| ClickError::bad_parameter_named(e, "count"))?;

    // Get and validate foo
    let foo = ctx.get_param::<String>("foo").cloned();

    if let Some(ref foo_val) = foo {
        if foo_val != "wat" {
            return Err(ClickError::bad_parameter_named(
                "If a value is provided it needs to be the value \"wat\".",
                "foo",
            ));
        }
    }

    // Get and validate URL
    let url_str = ctx.get_param::<String>("url").cloned();
    let url = match url_str {
        Some(ref s) => {
            let url_type = UrlType;
            Some(
                url_type
                    .convert(s)
                    .map_err(|e| ClickError::bad_parameter_named(e, "url"))?,
            )
        }
        None => None,
    };

    // Print results
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
            url.map(|u| format!("{}", u))
                .unwrap_or_else(|| "None".to_string())
        ),
        true,
        false,
        None,
    );

    Ok(())
}

fn main() {
    let cmd = build_command();
    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Err(e) = cmd.main(args) {
        eprintln!("{}", e.format_full());
        std::process::exit(e.exit_code());
    }
}
