//! Unit tests of the Dart emitter: what it writes for a shape no integration test can declare.

use proc_macro2::TokenTree;

use super::*;
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;

/// The class of a struct whose one field is named after the unjudged read, which
/// `tests/service_schema_emitted_client_tests/run_dart_read.rs` runs under the Dart SDK.
const LEDGER: &str =
    include_str!("../../../tests/dart_tests/field_named_after_the_unjudged_read.txt");

/// The Dart source the tokens of an item publish: the first string literal they hold.
fn definition(tokens: TokenStream) -> String {
    fn first_literal(tokens: TokenStream) -> Option<String> {
        tokens.into_iter().find_map(|tree| match tree {
            TokenTree::Group(group) => first_literal(group.stream()),
            TokenTree::Literal(literal) => syn::parse_str::<syn::LitStr>(&literal.to_string())
                .ok()
                .map(|text| text.value()),
            TokenTree::Ident(_) | TokenTree::Punct(_) => None,
        })
    }
    first_literal(tokens).unwrap()
}

/// The arguments of an item that writes none: each member's own `model_schema_prop`, as written.
fn plain_args() -> DartTypeArgs<'static> {
    DartTypeArgs {
        check: &[],
        default_types: &[],
        max_length: None,
        member_meta: &|member| parse_model_schema_prop_attributes(&member.attrs),
        min_length: None,
        name_override: None,
        pattern: None,
        preprocess: &[],
    }
}

/// A flattened `Option` of an enum under a tag key whose class was emitted above is looked for by
/// its tag, then its content key, and takes neither off the object the members after it are
/// handed. One that names a function on itself has no key to be looked for by, and is `null`
/// wherever its read throws.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_option_of_an_enum_under_a_tag_key_is_looked_for_by_its_tag() {
    let cleat: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind")]
        enum Cleat { Calm, Round { turns: u32 } }
    };
    let bollard: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind", content = "body")]
        enum Bollard { Calm, Round { turns: u32 } }
    };
    let bitt: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind")]
        enum Bitt { Calm, Round { turns: u32 } }
    };
    let item: ItemStruct = syn::parse_quote! {
        struct Belaying {
            #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
            cleat: Option<Cleat>,
            #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
            bollard: Option<Bollard>,
            #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
            bitt: Option<Bitt>,
            note: String,
        }
    };
    let functions = [FunctionName {
        name: "asGiven".to_owned(),
        written_at: proc_macro2::Span::call_site(),
    }];
    let written = [
        definition(enum_dart_tokens(&cleat, &plain_args())),
        definition(enum_dart_tokens(&bollard, &plain_args())),
        definition(enum_dart_tokens(
            &bitt,
            &DartTypeArgs {
                preprocess: &functions,
                ..plain_args()
            },
        )),
    ];
    assert_eq!(written.len(), 3, "each enum is written ahead of its holder");
    let handed = "({...json}..remove('note'))";
    let expected = format!(
        "Belaying(cleat: ({handed}.keys.any(const ['kind'].contains) ? \
         Cleat.fromJsonUnjudged({handed}) : null),bollard: ({handed}.keys.any(const ['kind', \
         'body'].contains) ? Bollard.fromJsonUnjudged({handed}) : null),bitt: (() {{ try {{ \
         return Bitt.fromJsonUnjudged({handed}); }} catch (_) {{ return null; }} }})(),note: \
         json['note'] as String,);"
    );
    let dart = definition(struct_dart_tokens(&item, &plain_args()));
    assert!(dart.contains(&expected), "missing `{expected}` in: {dart}");
}

/// A field of that name is no snake case, which the lints of an integration test refuse. Dart
/// takes a field and a constructor under one name, so the member keeps its own.
#[test]
fn a_field_named_after_the_unjudged_read_keeps_its_name_and_its_key() {
    let item: ItemStruct = syn::parse_quote! {
        pub struct Ledger {
            pub fromJsonUnjudged: bool,
        }
    };
    assert_eq!(
        definition(struct_dart_tokens(&item, &plain_args())),
        LEDGER.trim_end()
    );
}
