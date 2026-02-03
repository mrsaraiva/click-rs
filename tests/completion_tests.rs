//! Tests for shell completion functionality.

use click::command::Command;
use click::completion::{
    get_completion_class, get_completions, list_shells, make_completion_option, BashComplete,
    CompletionArgs, FishComplete, ShellComplete, ZshComplete,
};
use click::group::Group;
use click::option::ClickOption;
use click::CompletionItem;

// =============================================================================
// Shell Detection and Registry Tests
// =============================================================================

#[test]
fn test_get_completion_class_supported_shells() {
    assert!(get_completion_class("bash").is_some());
    assert!(get_completion_class("zsh").is_some());
    assert!(get_completion_class("fish").is_some());
}

#[test]
fn test_get_completion_class_unknown_shell() {
    assert!(get_completion_class("powershell").is_none());
    assert!(get_completion_class("cmd").is_none());
    assert!(get_completion_class("").is_none());
}

#[test]
fn test_get_completion_class_case_insensitive() {
    assert!(get_completion_class("BASH").is_some());
    assert!(get_completion_class("Bash").is_some());
    assert!(get_completion_class("bAsH").is_some());
}

#[test]
fn test_list_shells() {
    let shells = list_shells();
    assert!(shells.contains(&"bash"));
    assert!(shells.contains(&"zsh"));
    assert!(shells.contains(&"fish"));
    assert_eq!(shells.len(), 3);
}

// =============================================================================
// Shell Name Tests
// =============================================================================

#[test]
fn test_bash_complete_name() {
    let bash = BashComplete;
    assert_eq!(bash.name(), "bash");
}

#[test]
fn test_zsh_complete_name() {
    let zsh = ZshComplete;
    assert_eq!(zsh.name(), "zsh");
}

#[test]
fn test_fish_complete_name() {
    let fish = FishComplete;
    assert_eq!(fish.name(), "fish");
}

// =============================================================================
// Source Template Tests
// =============================================================================

#[test]
fn test_bash_source_template_structure() {
    let bash = BashComplete;
    let template = bash.source_template();

    // Should contain essential bash completion components
    assert!(template.contains("COMPREPLY"));
    assert!(template.contains("COMP_WORDS"));
    assert!(template.contains("COMP_CWORD"));
    assert!(template.contains("complete"));
}

#[test]
fn test_bash_get_source() {
    let bash = BashComplete;
    let source = bash.get_source("myapp", "_MYAPP_COMPLETE");

    // Placeholders should be replaced
    assert!(source.contains("myapp"));
    assert!(source.contains("_MYAPP_COMPLETE"));
    assert!(source.contains("_myapp_completion"));
    assert!(!source.contains("%(prog_name)s"));
    assert!(!source.contains("%(complete_var)s"));
    assert!(!source.contains("%(complete_func)s"));
}

#[test]
fn test_zsh_source_template_structure() {
    let zsh = ZshComplete;
    let template = zsh.source_template();

    // Should contain essential zsh completion components
    assert!(template.contains("#compdef"));
    assert!(template.contains("compadd"));
    assert!(template.contains("_describe"));
}

#[test]
fn test_zsh_get_source() {
    let zsh = ZshComplete;
    let source = zsh.get_source("myapp", "_MYAPP_COMPLETE");

    assert!(source.contains("#compdef myapp"));
    assert!(source.contains("_MYAPP_COMPLETE"));
}

#[test]
fn test_fish_source_template_structure() {
    let fish = FishComplete;
    let template = fish.source_template();

    // Should contain essential fish completion components
    assert!(template.contains("function"));
    assert!(template.contains("complete -c"));
    assert!(template.contains("commandline"));
}

#[test]
fn test_fish_get_source() {
    let fish = FishComplete;
    let source = fish.get_source("myapp", "_MYAPP_COMPLETE");

    assert!(source.contains("_MYAPP_COMPLETE"));
    assert!(source.contains("complete -c myapp"));
}

#[test]
fn test_prog_name_with_dashes() {
    let bash = BashComplete;
    let source = bash.get_source("my-cli-app", "_MY_CLI_APP_COMPLETE");

    // Function name should use underscores, not dashes
    assert!(source.contains("_my_cli_app_completion"));
    // But the prog name should remain as-is in certain places
    assert!(source.contains("my-cli-app"));
}

