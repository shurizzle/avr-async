use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Bracket,
    AttrStyle, Attribute, Field, Fields, FieldsNamed, FieldsUnnamed, Item, Path, PathArguments,
    PathSegment, Token, Type,
};

use crate::common::{unraw, AttributeName};

struct Attributes {
    pub krate: Option<Path>,
}

impl Parse for Attributes {
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

pub fn imp(attrs: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let krate = syn::parse::<Attributes>(attrs)?
        .krate
        .unwrap_or_else(|| syn::parse_str("::avr_async").unwrap());
    generate(krate, input)
}

fn generate(krate: Path, input: TokenStream) -> syn::Result<TokenStream> {
    let span = Span::call_site();

    let item: Item = syn::parse(input)?;

    let mut item = match item {
        Item::Struct(s) => s,
        _ => return Err(syn::Error::new_spanned(item, "A slab can only be a struct")),
    };

    let has_generics = !(item.generics.params.is_empty()
        && item.generics.lt_token.is_none()
        && item.generics.gt_token.is_none()
        && item.generics.where_clause.is_none());

    if has_generics {
        return Err(syn::Error::new(
            item.generics.span(),
            "Slab doesn't support generics or lifetimes",
        ));
    }

    let doc_hidden = Attribute {
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
    };

    let inst_ident = item.ident.clone();
    let mem_ident = format_ident!("__avr_async_{}_MEM", unraw(&item.ident), span = span);

    let mem_fields;
    let mut mem_init = quote!();
    let mut inst_init = quote!();

    item.fields = match item.fields {
        Fields::Named(mut fields) => {
            let mut mfs = FieldsNamed {
                brace_token: fields.brace_token,
                named: Punctuated::<Field, Token![,]>::new(),
            };

            for (i, f) in fields.named.iter_mut().enumerate() {
                if i != 0 {
                    mfs.named.push_punct(Token![,](span));
                }

                let ty = f.ty.clone();
                let ty: Type = parse_quote!(::core::mem::MaybeUninit<<#ty as #krate::slab::Slabbed>::InnerType>);

                mfs.named.push_value(Field {
                    attrs: vec![doc_hidden.clone()],
                    vis: f.vis.clone(),
                    ident: f.ident.clone(),
                    colon_token: f.colon_token,
                    ty,
                });

                let ty = f.ty.clone();
                let ty: Type = parse_quote!(#krate::slab::Slab<#ty>);
                let ty2 = f.ty.clone();

                f.ty = ty;

                let field = f.ident.clone().unwrap();

                mem_init = quote! {
                    #mem_init
                    #field: ::core::mem::MaybeUninit::uninit(),
                };

                inst_init = quote! {
                    #inst_init
                    #field: #krate::slab::Slab::<#ty2>::new(&mut mem.#field),
                };
            }

            let mfs = Fields::Named(mfs);
            mem_fields = quote!(#mfs);
            mem_init = quote!({#mem_init});
            inst_init = quote!({#inst_init});
            Fields::Named(fields)
        }
        Fields::Unnamed(mut fields) => {
            let mut mfs = FieldsUnnamed {
                paren_token: fields.paren_token,
                unnamed: Punctuated::<Field, Token![,]>::new(),
            };

            for (i, f) in fields.unnamed.iter_mut().enumerate() {
                if i != 0 {
                    mfs.unnamed.push_punct(Token![,](span));
                }

                let ty = f.ty.clone();
                let ty: Type = parse_quote!(::core::mem::MaybeUninit<<#ty as #krate::slab::Slabbed>::InnerType>);

                mfs.unnamed.push_value(Field {
                    attrs: vec![doc_hidden.clone()],
                    vis: f.vis.clone(),
                    ident: None,
                    colon_token: None,
                    ty,
                });

                let ty = f.ty.clone();
                let ty: Type = parse_quote!(#krate::slab::Slab<#ty>);
                let ty2 = f.ty.clone();

                f.ty = ty;

                mem_init = quote! {
                    #mem_init
                    ::core::mem::MaybeUninit::uninit(),
                };

                let i = syn::Index::from(i);
                inst_init = quote! {
                    #inst_init
                    #krate::slab::Slab::<#ty2>::new(&mut mem.#i),
                };
            }

            let mfs = Fields::Unnamed(mfs);
            mem_fields = quote!(#mfs;);
            mem_init = quote!((#mem_init));
            inst_init = quote!((#inst_init));
            Fields::Unnamed(fields)
        }
        Fields::Unit => {
            return Err(syn::Error::new_spanned(
                item,
                "Slab doesn't support unit structs",
            ))
        }
    };

    let mem_init = quote! {
        impl #mem_ident {
            #doc_hidden
            const fn new() -> Self {
                unsafe {
                    Self #mem_init
                }
            }
        }
    };

    let inst_init = quote! {
        impl #inst_ident {
            pub fn take() -> Option<Self> {
                unsafe {
                    static MEM: #krate::SyncUnsafeCell<#mem_ident> = #krate::SyncUnsafeCell::new(#mem_ident::new());
                    static TAKEN: #krate::SyncUnsafeCell<bool> = #krate::SyncUnsafeCell::new(false);
                    if *TAKEN.get() {
                        None
                    } else {
                        *TAKEN.get() = true;
                        Some(Self::new(MEM.get()))
                    }
                }
            }

            fn new(mem: *mut #mem_ident) -> Self {
                unsafe {
                    let mem = &mut *mem;
                    Self #inst_init
                }
            }
        }
    };

    Ok(quote! {
        #doc_hidden
        #[allow(non_camel_case_types)]
        struct #mem_ident #mem_fields
        #doc_hidden
        unsafe impl Sync for #mem_ident {}
        #mem_init
        #item
        #inst_init
    }
    .into())
}
