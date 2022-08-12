use proc_macro2::Span;
use syn::{
    parse::{Parse, ParseStream},
    token::Crate,
    Ident, Path, Token,
};

pub struct Parameters<T: Parse> {
    pub krate: Path,
    pub comma: Token![,],
    pub def: T,
}

impl<T: Parse> Parse for Parameters<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            krate: input.parse()?,
            comma: input.parse()?,
            def: input.parse()?,
        })
    }
}

pub struct AttributeName {
    pub span: Span,
    pub name: String,
}

impl Parse for AttributeName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            input.parse::<Ident>().map(|x| AttributeName {
                span: x.span(),
                name: unraw(&x),
            })
        } else if input.peek(Crate) {
            input.parse::<Crate>().map(|x| AttributeName {
                span: x.span,
                name: "crate".to_string(),
            })
        } else {
            Err(input.error("Expected ident"))
        }
    }
}

pub fn unraw(ident: &Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}
