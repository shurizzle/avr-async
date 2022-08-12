use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Bracket, Crate},
    AttrStyle, Attribute, Ident, Path, PathArguments, PathSegment, Token,
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

pub fn doc_hidden(span: Span) -> Attribute {
    Attribute {
        pound_token: Token![#](span),
        style: AttrStyle::Outer,
        bracket_token: Bracket { span },
        path: {
            let mut segments = Punctuated::new();
            segments.push_value(PathSegment {
                ident: format_ident!("doc", span = span),
                arguments: PathArguments::None,
            });
            Path {
                leading_colon: None,
                segments,
            }
        },
        tokens: quote!((hidden)),
    }
}

pub struct CrateOnlyAttributes {
    pub krate: Option<Path>,
}

impl Parse for CrateOnlyAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self { krate: None });
        }

        let mut krate = None;

        let key = input.parse::<AttributeName>()?;

        if key.name != "crate" {
            return Err(syn::Error::new(
                key.span,
                format!("Invalid attribute {}", key.name),
            ));
        }

        if krate.is_some() {
            return Err(input.error("crate attribute defined multiple times"));
        }

        input.parse::<Token![=]>()?;
        let value: Path = input.parse()?;
        krate = Some(value);
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }

        if !input.is_empty() {
            return Err(input.error("Invalid attributes"));
        }

        Ok(Self { krate })
    }
}
