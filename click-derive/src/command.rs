//! Command derive macro implementation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, Result, Type};

use crate::attrs::{
    extract_doc_comment, extract_inner_type, is_bool_type, is_option_type, is_vec_type,
    to_kebab_case, ArgumentAttr, CommandAttr, ConfirmationOptionAttr, FieldAttr, HelpOptionAttr,
    OptionAttr, PasswordOptionAttr, VersionOptionAttr,
};

/// Parsed field information
struct FieldInfo {
    name: Ident,
    ty: Type,
    attr: FieldAttr,
    doc: Option<String>,
}

/// Container-level convenience options parsed from attributes
struct ConvenienceOptions {
    version_option: Option<VersionOptionAttr>,
    help_option: Option<HelpOptionAttr>,
    confirmation_option: Option<ConfirmationOptionAttr>,
    password_option: Option<PasswordOptionAttr>,
}

/// Expand the Command derive macro
pub fn expand_command(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    // Parse container attributes
    let cmd_attr = CommandAttr::from_attrs(&input.attrs)?;

    // Parse convenience option attributes
    let convenience_opts = ConvenienceOptions {
        version_option: VersionOptionAttr::from_attrs(&input.attrs)?,
        help_option: HelpOptionAttr::from_attrs(&input.attrs)?,
        confirmation_option: ConfirmationOptionAttr::from_attrs(&input.attrs)?,
        password_option: PasswordOptionAttr::from_attrs(&input.attrs)?,
    };

    // Get doc comment for help
    let doc_comment = extract_doc_comment(&input.attrs);

    // Parse struct fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new_spanned(
                    name,
                    "Command derive only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                name,
                "Command derive only supports structs",
            ))
        }
    };

    // Collect field information
    let mut field_infos: Vec<FieldInfo> = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap().clone();
        let field_ty = field.ty.clone();
        let field_doc = extract_doc_comment(&field.attrs);

        if let Some(attr) = FieldAttr::from_attrs(&field.attrs)? {
            field_infos.push(FieldInfo {
                name: field_name,
                ty: field_ty,
                attr,
                doc: field_doc,
            });
        }
    }

    // Generate command name
    let cmd_name = cmd_attr
        .name
        .clone()
        .unwrap_or_else(|| to_kebab_case(&name.to_string()));

    // Generate help text
    let help_text = cmd_attr
        .help
        .clone()
        .or(doc_comment)
        .unwrap_or_default();

    // Generate option builders
    let option_builders = generate_option_builders(&field_infos)?;

    // Generate argument builders
    let argument_builders = generate_argument_builders(&field_infos)?;

    // Generate convenience option builders
    let convenience_option_builders = generate_convenience_option_builders(&convenience_opts, &cmd_name);

    // Generate field extraction from context
    let field_extractions = generate_field_extractions(&field_infos)?;

    // Generate the field names for struct construction
    let field_names: Vec<_> = field_infos.iter().map(|f| &f.name).collect();

    // Command builder attributes
    let no_args_is_help = cmd_attr.no_args_is_help;
    let add_help_option = cmd_attr.add_help_option;
    let allow_extra_args = cmd_attr.allow_extra_args;
    let allow_interspersed_args = cmd_attr.allow_interspersed_args;
    let ignore_unknown_options = cmd_attr.ignore_unknown_options;
    let auto_run = cmd_attr.run;

    // hidden() takes no args, so conditionally add it
    let hidden_opt = if cmd_attr.hidden {
        quote! { .hidden() }
    } else {
        quote! {}
    };

    let epilog_opt = match &cmd_attr.epilog {
        Some(e) => quote! { .epilog(#e) },
        None => quote! {},
    };

    let short_help_opt = match &cmd_attr.short_help {
        Some(h) => quote! { .short_help(#h) },
        None => quote! {},
    };

    let deprecated_opt = match &cmd_attr.deprecated {
        Some(d) if !d.is_empty() => quote! { .deprecated(Some(#d.to_string())) },
        Some(_) => quote! { .deprecated(Some(String::new())) },
        None => quote! {},
    };

    let auto_run_callback_opt = if auto_run {
        quote! {
            .callback(|ctx| {
                let instance = #name::from_context(ctx)?;
                instance.run(ctx)
            })
        }
    } else {
        quote! {}
    };

    let output = quote! {
        impl #name {
            /// Build a click Command from this struct's definition.
            pub fn command() -> click::Command {
                click::Command::new(#cmd_name)
                    .help(#help_text)
                    #hidden_opt
                    .no_args_is_help(#no_args_is_help)
                    .add_help_option(#add_help_option)
                    .allow_extra_args(#allow_extra_args)
                    .allow_interspersed_args(#allow_interspersed_args)
                    .ignore_unknown_options(#ignore_unknown_options)
                    #epilog_opt
                    #short_help_opt
                    #deprecated_opt
                    #(#option_builders)*
                    #(#argument_builders)*
                    #(#convenience_option_builders)*
                    #auto_run_callback_opt
                    .build()
            }

            /// Build a click Command with a callback that calls self.run().
            pub fn command_with_run<F>(run_fn: F) -> click::Command
            where
                F: Fn(#name, &click::Context) -> click::Result<()> + Send + Sync + 'static,
            {
                click::Command::new(#cmd_name)
                    .help(#help_text)
                    #hidden_opt
                    .no_args_is_help(#no_args_is_help)
                    .add_help_option(#add_help_option)
                    .allow_extra_args(#allow_extra_args)
                    .allow_interspersed_args(#allow_interspersed_args)
                    .ignore_unknown_options(#ignore_unknown_options)
                    #epilog_opt
                    #short_help_opt
                    #deprecated_opt
                    #(#option_builders)*
                    #(#argument_builders)*
                    #(#convenience_option_builders)*
                    .callback(move |ctx| {
                        let instance = #name::from_context(ctx)?;
                        run_fn(instance, ctx)
                    })
                    .build()
            }

            /// Extract field values from a Context to create an instance.
            pub fn from_context(ctx: &click::Context) -> click::Result<Self> {
                #(#field_extractions)*

                Ok(Self {
                    #(#field_names,)*
                })
            }

            /// Run this command with the given arguments.
            pub fn main_with<F>(args: Vec<String>, run_fn: F) -> click::Result<()>
            where
                F: Fn(#name, &click::Context) -> click::Result<()> + Send + Sync + 'static,
            {
                Self::command_with_run(run_fn).main(args)
            }
        }
    };

    Ok(output)
}

/// Generate convenience option builders (version, confirmation, password)
fn generate_convenience_option_builders(
    opts: &ConvenienceOptions,
    cmd_name: &str,
) -> Vec<TokenStream> {
    let mut builders = Vec::new();

    // Help option override.
    if let Some(ref help_attr) = opts.help_option {
        let mut names = help_attr
            .names
            .clone()
            .unwrap_or_else(|| vec!["--help".to_string()]);
        if !names.iter().any(|n| n == "--help") {
            names.push("--help".to_string());
        }
        let help = help_attr
            .help
            .clone()
            .unwrap_or_else(|| "Show this message and exit.".to_string());

        builders.push(quote! {
            .help_option(
                click::ClickOption::new(&[#(#names),*])
                    .help(#help)
                    .flag("true")
                    .eager()
                    .build()
            )
        });
    }

    // Version option: --version / -V
    if let Some(ref ver_attr) = opts.version_option {
        let names = ver_attr
            .names
            .clone()
            .unwrap_or_else(|| vec!["--version".to_string(), "-V".to_string()]);
        let help = ver_attr
            .help
            .clone()
            .unwrap_or_else(|| "Show the version and exit.".to_string());
        let prog_name = ver_attr.prog_name.clone().unwrap_or_else(|| cmd_name.to_string());
        let message = ver_attr
            .message
            .clone()
            .unwrap_or_else(|| "%(prog)s, version %(version)s".to_string());

        let version_expr = match ver_attr.version.as_ref() {
            Some(v) => quote! { #v },
            None => quote! { env!("CARGO_PKG_VERSION") },
        };

        builders.push(quote! {
            .option({
                let __click_version: &str = #version_expr;
                let __click_prog: &str = #prog_name;
                let __click_template: &str = #message;
                let __click_out = __click_template
                    .replace("%(prog)s", __click_prog)
                    .replace("%(version)s", __click_version);
                let __click_meta = format!("{}{}", "__click_version__:", __click_out);
                click::ClickOption::new(&[#(#names),*])
                    .help(#help)
                    .flag("true")
                    .eager()
                    .metavar(&__click_meta)
                    .build()
            })
        });
    }

    // Confirmation option: --yes / -y
    if let Some(ref conf_attr) = opts.confirmation_option {
        let names = conf_attr
            .names
            .clone()
            .unwrap_or_else(|| vec!["--yes".to_string(), "-y".to_string()]);
        let help = conf_attr
            .help
            .clone()
            .unwrap_or_else(|| "Confirm the action without prompting.".to_string());

        builders.push(quote! {
            .option(
                click::ClickOption::new(&[#(#names),*])
                    .help(#help)
                    .flag("true")
                    .build()
            )
        });
    }

    // Password option: --password with hidden input
    if let Some(ref pass_attr) = opts.password_option {
        let names = pass_attr
            .names
            .clone()
            .unwrap_or_else(|| vec!["--password".to_string()]);
        let prompt = pass_attr.prompt.clone().unwrap_or_else(|| "Password".to_string());
        let help = pass_attr.help.clone().unwrap_or_default();
        let confirmation = pass_attr.confirmation_prompt;

        builders.push(quote! {
            .option(
                click::ClickOption::new(&[#(#names),*])
                    .help(#help)
                    .prompt(#prompt)
                    .hide_input(true)
                    .confirmation_prompt(#confirmation)
                    .build()
            )
        });
    }

    builders
}

/// Generate option builder calls
fn generate_option_builders(fields: &[FieldInfo]) -> Result<Vec<TokenStream>> {
    let mut builders = Vec::new();

    for field in fields {
        if let FieldAttr::Option(opt_attr) = &field.attr {
            let field_name = &field.name;
            let field_name_str = field_name.to_string();
            let field_ty = &field.ty;

            // Determine option names
            let mut names = Vec::new();

            if opt_attr.has_long {
                let long_name = opt_attr
                    .long
                    .clone()
                    .unwrap_or_else(|| field_name_str.replace('_', "-"));
                names.push(format!("--{}", long_name));
            }

            if opt_attr.has_short {
                let short_char = opt_attr
                    .short
                    .unwrap_or_else(|| field_name_str.chars().next().unwrap());
                names.push(format!("-{}", short_char));
            }

            // If neither short nor long specified, default to long
            if names.is_empty() {
                names.push(format!("--{}", field_name_str.replace('_', "-")));
            }

            let names_arr: Vec<&str> = names.iter().map(|s| s.as_str()).collect();

            // Get help text (attribute or doc comment)
            let help_text = opt_attr.help.clone().or_else(|| field.doc.clone());
            let help_opt = match help_text {
                Some(h) => quote! { .help(#h) },
                None => quote! {},
            };

            // Handle default value
            let default_opt = match &opt_attr.default_str {
                Some(d) => quote! { .default(#d) },
                None => quote! {},
            };

            // Handle required
            let is_required = opt_attr.required;
            let required_opt = if is_required {
                quote! { .required() }
            } else {
                quote! {}
            };

            // Handle hidden
            let hidden_opt = if opt_attr.hidden {
                quote! { .hidden() }
            } else {
                quote! {}
            };

            // Handle is_flag (for bool types)
            let is_flag = opt_attr.is_flag || is_bool_type(field_ty);
            let flag_opt = if let Some(flag_value) = &opt_attr.flag_value {
                quote! { .flag(#flag_value) }
            } else if is_flag && !opt_attr.is_count {
                quote! { .bool_flag() }
            } else {
                quote! {}
            };

            // Handle count
            let count_opt = if opt_attr.is_count {
                quote! { .count() }
            } else {
                quote! {}
            };

            // Handle multiple
            let is_multiple = opt_attr.multiple || is_vec_type(field_ty);
            let multiple_opt = if is_multiple {
                quote! { .multiple() }
            } else {
                quote! {}
            };

            // Handle envvar
            let envvar_opt = match &opt_attr.envvar {
                Some(e) => quote! { .envvar(#e) },
                None => quote! {},
            };

            // Handle show_default
            let show_default_opt = if opt_attr.show_default {
                quote! { .show_default(true) }
            } else {
                quote! {}
            };

            // Handle metavar
            let metavar_opt = match &opt_attr.metavar {
                Some(m) => quote! { .metavar(#m) },
                None => quote! {},
            };

            // Handle nargs
            let nargs_opt = match opt_attr.nargs {
                Some(-1) => quote! { .nargs(click::Nargs::Variadic) },
                Some(n) if n >= 0 => quote! { .nargs(click::Nargs::Count(#n as usize)) },
                Some(other) => {
                    return Err(Error::new_spanned(
                        field_name,
                        format!("unsupported option nargs value: {}", other),
                    ))
                }
                None => quote! {},
            };

            // Handle explicit type converter expression
            let type_opt = match &opt_attr.type_expr {
                Some(expr) => quote! { .type_any(#expr) },
                None => quote! {},
            };

            // Handle validator callback
            let validate_opt = match &opt_attr.validate {
                Some(validator) => {
                    let validator_ty = if is_option_type(field_ty) {
                        let inner = extract_inner_type(field_ty).ok_or_else(|| {
                            Error::new_spanned(field_ty, "#[option(validate = ...)] requires Option<T> to use a concrete T")
                        })?;
                        quote! { #inner }
                    } else {
                        quote! { #field_ty }
                    };

                    quote! {
                        .callback(|_ctx, param, value| {
                            let __typed = value
                                .downcast_ref::<#validator_ty>()
                                .ok_or_else(|| {
                                    click::ClickError::bad_parameter_named(
                                        "validator type mismatch for parameter value",
                                        param.human_readable_name(),
                                    )
                                })?;
                            let __validator: fn(&#validator_ty) -> std::result::Result<(), String> = #validator;
                            __validator(__typed).map_err(|msg| {
                                click::ClickError::bad_parameter_named(msg, param.human_readable_name())
                            })?;
                            Ok(value)
                        })
                    }
                }
                None => quote! {},
            };

            let dest_opt = match &opt_attr.dest {
                Some(d) => quote! { .dest(#d) },
                None => quote! {},
            };

            // Handle custom shell completion callback
            let shell_complete_opt = match &opt_attr.shell_complete {
                Some(cb) => quote! { .shell_complete(#cb) },
                None => quote! {},
            };

            builders.push(quote! {
                .option(
                    click::ClickOption::new(&[#(#names_arr),*])
                        #help_opt
                        #default_opt
                        #required_opt
                        #hidden_opt
                        #flag_opt
                        #count_opt
                        #multiple_opt
                        #envvar_opt
                        #show_default_opt
                        #metavar_opt
                        #nargs_opt
                        #type_opt
                        #dest_opt
                        #validate_opt
                        #shell_complete_opt
                        .build()
                )
            });
        }
    }

    Ok(builders)
}

/// Generate argument builder calls
fn generate_argument_builders(fields: &[FieldInfo]) -> Result<Vec<TokenStream>> {
    let mut builders = Vec::new();

    for field in fields {
        if let FieldAttr::Argument(arg_attr) = &field.attr {
            let field_name = &field.name;
            let field_name_str = field_name.to_string();
            let field_ty = &field.ty;

            // Get help text (attribute or doc comment)
            let help_text = arg_attr.help.clone().or_else(|| field.doc.clone());
            let help_opt = match help_text {
                Some(h) => quote! { .help(#h) },
                None => quote! {},
            };

            // Handle required - default is true for non-Option types
            let is_required = arg_attr
                .required
                .unwrap_or(!is_option_type(field_ty));
            let required_opt = quote! { .required(#is_required) };

            // Handle hidden - Argument::hidden takes a bool
            let hidden = arg_attr.hidden;
            let hidden_opt = quote! { .hidden(#hidden) };

            // Handle multiple (for Vec types)
            let is_multiple = arg_attr.multiple || is_vec_type(field_ty);
            let multiple_opt = if is_multiple {
                quote! { .multiple() }
            } else {
                quote! {}
            };

            // Handle default
            let default_opt = match &arg_attr.default_str {
                Some(d) => quote! { .default(#d) },
                None => quote! {},
            };

            // Handle envvar
            let envvar_opt = match &arg_attr.envvar {
                Some(e) => quote! { .envvar(#e) },
                None => quote! {},
            };

            // Handle metavar
            let metavar_opt = match &arg_attr.metavar {
                Some(m) => quote! { .metavar(#m) },
                None => quote! {},
            };

            // Handle nargs
            let nargs_opt = match arg_attr.nargs {
                Some(-1) => quote! { .nargs(click::Nargs::Variadic) },
                Some(1) => quote! { .nargs(click::Nargs::Count(1)) },
                Some(0) => quote! { .nargs(click::Nargs::Count(0)) },
                Some(other) => {
                    return Err(Error::new_spanned(
                        field_name,
                        format!("unsupported argument nargs value: {}", other),
                    ))
                }
                None => quote! {},
            };

            // Handle explicit type converter expression
            let type_opt = match &arg_attr.type_expr {
                Some(expr) => quote! { .type_(#expr) },
                None => quote! {},
            };

            // Handle validator callback
            let validate_opt = match &arg_attr.validate {
                Some(validator) => {
                    let validator_ty = if is_option_type(field_ty) {
                        let inner = extract_inner_type(field_ty).ok_or_else(|| {
                            Error::new_spanned(field_ty, "#[argument(validate = ...)] requires Option<T> to use a concrete T")
                        })?;
                        quote! { #inner }
                    } else {
                        quote! { #field_ty }
                    };

                    quote! {
                        .callback(|_ctx, param, value| {
                            let __typed = value
                                .downcast_ref::<#validator_ty>()
                                .ok_or_else(|| {
                                    click::ClickError::bad_parameter_named(
                                        "validator type mismatch for parameter value",
                                        param.human_readable_name(),
                                    )
                                })?;
                            let __validator: fn(&#validator_ty) -> std::result::Result<(), String> = #validator;
                            __validator(__typed).map_err(|msg| {
                                click::ClickError::bad_parameter_named(msg, param.human_readable_name())
                            })?;
                            Ok(value)
                        })
                    }
                }
                None => quote! {},
            };

            // Handle custom shell completion callback
            let shell_complete_opt = match &arg_attr.shell_complete {
                Some(cb) => quote! { .shell_complete(#cb) },
                None => quote! {},
            };

            builders.push(quote! {
                .argument(
                    click::Argument::new(#field_name_str)
                        #help_opt
                        #required_opt
                        #hidden_opt
                        #multiple_opt
                        #default_opt
                        #envvar_opt
                        #metavar_opt
                        #nargs_opt
                        #type_opt
                        #validate_opt
                        #shell_complete_opt
                        .build()
                )
            });
        }
    }

    Ok(builders)
}

/// Generate field extraction code from Context
fn generate_field_extractions(fields: &[FieldInfo]) -> Result<Vec<TokenStream>> {
    let mut extractions = Vec::new();

    for field in fields {
        let field_name = &field.name;
        let field_name_str = field_name.to_string();
        let field_ty = &field.ty;

        match &field.attr {
            FieldAttr::Option(opt_attr) => {
                let option_key = opt_attr.dest.as_deref().unwrap_or(&field_name_str);
                let extraction =
                    generate_option_extraction(field_name, option_key, field_ty, opt_attr)?;
                extractions.push(extraction);
            }
            FieldAttr::Argument(arg_attr) => {
                let extraction = generate_argument_extraction(field_name, &field_name_str, field_ty, arg_attr)?;
                extractions.push(extraction);
            }
            FieldAttr::Subcommand(_) => {
                // Subcommands are handled differently in Group
                extractions.push(quote! {
                    let #field_name = Default::default();
                });
            }
            FieldAttr::PassContext => {
                let msg = format!(
                    "click-derive: #[pass_context] requires an active thread-local context (field `{}`)",
                    field_name_str
                );
                if is_option_type(field_ty) {
                    extractions.push(quote! {
                        let #field_name = click::context::get_current_context();
                    });
                } else {
                    extractions.push(quote! {
                        let #field_name = click::context::get_current_context()
                            .ok_or_else(|| click::ClickError::usage(#msg))?;
                    });
                }
            }
            FieldAttr::PassObj => {
                let msg = format!(
                    "click-derive: #[pass_obj] requires a context object of the expected type (field `{}`)",
                    field_name_str
                );
                if is_option_type(field_ty) {
                    let inner_ty = extract_inner_type(field_ty).ok_or_else(|| {
                        Error::new_spanned(field_ty, "#[pass_obj] requires a concrete Option<T> type")
                    })?;
                    extractions.push(quote! {
                        let #field_name = ctx.obj::<#inner_ty>().cloned();
                    });
                } else {
                    extractions.push(quote! {
                        let #field_name = ctx.obj::<#field_ty>()
                            .cloned()
                            .ok_or_else(|| click::ClickError::usage(#msg))?;
                    });
                }
            }
            FieldAttr::Skip => {
                extractions.push(quote! {
                    let #field_name = Default::default();
                });
            }
        }
    }

    Ok(extractions)
}

/// Generate extraction code for an option field
fn generate_option_extraction(
    field_name: &Ident,
    option_name: &str,
    field_ty: &Type,
    opt_attr: &OptionAttr,
) -> Result<TokenStream> {
    let use_typed = opt_attr.type_expr.is_some();

    // For count options
    if opt_attr.is_count {
        return Ok(quote! {
            let #field_name = ctx.get_param::<usize>(#option_name)
                .cloned()
                .unwrap_or(0) as #field_ty;
        });
    }

    // For flag options (bool)
    if (opt_attr.is_flag || is_bool_type(field_ty)) && opt_attr.flag_value.is_none() {
        return Ok(quote! {
            let #field_name = ctx.get_param::<bool>(#option_name)
                .cloned()
                .unwrap_or(false);
        });
    }

    // For Vec types
    if is_vec_type(field_ty) {
        if use_typed {
            return Ok(quote! {
                let #field_name = ctx.get_param::<#field_ty>(#option_name)
                    .cloned()
                    .unwrap_or_default();
            });
        }
        return Ok(quote! {
            let #field_name = ctx.get_param::<Vec<String>>(#option_name)
                .cloned()
                .map(|v| {
                    v.into_iter()
                        .filter_map(|s| s.parse().ok())
                        .collect()
                })
                .unwrap_or_default();
        });
    }

    // For Option types
    if is_option_type(field_ty) {
        if use_typed {
            let inner_ty = extract_inner_type(field_ty).ok_or_else(|| {
                Error::new_spanned(field_ty, "#[option(type = ...)] with Option<T> requires concrete T")
            })?;
            return Ok(quote! {
                let #field_name = ctx.get_param::<#inner_ty>(#option_name).cloned();
            });
        }
        return Ok(quote! {
            let #field_name = ctx.get_param::<String>(#option_name)
                .and_then(|s| s.parse().ok());
        });
    }

    // For regular types with defaults
    if let Some(default_expr) = &opt_attr.default {
        if use_typed {
            return Ok(quote! {
                let #field_name = ctx.get_param::<#field_ty>(#option_name)
                    .cloned()
                    .unwrap_or(#default_expr);
            });
        }
        return Ok(quote! {
            let #field_name = ctx.get_param::<String>(#option_name)
                .and_then(|s| s.parse().ok())
                .unwrap_or(#default_expr);
        });
    }

    // For required options
    if opt_attr.required {
        if use_typed {
            return Ok(quote! {
                let #field_name = ctx.get_param::<#field_ty>(#option_name)
                    .cloned()
                    .ok_or_else(|| click::ClickError::MissingParameter {
                        message: None,
                        param_name: Some(#option_name.to_string()),
                        param_hint: None,
                        param_type: click::ParamType::Option,
                        ctx: None,
                    })?;
            });
        }
        return Ok(quote! {
            let #field_name = ctx.get_param::<String>(#option_name)
                .ok_or_else(|| click::ClickError::MissingParameter {
                    message: None,
                    param_name: Some(#option_name.to_string()),
                    param_hint: None,
                    param_type: click::ParamType::Option,
                    ctx: None,
                })?
                .parse()
                .map_err(|_| click::ClickError::BadParameter {
                    message: format!("Invalid value for '{}'", #option_name),
                    param_name: Some(#option_name.to_string()),
                    param_hint: None,
                    ctx: None,
                })?;
        });
    }

    // Default case - try to get or use Default::default()
    if use_typed {
        return Ok(quote! {
            let #field_name = ctx.get_param::<#field_ty>(#option_name)
                .cloned()
                .unwrap_or_default();
        });
    }
    Ok(quote! {
        let #field_name = ctx.get_param::<String>(#option_name)
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();
    })
}

/// Generate extraction code for an argument field
fn generate_argument_extraction(
    field_name: &Ident,
    field_name_str: &str,
    field_ty: &Type,
    arg_attr: &ArgumentAttr,
) -> Result<TokenStream> {
    let use_typed = arg_attr.type_expr.is_some();

    // For Vec types
    if is_vec_type(field_ty) || arg_attr.multiple {
        if use_typed {
            return Ok(quote! {
                let #field_name = ctx.get_param::<#field_ty>(#field_name_str)
                    .cloned()
                    .unwrap_or_default();
            });
        }
        return Ok(quote! {
            let #field_name = ctx.get_param::<Vec<String>>(#field_name_str)
                .cloned()
                .map(|v| {
                    v.into_iter()
                        .filter_map(|s| s.parse().ok())
                        .collect()
                })
                .unwrap_or_default();
        });
    }

    // For Option types
    if is_option_type(field_ty) {
        if use_typed {
            let inner_ty = extract_inner_type(field_ty).ok_or_else(|| {
                Error::new_spanned(field_ty, "#[argument(type = ...)] with Option<T> requires concrete T")
            })?;
            return Ok(quote! {
                let #field_name = ctx.get_param::<#inner_ty>(#field_name_str).cloned();
            });
        }
        return Ok(quote! {
            let #field_name = ctx.get_param::<String>(#field_name_str)
                .and_then(|s| s.parse().ok());
        });
    }

    // For arguments with defaults
    if let Some(default_expr) = &arg_attr.default {
        if use_typed {
            return Ok(quote! {
                let #field_name = ctx.get_param::<#field_ty>(#field_name_str)
                    .cloned()
                    .unwrap_or(#default_expr);
            });
        }
        return Ok(quote! {
            let #field_name = ctx.get_param::<String>(#field_name_str)
                .and_then(|s| s.parse().ok())
                .unwrap_or(#default_expr);
        });
    }

    // Required is the default for arguments
    let is_required = arg_attr.required.unwrap_or(true);
    if is_required {
        if use_typed {
            return Ok(quote! {
                let #field_name = ctx.get_param::<#field_ty>(#field_name_str)
                    .cloned()
                    .ok_or_else(|| click::ClickError::MissingParameter {
                        message: None,
                        param_name: Some(#field_name_str.to_string()),
                        param_hint: None,
                        param_type: click::ParamType::Argument,
                        ctx: None,
                    })?;
            });
        }
        return Ok(quote! {
            let #field_name = ctx.get_param::<String>(#field_name_str)
                .ok_or_else(|| click::ClickError::MissingParameter {
                    message: None,
                    param_name: Some(#field_name_str.to_string()),
                    param_hint: None,
                    param_type: click::ParamType::Argument,
                    ctx: None,
                })?
                .parse()
                .map_err(|_| click::ClickError::BadParameter {
                    message: format!("Invalid value for '{}'", #field_name_str),
                    param_name: Some(#field_name_str.to_string()),
                    param_hint: None,
                    ctx: None,
                })?;
        });
    }

    // Optional argument
    if use_typed {
        return Ok(quote! {
            let #field_name = ctx.get_param::<#field_ty>(#field_name_str)
                .cloned()
                .unwrap_or_default();
        });
    }
    Ok(quote! {
        let #field_name = ctx.get_param::<String>(#field_name_str)
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();
    })
}
