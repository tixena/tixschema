//! The identity check of a range or a `Result` written by its bare name, and the refusals each
//! earns as a map key, under a bound key and under `#[serde(flatten)]`, in every build.

use proc_macro2::{TokenStream, TokenTree};

use super::identity_checks;
use crate::field_type::{RangeKind, written_std_range, written_std_result};
use crate::utils::claim_published_name;

/// What `#[model_schema()]` expands `source` to.
fn expanded(source: &str) -> TokenStream {
    super::super::exec_model_schema(TokenStream::new(), syn::parse_str(source).unwrap())
}

/// The text of every string literal `tokens` holds, at any depth.
fn texts(tokens: TokenStream) -> Vec<String> {
    tokens
        .into_iter()
        .flat_map(|tree| match tree {
            TokenTree::Group(group) => texts(group.stream()),
            TokenTree::Literal(literal) => {
                if let syn::Lit::Str(text) = syn::Lit::new(literal) {
                    vec![text.value()]
                } else {
                    Vec::new()
                }
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) => Vec::new(),
        })
        .collect()
}

/// The diagnostics this crate writes into `tokens`.
fn refusals(tokens: TokenStream) -> Vec<String> {
    texts(tokens)
        .into_iter()
        .filter(|text| text.starts_with("model_schema: "))
        .collect()
}

