use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Iter, DeriveInput, Ident, Variant};

use crate::utils::{get_enum_variant_type_path, Imports};

pub fn consensus_state_derive_impl(ast: DeriveInput) -> TokenStream {
    let enum_name = &ast.ident;
    let enum_variants = match ast.data {
        syn::Data::Enum(ref enum_data) => &enum_data.variants,
        _ => panic!("ConsensusState only supports enums"),
    };

    let root_impl = delegate_call_in_match(enum_name, enum_variants.iter(), quote! {root(cs)});
    let timestamp_impl =
        delegate_call_in_match(enum_name, enum_variants.iter(), quote! {timestamp(cs)});
    let encode_vec_impl =
        delegate_call_in_match(enum_name, enum_variants.iter(), quote! {encode_vec(cs)});

    let CommitmentRoot = Imports::CommitmentRoot();
    let ConsensusState = Imports::ConsensusState();
    let Timestamp = Imports::Timestamp();

    quote! {
        impl #ConsensusState for #enum_name {
            fn root(&self) -> &#CommitmentRoot {
                match self {
                    #(#root_impl),*
                }
            }

            fn timestamp(&self) -> #Timestamp {
                match self {
                    #(#timestamp_impl),*
                }
            }

            fn encode_vec(&self) -> Vec<u8> {
                match self {
                    #(#encode_vec_impl),*
                }
            }
        }
    }
}

fn delegate_call_in_match(
    enum_name: &Ident,
    enum_variants: Iter<'_, Variant>,
    fn_call: TokenStream,
) -> Vec<TokenStream> {
    let ConsensusState = Imports::ConsensusState();

    enum_variants
        .map(|variant| {
            let variant_name = &variant.ident;
            let variant_type_name = get_enum_variant_type_path(variant);

            quote! {
                #enum_name::#variant_name(cs) => <#variant_type_name as #ConsensusState>::#fn_call
            }
        })
        .collect()
}
