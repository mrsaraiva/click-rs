//! Built-in shell completion helpers for common patterns.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{CompletionItem, Context};

/// Complete environment variable names using case-insensitive substring matching.
pub fn env_vars(_ctx: &Context, incomplete: &str) -> Vec<CompletionItem> {
    let needle = incomplete.to_lowercase();
    env::vars()
        .filter(|(key, _)| key.to_lowercase().contains(&needle))
        .map(|(key, _)| CompletionItem::new(key))
        .collect()
}

/// Complete directory paths.
///
/// Supports both bare names (`src`) and nested prefixes (`foo/bar`).
pub fn directories(_ctx: &Context, incomplete: &str) -> Vec<CompletionItem> {
    let (base_dir, prefix) = match incomplete.rsplit_once(std::path::MAIN_SEPARATOR) {
        Some((parent, tail)) if !parent.is_empty() => (PathBuf::from(parent), tail),
        _ => (PathBuf::from("."), incomplete),
    };

    fs::read_dir(&base_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(prefix) {
                return None;
            }

            let value = if base_dir == Path::new(".") {
                name
            } else {
                format!("{}{}{}", base_dir.display(), std::path::MAIN_SEPARATOR, name)
            };
            Some(CompletionItem::new(value))
        })
        .collect()
}

/// Build a completion callback from static values.
pub fn items(
    values: &'static [&'static str],
) -> impl Fn(&Context, &str) -> Vec<CompletionItem> + Send + Sync + 'static {
    move |_ctx: &Context, incomplete: &str| {
        values
            .iter()
            .copied()
            .filter(|value| value.contains(incomplete))
            .map(CompletionItem::new)
            .collect()
    }
}

/// Build a completion callback from static `(value, help)` pairs.
pub fn items_with_help(
    values: &'static [(&'static str, &'static str)],
) -> impl Fn(&Context, &str) -> Vec<CompletionItem> + Send + Sync + 'static {
    move |_ctx: &Context, incomplete: &str| {
        values
            .iter()
            .copied()
            .filter(|(value, help)| value.contains(incomplete) || help.contains(incomplete))
            .map(|(value, help)| CompletionItem::new(value).with_help(help))
            .collect()
    }
}
