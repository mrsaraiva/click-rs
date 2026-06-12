//! Attribute parsing for derive macros.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Expr, ExprArray, ExprLit, ExprTuple, Lit, LitBool, LitChar, LitStr, Meta,
    MetaNameValue, Result, Token, Type,
};

fn parse_names_expr(expr: Expr) -> Result<Vec<String>> {
    fn parse_items(items: impl IntoIterator<Item = Expr>) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for item in items {
            match item {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => out.push(s.value()),
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "names must be string literals (e.g. names = [\"--help\", \"-h\"])",
                    ));
                }
            }
        }
        Ok(out)
    }

    match expr {
        Expr::Array(ExprArray { elems, .. }) => parse_items(elems.into_iter().collect::<Vec<_>>()),
        Expr::Tuple(ExprTuple { elems, .. }) => parse_items(elems.into_iter().collect::<Vec<_>>()),
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => Ok(vec![s.value()]),
        other => Err(syn::Error::new_spanned(
            other,
            "names must be a string literal or an array/tuple of string literals",
        )),
    }
}

fn parse_i32_expr(expr: Expr) -> Result<i32> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_parse::<i32>(),
        Expr::Unary(unary) => {
            if matches!(unary.op, syn::UnOp::Neg(_)) {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Int(i), ..
                }) = *unary.expr
                {
                    return i.base10_parse::<i32>().map(|v| -v);
                }
            }
            Err(syn::Error::new_spanned(
                unary,
                "nargs must be an integer literal",
            ))
        }
        other => Err(syn::Error::new_spanned(
            other,
            "nargs must be an integer literal",
        )),
    }
}

// =============================================================================
// Version Option Attributes
// =============================================================================

/// Parsed #[version_option(...)] attributes
#[derive(Debug, Clone, Default)]
pub struct VersionOptionAttr {
    /// The version string. If None, uses env!("CARGO_PKG_VERSION")
    pub version: Option<String>,
    /// The option names (default: ["--version", "-V"])
    #[allow(dead_code)]
    pub names: Option<Vec<String>>,
    /// The help text (default: "Show the version and exit.")
    pub help: Option<String>,
    /// The name of the program (default: inferred from command)
    pub prog_name: Option<String>,
    /// The message format (default: "%(prog)s, version %(version)s")
    pub message: Option<String>,
}

impl VersionOptionAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Option<Self>> {
        for attr in attrs {
            if attr.path().is_ident("version_option") {
                let mut result = VersionOptionAttr::default();

                // Handle #[version_option] with no parameters
                if matches!(attr.meta, Meta::Path(_)) {
                    return Ok(Some(result));
                }

                attr.parse_nested_meta(|meta| {
                    let ident = meta.path.get_ident().map(|i| i.to_string());

                    match ident.as_deref() {
                        Some("version") => {
                            let _: Token![=] = meta.input.parse()?;
                            let lit: LitStr = meta.input.parse()?;
                            result.version = Some(lit.value());
                        }
                        Some("help") => {
                            let _: Token![=] = meta.input.parse()?;
                            let lit: LitStr = meta.input.parse()?;
                            result.help = Some(lit.value());
                        }
                        Some("prog_name") => {
                            let _: Token![=] = meta.input.parse()?;
                            let lit: LitStr = meta.input.parse()?;
                            result.prog_name = Some(lit.value());
                        }
                        Some("message") => {
                            let _: Token![=] = meta.input.parse()?;
                            let lit: LitStr = meta.input.parse()?;
                            result.message = Some(lit.value());
                        }
                        Some("names") => {
                            let _: Token![=] = meta.input.parse()?;
                            let expr: Expr = meta.input.parse()?;
                            result.names = Some(parse_names_expr(expr)?);
                        }
                        _ => {
                            return Err(meta
                                .error(format!("unknown version_option attribute: {:?}", ident)));
                        }
                    }

                    Ok(())
                })?;

                return Ok(Some(result));
            }
        }
        Ok(None)
    }
}

