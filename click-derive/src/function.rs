//! Function-first command macro implementation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{spanned::Spanned, Attribute, Error, Expr, FnArg, ItemFn, Meta, Pat, Result, Token};

fn is_click_param_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("option")
        || attr.path().is_ident("argument")
        || attr.path().is_ident("pass_context")
        || attr.path().is_ident("pass_obj")
}

fn to_pascal_case(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<String>()
}

fn collect_fn_fields_and_call_args(
    func: &mut ItemFn,
    macro_name: &str,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut fields = Vec::new();
    let mut call_args = Vec::new();

    for input in &mut func.sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                return Err(Error::new(
                    receiver.span(),
                    format!(
                        "#[click::{}] is only supported on free functions",
                        macro_name
                    ),
                ))
            }
            FnArg::Typed(pat_type) => {
                let mut click_attrs = Vec::new();
                let mut kept_attrs = Vec::new();
                for attr in pat_type.attrs.drain(..) {
                    if is_click_param_attr(&attr) {
                        click_attrs.push(attr);
                    } else {
                        kept_attrs.push(attr);
                    }
                }
                pat_type.attrs = kept_attrs;

                if click_attrs.is_empty() {
                    return Err(Error::new(
                        pat_type.span(),
                        format!(
                            "every parameter in #[click::{}] functions must have #[option], #[argument], #[pass_context], or #[pass_obj]",
                            macro_name
                        ),
                    ));
                }

                let ident = match pat_type.pat.as_ref() {
                    Pat::Ident(pat_ident) => pat_ident.ident.clone(),
                    other => {
                        return Err(Error::new(
                            other.span(),
                            "unsupported parameter pattern: use a simple identifier",
                        ))
                    }
                };

                let ty = pat_type.ty.as_ref().clone();
                fields.push(quote! {
                    #(#click_attrs)*
                    #ident: #ty
                });
                call_args.push(quote! { __cmd.#ident });
            }
        }
    }
    Ok((fields, call_args))
}

pub fn expand_command_fn(args: TokenStream, mut func: ItemFn) -> Result<TokenStream> {
    let fn_name = func.sig.ident.clone();
    let helper_name = format_ident!("{}_command", fn_name);
    let struct_name = format_ident!("{}Command", to_pascal_case(&fn_name.to_string()));
    let (fields, call_args) = collect_fn_fields_and_call_args(&mut func, "command")?;
    let struct_doc_attrs: Vec<Attribute> = func
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();

    let command_attr = if args.is_empty() {
        quote! {}
    } else {
        quote! { #[command(#args)] }
    };

    let vis = func.vis.clone();
    let output = quote! {
        #func

        #[derive(click::Command)]
        #command_attr
        #(#struct_doc_attrs)*
        #vis struct #struct_name {
            #(#fields,)*
        }

        #vis fn #helper_name() -> click::Command {
            #struct_name::command_with_run(|__cmd, _ctx| #fn_name(#(#call_args),*))
        }
    };

    Ok(output)
}

#[derive(Default)]
struct GroupFnArgs {
    derive_metas: Vec<Meta>,
    commands: Vec<Expr>,
    groups: Vec<Expr>,
}

fn parse_expr_array(expr: &Expr, field: &str) -> Result<Vec<Expr>> {
    match expr {
        Expr::Array(array) => Ok(array.elems.iter().cloned().collect()),
        _ => Err(Error::new(
            expr.span(),
            format!("group attribute '{}' must be an array expression", field),
        )),
    }
}

fn parse_group_fn_args(args: TokenStream) -> Result<GroupFnArgs> {
    if args.is_empty() {
        return Ok(GroupFnArgs::default());
    }

    let metas = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
    let mut parsed = GroupFnArgs::default();

    for meta in metas {
        match &meta {
            Meta::NameValue(name_value) if name_value.path.is_ident("commands") => {
                parsed.commands = parse_expr_array(&name_value.value, "commands")?;
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("groups") => {
                parsed.groups = parse_expr_array(&name_value.value, "groups")?;
            }
            _ => parsed.derive_metas.push(meta),
        }
    }

    Ok(parsed)
}

fn to_attach_call(expr: &Expr, default_suffix: &str, kind_name: &str) -> Result<TokenStream> {
    match expr {
        Expr::Call(call) => Ok(quote! { #call }),
        Expr::Path(path_expr) => {
            let mut helper_path = path_expr.path.clone();
            let Some(last_segment) = helper_path.segments.last_mut() else {
                return Err(Error::new(
                    expr.span(),
                    format!("invalid {} attachment path", kind_name),
                ));
            };

            let last_name = last_segment.ident.to_string();
            if !last_name.ends_with("_command") && !last_name.ends_with("_group") {
                last_segment.ident = format_ident!("{}_{}", last_name, default_suffix);
            }

            Ok(quote! { #helper_path() })
        }
        _ => Err(Error::new(
            expr.span(),
            format!(
                "{} attachment entries must be function paths or function calls",
                kind_name
            ),
        )),
    }
}

pub fn expand_group_fn(args: TokenStream, mut func: ItemFn) -> Result<TokenStream> {
    let parsed_args = parse_group_fn_args(args)?;
    let fn_name = func.sig.ident.clone();
    let helper_name = format_ident!("{}_group", fn_name);
    let struct_name = format_ident!("{}Group", to_pascal_case(&fn_name.to_string()));
    let (fields, call_args) = collect_fn_fields_and_call_args(&mut func, "group")?;

    let struct_doc_attrs: Vec<Attribute> = func
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();

    let group_attr = if parsed_args.derive_metas.is_empty() {
        quote! {}
    } else {
        let metas = parsed_args.derive_metas;
        quote! { #[group(#(#metas),*)] }
    };

    let command_attachments: Vec<TokenStream> = parsed_args
        .commands
        .iter()
        .map(|expr| to_attach_call(expr, "command", "commands"))
        .collect::<Result<Vec<_>>>()?;

    let group_attachments: Vec<TokenStream> = parsed_args
        .groups
        .iter()
        .map(|expr| to_attach_call(expr, "group", "groups"))
        .collect::<Result<Vec<_>>>()?;

    let vis = func.vis.clone();
    let output = quote! {
        #func

        #[derive(click::Group)]
        #group_attr
        #(#struct_doc_attrs)*
        #vis struct #struct_name {
            #(#fields,)*
        }

        #vis fn #helper_name() -> click::Group {
            let mut __group = #struct_name::group_with_run(|__cmd, _ctx| #fn_name(#(#call_args),*));
            #( __group.add_command(#command_attachments, None); )*
            #( __group.add_command(#group_attachments, None); )*
            __group
        }
    };

    Ok(output)
}
