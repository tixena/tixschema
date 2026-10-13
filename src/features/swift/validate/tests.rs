//! What the Swift `validate()` is written from: a pattern as Foundation reads it, a property read
//! past a local, and the string a bound measures off a named type.

use syn::Item;

use super::{property_read, swift_pattern};
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;
use crate::features::swift::{SwiftTypeArgs, swift_schema_dispatch};

/// The tokens the Swift emitter answers for `item`, written with no argument of `model_schema`.
fn emitted(item: &Item) -> String {
    swift_schema_dispatch(
        item,
        &SwiftTypeArgs {
            check: &[],
            default_types: &[],
            max_length: None,
            member_meta: &|member| parse_model_schema_prop_attributes(&member.attrs),
            min_length: None,
            name_override: None,
            pattern: None,
            preprocess: &[],
        },
    )
    .to_string()
}

#[test]
fn an_end_anchor_is_written_as_the_end_of_the_text() {
    assert_eq!(swift_pattern("^[a-z]+$"), "^[a-z]+\\z");
    assert_eq!(swift_pattern("^(a|b)$|^c$"), "^(a|b)\\z|^c\\z");
    assert_eq!(swift_pattern("^a[$]b\\$"), "^a[$]b\\$");
    assert_eq!(swift_pattern("^[A-Z]{3}"), "^[A-Z]{3}");
}

#[test]
fn a_vertical_tab_is_written_as_its_code_point() {
    assert_eq!(
        swift_pattern("^[\\t\\n\\v\\f\\r ]+$"),
        "^[\\t\\n\\u000B\\f\\r ]+\\z"
    );
    assert_eq!(swift_pattern("a\\vb"), "a\\u000Bb");
}

#[test]
fn a_property_named_after_a_local_of_the_method_is_read_through_self() {
    assert_eq!(property_read("errors"), "self.errors");
    assert_eq!(property_read("held"), "self.held");
    assert_eq!(property_read("index1"), "self.index1");
    assert_eq!(property_read("key"), "self.key");
    assert_eq!(property_read("label"), "label");
    assert_eq!(property_read("`class`"), "`class`");
}

/// A brand over a string answers its text with a member, and a brand over a brand through the
/// brand it holds. A brand over a number has none.
#[test]
fn a_brand_over_a_string_carries_the_member_that_answers_its_text() {
    let code: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct TextCode(pub String);
    };
    let sealed: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct TextSealedCode(pub TextCode);
    };
    let count: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct TextCount(pub u32);
    };
    assert!(
        emitted(&code).contains("public var swiftSchemaBoundText: String { value }"),
        "got: {}",
        emitted(&code)
    );
    assert!(
        emitted(&sealed)
            .contains("public var swiftSchemaBoundText: String { value.swiftSchemaBoundText }"),
        "got: {}",
        emitted(&sealed)
    );
    assert!(
        !emitted(&count).contains("swiftSchemaBoundText"),
        "got: {}",
        emitted(&count)
    );
}

/// A brand declared above, a brand declared below and a brand over a brand are measured through
/// the member, and an alias of a string declared above as the string it is. Nothing is refused as
/// the type is expanded: a type with no such member is the Swift compiler's to refuse.
#[test]
fn a_length_on_a_named_type_is_measured_through_the_member_or_as_the_string() {
    let code: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct MeasuredCode(pub String);
    };
    let sealed: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct MeasuredSealedCode(pub MeasuredCode);
    };
    let title: Item = syn::parse_quote! {
        pub type MeasuredTitle = String;
    };
    let version: Item = syn::parse_quote! {
        pub struct MeasuredVersion {
            pub number: i32,
        }
    };
    for declared in [&code, &sealed, &title, &version] {
        assert!(!emitted(declared).contains("compile_error"));
    }
    let holder: Item = syn::parse_quote! {
        pub struct MeasuredHolder {
            #[model_schema_prop(minLength = 2)]
            pub code: MeasuredCode,
            #[model_schema_prop(minLength = 2)]
            pub later: MeasuredLater,
            #[model_schema_prop(minLength = 2)]
            pub sealed: MeasuredSealedCode,
            #[model_schema_prop(minLength = 2)]
            pub title: MeasuredTitle,
            #[model_schema_prop(minLength = 2)]
            pub version: MeasuredVersion,
        }
    };
    let tokens = emitted(&holder);
    assert!(!tokens.contains("compile_error"), "got: {tokens}");
    for written in [
        "if code.swiftSchemaBoundText.unicodeScalars.count < 2",
        "if later.swiftSchemaBoundText.unicodeScalars.count < 2",
        "if sealed.swiftSchemaBoundText.unicodeScalars.count < 2",
        "if title.unicodeScalars.count < 2",
        "if version.swiftSchemaBoundText.unicodeScalars.count < 2",
    ] {
        assert!(tokens.contains(written), "{written} is not in: {tokens}");
    }
}