// =============================================================================
// Help Option Attributes
// =============================================================================

/// Parsed #[help_option(...)] attributes
#[derive(Debug, Clone, Default)]
pub struct HelpOptionAttr {
    /// The option names (default: ["--help", "-h"])
    #[allow(dead_code)]
    pub names: Option<Vec<String>>,
    /// The help text (default: "Show this message and exit.")
    pub help: Option<String>,
}

impl HelpOptionAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Option<Self>> {
        for attr in attrs {
            if attr.path().is_ident("help_option") {
                let mut result = HelpOptionAttr::default();

                // Handle #[help_option] with no parameters
                if matches!(attr.meta, Meta::Path(_)) {
                    return Ok(Some(result));
                }

                attr.parse_nested_meta(|meta| {
                    let ident = meta.path.get_ident().map(|i| i.to_string());

                    match ident.as_deref() {
                        Some("help") => {
                            let _: Token![=] = meta.input.parse()?;
                            let lit: LitStr = meta.input.parse()?;
                            result.help = Some(lit.value());
                        }
                        Some("names") => {
                            let _: Token![=] = meta.input.parse()?;
                            let expr: Expr = meta.input.parse()?;
                            result.names = Some(parse_names_expr(expr)?);
                        }
                        _ => {
                            return Err(
                                meta.error(format!("unknown help_option attribute: {:?}", ident))
                            );
                        }
                    }

                    Ok(())
                })?;

                return Ok(Some(result));
            }
        }
        Ok(None)
    }
}

// =============================================================================
// Confirmation Option Attributes
// =============================================================================

/// Parsed #[confirmation_option(...)] attributes
/// Adds a --yes/-y option for skipping confirmation prompts
#[derive(Debug, Clone, Default)]
pub struct ConfirmationOptionAttr {
    /// The option names (default: ["--yes", "-y"])
    #[allow(dead_code)]
    pub names: Option<Vec<String>>,
    /// The help text (default: "Confirm the action without prompting.")
    pub help: Option<String>,
}

impl ConfirmationOptionAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Option<Self>> {
        for attr in attrs {
            if attr.path().is_ident("confirmation_option") {
                let mut result = ConfirmationOptionAttr::default();

                // Handle #[confirmation_option] with no parameters
                if matches!(attr.meta, Meta::Path(_)) {
                    return Ok(Some(result));
                }

                attr.parse_nested_meta(|meta| {
                    let ident = meta.path.get_ident().map(|i| i.to_string());

                    match ident.as_deref() {
                        Some("help") => {
                            let _: Token![=] = meta.input.parse()?;
                            let lit: LitStr = meta.input.parse()?;
                            result.help = Some(lit.value());
                        }
                        Some("names") => {
                            let _: Token![=] = meta.input.parse()?;
                            let expr: Expr = meta.input.parse()?;
                            result.names = Some(parse_names_expr(expr)?);
                        }
                        _ => {
                            return Err(meta.error(format!(
                                "unknown confirmation_option attribute: {:?}",
                                ident
                            )));
                        }
                    }

                    Ok(())
                })?;

                return Ok(Some(result));
            }
        }
        Ok(None)
    }
}

// =============================================================================
// Password Option Attributes
// =============================================================================

/// Parsed #[password_option(...)] attributes
/// Adds a --password option with hidden input and optional confirmation
#[derive(Debug, Clone, Default)]
pub struct PasswordOptionAttr {
    /// The option names (default: ["--password"])
    #[allow(dead_code)]
    pub names: Option<Vec<String>>,
    /// Prompt text (default: "Password")
    pub prompt: Option<String>,
    /// Whether to ask for confirmation (default: false)
    pub confirmation_prompt: bool,
    /// The help text (default: "")
    pub help: Option<String>,
}

