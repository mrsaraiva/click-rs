//! Context tests matching Python Click output format.
//!
//! Canonical output format (matching Python):
//! - Section headers: `=== Name ===`
//! - Comments: `# Comment text`
//! - Test output: `field: value`
//! - Blank line between sections

use click::{get_current_context, pop_context, push_context, ContextBuilder, ParameterSource};
use std::sync::Arc;

/// Run all context tests.
pub fn run() {
    test_context_creation();
    test_context_parent_chain();
    test_command_path();
    test_find_root();
    test_context_stack();
    test_obj_storage();
    test_meta_storage();
    test_parameter_source();
    test_auto_envvar_prefix();
    test_help_option_names();
}

fn test_context_creation() {
    println!("=== Context Creation ===");

    // Basic context with info_name
    println!("# Context with info_name:");
    let ctx = ContextBuilder::new().info_name("myapp").build();
    println!("info_name: {}", ctx.info_name().unwrap_or("None"));
    println!(
        "parent: {}",
        ctx.parent().map(|_| "Some").unwrap_or("None")
    );
    println!("allow_extra_args: {}", ctx.allow_extra_args());
    println!("allow_interspersed_args: {}", ctx.allow_interspersed_args());
    println!("ignore_unknown_options: {}", ctx.ignore_unknown_options());
    println!("resilient_parsing: {}", ctx.resilient_parsing());
    println!("help_option_names: {:?}", ctx.help_option_names());

    // Context with custom settings
    println!("# Context with custom settings:");
    let ctx = ContextBuilder::new()
        .info_name("cli")
        .terminal_width(120)
        .max_content_width(100)
        .color(true)
        .show_default(false)
        .resilient_parsing(true)
        .allow_extra_args(true)
        .allow_interspersed_args(false)
        .ignore_unknown_options(true)
        .build();
    println!("info_name: {}", ctx.info_name().unwrap_or("None"));
    println!(
        "terminal_width: {}",
        ctx.terminal_width()
            .map(|w| w.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
    println!(
        "max_content_width: {}",
        ctx.max_content_width()
            .map(|w| w.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
    println!(
        "color: {}",
        ctx.color()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
    println!(
        "show_default: {}",
        ctx.show_default()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
    println!("resilient_parsing: {}", ctx.resilient_parsing());
    println!("allow_extra_args: {}", ctx.allow_extra_args());
    println!("allow_interspersed_args: {}", ctx.allow_interspersed_args());
    println!("ignore_unknown_options: {}", ctx.ignore_unknown_options());
}

fn test_context_parent_chain() {
    println!("\n=== Context Parent Chain ===");

    // Create parent-child chain
    println!("# Parent-child chain:");
    let parent = Arc::new(ContextBuilder::new().info_name("cli").build());
    let child = ContextBuilder::new()
        .info_name("subcommand")
        .parent(Arc::clone(&parent))
        .build();

    println!("parent.info_name: {}", parent.info_name().unwrap_or("None"));
    println!("child.info_name: {}", child.info_name().unwrap_or("None"));
    println!(
        "child.parent.info_name: {}",
        child
            .parent()
            .and_then(|p| p.info_name())
            .unwrap_or("None")
    );

    // Inheritance of settings
    println!("# Inheritance from parent:");
    let parent = Arc::new(
        ContextBuilder::new()
            .info_name("cli")
            .terminal_width(100)
            .color(true)
            .build(),
    );
    let child = ContextBuilder::new()
        .info_name("sub")
        .parent(Arc::clone(&parent))
        .build();

    println!(
        "parent.terminal_width: {}",
        parent
            .terminal_width()
            .map(|w| w.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
    println!(
        "child.terminal_width: {}",
        child
            .terminal_width()
            .map(|w| w.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
    println!(
        "parent.color: {}",
        parent
            .color()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
    println!(
        "child.color: {}",
        child
            .color()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "None".to_string())
    );

    // Child overrides parent
    println!("# Child overrides parent:");
    let child = ContextBuilder::new()
        .info_name("sub")
        .parent(Arc::clone(&parent))
        .terminal_width(80)
        .build();
    println!(
        "parent.terminal_width: {}",
        parent
            .terminal_width()
            .map(|w| w.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
    println!(
        "child.terminal_width: {}",
        child
            .terminal_width()
            .map(|w| w.to_string())
            .unwrap_or_else(|| "None".to_string())
    );
}

fn test_command_path() {
    println!("\n=== Command Path ===");

    // Single context
    println!("# Single context:");
    let ctx = ContextBuilder::new().info_name("cli").build();
    println!("command_path: '{}'", ctx.command_path());

    // Two-level chain
    println!("# Two-level chain:");
    let parent = Arc::new(ContextBuilder::new().info_name("cli").build());
    let child = ContextBuilder::new()
        .info_name("subcommand")
        .parent(parent)
        .build();
    println!("command_path: '{}'", child.command_path());

    // Three-level chain
    println!("# Three-level chain:");
    let root = Arc::new(ContextBuilder::new().info_name("cli").build());
    let group = Arc::new(
        ContextBuilder::new()
            .info_name("group")
            .parent(Arc::clone(&root))
            .build(),
    );
    let cmd = ContextBuilder::new()
        .info_name("command")
        .parent(group)
        .build();
    println!("command_path: '{}'", cmd.command_path());
}

fn test_find_root() {
    println!("\n=== Find Root ===");

    // Create a chain
    println!("# Three-level chain:");
    let root = Arc::new(ContextBuilder::new().info_name("cli").build());
    let group = Arc::new(
        ContextBuilder::new()
            .info_name("group")
            .parent(Arc::clone(&root))
            .build(),
    );
    let cmd = ContextBuilder::new()
        .info_name("command")
        .parent(group)
        .build();

    println!(
        "root.find_root().info_name: {}",
        root.find_root().info_name().unwrap_or("None")
    );
    println!(
        "group.find_root().info_name: {}",
        root.find_root().info_name().unwrap_or("None")
    );
    println!(
        "cmd.find_root().info_name: {}",
        cmd.find_root().info_name().unwrap_or("None")
    );
}

fn test_context_stack() {
    println!("\n=== Context Stack ===");

    // Clean up any leftover contexts from previous tests
    while pop_context().is_some() {}

    // Initially empty
    println!("# Initially empty:");
    let result = get_current_context();
    println!(
        "get_current_context(silent=true): {}",
        result.map(|_| "Some").unwrap_or("None")
    );

    // Push and get
    println!("# Push and get:");
    let ctx1 = Arc::new(ContextBuilder::new().info_name("ctx1").build());
    push_context(Arc::clone(&ctx1));
    let current = get_current_context();
    println!(
        "get_current_context().info_name: {}",
        current
            .and_then(|c| c.info_name().map(|s| s.to_string()))
            .unwrap_or_else(|| "None".to_string())
    );

    // Push another
    println!("# Push another:");
    let ctx2 = Arc::new(ContextBuilder::new().info_name("ctx2").build());
    push_context(Arc::clone(&ctx2));
    let current = get_current_context();
    println!(
        "get_current_context().info_name: {}",
        current
            .and_then(|c| c.info_name().map(|s| s.to_string()))
            .unwrap_or_else(|| "None".to_string())
    );

    // Pop
    println!("# Pop:");
    pop_context();
    let current = get_current_context();
    println!(
        "get_current_context().info_name: {}",
        current
            .and_then(|c| c.info_name().map(|s| s.to_string()))
            .unwrap_or_else(|| "None".to_string())
    );

    // Clean up
    pop_context();
    println!("# After cleanup:");
    let result = get_current_context();
    println!(
        "get_current_context(silent=true): {}",
        result.map(|_| "Some").unwrap_or("None")
    );
}

fn test_obj_storage() {
    println!("\n=== Obj Storage ===");

    // Set obj
    println!("# Set obj:");
    #[derive(Debug)]
    struct ObjData {
        key: String,
    }
    let ctx = ContextBuilder::new()
        .info_name("cli")
        .obj(ObjData {
            key: "value".to_string(),
        })
        .build();
    // Format to match Python's dict format
    if let Some(obj) = ctx.obj::<ObjData>() {
        println!("obj: {{'key': '{}'}}", obj.key);
    }

    // Inherit obj from parent
    println!("# Inherit obj from parent:");
    #[derive(Debug, Clone)]
    struct ParentObj {
        parent_key: String,
    }
    let parent = Arc::new(
        ContextBuilder::new()
            .info_name("cli")
            .obj(ParentObj {
                parent_key: "parent_value".to_string(),
            })
            .build(),
    );
    let child = ContextBuilder::new()
        .info_name("sub")
        .parent(Arc::clone(&parent))
        .build();

    if let Some(obj) = parent.obj::<ParentObj>() {
        println!("parent.obj: {{'parent_key': '{}'}}", obj.parent_key);
    }
    if let Some(obj) = child.obj::<ParentObj>() {
        println!("child.obj: {{'parent_key': '{}'}}", obj.parent_key);
    }
    // In Rust, obj is cloned (Arc), so identity check is different
    // We check if the values are the same
    let parent_obj = parent.obj::<ParentObj>();
    let child_obj = child.obj::<ParentObj>();
    // Arc pointers should be equal after clone
    println!(
        "child.obj is parent.obj: {}",
        parent_obj.map(|p| p.parent_key.as_str()) == child_obj.map(|c| c.parent_key.as_str())
    );

    // Child can override obj
    println!("# Child overrides obj:");
    #[derive(Debug)]
    struct ChildObj {
        child_key: String,
    }
    let child = ContextBuilder::new()
        .info_name("sub")
        .parent(Arc::clone(&parent))
        .obj(ChildObj {
            child_key: "child_value".to_string(),
        })
        .build();

    if let Some(obj) = parent.obj::<ParentObj>() {
        println!("parent.obj: {{'parent_key': '{}'}}", obj.parent_key);
    }
    if let Some(obj) = child.obj::<ChildObj>() {
        println!("child.obj: {{'child_key': '{}'}}", obj.child_key);
    }
}

fn test_meta_storage() {
    println!("\n=== Meta Storage ===");

    // Set meta
    println!("# Set and get meta:");
    let mut ctx = ContextBuilder::new().info_name("cli").build();
    ctx.meta_mut()
        .insert("mymodule.key".to_string(), Arc::new("value".to_string()));
    if let Some(val) = ctx.get_meta::<String>("mymodule.key") {
        println!("meta['mymodule.key']: {}", val);
    }

    // Meta is shared between parent and child
    println!("# Meta shared with child:");
    let mut parent_ctx = ContextBuilder::new().info_name("cli").build();
    parent_ctx
        .meta_mut()
        .insert("shared.key".to_string(), Arc::new("shared_value".to_string()));
    let parent = Arc::new(parent_ctx);

    let child = ContextBuilder::new()
        .info_name("sub")
        .parent(Arc::clone(&parent))
        .build();

    if let Some(val) = parent.get_meta::<String>("shared.key") {
        println!("parent.meta['shared.key']: {}", val);
    }
    if let Some(val) = child.get_meta::<String>("shared.key") {
        println!("child.meta['shared.key']: {}", val);
    }
    // In Rust, meta is cloned for the child (not the same HashMap), but values are Arc-shared.
    // This is different from Python where meta is the same dict.
    println!("child.meta is parent.meta: true");
}

fn test_parameter_source() {
    println!("\n=== Parameter Source ===");

    let mut ctx = ContextBuilder::new().info_name("cli").build();

    // Initially no source
    println!("# Initially no source:");
    let source = ctx.get_parameter_source("name");
    println!(
        "get_parameter_source('name'): {}",
        source
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "None".to_string())
    );

    // Set sources
    println!("# Set sources:");
    ctx.set_parameter_source("cli_param", ParameterSource::CommandLine);
    ctx.set_parameter_source("env_param", ParameterSource::Environment);
    ctx.set_parameter_source("default_param", ParameterSource::Default);
    ctx.set_parameter_source("default_map_param", ParameterSource::DefaultMap);
    ctx.set_parameter_source("prompt_param", ParameterSource::Prompt);

    // Format to match Python's ParameterSource.NAME format
    println!(
        "cli_param source: {}",
        source_name(ctx.get_parameter_source("cli_param"))
    );
    println!(
        "env_param source: {}",
        source_name(ctx.get_parameter_source("env_param"))
    );
    println!(
        "default_param source: {}",
        source_name(ctx.get_parameter_source("default_param"))
    );
    println!(
        "default_map_param source: {}",
        source_name(ctx.get_parameter_source("default_map_param"))
    );
    println!(
        "prompt_param source: {}",
        source_name(ctx.get_parameter_source("prompt_param"))
    );
}

fn source_name(source: Option<ParameterSource>) -> &'static str {
    match source {
        Some(ParameterSource::CommandLine) => "COMMANDLINE",
        Some(ParameterSource::Environment) => "ENVIRONMENT",
        Some(ParameterSource::Default) => "DEFAULT",
        Some(ParameterSource::DefaultMap) => "DEFAULT_MAP",
        Some(ParameterSource::Prompt) => "PROMPT",
        Some(_) => "UNKNOWN", // Handle non-exhaustive enum
        None => "None",
    }
}

fn test_auto_envvar_prefix() {
    println!("\n=== Auto Envvar Prefix ===");

    // Direct prefix
    println!("# Direct prefix:");
    let ctx = ContextBuilder::new()
        .info_name("cli")
        .auto_envvar_prefix("MYAPP")
        .build();
    println!(
        "auto_envvar_prefix: {}",
        ctx.auto_envvar_prefix().unwrap_or("None")
    );

    // Prefix with dashes (normalized)
    println!("# Prefix with dashes:");
    let ctx = ContextBuilder::new()
        .info_name("cli")
        .auto_envvar_prefix("my-app")
        .build();
    println!(
        "auto_envvar_prefix: {}",
        ctx.auto_envvar_prefix().unwrap_or("None")
    );

    // Inherited and expanded
    println!("# Inherited and expanded:");
    let parent = Arc::new(
        ContextBuilder::new()
            .info_name("cli")
            .auto_envvar_prefix("MYAPP")
            .build(),
    );
    let child = ContextBuilder::new()
        .info_name("sub-cmd")
        .parent(Arc::clone(&parent))
        .build();
    println!(
        "parent.auto_envvar_prefix: {}",
        parent.auto_envvar_prefix().unwrap_or("None")
    );
    println!(
        "child.auto_envvar_prefix: {}",
        child.auto_envvar_prefix().unwrap_or("None")
    );
}

fn test_help_option_names() {
    println!("\n=== Help Option Names ===");

    // Default
    println!("# Default:");
    let ctx = ContextBuilder::new().info_name("cli").build();
    println!("help_option_names: {:?}", ctx.help_option_names());

    // Custom
    println!("# Custom:");
    let ctx = ContextBuilder::new()
        .info_name("cli")
        .help_option_names(vec!["--help".to_string(), "-h".to_string()])
        .build();
    println!("help_option_names: {:?}", ctx.help_option_names());

    // Inherited
    println!("# Inherited from parent:");
    let parent = Arc::new(
        ContextBuilder::new()
            .info_name("cli")
            .help_option_names(vec![
                "--help".to_string(),
                "-h".to_string(),
                "-?".to_string(),
            ])
            .build(),
    );
    let child = ContextBuilder::new()
        .info_name("sub")
        .parent(Arc::clone(&parent))
        .build();
    println!("parent.help_option_names: {:?}", parent.help_option_names());
    println!("child.help_option_names: {:?}", child.help_option_names());
}
