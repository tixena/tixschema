//! Unit tests of the Kotlin emitter: a unit struct renders as an `object`, a braced empty struct as
//! a class, and what `validate()` writes for the shapes no integration test can declare.

use proc_macro2::{Span, TokenTree};

use super::validate::jvm_pattern;
use super::*;
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;

/// The Kotlin source the tokens of an item publish: the first string literal they hold.
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

/// The Kotlin source of a struct that writes no argument.
fn plain(item: &ItemStruct) -> String {
    definition(struct_kotlin_tokens(item, &plain_args()))
}

/// The arguments of an item that writes none: each member's own `model_schema_prop`, as written.
fn plain_args() -> KotlinArgs<'static> {
    KotlinArgs {
        check: &[],
        default_types: &[],
        max_length: None,
        member_meta: &|member| parse_model_schema_prop_attributes(&member.attrs),
        min_length: None,
        name_override: None,
        pattern: None,
    }
}

/// A unit struct is Kotlin's own no-data type: `@Serializable object`, never a `class`.
#[test]
fn a_unit_struct_renders_as_an_object() {
    let item: ItemStruct = syn::parse_quote! {
        struct Ping;
    };
    let rendered = struct_kotlin_tokens(&item, &plain_args()).to_string();
    assert!(rendered.contains("object Ping"), "got: {rendered}");
    assert!(!rendered.contains("class Ping"), "got: {rendered}");
}

#[test]
fn a_braced_empty_struct_still_renders_as_a_class() {
    let item: ItemStruct = syn::parse_quote! {
        struct BracedEmpty {}
    };
    let rendered = struct_kotlin_tokens(&item, &plain_args()).to_string();
    assert!(rendered.contains("class BracedEmpty"), "got: {rendered}");
    assert!(!rendered.contains("object BracedEmpty"), "got: {rendered}");
}

#[test]
fn a_pattern_is_written_as_the_jvm_reads_what_rust_reads() {
    for (written, read) in [
        ("^[a-z]+$", "^[a-z]+\\z"),
        ("^[\\t\\n\\v\\f\\r ]+$", "^[\\t\\n\\x0B\\f\\r ]+\\z"),
        ("^[$]\\$", "^[$]\\$"),
        ("^(?<first_name>[a-z]+)-(?:x|y)$", "^([a-z]+)-(?:x|y)\\z"),
        ("a$|b", "a\\z|b"),
    ] {
        assert_eq!(jvm_pattern(written), read);
    }
}

#[test]
fn text_inside_a_kotlin_string_opens_no_template_and_closes_no_string() {
    assert_eq!(
        kotlin_template_text("^\\d+\"x\"$oid\n"),
        "^\\\\d+\\\"x\\\"\\$oid\\n"
    );
}

/// A number is compared in the Kotlin type it is held as, and as a double where the bound is no
/// value of that type.
#[test]
fn a_range_is_compared_in_the_numbers_own_type() {
    let item: ItemStruct = syn::parse_quote! {
        struct Ranges {
            #[model_schema_prop(minimum = 1)]
            count: u8,
            #[model_schema_prop(minimum = -5)]
            depth: i64,
            #[model_schema_prop(minimum = 0.5)]
            half: u32,
            #[model_schema_prop(maximum = 120)]
            ratio: f64,
            #[model_schema_prop(maximum = 1.5)]
            tilt: f32,
        }
    };
    let kotlin = plain(&item);
    for compared in [
        "if (count < 1u) violations.add(\"'count': too small: minimum is 1, got ${count}\")",
        "if (depth < -5) violations.add(\"'depth': too small: minimum is -5, got ${depth}\")",
        "if (half.toDouble() < 0.5) violations.add(",
        "if (ratio > 120.0) violations.add(\"'ratio': too large: maximum is 120, got \
         ${kotlinSchemaShown(ratio.toString())}\")",
        "if (tilt > 1.5f) violations.add(",
    ] {
        assert!(kotlin.contains(compared), "no `{compared}` in: {kotlin}");
    }
}