impl PasswordOptionAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Option<Self>> {
        for attr in attrs {
            if attr.path().is_ident("password_option") {
                let mut result = PasswordOptionAttr::default();

                // Handle #[password_option] with no parameters
                if matches!(attr.meta, Meta::Path(_)) {
                    return Ok(Some(result));
                }

                attr.parse_nested_meta(|meta| {
                    let ident = meta.path.get_ident().map(|i| i.to_string());

                    match ident.as_deref() {
                        Some("prompt") => {
                            let _: Token![=] = meta.input.parse()?;
                            let lit: LitStr = meta.input.parse()?;
                            result.prompt = Some(lit.value());
                        }
                        Some("confirmation_prompt") => {
                            result.confirmation_prompt = true;
                        }
                        Some("help") => {
                            let _: Token![=] = meta.input.parse()?;
                            let lit: LitStr = meta.input.parse()?;
                            result.help = Some(lit.value());
                        }
                        Some("names") => {
                            let _: Token![=] = meta.input.parse()?;
                            let expr: Expr = meta.input.parse()?;
                            result.names = Some(parse_names_expr(expr)?);
                        }
                        _ => {
                            return Err(meta
                                .error(format!("unknown password_option attribute: {:?}", ident)));
                        }
                    }

                    Ok(())
                })?;

                return Ok(Some(result));
            }
        }
        Ok(None)
    }
}

/// Parsed #[command(...)] attributes
#[derive(Debug, Default)]
pub struct CommandAttr {
    pub name: Option<String>,
    pub help: Option<String>,
    pub short_help: Option<String>,
    pub epilog: Option<String>,
    pub run: bool,
    pub hidden: bool,
    pub deprecated: Option<String>,
    pub no_args_is_help: bool,
    pub add_help_option: bool,
    pub allow_extra_args: bool,
    pub allow_interspersed_args: bool,
    pub ignore_unknown_options: bool,
}

impl CommandAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut result = CommandAttr {
            add_help_option: true,         // default true
            allow_interspersed_args: true, // default true
            ..Default::default()
        };

        for attr in attrs {
            if attr.path().is_ident("command") {
                result.parse_command_attr(attr)?;
            }
        }

        Ok(result)
    }

    fn parse_command_attr(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            let ident = meta.path.get_ident().map(|i| i.to_string());

            match ident.as_deref() {
                Some("name") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    self.name = Some(lit.value());
                }
                Some("help") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    self.help = Some(lit.value());
                }
                Some("short_help") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    self.short_help = Some(lit.value());
                }
                Some("epilog") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    self.epilog = Some(lit.value());
                }
                Some("run") => {
                    self.run = true;
                }
                Some("hidden") => {
                    self.hidden = true;
                }
                Some("deprecated") => {
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: LitStr = meta.input.parse()?;
                        self.deprecated = Some(lit.value());
                    } else {
                        self.deprecated = Some(String::new());
                    }
                }
                Some("no_args_is_help") => {
                    self.no_args_is_help = true;
                }
                Some("add_help_option") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitBool = meta.input.parse()?;
                    self.add_help_option = lit.value();
                }
                Some("allow_extra_args") => {
                    self.allow_extra_args = true;
                }
                Some("allow_interspersed_args") => {
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: LitBool = meta.input.parse()?;
                        self.allow_interspersed_args = lit.value();
                    }
                }
                Some("ignore_unknown_options") => {
                    self.ignore_unknown_options = true;
                }
                _ => {
                    return Err(meta.error(format!("unknown command attribute: {:?}", ident)));
                }
            }

            Ok(())
        })
    }
}

/// Parsed #[group(...)] attributes
#[derive(Debug, Default)]
pub struct GroupAttr {
    pub command: CommandAttr,
    pub chain: bool,
    pub invoke_without_command: bool,
    pub subcommand_metavar: Option<String>,
    pub subcommand_required: bool,
}

