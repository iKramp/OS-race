use core::panic;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(EnumNewMacro)]
pub fn derive_enum_new(input: TokenStream) -> TokenStream {

    let input = parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let variants = match &input.data {
        syn::Data::Enum(data) => &data.variants,
        _ => panic!("EnumNew can only be derived for enums"),
    };


    let variant_arms = variants.iter().map(|variant| {
        let ident = &variant.ident;
        let syn::Fields::Unnamed(field) = &variant.fields else {
            panic!("EnumNew can only be derived for enums with unnamed fields");
        };
        let field = field.unnamed.first().expect("EnumNew can only be derived for enums with at least one field");


        quote::quote! {
            if let Some((data, skip)) = #field::aml_new(data) {
                return Some((#name::#ident(data), skip));
            }
        }
    }).collect::<Vec<_>>();

    let gen = quote::quote! {
        impl #impl_generics EnumNew for #name #ty_generics #where_clause {
            fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
                #(#variant_arms)*
                None
            }
        }
    };
    gen.into()
}

//TODO: remove this, make a new attribute macro to add a prefix to the struct, then check in derive
//macro
#[proc_macro_attribute]//struct with prefix
pub fn new_aml_struct(input: TokenStream, args: TokenStream) -> TokenStream {

    let input = parse_macro_input!(input as syn::DeriveInput);
    let identifier = parse_macro_input!(args as syn::Ident);

    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("StructNew can only be derived for structs"),
    };

    let field_arms = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let ty = field.ty.clone();

        quote::quote! {
            let (#ident, new_skip) = #ty.aml_new(data[skip..]).unwrap();
            skip += new_skip;
        }
    });

    let field_packing = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        quote::quote! {
            #ident,
        }
    });

    let gen = quote::quote! {
        impl #impl_generics StructNew for #name #ty_generics #where_clause {
            fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
                if data[0] != #identifier {
                    return None;
                }
                let mut skip = 1;

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
    gen.into()
}

#[proc_macro_derive(StructNewMacro)]//struct without prefix
pub fn derive_struct_new(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("StructNew can only be derived for structs"),
    };

    let field_arms = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let ty = field.ty.clone();

        quote::quote! {
            let (#ident, new_skip) = #ty.aml_new(data[skip..])?;
            skip += new_skip;
        }
    });

    let field_packing = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        quote::quote! {
            #ident,
        }
    });

    let gen = quote::quote! {
        impl #impl_generics StructNew for #name #ty_generics #where_clause {
            fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
                let mut skip = 1;

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
    gen.into()
}
