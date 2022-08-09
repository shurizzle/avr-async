use proc_macro::TokenStream;

mod slab;
mod task;

#[proc_macro]
pub fn task_compose_internal(input: TokenStream) -> TokenStream {
    task::task_compose_internal(input)
}

#[proc_macro]
pub fn slab(input: TokenStream) -> TokenStream {
    slab::slab(input)
}
