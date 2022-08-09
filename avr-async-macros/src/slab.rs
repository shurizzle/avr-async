use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::Parse, parse_quote, punctuated::Punctuated, token::Bracket, AttrStyle, Attribute, Field,
    FieldsNamed, Ident, Path, PathArguments, PathSegment, Token, Type, Visibility,
};

use crate::common::Parameters;

pub struct SlabDef {
    pub ident: Ident,
    pub fields: FieldsNamed,
    pub semi_token: Option<Token![;]>,
}

impl Parse for SlabDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            fields: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

pub fn imp(input: TokenStream) -> TokenStream {
    let span = Span::call_site();

    let Parameters {
        krate,
        comma: _,
        def,
    } = syn::parse_macro_input!(input as Parameters<SlabDef>);

    let inst_ident = def.ident.clone();
    let mem_ident = format_ident!("__avr_async_{}_MEM", def.ident, span = span);

    let mut mem_fields = FieldsNamed {
        brace_token: def.fields.brace_token,
        named: Punctuated::<Field, Token![,]>::new(),
    };
    let mut inst_fields = FieldsNamed {
        brace_token: def.fields.brace_token,
        named: Punctuated::<Field, Token![,]>::new(),
    };

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

    let mut mem_init = quote!();
    let mut inst_init = quote!();

    for (i, f) in def.fields.named.iter().enumerate() {
        if i != 0 {
            mem_fields.named.push_punct(Token![,](span));
        }

        let ty = f.ty.clone();
        let ty: Type =
            parse_quote!(::core::mem::MaybeUninit<<#ty as #krate::slab::Slabbed>::InnerType>);

        mem_fields.named.push_value(Field {
            attrs: vec![doc_hidden.clone()],
            vis: Visibility::Inherited,
            ident: f.ident.clone(),
            colon_token: f.colon_token,
            ty,
        });

        let ty = f.ty.clone();
        let ty: Type = parse_quote!(#krate::slab::Slab<#ty>);

        inst_fields.named.push_value(Field {
            attrs: f.attrs.clone(),
            vis: f.vis.clone(),
            ident: f.ident.clone(),
            colon_token: f.colon_token,
            ty,
        });

        let field = f.ident.clone().unwrap();
        let ty = f.ty.clone();

        mem_init = quote! {
            #mem_init
            #field: ::core::mem::MaybeUninit::uninit(),
        };

        inst_init = quote! {
            #inst_init
            #field: #krate::slab::Slab::<#ty>::new(&mut mem.#field),
        };
    }

    let mem_init = quote! {
        impl #mem_ident {
            #doc_hidden
            const fn new() -> Self {
                unsafe {
                    Self {
                        #mem_init
                    }
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
                    Self {
                        #inst_init
                    }
                }
            }
        }
    };

    quote! {
        #doc_hidden
        #[allow(non_camel_case_types)]
        struct #mem_ident #mem_fields
        unsafe impl Sync for #mem_ident {}
        #mem_init
        #doc_hidden
        pub struct #inst_ident #inst_fields
        #inst_init
    }
    .into()
}