#[test]
fn a_range_written_by_its_bare_name_is_checked_wherever_it_is_written() {
    for (field, checks) in [
        ("Range<u32>", 1),
        ("RangeInclusive<u32>", 1),
        ("RangeFrom<u32>", 1),
        ("RangeTo<u32>", 1),
        ("ops::Range<u32>", 1),
        ("Vec<Option<Range<u32>>>", 1),
        ("(Range<u32>, RangeTo<u32>)", 2),
        ("HashMap<String, Range<u32>>", 1),
        ("Holder<Range<Range<u32>>>", 2),
        ("[Range<u32>; 2]", 1),
        ("core::ops::Range<u32>", 0),
        ("std::ops::RangeInclusive<u32>", 0),
        ("::core::ops::RangeFrom<u32>", 0),
        ("Range", 0),
        ("u32", 0),
    ] {
        let item: syn::Item =
            syn::parse_str(&format!("struct Holder {{ span: {field} }}")).unwrap();
        assert_eq!(identity_checks(&item).len(), checks, "for {field}");
    }
    let skipped: syn::Item =
        syn::parse_quote! { struct Holder { #[serde(skip)] span: Range<u32>, kept: u32 } };
    assert!(identity_checks(&skipped).is_empty());
    let variant: syn::Item = syn::parse_quote! { enum Choice { Spanned(Range<u32>), Bare } };
    assert_eq!(identity_checks(&variant).len(), 1);
    let alias: syn::Item = syn::parse_quote! { type Spans = Vec<RangeTo<u32>>; };
    assert_eq!(identity_checks(&alias).len(), 1);
}

#[test]
fn a_full_path_is_the_std_range_where_the_crate_declares_a_type_of_the_name() {
    let range = |written: &str| {
        let syn::Type::Path(path) = syn::parse_str::<syn::Type>(written).unwrap() else {
            return None;
        };
        written_std_range(&path)
    };
    assert_eq!(range("Range<u32>"), Some(RangeKind::HalfOpen));
    assert_eq!(range("RangeFrom<u32>"), Some(RangeKind::From));
    assert_eq!(claim_published_name("Range", "Range"), None);
    assert_eq!(range("Range<u32>"), None);
    assert_eq!(range("core::ops::Range<u32>"), Some(RangeKind::HalfOpen));
    assert_eq!(range("::std::ops::Range<u32>"), Some(RangeKind::HalfOpen));
    assert_eq!(range("RangeFrom<u32>"), Some(RangeKind::From));
    assert_eq!(range("Range<u32, u8>"), None);
    assert_eq!(range("<T as Spans>::Range<u32>"), None);
}

#[test]
fn the_check_hands_the_std_range_back_as_the_name_written() {
    let item: syn::Item = syn::parse_quote! { struct Holder { span: RangeInclusive<u32> } };
    let written = identity_checks(&item)
        .into_iter()
        .map(|check| check.to_string())
        .collect::<Vec<_>>();
    let expected: TokenStream = quote::quote! {
        const _: fn(::core::ops::RangeInclusive<()>) -> RangeInclusive<()> = {
            fn written_std_range<Bound>(held: ::core::ops::RangeInclusive<Bound>) -> RangeInclusive<Bound> {
                held
            }
            written_std_range::<()>
        };
    };
    assert_eq!(written, vec![expected.to_string()]);
}

#[test]
fn a_range_as_a_map_key_is_refused_naming_the_field() {
    for written in [
        "HashMap<Range<u32>, u8>",
        "BTreeMap<RangeInclusive<u32>, u8>",
        "Vec<HashMap<RangeTo<u32>, u8>>",
    ] {
        let messages = refusals(expanded(&format!(
            "#[derive(Serialize, Deserialize)] pub struct Shelf {{ pub spans: {written} }}"
        )));
        assert_eq!(messages.len(), 1, "for {written}: {messages:?}");
        assert!(
            messages[0].starts_with(
                "model_schema: field `spans`: a map key must be a value serde writes as a string"
            ),
            "for {written}: {}",
            messages[0]
        );
        assert!(
            messages[0].contains("<_>` as a JSON object"),
            "for {written}: {}",
            messages[0]
        );
    }
}

#[test]
fn a_bound_key_on_a_range_field_is_refused_naming_the_field() {
    for key in [
        "pattern = \"^a$\"",
        "minLength = 1",
        "maxLength = 1",
        "minimum = 1",
        "maximum = 1",
    ] {
        let messages = refusals(expanded(&format!(
            "#[derive(Serialize, Deserialize)] pub struct Shelf {{ #[model_schema_prop({key})] \
             pub span: Range<u32> }}"
        )));
        assert_eq!(messages.len(), 1, "for {key}: {messages:?}");
        assert!(
            messages[0].starts_with("model_schema: field `span`: `")
                && messages[0].contains("` cannot apply to a range \u{2014} "),
            "for {key}: {}",
            messages[0]
        );
    }
}

/// With Swift or Kotlin on, a `usize` is refused wherever it is written, a range's bound among
/// them: the reproduction's `Range<usize>` is refused for its width alone, and nothing else is
/// refused of it.
#[cfg(any(feature = "swift", feature = "kotlin"))]
#[test]
fn a_range_of_usize_is_refused_for_its_width_alone() {
    let messages = refusals(expanded(
        "#[derive(Serialize, Deserialize)] pub struct Text { pub span: Range<usize>, pub value: \
         String }",
    ));
    assert_ne!(messages, Vec::<String>::new());
    assert!(
        messages
            .iter()
            .all(|message| message.contains("`usize` has no")),
        "{messages:?}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_range_is_refused_naming_the_field() {
    let messages = refusals(expanded(
        "#[derive(Serialize, Deserialize)] pub struct Shelf { pub label: String, \
         #[serde(flatten)] pub span: Range<u32> }",
    ));
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].starts_with(
            "model_schema: field `span`: `#[serde(flatten)]` cannot hold a `Range<_>`"
        ),
        "{}",
        messages[0]
    );
}

#[test]
fn a_result_written_by_its_bare_name_is_checked_wherever_it_is_written() {
    for (field, checks) in [
        ("Result<u32, String>", 1),
        ("Option<Result<u32, String>>", 1),
        ("Vec<Result<Range<u32>, String>>", 2),
        ("(Result<u32, String>, Result<(), u8>)", 2),
        ("Result<Result<u32, String>, String>", 2),
        ("core::result::Result<u32, String>", 0),
        ("::std::result::Result<u32, String>", 0),
        ("Result<u32>", 0),
        ("Result", 0),
    ] {
        let item: syn::Item =
            syn::parse_str(&format!("struct Holder {{ outcome: {field} }}")).unwrap();
        assert_eq!(identity_checks(&item).len(), checks, "for {field}");
    }
}

#[test]
fn a_result_is_read_by_its_last_name_with_two_type_arguments() {
    let arms = |written: &str| {
        let syn::Type::Path(path) = syn::parse_str::<syn::Type>(written).unwrap() else {
            return None;
        };
        written_std_result(&path).map(|(ok, err)| {
            (
                quote::ToTokens::to_token_stream(ok).to_string(),
                quote::ToTokens::to_token_stream(err).to_string(),
            )
        })
    };
    assert_eq!(
        arms("Result<u32, String>"),
        Some(("u32".to_owned(), "String".to_owned()))
    );
    assert_eq!(arms("Result<u32>"), None);
    assert_eq!(arms("Result<u32, String, u8>"), None);
    assert_eq!(arms("<T as Outcomes>::Result<u32, String>"), None);
    assert_eq!(claim_published_name("Result", "Result"), None);
    assert_eq!(arms("Result<u32, String>"), None);
    assert_eq!(
        arms("core::result::Result<u32, String>"),
        Some(("u32".to_owned(), "String".to_owned()))
    );
    assert_eq!(
        arms("std::result::Result<(), String>"),
        Some(("()".to_owned(), "String".to_owned()))
    );
}

#[cfg(feature = "typescript")]
#[test]
fn a_result_written_in_full_is_the_std_result_beside_a_declared_one() {
    assert_eq!(claim_published_name("Result", "Result"), None);
    let written = texts(expanded(
        "#[derive(Serialize, Deserialize)] pub struct Shelf { pub outcome: Result<u32, String>, \
         pub settled: core::result::Result<u32, String> }",
    ));
    let holds = |member: &str| written.iter().any(|text| text.contains(member));
    assert!(holds("outcome: Result<number, string>;"), "{written:#?}");
    assert!(
        holds("settled: { Ok: number } | { Err: string };"),
        "{written:#?}"
    );
}

#[test]
fn the_check_hands_the_std_result_back_as_the_name_written() {
    let item: syn::Item = syn::parse_quote! { struct Holder { outcome: Result<u32, String> } };
    let written = identity_checks(&item)
        .into_iter()
        .map(|check| check.to_string())
        .collect::<Vec<_>>();
    let expected: TokenStream = quote::quote! {
        const _: fn(::core::result::Result<(), ()>) -> Result<(), ()> = {
            fn is_std_result<T, E>(held: ::core::result::Result<T, E>) -> Result<T, E> {
                held
            }
            is_std_result::<(), ()>
        };
    };
    assert_eq!(written, vec![expected.to_string()]);
}

#[test]
fn a_result_as_a_map_key_is_refused_naming_the_field() {
    for written in [
        "HashMap<Result<u32, String>, u8>",
        "Vec<BTreeMap<Result<(), String>, u8>>",
    ] {
        let messages = refusals(expanded(&format!(
            "#[derive(Serialize, Deserialize)] pub struct Shelf {{ pub outcomes: {written} }}"
        )));
        assert_eq!(messages.len(), 1, "for {written}: {messages:?}");
        assert!(
            messages[0].starts_with(
                "model_schema: field `outcomes`: a map key must be a value serde writes as a string"
            ),
            "for {written}: {}",
            messages[0]
        );
        assert!(
            messages[0].contains("`Result<_, _>` as a JSON object"),
            "for {written}: {}",
            messages[0]
        );
    }
}

#[test]
fn a_bound_key_on_a_result_field_is_refused_naming_the_field() {
    for key in [
        "pattern = \"^a$\"",
        "minLength = 1",
        "maxLength = 1",
        "minimum = 1",
        "maximum = 1",
    ] {
        let messages = refusals(expanded(&format!(
            "#[derive(Serialize, Deserialize)] pub struct Shelf {{ #[model_schema_prop({key})] \
             pub outcome: Result<u32, String> }}"
        )));
        assert_eq!(messages.len(), 1, "for {key}: {messages:?}");
        assert!(
            messages[0].starts_with("model_schema: field `outcome`: `")
                && messages[0].contains("` cannot apply to a result \u{2014} "),
            "for {key}: {}",
            messages[0]
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_result_is_refused_naming_the_field() {
    let messages = refusals(expanded(
        "#[derive(Serialize, Deserialize)] pub struct Shelf { pub label: String, \
         #[serde(flatten)] pub outcome: Result<u32, String> }",
    ));
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].starts_with(
            "model_schema: field `outcome`: `#[serde(flatten)]` cannot hold a `Result<_, _>`"
        ),
        "{}",
        messages[0]
    );
}
