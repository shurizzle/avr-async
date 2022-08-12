use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse_quote, Fields, GenericParam, Item, Path, Type};

use crate::common::{doc_hidden, unraw, CrateOnlyAttributes};

pub fn imp(attrs: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    generate(
        syn::parse::<CrateOnlyAttributes>(attrs)?
            .krate
            .map_or_else(|| syn::parse_str("::avr_async"), Ok)?,
        input,
    )
}

fn generate(krate: Path, input: TokenStream) -> syn::Result<TokenStream> {
    let span = Span::call_site();

    let item: Item = syn::parse(input)?;

    let mut item = match item {
        Item::Struct(s) => s,
        _ => return Err(syn::Error::new_spanned(item, "A slab can only be a struct")),
    };

    let mut mem_item = item.clone();

    let dh = doc_hidden(span);

    let inst_ident = item.ident.clone();
    let mem_ident = format_ident!("__avr_async_{}_MEM", unraw(&item.ident), span = span);

    let mut mem_init = quote!();
    let mut inst_init = quote!();

    mem_item.ident = mem_ident.clone();
    mem_item.fields = match mem_item.fields {
        Fields::Named(mut fields) => {
            for f in fields.named.iter_mut() {
                let ty = f.ty.clone();
                f.ty = parse_quote!(::core::mem::MaybeUninit<<#ty as #krate::slab::Slabbed>::InnerType>);
            }
            Fields::Named(fields)
        }
        Fields::Unnamed(mut fields) => {
            for f in fields.unnamed.iter_mut() {
                let ty = f.ty.clone();
                f.ty = parse_quote!(::core::mem::MaybeUninit<<#ty as #krate::slab::Slabbed>::InnerType>);
            }
            Fields::Unnamed(fields)
        }
        Fields::Unit => {
            return Err(syn::Error::new_spanned(
                item,
                "Slab doesn't support unit structs",
            ))
        }
    };

    let mut generics = quote!();
    let mut ty_generics = quote!();

    if let Some(c) = mem_item.generics.lt_token {
        generics = quote!(#generics #c);
        ty_generics = quote!(#ty_generics #c);
    }

    {
        let gen = mem_item.generics.params.clone();
        generics = quote!(#generics #gen);
        for (v, p) in gen.into_pairs().map(|x| x.into_tuple()) {
            match v {
                GenericParam::Type(t) => {
                    let i = t.ident.clone();
                    ty_generics = quote!(#ty_generics #i);
                }
                GenericParam::Const(c) => {
                    let i = c.ident;
                    ty_generics = quote!(#ty_generics #i);
                }
                GenericParam::Lifetime(l) => {
                    let l = l.lifetime;
                    ty_generics = quote!(#ty_generics #l);
                }
            }

            if let Some(p) = p {
                ty_generics = quote!(#ty_generics #p);
            }
        }
    }

    if let Some(c) = mem_item.generics.gt_token {
        generics = quote!(#generics #c);
        ty_generics = quote!(#ty_generics #c);
    }

    let where_clause = if let Some(wc) = mem_item.generics.where_clause.clone() {
        quote!(#wc)
    } else {
        quote!()
    };

    item.fields = match item.fields {
        Fields::Named(mut fields) => {
            for f in fields.named.iter_mut() {
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

            mem_init = quote!({#mem_init});
            inst_init = quote!({#inst_init});
            Fields::Named(fields)
        }
        Fields::Unnamed(mut fields) => {
            for (i, f) in fields.unnamed.iter_mut().enumerate() {
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

    let inst_init = quote! {
        impl #generics #krate::runtime::Memory for #inst_ident #ty_generics #where_clause {
            type Slab = #mem_ident #ty_generics;

            fn alloc() -> Self::Slab {
                Self::Slab #mem_init
            }

            unsafe fn from_ptr(mem: *mut Self::Slab) -> Self {
                let mem = &mut *mem;
                Self #inst_init
            }
        }
    };

    Ok(quote! {
        #dh
        #[allow(non_camel_case_types)]
        #mem_item
        #item
        #inst_init
    }
    .into())
}
