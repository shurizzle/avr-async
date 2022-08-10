use proc_macro::TokenStream;
use quote::quote_spanned;

pub(crate) mod common;
mod slab;
mod task;

fn wrap_imp(res: syn::Result<TokenStream>) -> TokenStream {
    match res {
        Ok(ts) => ts,
        Err(err) => {
            let error = err.to_string();
            quote_spanned! {
                err.span() => compile_error!(#error);
            }
            .into()
        }
    }
}

#[proc_macro]
pub fn task_compose(input: TokenStream) -> TokenStream {
    task::imp(input)
}

#[proc_macro_attribute]
pub fn slab(attrs: TokenStream, input: TokenStream) -> TokenStream {
    wrap_imp(slab::imp(attrs, input))
}
