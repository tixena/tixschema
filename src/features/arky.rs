//! What `enable = ["arky"]` adds to a type: the conversion of a value of it to arky's
//! frontmatter `Value`, and the read of one back, both through serde.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Item;

/// `TryFrom<&Type> for Value`, through `arky_core::value::to_value`, and `TryFrom<&Value> for
/// Type`, through `arky_core::value::from_value`. A type with parameters bounds each impl on
/// the type itself, so the bounds its serde derives write are the ones asked. Nothing for an item
/// that is no struct or enum.
pub fn conversions(item: &Item) -> TokenStream {
    let (ident, generics) = if let Item::Struct(item_struct) = item {
        (&item_struct.ident, &item_struct.generics)
    } else if let Item::Enum(item_enum) = item {
        (&item_enum.ident, &item_enum.generics)
    } else {
        return TokenStream::new();
    };
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let own = where_clause.map(|clause| &clause.predicates);
    let (written, read) = if generics.params.is_empty() {
        (TokenStream::new(), TokenStream::new())
    } else {
        (
            quote! { where #ident #type_generics: ::serde::Serialize, #own },
            quote! { where #ident #type_generics: ::serde::de::DeserializeOwned, #own },
        )
    };
    quote! {
        impl #impl_generics ::core::convert::TryFrom<&#ident #type_generics>
            for ::arky_core::parser::ast::Value
        #written
        {
            type Error = ::arky_core::value::ValueError;

            fn try_from(
                value: &#ident #type_generics,
            ) -> ::core::result::Result<Self, Self::Error> {
                ::arky_core::value::to_value(value)
            }
        }

        impl #impl_generics ::core::convert::TryFrom<&::arky_core::parser::ast::Value>
            for #ident #type_generics
        #read
        {
            type Error = ::arky_core::value::ValueError;

            fn try_from(
                value: &::arky_core::parser::ast::Value,
            ) -> ::core::result::Result<Self, Self::Error> {
                ::arky_core::value::from_value(value)
            }
        }
    }
}
