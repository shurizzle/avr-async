use proc_macro::TokenStream;

pub(crate) mod common;
mod slab;
mod task;

#[proc_macro]
pub fn task_compose(input: TokenStream) -> TokenStream {
    task::imp(input)
}

#[proc_macro]
pub fn slab(input: TokenStream) -> TokenStream {
    slab::imp(input)
}
