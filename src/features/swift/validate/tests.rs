//! What the Swift `validate()` is written from: a pattern as Foundation reads it, a property read
//! past a local, and the string a bound measures off a named type.

use syn::Item;

use super::{property_read, swift_pattern};
use crate::features::model_schema_prop::{FunctionName, parse_model_schema_prop_attributes};
use crate::features::swift::{SwiftTypeArgs, swift_schema_dispatch};

/// The arguments of an item that writes none: each member's own `model_schema_prop`, as written.
fn args() -> SwiftTypeArgs<'static> {
    SwiftTypeArgs {
        check: &[],
        default_types: &[],
        max_length: None,
        member_meta: &|member| parse_model_schema_prop_attributes(&member.attrs),
        min_length: None,
        name_override: None,
        pattern: None,
        preprocess: &[],
        type_name: "Probe",
    }
}

/// The tokens the Swift emitter answers for `item`, written with no argument of `model_schema`.
fn emitted(item: &Item) -> String {
    swift_schema_dispatch(item, &args()).to_string()
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
#[cfg(feature = "serde")]
#[test]
fn a_brand_over_a_string_carries_the_member_that_answers_its_text() {
    let code: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct TextCode(String);
    };
    let sealed: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct TextSealedCode(TextCode);
    };
    let count: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct TextCount(u32);
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
        pub struct MeasuredCode(String);
    };
    let sealed: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct MeasuredSealedCode(MeasuredCode);
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
        "if code.swiftSchemaBoundLength < 2",
        "if later.swiftSchemaBoundLength < 2",
        "if sealed.swiftSchemaBoundLength < 2",
        "if title.unicodeScalars.count < 2",
        "if version.swiftSchemaBoundLength < 2",
    ] {
        assert!(tokens.contains(written), "{written} is not in: {tokens}");
    }
}

/// A map keyed by a brand with a check judges each key under the step of its entry, as Rust's
/// `validate()` names it, and binds no value it does not walk. A map whose key has no rule walks
/// nothing of it.
#[cfg(feature = "serde")]
#[test]
fn a_map_keyed_by_a_brand_with_a_check_judges_each_key_under_its_step() {
    let pitch: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct ScalePitch(u32);
    };
    let checks = [FunctionName {
        name: "withinScale".to_owned(),
        written_at: proc_macro2::Span::call_site(),
    }];
    let brand = swift_schema_dispatch(
        &pitch,
        &SwiftTypeArgs {
            check: &checks,
            ..args()
        },
    )
    .to_string();
    assert!(brand.contains("withinScale"), "got: {brand}");
    let holder: Item = syn::parse_quote! {
        pub struct ScalePeal {
            pub pitches: BTreeMap<ScalePitch, u32>,
            pub towers: BTreeMap<String, u32>,
        }
    };
    let tokens = emitted(&holder);
    let walked = r#"for (key, _) in pitches.sorted(by: { $0.key.value < $1.key.value }) { errors += key.validate().map { ScalePeal.swiftSchemaUnder(\"pitches.\\(key.value)\", $0) } }"#;
    assert!(tokens.contains(walked), "{walked} is not in: {tokens}");
    assert!(!tokens.contains("in towers.sorted"), "got: {tokens}");
}
