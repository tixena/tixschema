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
    }
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
