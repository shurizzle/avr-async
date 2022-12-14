use proc_macro::TokenStream;

pub(crate) mod chip;
pub(crate) mod common;
mod main;
mod memory;
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

#[proc_macro_attribute]
pub fn main(attrs: TokenStream, input: TokenStream) -> TokenStream {
    wrap_imp(main::imp(attrs, input))
}

#[proc_macro_attribute]
pub fn memory(attrs: TokenStream, input: TokenStream) -> TokenStream {
    wrap_imp(memory::imp(attrs, input))
}