impl GroupAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        // First parse command attrs (they share many fields)
        let mut result = GroupAttr {
            command: CommandAttr::from_attrs(attrs)?,
            ..Default::default()
        };

        for attr in attrs {
            if attr.path().is_ident("group") {
                result.parse_group_attr(attr)?;
            }
        }

        Ok(result)
    }

    fn parse_group_attr(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            let ident = meta.path.get_ident().map(|i| i.to_string());

            match ident.as_deref() {
                // Group-specific attributes
                Some("chain") => {
                    self.chain = true;
                }
                Some("invoke_without_command") => {
                    self.invoke_without_command = true;
                }
                Some("subcommand_metavar") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    self.subcommand_metavar = Some(lit.value());
                }
                Some("subcommand_required") => {
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: LitBool = meta.input.parse()?;
                        self.subcommand_required = lit.value();
                    } else {
                        self.subcommand_required = true;
                    }
                }
                // Also accept command-level attrs
                Some("name") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    self.command.name = Some(lit.value());
                }
                Some("help") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    self.command.help = Some(lit.value());
                }
                Some("hidden") => {
                    self.command.hidden = true;
                }
                Some("run") => {
                    self.command.run = true;
                }
                _ => {
                    return Err(meta.error(format!("unknown group attribute: {:?}", ident)));
                }
            }

            Ok(())
        })
    }
}

/// Parsed field attributes (options or arguments)
#[derive(Debug, Clone)]
pub enum FieldAttr {
    Option(OptionAttr),
    Argument(ArgumentAttr),
    #[allow(dead_code)]
    Subcommand(SubcommandAttr),
    /// Field marked with #[pass_context] - receives &Context in callback
    PassContext,
    /// Field marked with #[pass_obj] - receives ctx.obj::<T>() in callback
    PassObj,
    #[allow(dead_code)]
    Skip,
}

/// Parsed #[option(...)] attributes
#[derive(Debug, Clone, Default)]
pub struct OptionAttr {
    pub short: Option<char>,
    pub has_short: bool,
    pub long: Option<String>,
    pub has_long: bool,
    pub help: Option<String>,
    pub default: Option<TokenStream>,
    pub default_str: Option<String>,
    pub required: bool,
    pub hidden: bool,
    pub is_flag: bool,
    pub flag_value: Option<String>,
    pub is_count: bool,
    pub multiple: bool,
    pub envvar: Option<String>,
    pub prompt: Option<String>,
    pub confirmation_prompt: bool,
    pub hide_input: bool,
    pub show_default: bool,
    pub show_envvar: bool,
    pub metavar: Option<String>,
    pub nargs: Option<i32>,
    pub type_name: Option<String>,
    pub type_expr: Option<TokenStream>,
    pub value_name: Option<String>,
    pub dest: Option<String>,
    pub validate: Option<TokenStream>,
    pub shell_complete: Option<TokenStream>,
}

