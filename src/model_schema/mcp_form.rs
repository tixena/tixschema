//! `mcp_form_value()` and `mcp_form_flat()`: whether a model type is a choice among plain names,
//! and whether it is a form MCP can show, which takes text, numbers, yes or no, and choices.
//!
//! A type that names another reads the answer recorded when that one was expanded, and takes
//! `false` for one it has not met, so a field never fails the build of the type that holds it.

use super::{Item, Tagging, type_parameters_in_scope, variant_wire_names};
use crate::features::serde::{parse_serde_field_attributes, variant_on_the_wire};
use crate::utils::{record_mcp_form_value, recorded_mcp_form_value};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, GenericArgument, PathArguments, Type};

const SCALARS: [&str; 14] = [
    "String", "bool", "f32", "f64", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64",
    "usize",
];

/// The two `const fn` an item earns. An alias earns none: it resolves to the type it stands for.
pub fn items(item: &Item) -> TokenStream {
    let (name, generics, value, flat) = match item {
        Item::Struct(declared) => (
            &declared.ident,
            &declared.generics,
            false,
            struct_flat(declared),
        ),
        Item::Enum(declared) => (
            &declared.ident,
            &declared.generics,
            of_enum(declared),
            false,
        ),
        Item::Const(_)
        | Item::ExternCrate(_)
        | Item::Fn(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Macro(_)
        | Item::Mod(_)
        | Item::Static(_)
        | Item::Trait(_)
        | Item::TraitAlias(_)
        | Item::Type(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_)
        | _ => return TokenStream::new(),
    };
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let variant_tag = if let Item::Enum(declared) = item {
        Some(variant_tag(declared))
    } else {
        None
    };
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            pub const fn mcp_form_value() -> bool {
                #value
            }

            pub const fn mcp_form_flat() -> bool {
                #flat
            }

            #variant_tag
        }
    }
}

/// `mcp_variant_tag`: the key serde tags an enum's variant with, the name it writes the variant
/// under and the key it holds the variant's members under, for a variant serde writes. The tag is
/// empty for an enum with no tag, the content key for one that is not adjacently tagged, and no
/// variant of an untagged enum has a name.
fn variant_tag(declared: &syn::ItemEnum) -> TokenStream {
    let key = match Tagging::of(declared) {
        Tagging::Adjacent { content, tag } => Some((tag, content)),
        Tagging::Internal { tag } => Some((tag, String::new())),
        Tagging::External | Tagging::Plain => Some((String::new(), String::new())),
        Tagging::Untagged => None,
    };
    let arms = key.into_iter().flat_map(|(tag, content)| {
        declared
            .variants
            .iter()
            .filter(|variant| variant_on_the_wire(variant))
            .filter_map(|variant| {
                let (_, wire) = variant_wire_names(declared, variant).into_iter().next()?;
                let ident = variant.ident.to_string();
                Some(quote! {
                    if same(variant.as_bytes(), #ident.as_bytes()) {
                        return ::core::option::Option::Some((#tag, #wire, #content));
                    }
                })
            })
            .collect::<Vec<_>>()
    });
    quote! {
        pub const fn mcp_variant_tag(variant: &str) -> ::core::option::Option<(&'static str, &'static str, &'static str)> {
            const fn same(left: &[u8], right: &[u8]) -> bool {
                if left.len() != right.len() {
                    return false;
                }
                let mut at = 0;
                while at < left.len() {
                    if left[at] != right[at] {
                        return false;
                    }
                    at += 1;
                }
                true
            }
            #(#arms)*
            ::core::option::Option::None
        }
    }
}

/// Records the answer for `item`, for the forms that name it.
pub fn record(item: &Item) {
    let (name, value) = match item {
        Item::Enum(declared) => (&declared.ident, of_enum(declared)),
        Item::Struct(declared) => (&declared.ident, false),
        Item::Type(declared) => (&declared.ident, is_form_value(&declared.ty)),
        Item::Const(_)
        | Item::ExternCrate(_)
        | Item::Fn(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Macro(_)
        | Item::Mod(_)
        | Item::Static(_)
        | Item::Trait(_)
        | Item::TraitAlias(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_)
        | _ => return,
    };
    record_mcp_form_value(&name.to_string(), value);
}

fn of_enum(declared: &syn::ItemEnum) -> bool {
    matches!(Tagging::of(declared), Tagging::Plain)
}

fn struct_flat(declared: &syn::ItemStruct) -> bool {
    let Fields::Named(fields) = &declared.fields else {
        return false;
    };
    type_parameters_in_scope(&declared.generics).is_empty()
        && fields.named.iter().all(|field| {
            !parse_serde_field_attributes(&field.attrs).flatten && is_form_field(&field.ty)
        })
}

fn is_form_field(ty: &Type) -> bool {
    if let Some(inner) = only_argument(ty, "Option") {
        return is_form_field(inner);
    }
    if let Some(inner) = only_argument(ty, "Vec") {
        return is_form_value(inner);
    }
    is_scalar(ty) || is_form_value(ty)
}

fn is_form_value(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|last| recorded_mcp_form_value(&last.ident.to_string()) == Some(true))
}

fn only_argument<'ty>(ty: &'ty Type, wrapper: &str) -> Option<&'ty Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    if last.ident != wrapper {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    match arguments.args.first() {
        Some(GenericArgument::Type(inner)) if arguments.args.len() == 1 => Some(inner),
        _ => None,
    }
}

