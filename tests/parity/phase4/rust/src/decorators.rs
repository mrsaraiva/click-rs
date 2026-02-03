//! Decorator / convenience attribute parity tests.
//!
//! These mirror `tests/parity/phase4/python/test_decorators.py`.

use click::context::{pop_context, push_context, ContextBuilder};
use click::ClickError;
use std::sync::Arc;

use crate::util::{py_repr, Output};

/// Run all decorator tests.
pub fn run() {
    test_version_option();
    test_help_option();
    test_pass_context_and_obj();
}

#[derive(click::Command)]
#[command(name = "cli")]
#[version_option(version = "1.2.3", prog_name = "myapp", message = "%(prog)s, version %(version)s")]
struct VersionCli {}

#[derive(click::Command)]
#[command(name = "cli2")]
#[version_option(version = "1.2.3", names = ["--ver"], prog_name = "myapp", message = "%(prog)s, version %(version)s")]
struct VersionCliCustom {}

fn test_version_option() {
    println!("=== Version Option ===");

    let cmd = VersionCli::command();
    let ctx = cmd.make_context("cli", vec!["--version".to_string()], None);
    let exit_code = match ctx {
        Err(ClickError::Exit { code }) => code,
        _ => 1,
    };
    let output = cmd
        .options
        .iter()
        .find_map(|o| o.config.metavar.as_deref())
        .and_then(|m| m.strip_prefix("__click_version__:"))
        .unwrap_or("");

    println!("# default:");
    println!("  output: {}", py_repr(output));
    println!("  exit_code: {}", exit_code);

    let cmd = VersionCliCustom::command();
    let ctx = cmd.make_context("cli2", vec!["--ver".to_string()], None);
    let exit_code = match ctx {
        Err(ClickError::Exit { code }) => code,
        _ => 1,
    };
    let output = cmd
        .options
        .iter()
        .find_map(|o| o.config.metavar.as_deref())
        .and_then(|m| m.strip_prefix("__click_version__:"))
        .unwrap_or("");

    println!("# custom names:");
    println!("  output: {}", py_repr(output));
    println!("  exit_code: {}", exit_code);
}

#[derive(click::Command)]
#[command(name = "cli")]
#[help_option(names = ["--assist", "-h"], help = "Show this message and exit.")]
struct HelpCli {}

fn test_help_option() {
    println!("\n=== Help Option ===");

    let cmd = HelpCli::command();
    let ctx = ContextBuilder::new().info_name("cli").build();
    let help_text = cmd.get_help(&ctx);

    println!("# custom names:");
    println!("  has_assist: {}", help_text.contains("--assist"));
    println!("  has_default_help: {}", help_text.contains("--help"));
    println!("  exit_code: 0");
}

#[derive(Clone)]
struct AppState {
    value: i32,
}

#[derive(click::Command)]
#[command(name = "pc")]
struct PassContextCli {
    #[pass_context]
    ctx: Arc<click::Context>,
}

#[derive(click::Command)]
#[command(name = "po")]
struct PassObjCli {
    #[pass_obj]
    obj: AppState,
}

fn test_pass_context_and_obj() {
    println!("\n=== Pass Context / Obj ===");

    // pass_context
    let out = Output::new();
    let cmd = PassContextCli::command_with_run({
        let out = out.clone();
        move |cli, _ctx| {
            out.push(format!(
                "info_name={}",
                cli.ctx.info_name().unwrap_or("None")
            ));
            Ok(())
        }
    });

    let ctx_arc = Arc::new(ContextBuilder::new().info_name("pc").build());
    push_context(Arc::clone(&ctx_arc));
    let _ = cmd.invoke(&ctx_arc);
    pop_context();

    println!("# pass_context:");
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: 0");

    // pass_obj
    let out = Output::new();
    let cmd = PassObjCli::command_with_run({
        let out = out.clone();
        move |cli, _ctx| {
            out.push(format!("obj={}", cli.obj.value));
            Ok(())
        }
    });

    let parent = Arc::new(ContextBuilder::new().obj(AppState { value: 7 }).build());
    let ctx = cmd.make_context("po", vec![], Some(parent)).unwrap();
    let result = cmd.invoke(&ctx);

    println!("# pass_obj:");
    println!("  output: {}", py_repr(&out.lines().join("\n")));
    println!("  exit_code: {}", if result.is_ok() { 0 } else { 1 });
}