impl OptionAttr {
    pub fn from_attr(attr: &Attribute) -> Result<Self> {
        let mut result = OptionAttr::default();

        attr.parse_nested_meta(|meta| {
            let ident = meta.path.get_ident().map(|i| i.to_string());

            match ident.as_deref() {
                Some("short") => {
                    result.has_short = true;
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: LitChar = meta.input.parse()?;
                        result.short = Some(lit.value());
                    }
                }
                Some("long") => {
                    result.has_long = true;
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: LitStr = meta.input.parse()?;
                        result.long = Some(lit.value());
                    }
                }
                Some("help") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.help = Some(lit.value());
                }
                Some("default") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    // Store string representation for default
                    result.default_str = Some(expr_to_string(&expr));
                    result.default = Some(quote! { #expr });
                }
                Some("required") => {
                    result.required = true;
                }
                Some("hidden") => {
                    result.hidden = true;
                }
                Some("flag") => {
                    result.is_flag = true;
                }
                Some("flag_value") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.flag_value = Some(lit.value());
                }
                Some("count") => {
                    result.is_count = true;
                }
                Some("multiple") => {
                    result.multiple = true;
                }
                Some("envvar") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.envvar = Some(lit.value());
                }
                Some("prompt") => {
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: LitStr = meta.input.parse()?;
                        result.prompt = Some(lit.value());
                    } else {
                        result.prompt = Some(String::new()); // Will use field name
                    }
                }
                Some("confirmation_prompt") => {
                    result.confirmation_prompt = true;
                }
                Some("hide_input") => {
                    result.hide_input = true;
                }
                Some("show_default") => {
                    result.show_default = true;
                }
                Some("show_envvar") => {
                    result.show_envvar = true;
                }
                Some("metavar") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.metavar = Some(lit.value());
                }
                Some("nargs") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    result.nargs = Some(parse_i32_expr(expr)?);
                }
                Some("type") | Some("type_name") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = &expr
                    {
                        result.type_name = Some(s.value());
                    } else {
                        result.type_expr = Some(quote! { #expr });
                    }
                }
                Some("value_name") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.value_name = Some(lit.value());
                }
                Some("dest") | Some("param") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.dest = Some(lit.value());
                }
                Some("validate") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    result.validate = Some(quote! { #expr });
                }
                Some("shell_complete") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    result.shell_complete = Some(quote! { #expr });
                }
                _ => {
                    return Err(meta.error(format!("unknown option attribute: {:?}", ident)));
                }
            }

            Ok(())
        })?;

        Ok(result)
    }
}

/// Parsed #[argument(...)] attributes
#[derive(Debug, Clone, Default)]
pub struct ArgumentAttr {
    pub help: Option<String>,
    pub required: Option<bool>, // None means infer from type
    pub hidden: bool,
    pub multiple: bool,
    pub default: Option<TokenStream>,
    pub default_str: Option<String>,
    pub metavar: Option<String>,
    pub nargs: Option<i32>,
    pub envvar: Option<String>,
    pub type_name: Option<String>,
    pub type_expr: Option<TokenStream>,
    pub validate: Option<TokenStream>,
    pub shell_complete: Option<TokenStream>,
}

impl ArgumentAttr {
    pub fn from_attr(attr: &Attribute) -> Result<Self> {
        let mut result = ArgumentAttr::default();

        // Handle #[argument] with no parameters
        if matches!(attr.meta, Meta::Path(_)) {
            return Ok(result);
        }

        attr.parse_nested_meta(|meta| {
            let ident = meta.path.get_ident().map(|i| i.to_string());

            match ident.as_deref() {
                Some("help") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.help = Some(lit.value());
                }
                Some("required") => {
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: LitBool = meta.input.parse()?;
                        result.required = Some(lit.value());
                    } else {
                        result.required = Some(true);
                    }
                }
                Some("hidden") => {
                    result.hidden = true;
                }
                Some("multiple") => {
                    result.multiple = true;
                }
                Some("default") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    result.default_str = Some(expr_to_string(&expr));
                    result.default = Some(quote! { #expr });
                }
                Some("metavar") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.metavar = Some(lit.value());
                }
                Some("nargs") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    result.nargs = Some(parse_i32_expr(expr)?);
                }
                Some("envvar") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.envvar = Some(lit.value());
                }
                Some("type") | Some("type_name") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = &expr
                    {
                        result.type_name = Some(s.value());
                    } else {
                        result.type_expr = Some(quote! { #expr });
                    }
                }
                Some("validate") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    result.validate = Some(quote! { #expr });
                }
                Some("shell_complete") => {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    result.shell_complete = Some(quote! { #expr });
                }
                _ => {
                    return Err(meta.error(format!("unknown argument attribute: {:?}", ident)));
                }
            }

            Ok(())
        })?;

        Ok(result)
    }
}

/// Parsed #[subcommand] attributes
#[derive(Debug, Clone, Default)]
pub struct SubcommandAttr {
    pub name: Option<String>,
}

