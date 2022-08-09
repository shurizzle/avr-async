use syn::{parse::Parse, Path, Token};

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