// =============================================================================
// Completion Formatting Tests
// =============================================================================

#[test]
fn test_bash_format_completion_plain() {
    let bash = BashComplete;
    let item = CompletionItem::new("--help");

    assert_eq!(bash.format_completion(&item), "plain,--help");
}

#[test]
fn test_bash_format_completion_with_help() {
    let bash = BashComplete;
    let item = CompletionItem::new("--name").with_help("Specify name");

    // Bash includes type + value (help is not included on bash output)
    assert_eq!(bash.format_completion(&item), "plain,--name");
}

#[test]
fn test_zsh_format_completion_plain() {
    let zsh = ZshComplete;
    let item = CompletionItem::new("--help");

    assert_eq!(zsh.format_completion(&item), "plain\n--help\n_");
}

#[test]
fn test_zsh_format_completion_with_help() {
    let zsh = ZshComplete;
    let item = CompletionItem::new("--name").with_help("Specify name");

    assert_eq!(zsh.format_completion(&item), "plain\n--name\nSpecify name");
}

#[test]
fn test_fish_format_completion_plain() {
    let fish = FishComplete;
    let item = CompletionItem::new("--help");

    // Fish uses type,value format
    assert_eq!(fish.format_completion(&item), "plain,--help");
}

#[test]
fn test_fish_format_completion_file_type() {
    let fish = FishComplete;
    let item = CompletionItem::with_type("path", "file");

    assert_eq!(fish.format_completion(&item), "file,path");
}

#[test]
fn test_fish_format_completion_dir_type() {
    let fish = FishComplete;
    let item = CompletionItem::with_type("path", "dir");

    assert_eq!(fish.format_completion(&item), "dir,path");
}

// =============================================================================
// Completion Generation Tests
// =============================================================================

#[test]
fn test_get_completions_help_option() {
    let cmd = Command::new("test").build();
    let completions = get_completions(&cmd, "test", &[], "");

    // Should always have --help
    assert!(completions.iter().any(|c| c.value == "--help"));
}

#[test]
fn test_get_completions_custom_options() {
    let cmd = Command::new("test")
        .option(
            ClickOption::new(&["--name", "-n"])
                .help("The name")
                .build(),
        )
        .option(
            ClickOption::new(&["--verbose", "-v"])
                .flag("true")
                .help("Be verbose")
                .build(),
        )
        .build();

    let completions = get_completions(&cmd, "test", &[], "--");

    assert!(completions.iter().any(|c| c.value == "--name"));
    assert!(completions.iter().any(|c| c.value == "--verbose"));
    assert!(completions.iter().any(|c| c.value == "--help"));
}

#[test]
fn test_get_completions_short_options() {
    let cmd = Command::new("test")
        .option(
            ClickOption::new(&["--name", "-n"])
                .help("The name")
                .build(),
        )
        .build();

    let completions = get_completions(&cmd, "test", &[], "-");

    assert!(completions.iter().any(|c| c.value == "-n"));
}

#[test]
fn test_get_completions_subcommands() {
    let group = Group::new("cli")
        .command(Command::new("init").help("Initialize").build())
        .command(Command::new("build").help("Build").build())
        .command(Command::new("deploy").help("Deploy").build())
        .build();

    let completions = get_completions(&group, "cli", &[], "");

    assert!(completions.iter().any(|c| c.value == "init"));
    assert!(completions.iter().any(|c| c.value == "build"));
    assert!(completions.iter().any(|c| c.value == "deploy"));
}

#[test]
fn test_get_completions_subcommand_prefix_filter() {
    let group = Group::new("cli")
        .command(Command::new("init").build())
        .command(Command::new("install").build())
        .command(Command::new("build").build())
        .build();

    let completions = get_completions(&group, "cli", &[], "in");

    assert!(completions.iter().any(|c| c.value == "init"));
    assert!(completions.iter().any(|c| c.value == "install"));
    assert!(!completions.iter().any(|c| c.value == "build"));
}

#[test]
fn test_get_completions_subcommand_with_help() {
    let group = Group::new("cli")
        .command(
            Command::new("init")
                .short_help("Initialize the project")
                .build(),
        )
        .build();

    let completions = get_completions(&group, "cli", &[], "");

    let init = completions.iter().find(|c| c.value == "init").unwrap();
    assert!(init.help.is_some());
    assert!(init.help.as_ref().unwrap().contains("Initialize"));
}