/// A member named after the list the method fills is read through `this`, and one named after
/// the method builds beside it.
#[test]
fn a_member_named_after_what_the_method_writes_is_still_read() {
    let item: ItemStruct = syn::parse_quote! {
        struct Probe {
            #[model_schema_prop(minLength = 1)]
            validate: String,
            #[model_schema_prop(minimum = 1)]
            violations: i32,
        }
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.contains("data class Probe(val validate: String, val violations: Int) { fun validate(): List<String> {"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("if (validate.codePointCount(0, validate.length) < 1) violations.add("),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("if (this.violations < 1) violations.add(\"'violations': too small: minimum is 1, got ${this.violations}\")"),
        "got: {kotlin}"
    );
}

/// A map's key typed with a parameter is written into a place where the type declares
/// `Serialize` for it, and left out where it declares none, as Rust writes it.
#[test]
fn a_key_typed_with_a_parameter_is_written_where_the_type_declares_serialize() {
    let keyed: ItemStruct = syn::parse_quote! {
        struct Rack<Key: Ord + Serialize> {
            by_key: BTreeMap<Key, Parcel>,
        }
    };
    let written_under_its_key = plain(&keyed);
    assert!(
        written_under_its_key.contains(
            "for ((key, held) in byKey) { violations.addAll(held.validate().map { \
             kotlinSchemaUnder(\"by_key.$key\", it) }) }"
        ),
        "got: {written_under_its_key}"
    );
    let unkeyed: ItemStruct = syn::parse_quote! {
        struct Shelf<Key> where Key: Ord {
            by_key: BTreeMap<Key, Parcel>,
        }
    };
    let written_under_the_field = plain(&unkeyed);
    assert!(
        written_under_the_field.contains(
            "for ((_, held) in byKey) { violations.addAll(held.validate().map { \
             kotlinSchemaUnder(\"by_key\", it) }) }"
        ),
        "got: {written_under_the_field}"
    );
}

/// A key that is no string is written as its serializer writes it: a plain enum under the name
/// serde writes the variant.
#[test]
fn a_key_of_a_declared_type_is_written_as_its_serializer_writes_it() {
    let item: ItemStruct = syn::parse_quote! {
        struct Docks {
            by_number: BTreeMap<u32, Parcel>,
            by_wing: BTreeMap<Wing, Parcel>,
        }
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.contains("kotlinSchemaUnder(\"by_number.$key\", it)"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "kotlinSchemaUnder(\"by_wing.${Json.encodeToJsonElement(serializer<Wing>(), \
             key).jsonPrimitive.content}\", it)"
        ),
        "got: {kotlin}"
    );
}

/// Only a brand that holds a string, or a brand or an alias that answers one, answers its text.
#[test]
fn a_brand_over_anything_but_a_text_answers_none() {
    let text: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct TextBrand(String);
    };
    let over_text: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct OverTextBrand(TextBrand);
    };
    let record: ItemStruct = syn::parse_quote! {
        struct PlainRecord { label: String }
    };
    assert!(plain(&text).contains("fun kotlinSchemaText(): String = value;"));
    assert!(plain(&record).contains("fun validate(): List<String> = emptyList()"));
    assert!(
        plain(&over_text).contains("fun kotlinSchemaText(): String = value.kotlinSchemaText();"),
        "a brand over a brand written above answers through it"
    );
    for silent in [
        syn::parse_quote! { #[serde(transparent)] struct NumberBrand(u32); },
        syn::parse_quote! { #[serde(transparent)] struct ListBrand(Vec<String>); },
        syn::parse_quote! { #[serde(transparent)] struct RecordBrand(PlainRecord); },
        syn::parse_quote! { #[serde(transparent)] struct UnseenBrand(NotWrittenYet); },
        syn::parse_quote! { #[serde(transparent)] struct ParameterBrand<Held>(Held); },
    ] {
        let kotlin = plain(&silent);
        assert!(!kotlin.contains("kotlinSchemaText"), "got: {kotlin}");
    }
}

/// A brand that holds one of its parameters bare runs its bounds where the value is of the type
/// declared for the parameter, and writes none where that type is written with arguments.
#[test]
fn a_brand_with_a_parameter_is_bounded_at_its_declared_default() {
    let item: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct UserId<IdType>(IdType);
    };
    let bounded = |default_types: &[(Ident, Type)]| {
        definition(struct_kotlin_tokens(
            &item,
            &KotlinArgs {
                default_types,
                min_length: Some(3),
                ..plain_args()
            },
        ))
    };
    let parameter = Ident::new("IdType", Span::call_site());
    assert!(
        bounded(&[]).contains(
            "{ fun validate(): List<String> { val violations = mutableListOf<String>(); if (value \
             is String) { if (value.codePointCount(0, value.length) < 3) violations.add(\"too \
             short: minimum length is 3, got ${value.codePointCount(0, value.length)}\") }; \
             return violations } }"
        ),
        "a parameter with no declared type is bounded as a string"
    );
    let branded = bounded(&[(parameter.clone(), syn::parse_quote!(Slug))]);
    assert!(
        branded.contains("if (value is Slug) { if (value.kotlinSchemaText().codePointCount(0, "),
        "got: {branded}"
    );
    let filled = bounded(&[(parameter, syn::parse_quote!(Other<String>))]);
    assert!(
        filled.contains("{ fun validate(): List<String> = emptyList() }"),
        "got: {filled}"
    );
}

/// A type's own check is called on the whole value, on a plain enum and on a unit struct as well.
#[test]
fn a_types_own_check_is_called_on_the_whole_value() {
    let check = [FunctionName {
        name: "rated".to_owned(),
        written_at: Span::call_site(),
    }];
    let args = KotlinArgs {
        check: &check,
        ..plain_args()
    };
    let called = "fun validate(): List<String> { val violations = mutableListOf<String>(); \
                  violations.addAll(rated(this)); return violations }";
    let graded: ItemEnum = syn::parse_quote! {
        enum Grade { Express, Unrated }
    };
    assert_eq!(
        definition(enum_kotlin_tokens(&graded, &args)),
        format!("@Serializable enum class Grade {{ Express, Unrated; {called} }}")
    );
    let closed: ItemStruct = syn::parse_quote! {
        struct ClosedDock;
    };
    assert_eq!(
        definition(struct_kotlin_tokens(&closed, &args)),
        format!("@Serializable object ClosedDock {{ {called} }}")
    );
}

/// The README shows the parcel and the manifest of its checks as the emitter writes them, each
/// line break of the block standing for one space.
#[cfg(feature = "serde")]
#[test]
fn the_readme_shows_what_the_parcel_of_the_documents_emits() {
    let within = [FunctionName {
        name: "withinParcel".to_owned(),
        written_at: Span::call_site(),
    }];
    let parcel: ItemStruct = syn::parse_quote! {
        #[serde(rename_all = "camelCase")]
        pub struct Parcel {
            #[model_schema_prop(minLength = 1)]
            pub label: String,
            #[model_schema_prop(check = ["withoutDuplicates"])]
            pub item_code_list: Vec<String>,
            pub fragile_item_codes: Vec<String>,
        }
    };
    let manifest: ItemStruct = syn::parse_quote! {
        pub struct Manifest {
            pub parcel: Parcel,
        }
    };
    let checked = KotlinArgs {
        check: &within,
        ..plain_args()
    };
    let emitted = [
        definition(struct_kotlin_tokens(&parcel, &checked)),
        plain(&manifest),
    ];
    let readme = include_str!("../../../README.md");
    let opens = "```kotlin\n@Serializable data class Parcel(val label";
    let block = readme
        .find(opens)
        .and_then(|start| readme.get(start + "```kotlin\n".len()..))
        .and_then(|rest| rest.split_once("\n```"))
        .unwrap()
        .0;
    let shown: Vec<String> = block
        .split("\n\n")
        .map(|declaration| {
            declaration
                .lines()
                .map(str::trim_start)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    assert_eq!(shown, emitted);
}
