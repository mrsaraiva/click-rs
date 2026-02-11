//! Function-first command macro implementation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Attribute, Error, FnArg, ItemFn, Pat, Result};

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

pub fn expand_command_fn(args: TokenStream, mut func: ItemFn) -> Result<TokenStream> {
    let fn_name = func.sig.ident.clone();
    let helper_name = format_ident!("{}_command", fn_name);
    let struct_name = format_ident!("{}Command", to_pascal_case(&fn_name.to_string()));

    let mut fields = Vec::new();
    let mut call_args = Vec::new();

    for input in &mut func.sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                return Err(Error::new(
                    receiver.span(),
                    "#[click::command] is only supported on free functions",
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
                        "every parameter in #[click::command] functions must have #[option], #[argument], #[pass_context], or #[pass_obj]",
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
