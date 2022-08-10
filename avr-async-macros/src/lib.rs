use proc_macro::TokenStream;

pub(crate) mod common;
mod slab;
mod task;

fn wrap_imp(res: syn::Result<TokenStream>) -> TokenStream {
    match res {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error().into(),
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