#[test]
fn test_get_completions_nested_subcommand() {
    let sub_group = Group::new("db")
        .command(Command::new("migrate").build())
        .command(Command::new("seed").build())
        .build();

    let main_group = Group::new("cli").command(sub_group).build();

    // Complete at top level
    let completions = get_completions(&main_group, "cli", &[], "");
    assert!(completions.iter().any(|c| c.value == "db"));

    // Complete within subgroup
    let completions = get_completions(&main_group, "cli", &["db".to_string()], "");
    assert!(completions.iter().any(|c| c.value == "migrate"));
    assert!(completions.iter().any(|c| c.value == "seed"));
}

// =============================================================================
// Completion Option Tests
// =============================================================================

#[test]
fn test_make_completion_option() {
    let opt = make_completion_option("_MYAPP_COMPLETE");
    assert_eq!(opt.complete_var, "_MYAPP_COMPLETE");
}

#[test]
fn test_completion_option_not_requested() {
    std::env::remove_var("_TEST_COMPLETE_VAR");

    let opt = make_completion_option("_TEST_COMPLETE_VAR");
    assert!(!opt.is_completion_requested());
    assert!(opt.get_completion_shell().is_none());
    assert!(!opt.is_source_request());
}

#[test]
fn test_completion_option_requested() {
    std::env::set_var("_TEST_COMPLETE_VAR2", "bash_complete");

    let opt = make_completion_option("_TEST_COMPLETE_VAR2");
    assert!(opt.is_completion_requested());
    assert_eq!(opt.get_completion_shell(), Some("bash".to_string()));
    assert!(!opt.is_source_request());

    std::env::remove_var("_TEST_COMPLETE_VAR2");
}

#[test]
fn test_completion_option_source_request() {
    std::env::set_var("_TEST_COMPLETE_VAR3", "zsh_source");

    let opt = make_completion_option("_TEST_COMPLETE_VAR3");
    assert!(opt.is_completion_requested());
    assert_eq!(opt.get_completion_shell(), Some("zsh".to_string()));
    assert!(opt.is_source_request());

    std::env::remove_var("_TEST_COMPLETE_VAR3");
}

// =============================================================================
// CompletionArgs Tests
// =============================================================================

#[test]
fn test_completion_args_default() {
    let args = CompletionArgs::default();
    assert!(args.args.is_empty());
    assert!(args.incomplete.is_empty());
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_completions_empty_command() {
    let cmd = Command::new("empty").build();
    let completions = get_completions(&cmd, "empty", &[], "");

    // Should still have help
    assert!(completions.iter().any(|c| c.value == "--help"));
}

#[test]
fn test_completions_hidden_options() {
    let cmd = Command::new("test")
        .option(
            ClickOption::new(&["--visible"])
                .help("Visible option")
                .build(),
        )
        .option(
            ClickOption::new(&["--hidden"])
                .help("Hidden option")
                .hidden()
                .build(),
        )
        .build();

    let completions = get_completions(&cmd, "test", &[], "--");

    // Both should be in completions (hidden affects help display, not completion)
    assert!(completions.iter().any(|c| c.value == "--visible"));
    // Hidden options are still completable
    assert!(completions.iter().any(|c| c.value == "--hidden"));
}

#[test]
fn test_completions_no_matching_prefix() {
    let group = Group::new("cli")
        .command(Command::new("build").build())
        .command(Command::new("test").build())
        .build();

    let completions = get_completions(&group, "cli", &[], "xyz");

    // No subcommands should match
    let subcommand_completions: Vec<_> = completions
        .iter()
        .filter(|c| !c.value.starts_with('-'))
        .collect();
    assert!(subcommand_completions.is_empty());
}

#[test]
fn test_completions_exact_match_prefix() {
    let group = Group::new("cli")
        .command(Command::new("build").build())
        .command(Command::new("builder").build())
        .build();

    let completions = get_completions(&group, "cli", &[], "build");

    assert!(completions.iter().any(|c| c.value == "build"));
    assert!(completions.iter().any(|c| c.value == "builder"));
}
