#![recursion_limit="128"]

extern crate proc_macro;

use quote::{quote, quote_spanned};
use syn::{parse_macro_input, Fields, DeriveInput, Data};
use syn::spanned::Spanned;
use deriving::{has_attribute, normalized_fields, is_fields_variant_unnamed, normalized_variant_match_cause};

use proc_macro::TokenStream;

#[proc_macro_derive(IntoTree, attributes(bm))]
pub fn into_tree_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let build_fields = |fs, prefix| {
        let where_fields = normalized_fields(fs)
            .iter()
            .map(|f| {
                let ty = &f.1.ty;

                if has_attribute("bm", &f.1.attrs, "compact") {
                    quote_spanned! {
		        f.1.span() => for<'a> bm_le::CompactRef<'a, #ty>: bm_le::IntoTree
	            }
                } else {
	            quote_spanned! {
		        f.1.span() => #ty: bm_le::IntoTree
	            }
                }
	    }).collect::<Vec<_>>();

        let fields = normalized_fields(fs)
            .iter()
            .map(|f| {
                let ident = &f.0;

                if has_attribute("bm", &f.1.attrs, "compact") {
                    quote_spanned! { f.1.span() => {
                        vector.push(bm_le::IntoTree::into_tree(&bm_le::CompactRef(#prefix #ident), db)?);
                    } }
                } else {
                    quote_spanned! { f.1.span() => {
                        vector.push(bm_le::IntoTree::into_tree(#prefix #ident, db)?);
                    } }
                }
            }).collect::<Vec<_>>();

        let inner = quote! {
            let mut vector = Vec::new();
            #(#fields)*
            bm_le::utils::vector_tree(&vector, db, None)
        };

        (where_fields, inner)
    };

    let (where_fields, inner) = match input.data {
        Data::Struct(ref data) => {
            let (where_fields, inner) = build_fields(&data.fields, quote! { &self. });

            (where_fields, inner)
        },
        Data::Enum(ref data) => {
            let mut where_fields = Vec::new();

            let variants = data.variants
                .iter()
                .enumerate()
                .map(|(i, variant)| {
                    let (mut variant_where_fields, variant_inner) = build_fields(
                        &variant.fields,
                        if is_fields_variant_unnamed(variant) { quote! { variant. } } else { quote! {} }
                    );

                    where_fields.append(&mut variant_where_fields);

                    normalized_variant_match_cause(&input.ident, &variant, quote! {
                        let vector_root = { #variant_inner }?;
                        bm_le::utils::mix_in_type(&vector_root, db, #i)
                    })
                }).collect::<Vec<_>>();

            (where_fields, quote! {
                match self {
                    #(#variants)*
                }
            })
        },
        Data::Union(_) => panic!("Unsupported"),
    };

    let expanded = quote! {
        impl #impl_generics bm_le::IntoTree for #name #ty_generics where
            #where_clause
            #(#where_fields),*
        {
            fn into_tree<DB: bm_le::WriteBackend>(
                &self,
                db: &mut DB
            ) -> Result<<DB::Construct as bm_le::Construct>::Value, bm_le::Error<DB::Error>> where
                DB::Construct: bm_le::CompatibleConstruct
            {
                #inner
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(FromTree, attributes(bm))]
pub fn from_tree_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let build_fields = |fs| {
        let where_fields = normalized_fields(fs)
            .iter()
            .map(|f| {
	        let ty = &f.1.ty;

                if has_attribute("bm", &f.1.attrs, "compact") {
                    quote_spanned! {
		        f.1.span() => bm_le::Compact<#ty>: bm_le::FromTree
	            }
                } else {
	            quote_spanned! {
		        f.1.span() => #ty: bm_le::FromTree
	            }
                }
	    }).collect::<Vec<_>>();

        let fields = normalized_fields(fs)
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let name = &f.0;
                let ty = &f.1.ty;

                (quote_spanned! { f.1.span() => #name },
                 if has_attribute("bm", &f.1.attrs, "compact") {
                     quote_spanned! {
                         f.1.span() =>
                             <bm_le::Compact<#ty> as bm_le::FromTree>::from_tree(
                                 &vector.get(db, #i)?,
                                 db,
                             )?.0
                     }
                 } else {
                     quote_spanned! {
                         f.1.span() =>
                             bm_le::FromTree::from_tree(
                                 &vector.get(db, #i)?,
                                 db,
                             )?
                     }
                 })
            }).collect::<Vec<_>>();

        (where_fields, fields)
    };

    let (where_fields, inner) = match input.data {
        Data::Struct(ref data) => {
            let (where_fields, fields) = build_fields(&data.fields);

            let fields_count = fields.iter().count();
            let fields = fields.into_iter().map(|f| {
                let name = f.0;
                let value = f.1;

                quote! {
                    #name: #value,
                }
            });

            let inner = quote! {
                {
                    use bm_le::Leak;

                    let vector = bm_le::DanglingVector::<DB::Construct>::from_leaked(
                        (root.clone(), #fields_count, None)
                    );

                    Ok(Self {
                        #(#fields)*
                    })
                }
            };

            (where_fields, inner)
        },
        Data::Enum(ref data) => {
            let mut where_fields = Vec::new();

            let variants = data.variants
                .iter()
                .enumerate()
                .map(|(i, variant)| {
                    let (mut variant_where_fields, variant_fields) = build_fields(
                        &variant.fields,
                    );
                    let ident = &variant.ident;

                    where_fields.append(&mut variant_where_fields);
                    let fields_count = variant_fields.iter().count();

                    match variant.fields {
                        Fields::Named(_) => {
                            let fields = variant_fields.into_iter().map(|f| {
                                let name = f.0;
                                let value = f.1;

                                quote! {
                                    #name: #value,
                                }
                            });

                            quote! {
                                #i => {
                                    use bm_le::Leak;

                                    let vector = bm_le::DanglingVector::<DB::Construct>::from_leaked(
                                        (vector_root.clone(), #fields_count, None)
                                    );

                                    Ok(#name::#ident {
                                        #(#fields)*
                                    })
                                },
                            }
                        },
                        Fields::Unnamed(_) => {
                            let fields = variant_fields.into_iter().map(|f| {
                                let value = f.1;

                                quote! {
                                    #value,
                                }
                            });

                            quote! {
                                #i => {
                                    use bm_le::Leak;

                                    let vector = bm_le::DanglingVector::<DB::Construct>::from_leaked(
                                        (vector_root.clone(), #fields_count, None)
                                    );

                                    Ok(#name::#ident(
                                        #(#fields)*
                                    ))
                                },
                            }
                        },
                        Fields::Unit => {
                            quote! {
                                #i => {
                                    if vector_root != &Default::default() {
                                        return Err(bm_le::Error::CorruptedDatabase)
                                    }

                                    Ok(#name::#ident)
                                },
                            }
                        },
                    }
                }).collect::<Vec<_>>();

            (where_fields, quote! {
                bm_le::utils::decode_with_type(root, db, |vector_root, db, ty| {
                    match ty {
                        #(#variants)*
                        _ => return Err(bm_le::Error::CorruptedDatabase)
                    }
                })
            })
        },
        Data::Union(_) => panic!("Not supported"),
    };

    let expanded =
        quote! {
            impl #impl_generics bm_le::FromTree for #name #ty_generics where
                #where_clause
                #(#where_fields),*
            {
                fn from_tree<DB: bm_le::ReadBackend>(
                    root: &<DB::Construct as bm_le::Construct>::Value,
                    db: &mut DB,
                ) -> Result<Self, bm_le::Error<DB::Error>> where
                    DB::Construct: bm_le::CompatibleConstruct
                {
                    #inner
                }
            }
        };

    proc_macro::TokenStream::from(expanded)
}