fn is_scalar(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.qself.is_none()
        && path.path.segments.len() == 1
        && path.path.segments[0].arguments.is_none()
        && SCALARS.iter().any(|scalar| path.path.segments[0].ident == scalar))
}

#[cfg(test)]
mod tests {
    use super::items;
    use syn::{Item, parse_quote};

    fn written(declared: syn::ItemEnum) -> String {
        items(&Item::Enum(declared)).to_string()
    }

    #[test]
    fn a_variant_is_answered_under_the_key_and_the_name_serde_writes_it_with() {
        let renamed = written(parse_quote! {
            #[serde(rename_all = "kebab-case", tag = "kind")]
            enum Asked {
                NeedsConfirmation { message: ::std::string::String },
                #[serde(rename = "gone-for-good")]
                NotFound,
            }
        });
        for expected in [
            "same (variant . as_bytes () , \"NeedsConfirmation\" . as_bytes ()) { return :: core :: option :: Option :: Some ((\"kind\" , \"needs-confirmation\" , \"\")) ; }",
            "same (variant . as_bytes () , \"NotFound\" . as_bytes ()) { return :: core :: option :: Option :: Some ((\"kind\" , \"gone-for-good\" , \"\")) ; }",
        ] {
            assert!(
                renamed.contains(expected),
                "missing `{expected}` in {renamed}"
            );
        }
    }

    #[test]
    fn an_adjacently_tagged_variant_is_answered_with_its_content_key() {
        let adjacent = written(parse_quote! {
            #[serde(rename_all = "kebab-case", tag = "kind", content = "data")]
            enum Asked {
                NeedsConnection { message: ::std::string::String },
            }
        });
        assert!(
            adjacent.contains("Some ((\"kind\" , \"needs-connection\" , \"data\"))"),
            "got: {adjacent}"
        );
    }

    #[test]
    fn an_enum_with_no_tag_answers_an_empty_key_and_a_skipped_variant_none() {
        let external = written(parse_quote! {
            enum Asked {
                Wanted { message: ::std::string::String },
                #[serde(skip)]
                Hidden,
            }
        });
        assert!(
            external.contains("Some ((\"\" , \"Wanted\" , \"\"))"),
            "got: {external}"
        );
        assert!(!external.contains("\"Hidden\""), "got: {external}");
    }

    #[test]
    fn no_variant_of_an_untagged_enum_has_a_name() {
        let untagged = written(parse_quote! {
            #[serde(untagged)]
            enum Asked {
                Wanted { message: ::std::string::String },
            }
        });
        assert!(!untagged.contains("\"Wanted\""), "got: {untagged}");
    }
}
