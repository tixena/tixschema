//! What a type publishes, in hidden constants, of how a holder that flattens it reads it, for a
//! holder written before it can see the type: one declared below it or in another crate.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Item;

use crate::features::model_schema_prop::FunctionName;

/// The value of the hidden constant of the names each variant of `item` is read under when it is
/// flattened: one list per variant, none for a type that is no externally tagged enum.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub(super) fn entries(item: &Item, preprocess: &[FunctionName]) -> TokenStream {
    use crate::features::serde::{variant_names_when_flattened, variants_on_the_wire};
    let Item::Enum(declared) = item else {
        return quote! { &[] };
    };
    let names = variant_names_when_flattened(&variants_on_the_wire(declared), preprocess);
    let groups = names.iter().map(|variant| quote! { &[#(#variant),*] });
    quote! { &[#(#groups),*] }
}

/// The value of the hidden constant of the keys `item` takes off the object a holder that flattens
/// it hands it: a struct's own keys, and the tag and the content key of an enum under both. A
/// struct that flattens a member or names a function on itself, a brand, and any other type take
/// none.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub(super) fn taken(item: &Item, preprocess: &[FunctionName]) -> TokenStream {
    use syn::Fields;

    use crate::features::serde::{
        brand_field, keys_taken_when_flattened, parse_serde_type_attributes,
        tag_and_content_when_flattened, variants_on_the_wire,
    };
    use crate::rename_rule::resolve_rename_rule;
    let keys = if let Item::Struct(declared) = item
        && brand_field(declared).is_none()
        && matches!(declared.fields, Fields::Named(_))
    {
        let rule = resolve_rename_rule(
            parse_serde_type_attributes(&declared.attrs)
                .rename_all
                .as_deref(),
        );
        keys_taken_when_flattened(&declared.fields, rule, preprocess)
    } else if let Item::Enum(declared) = item {
        tag_and_content_when_flattened(&variants_on_the_wire(declared), preprocess)
    } else {
        Vec::new()
    };
    quote! { &[#(#keys),*] }
}

/// The value of the hidden constant of the names the unit variants of `item` are read under when it
/// is flattened, the first of the names each is read under: none for a type that is no externally
/// tagged enum.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub(super) fn units(item: &Item, preprocess: &[FunctionName]) -> TokenStream {
    use crate::features::serde::{
        parse_serde_key_omission, variant_names_when_flattened, variants_on_the_wire,
    };
    use crate::field_type::VariantKind;
    use crate::model_schema::variant_wire_kind;
    let Item::Enum(declared) = item else {
        return quote! { &[] };
    };
    let on_the_wire = variants_on_the_wire(declared);
    let names = variant_names_when_flattened(&on_the_wire, preprocess);
    let first_names: Vec<String> = on_the_wire
        .variants
        .iter()
        .filter(|variant| !parse_serde_key_omission(&variant.attrs).skips_deserializing)
        .zip(&names)
        .filter(|(variant, _)| matches!(variant_wire_kind(variant), VariantKind::Unit))
        .filter_map(|(_, read)| read.first().cloned())
        .collect();
    quote! { &[#(#first_names),*] }
}

/// With no `serde`, or no client that reads a flattened member, no enum takes an entry.
#[cfg(not(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
)))]
pub(super) fn entries(_item: &Item, _preprocess: &[FunctionName]) -> TokenStream {
    quote! { &[] }
}

/// With no `serde`, or no client that reads a flattened member, nothing is taken.
#[cfg(not(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
)))]
pub(super) fn taken(_item: &Item, _preprocess: &[FunctionName]) -> TokenStream {
    quote! { &[] }
}

/// With no `serde`, or no client that reads a flattened member, no enum has a unit entry.
#[cfg(not(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
)))]
pub(super) fn units(_item: &Item, _preprocess: &[FunctionName]) -> TokenStream {
    quote! { &[] }
}