impl SubcommandAttr {
    pub fn from_attr(attr: &Attribute) -> Result<Self> {
        let mut result = SubcommandAttr::default();

        // Handle #[subcommand] with no parameters
        if matches!(attr.meta, Meta::Path(_)) {
            return Ok(result);
        }

        attr.parse_nested_meta(|meta| {
            let ident = meta.path.get_ident().map(|i| i.to_string());

            match ident.as_deref() {
                Some("name") => {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.name = Some(lit.value());
                }
                _ => {
                    return Err(meta.error(format!("unknown subcommand attribute: {:?}", ident)));
                }
            }

            Ok(())
        })?;

        Ok(result)
    }
}

impl FieldAttr {
    /// Parse field attributes to determine if it's an option, argument, subcommand,
    /// pass_context, or pass_obj
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Option<Self>> {
        for attr in attrs {
            if attr.path().is_ident("option") {
                return Ok(Some(FieldAttr::Option(OptionAttr::from_attr(attr)?)));
            }
            if attr.path().is_ident("argument") {
                return Ok(Some(FieldAttr::Argument(ArgumentAttr::from_attr(attr)?)));
            }
            if attr.path().is_ident("subcommand") {
                return Ok(Some(FieldAttr::Subcommand(SubcommandAttr::from_attr(
                    attr,
                )?)));
            }
            if attr.path().is_ident("pass_context") {
                return Ok(Some(FieldAttr::PassContext));
            }
            if attr.path().is_ident("pass_obj") {
                return Ok(Some(FieldAttr::PassObj));
            }
        }
        Ok(None)
    }
}

/// Extract doc comments from attributes
pub fn extract_doc_comment(attrs: &[Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let Meta::NameValue(MetaNameValue {
                    value:
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(s), ..
                        }),
                    ..
                }) = &attr.meta
                {
                    return Some(s.value());
                }
            }
            None
        })
        .collect();

    if docs.is_empty() {
        None
    } else {
        // Join doc lines and trim
        let joined = docs.iter().map(|s| s.trim()).collect::<Vec<_>>().join("\n");
        Some(joined.trim().to_string())
    }
}

/// Convert struct name to kebab-case for command name
pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert identifier to snake_case
#[allow(dead_code)]
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else if c == '-' {
            result.push('_');
        } else {
            result.push(c);
        }
    }
    result
}

/// Helper to convert expression to string for default values
fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Lit(ExprLit { lit, .. }) => match lit {
            Lit::Str(s) => s.value(),
            Lit::Int(i) => i.base10_digits().to_string(),
            Lit::Float(f) => f.base10_digits().to_string(),
            Lit::Bool(b) => b.value.to_string(),
            _ => quote!(#expr).to_string(),
        },
        _ => quote!(#expr).to_string(),
    }
}

/// Check if a type is Option<T>
pub fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Check if a type is Vec<T>
pub fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Vec";
        }
    }
    false
}

/// Check if a type is bool
pub fn is_bool_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "bool";
        }
    }
    false
}

/// Extract inner type from Option<T> or Vec<T>
pub fn extract_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

/// Get the type converter name for a Rust type
#[allow(dead_code)]
pub fn type_to_converter(ty: &Type) -> TokenStream {
    // Handle Option<T> and Vec<T>
    let inner_ty = extract_inner_type(ty).unwrap_or(ty);

    if let Type::Path(type_path) = inner_ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            match ident.as_str() {
                "String" | "str" => return quote! { click::STRING },
                "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
                    return quote! { click::INT }
                }
                "f32" | "f64" => return quote! { click::FLOAT },
                "bool" => return quote! { click::BOOL },
                "PathBuf" | "Path" => return quote! { click::PathType::new() },
                "Uuid" => return quote! { click::UUID },
                _ => {}
            }
        }
    }
    // Default to string
    quote! { click::STRING }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("HelloWorld"), "hello-world");
        assert_eq!(to_kebab_case("MyCommand"), "my-command");
        assert_eq!(to_kebab_case("CLI"), "c-l-i");
        assert_eq!(to_kebab_case("simple"), "simple");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("my-option"), "my_option");
    }
}
