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
/// its tag, then its content key, in the whole object its holder reads, as TypeScript and Zod read
/// it. One under a tag alone takes no key off the object the members after it are handed, and one
/// under a tag and a content key takes both, as serde reads it. One that names a function on itself
/// has no key to be looked for by, and is `null` wherever its read throws.
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
    let handed = "({...json}..remove('note')..remove('kind')..remove('body'))";
    let expected = format!(
        "Belaying(cleat: (json.keys.any(const ['kind'].contains) ? Cleat.fromJsonUnjudged(json) : \
         null),bollard: (json.keys.any(const ['kind', 'body'].contains) ? \
         Bollard.fromJsonUnjudged(json) : null),bitt: (() {{ try {{ return \
         Bitt.fromJsonUnjudged({handed}); }} catch (_) {{ return null; }} }})(),note: \
         json['note'] as String,);"
    );
    let dart = definition(struct_dart_tokens(&item, &plain_args()));
    assert!(dart.contains(&expected), "missing `{expected}` in: {dart}");
}

/// A `u64` and an `i64` are each held as an `int` and read and written as the JSON number serde
/// writes, the `u64` held to be no less than zero. A build with Swift or Kotlin refuses a `u64` in
/// a member, so no integration test declares one beside the Dart client's runs.
#[cfg(feature = "serde")]
#[test]
fn a_u64_and_an_i64_are_ints_read_and_written_as_numbers() {
    let item: ItemStruct = syn::parse_quote! {
        #[derive(Serialize, Deserialize)]
        pub struct Wide {
            pub big: u64,
            pub signed: i64,
        }
    };
    let dart = definition(struct_dart_tokens(&item, &plain_args()));
    for expected in [
        "final int big;",
        "final int signed;",
        "signed: json['signed'] as int,",
        "'signed': signed,",
    ] {
        assert!(dart.contains(expected), "missing `{expected}` in: {dart}");
    }
    assert!(
        !dart.contains("BigInt"),
        "a `u64` is held as an `int`: {dart}"
    );
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

/// A struct under its own tag refuses an object without the tag, or with another value under it,
/// ahead of its members: the object its functions answer where it names one, and the copy its
/// aliases are read from where a member has one.
#[cfg(feature = "serde")]
#[test]
fn a_struct_under_its_own_tag_checks_the_object_its_members_are_read_from() {
    let item: ItemStruct = syn::parse_quote! {
        #[serde(tag = "kind")]
        struct Halyard {
            #[serde(alias = "name")]
            label: String,
        }
    };
    let functions = [FunctionName {
        name: "asGiven".to_owned(),
        written_at: proc_macro2::Span::call_site(),
    }];
    let dart = definition(struct_dart_tokens(
        &item,
        &DartTypeArgs {
            preprocess: &functions,
            ..plain_args()
        },
    ));
    let expected = "factory Halyard.fromJsonUnjudged(dynamic payload) { final json = \
                    asGiven(payload) as Map<String, dynamic>; final named = <String, dynamic>{}; \
                    for (final entry in json.entries) { final key = switch (entry.key) { 'name' => \
                    'label', final other => other }; if (named.containsKey(key)) throw \
                    FormatException('duplicate field `$key`'); named[key] = entry.value; } if \
                    (!named.containsKey('kind')) throw FormatException('missing field `kind`'); if \
                    (named['kind'] != 'Halyard') throw FormatException('expected tag \"Halyard\", \
                    found ' + jsonEncode(named['kind'])); return Halyard(label: named['label'] as \
                    String,); }";
    assert!(dart.contains(expected), "missing `{expected}` in: {dart}");
}
