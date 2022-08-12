use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Item, Path, Token, Type,
};

use crate::{chip::VECTORS, common::AttributeName};

pub struct Attributes {
    pub krate: Option<Path>,
    pub runtime: Type,
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let runtime_err = Err(input.error("Runtime type not defined"));

        if input.is_empty() {
            return Err(input.error("Runtime not specified"));
        }

        let mut krate: Option<Path> = None;
        let mut runtime: Option<Type> = None;

        while !input.is_empty() {
            let key = input.parse::<AttributeName>()?;

            match key.name.as_str() {
                "crate" => {
                    if krate.is_some() {
                        return Err(syn::Error::new(key.span, "Attribute crate already defined"));
                    }
                    input.parse::<Token![=]>()?;
                    krate = Some(input.parse()?);
                }
                "runtime" => {
                    if runtime.is_some() {
                        return Err(syn::Error::new(
                            key.span,
                            "Attribute runtime already defined",
                        ));
                    }
                    input.parse::<Token![=]>()?;
                    runtime = Some(input.parse()?);
                }
                other => {
                    return Err(syn::Error::new(
                        key.span,
                        format!("Invalid attribute {}", other),
                    ))
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        if !input.is_empty() {
            return Err(input.error("Invalid attributes"));
        }

        if let Some(runtime) = runtime {
            Ok(Self { krate, runtime })
        } else {
            runtime_err
        }
    }
}

fn generate(krate: Path, runtime: Type, input: TokenStream) -> syn::Result<TokenStream> {
    let span = Span::call_site();
    let item: Item = syn::parse(input)?;

    let item = match item {
        Item::Fn(s) => s,
        _ => return Err(syn::Error::new_spanned(item, "A slab can only be a struct")),
    };

    let has_generics = !(item.sig.generics.params.is_empty()
        && item.sig.generics.lt_token.is_none()
        && item.sig.generics.gt_token.is_none()
        && item.sig.generics.where_clause.is_none());

    if has_generics {
        return Err(syn::Error::new(
            item.sig.generics.span(),
            "main function doesn't support generics or lifetimes",
        ));
    }

    if item.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            item.sig.span(),
            "Only async functions are supported",
        ));
    }

    let mut code = quote! {
        #item

        #[doc(hidden)]
        #[export_name = "main"]
        unsafe extern "C" fn __avr_async_main() -> ! {
            #krate::executor::run::<#runtime, _, _>(main)
        }

        #[no_mangle]
        unsafe fn __avr_async_runtime_wake() {
            <#runtime as #krate::runtime::Runtime>::wake(#krate::executor::__private::get());
        }
    };

    for &(i, name) in VECTORS {
        if !name.starts_with("reserved") {
            let mname = format_ident!("{}", name, span = span);
            let vname = format!("__vector_{}", i);
            let fnname = format_ident!("__vector_{}", i, span = span);

            code = quote! {
                #code

                #[doc(hidden)]
                #[export_name = #vname]
                unsafe extern "avr-interrupt" fn #fnname() {
                    <#runtime as #krate::runtime::Runtime>::#mname(#krate::executor::__private::get(), &#krate::CriticalSection::new());
                }
            };
        }
    }

    Ok(code.into())
}

pub fn imp(attrs: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let Attributes { krate, runtime } = syn::parse(attrs)?;

    let krate = if let Some(krate) = krate {
        krate
    } else {
        syn::parse_str("::avr_async")?
    };

    generate(krate, runtime, input)
}
