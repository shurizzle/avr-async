use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse::Parse, punctuated::Punctuated, token::Comma, Expr};

use crate::common::Parameters;

#[derive(Default)]
struct TaskList {
    pub list: Punctuated<Expr, Comma>,
}

impl Parse for TaskList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut list = Punctuated::new();

        while !input.is_empty() {
            list.push_value(input.parse()?);
            if input.is_empty() {
                break;
            }
            list.push_punct(input.parse()?);
        }

        Ok(Self { list })
    }
}

pub fn imp(input: TokenStream) -> TokenStream {
    let span = Span::call_site();

    let mut defs;
    let mut poll_futures;

    let krate = {
        let Parameters {
            def: parsed,
            comma: _,
            krate,
        } = syn::parse_macro_input!(input as Parameters<TaskList>);

        defs = Vec::with_capacity(parsed.list.len());
        poll_futures = Vec::with_capacity(parsed.list.len());
        for (mut i, expr) in parsed.list.into_iter().enumerate() {
            if i != 0 {
                poll_futures.push(quote!( && ));
            }
            i += 1;

            let name = format_ident!("_fut{}", i, span = span);

            defs.push(quote! {
                let mut #name = #krate::task::Task::new(#i, #expr);
            });

            poll_futures.push(quote! {
                #name.poll(cx)
            });
        }
        krate
    };

    TokenStream::from(quote! { #krate::task::TaskContext::acquire({
        #( #defs )*
        ::core::future::poll_fn(move |cx| {
            if #( #poll_futures )* {
                ::core::task::Poll::Ready(())
            } else {
                ::core::task::Poll::Pending
            }
        })
    }) })
}
