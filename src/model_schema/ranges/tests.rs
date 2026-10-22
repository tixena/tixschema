//! The identity check of a range or a `Result` written by its bare name, and the refusals each
//! earns as a map key, under a bound key and under `#[serde(flatten)]`, in every build.

use proc_macro2::{TokenStream, TokenTree};

use super::identity_checks;
use crate::field_type::{RangeKind, written_std_range, written_std_result};
use crate::utils::claim_published_name;

/// What a type that builds is refused with.
#[cfg(feature = "serde")]
const NONE: [&str; 0] = [];

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

/// The refusals `#[model_schema()]` writes for `source`, a struct or an enum deriving both of
/// serde's traits.
#[cfg(feature = "serde")]
fn refused(source: &str) -> Vec<String> {
    refusals(expanded(&format!(
        "#[derive(Serialize, Deserialize)] {source}"
    )))
}

#[cfg(feature = "serde")]
#[test]
fn a_map_flattened_beside_a_member_is_refused_naming_both() {
    let messages = refused(
        "pub struct Stocktake { #[serde(flatten)] pub counts: BTreeMap<String, u8>, \
         pub note: String }",
    );
    assert_eq!(
        messages,
        vec![
            "model_schema: type `Stocktake`: field `counts`: `#[serde(flatten)]` cannot hold a map \
             beside other members \u{2014} serde writes the map's entries at one level with \
             `note`, and no TypeScript type describes both. Write the map under a key of its own."
                .to_owned()
        ]
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_map_flattened_beside_a_flattened_struct_another_map_or_through_an_option_is_refused() {
    let _: Vec<String> = refused("pub struct ShelfMark { pub label: String }");
    for (source, refused_fields) in [
        (
            "pub struct Tagged { #[serde(flatten)] pub marking: ShelfMark, \
             #[serde(flatten)] pub spare: BTreeMap<String, u8> }",
            vec!["spare"],
        ),
        (
            "pub struct Twice { #[serde(flatten)] pub a: BTreeMap<String, u8>, \
             #[serde(flatten)] pub b: BTreeMap<String, String> }",
            vec!["a", "b"],
        ),
        (
            "pub struct Maybe { #[serde(flatten)] pub rest: Option<BTreeMap<String, u8>>, \
             pub note: String }",
            vec!["rest"],
        ),
        (
            "pub struct Boxed { pub note: String, \
             #[serde(flatten)] pub rest: Box<HashMap<String, u8>> }",
            vec!["rest"],
        ),
    ] {
        let messages = refused(source);
        assert_eq!(
            messages.len(),
            refused_fields.len(),
            "{source}: {messages:?}"
        );
        for (message, field) in messages.iter().zip(refused_fields) {
            assert!(
                message.contains(&format!(
                    ": field `{field}`: `#[serde(flatten)]` cannot hold a map beside other members"
                )),
                "{source}: {message}"
            );
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_map_flattened_alone_builds() {
    for source in [
        "pub struct Counted { #[serde(flatten)] pub counts: BTreeMap<String, u8> }",
        "pub struct Cached { #[serde(flatten)] pub counts: BTreeMap<String, u8>, \
         #[serde(skip)] pub cache: u32 }",
        "pub struct ShelfLabel { #[serde(flatten)] pub marking: ShelfMark, pub note: String }",
        "pub enum Counting { Tally { #[serde(flatten)] counts: BTreeMap<String, u8> } }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
}

/// The README quotes the refusal as the build answers it: where `#[serde(flatten)]` of a map is
/// described, and in the list of what an upgrade asks of a crate.
#[cfg(feature = "serde")]
#[test]
fn the_readme_states_the_refusal_of_a_map_beside_a_member_as_the_build_answers_it() {
    let messages = refused(
        "pub struct Stocktake { #[serde(flatten)] pub counts: BTreeMap<String, u8>, \
         pub note: String }",
    );
    assert_eq!(messages.len(), 1, "{messages:?}");
    let answered = format!("error: {}", messages[0]);
    assert_eq!(
        include_str!("../../../README.md")
            .matches(&answered)
            .count(),
        2,
        "the README no longer quotes: {answered}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_declared_above_that_writes_a_map_is_refused_flattened_beside_a_member() {
    let _: Vec<String> = refused("pub struct ShelfMark { pub label: String }");
    assert_eq!(
        refused("pub struct Spare { #[serde(flatten)] pub rest: BTreeMap<String, u8> }"),
        NONE
    );
    assert_eq!(
        refusals(expanded("type SpareCounts = BTreeMap<String, u8>;")),
        NONE
    );
    assert_eq!(
        refused("#[serde(transparent)] pub struct Wrapped(pub BTreeMap<String, u8>);"),
        NONE
    );
    assert_eq!(
        refused("pub struct Slotted(pub Option<HashMap<String, u8>>);"),
        NONE
    );
    for name in ["Spare", "SpareCounts", "Wrapped", "Slotted"] {
        let messages = refused(&format!(
            "pub struct Shelved {{ #[serde(flatten)] pub marking: ShelfMark, \
             #[serde(flatten)] pub spare: {name} }}"
        ));
        assert_eq!(messages.len(), 1, "{name}: {messages:?}");
        assert!(
            messages[0].starts_with(
                "model_schema: type `Shelved`: field `spare`: `#[serde(flatten)]` cannot hold a \
                 map beside other members \u{2014} serde writes the map's entries at one level \
                 with `marking`"
            ),
            "{name}: {}",
            messages[0]
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_type_declared_below_its_holder_is_not_looked_into() {
    let _: Vec<String> = refused("pub struct ShelfMark { pub label: String }");
    assert_eq!(
        refused(
            "pub struct Unseen { #[serde(flatten)] pub marking: ShelfMark, \
             #[serde(flatten)] pub spare: SpareBelow }"
        ),
        NONE
    );
    assert_eq!(
        refused("pub struct SpareBelow { #[serde(flatten)] pub rest: BTreeMap<String, u8> }"),
        NONE
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_variant_that_flattens_a_map_beside_a_member_is_refused_under_each_tagging() {
    for tagging in [
        "",
        "#[serde(tag = \"kind\")]",
        "#[serde(tag = \"kind\", content = \"held\")]",
        "#[serde(untagged)]",
    ] {
        let messages = refused(&format!(
            "{tagging} pub enum Berth {{ Open {{ #[serde(flatten)] counts: BTreeMap<String, u8>, \
             note: String }}, Closed {{ note: String }} }}"
        ));
        assert_eq!(
            messages,
            vec![
                "model_schema: type `Berth`: variant `Open`, field `counts`: `#[serde(flatten)]` \
                 cannot hold a map beside other members \u{2014} serde writes the map's entries \
                 at one level with `note`, and no TypeScript type describes both. Write the map \
                 under a key of its own."
                    .to_owned()
            ],
            "{tagging}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_map_flattened_alone_beside_a_tag_is_refused_naming_the_tag() {
    assert_eq!(
        refused(
            "#[serde(tag = \"kind\")] pub enum Keelson { Open { #[serde(flatten)] \
             counts: BTreeMap<String, u8> } }"
        ),
        vec![
            "model_schema: type `Keelson`: variant `Open`, field `counts`: `#[serde(flatten)]` \
             cannot hold a map beside the tag `kind` \u{2014} serde writes the tag and the map's \
             entries at one level, and no TypeScript type describes both. Write the map under a \
             key of its own."
                .to_owned()
        ]
    );
    assert_eq!(
        refused(
            "#[serde(tag = \"kind\")] pub struct Hull { #[serde(flatten)] \
             counts: BTreeMap<String, u8> }"
        ),
        vec![
            "model_schema: type `Hull`: field `counts`: `#[serde(flatten)]` cannot hold a map \
             beside the tag `kind` \u{2014} serde writes the tag and the map's entries at one \
             level, and no TypeScript type describes both. Write the map under a key of its own."
                .to_owned()
        ]
    );
}

/// The README quotes the refusal of a map beside a tag as the build answers it, as it quotes the
/// refusal of a map beside a member.
#[cfg(feature = "serde")]
#[test]
fn the_readme_states_the_refusal_of_a_map_beside_a_tag_as_the_build_answers_it() {
    let messages = refused(
        "#[serde(tag = \"kind\")] pub enum Keelson { Open { #[serde(flatten)] \
         counts: BTreeMap<String, u8> } }",
    );
    assert_eq!(messages.len(), 1, "{messages:?}");
    let answered = format!("error: {}", messages[0]);
    assert_eq!(
        include_str!("../../../README.md")
            .matches(&answered)
            .count(),
        2,
        "the README no longer quotes: {answered}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_map_flattened_alone_beside_a_tag_is_refused_whatever_it_holds() {
    assert_eq!(
        refused("pub struct Spare { #[serde(flatten)] pub rest: BTreeMap<String, u8> }"),
        NONE
    );
    for (source, refused_field) in [
        (
            "#[serde(tag = \"kind\")] pub enum Keelson { Open { #[serde(flatten)] \
             counts: Option<BTreeMap<String, u8>> } }",
            "variant `Open`, field `counts`",
        ),
        (
            "#[serde(tag = \"kind\")] pub enum Keelson { Open { #[serde(flatten)] \
             spare: Spare } }",
            "variant `Open`, field `spare`",
        ),
        (
            "#[serde(tag = \"kind\")] pub struct Label { #[serde(flatten)] \
             words: BTreeMap<String, String> }",
            "field `words`",
        ),
        (
            "#[serde(tag = \"kind\")] pub struct Hull { #[serde(flatten)] \
             counts: BTreeMap<String, u8>, #[serde(skip)] cache: u32 }",
            "field `counts`",
        ),
    ] {
        let messages = refused(source);
        assert_eq!(messages.len(), 1, "{source}: {messages:?}");
        assert!(
            messages[0].contains(&format!(
                ": {refused_field}: `#[serde(flatten)]` cannot hold a map beside the tag `kind`"
            )),
            "{source}: {}",
            messages[0]
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_map_flattened_alone_where_serde_writes_no_tag_beside_it_builds() {
    let _: Vec<String> = refused("pub struct ShelfMark { pub label: String }");
    for source in [
        "#[serde(tag = \"kind\", content = \"held\")] pub enum Transom { Open { \
         #[serde(flatten)] counts: BTreeMap<String, u8> } }",
        "pub enum Coaming { Open { #[serde(flatten)] counts: BTreeMap<String, u8> } }",
        "#[serde(untagged)] pub enum Bilge { Open { #[serde(flatten)] \
         counts: BTreeMap<String, u8> } }",
        "pub struct Thwart { #[serde(flatten)] pub counts: BTreeMap<String, u8> }",
        "#[serde(tag = \"kind\")] pub struct Marked { #[serde(flatten)] pub marking: ShelfMark }",
        "#[serde(tag = \"kind\")] pub enum Marking { Open { #[serde(flatten)] \
         marking: ShelfMark } }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
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
