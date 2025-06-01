use core::panic;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(EnumNewMacro, attributes(op_prefix))]
pub fn derive_enum_new(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let has_prefix = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("op_prefix") || attr.path().is_ident("ext_op_prefix"));

    if has_prefix.is_some() {
        panic!("EnumNew cannot be derived for enums with prefix attributes");
    }

    let variants = match &input.data {
        syn::Data::Enum(data) => &data.variants,
        _ => panic!("EnumNew can only be derived for enums"),
    };

    let variant_arms = variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let syn::Fields::Unnamed(field) = &variant.fields else {
                panic!("EnumNew can only be derived for enums with unnamed fields");
            };
            let field = field
                .unnamed
                .first()
                .expect("EnumNew can only be derived for enums with at least one field");

            let field_type = &field.ty;

            quote::quote! {
                if let Some((data, skip)) = <#field_type as AmlNew>::aml_new(data) {
                    return Some((#name::#ident(data), skip));
                }
            }
        })
        .collect::<Vec<_>>();

    let r#gen = quote::quote! {
        impl #impl_generics AmlNew for #name #ty_generics #where_clause {
            fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
                #(#variant_arms)*
                None
            }
        }
    };
    r#gen.into()
}

#[proc_macro_attribute]
pub fn op_prefix(_attributes: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn ext_op_prefix(_attributes: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_derive(StructNewMacro)] //struct without prefix
pub fn derive_struct_new(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Extract the `op_orefix` attribute
    let op_prefix = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("op_prefix") {
            Some(attr.parse_args::<syn::Ident>().unwrap())
        } else {
            None
        }
    });

    let ext_op_prefix = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("ext_op_prefix") {
            Some(attr.parse_args::<syn::Ident>().unwrap())
        } else {
            None
        }
    });

    let fields = match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("StructNew can only be derived for structs"),
    };

    let field_arms = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let ty = field.ty.clone();

        if op_prefix.is_some() || ext_op_prefix.is_some() {
            let error_str = format!("Failed to parse field {}", ident);
            quote::quote! {
                let (#ident, new_skip) = #ty::aml_new(&data[skip..]).expect(#error_str);
                skip += new_skip;
            }
        } else {
            quote::quote! {
                let (#ident, new_skip) = #ty::aml_new(&data[skip..])?;
                skip += new_skip;
            }
        }
    });

    let field_packing = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        quote::quote! {
            #ident
        }
    });

    let prefix_check = if let Some(op_prefix) = &op_prefix {
        quote::quote! {
            if data.len() < 1 || data[0] != #op_prefix {
                return None;
            }
            let mut skip = 1;
        }
    } else if let Some(ext_op_prefix) = &ext_op_prefix {
        quote::quote! {
            if data.len() < 2 || data[0] != #ext_op_prefix[0] || data[1] != #ext_op_prefix[1] {
                return None;
            }
            let mut skip = 2;
        }
    } else {
        quote::quote! {
            let mut skip = 0;
        }
    };

    let r#gen = quote::quote! {
        impl #impl_generics AmlNew for #name #ty_generics #where_clause {
            fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
                #prefix_check

                #(#field_arms)*

                Some((
                    Self {
                        #(#field_packing),*
                    },
                    skip
                ))
            }
        }
    };
    r#gen.into()
}
