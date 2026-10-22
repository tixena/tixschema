//! The identity check of a range or a `Result` written by its bare name, and the refusals each
//! earns as a map key, under a bound key and under `#[serde(flatten)]`, in every build.

use proc_macro2::{TokenStream, TokenTree};

use super::identity_checks;
use crate::field_type::{RangeKind, written_std_range, written_std_result};
use crate::utils::claim_published_name;

/// What a type that builds is refused with.
#[cfg(feature = "serde")]
const NONE: [&str; 0] = [];

/// Each filling of a type parameter flattened or held beside a tag that serde writes as no object,
/// or the lone slot's as an `Option`, in a member of `Manifest`: the type written, the type that
/// flattens the parameter, the parameter, the key beside it, the filling, and what serde writes it
/// as.
#[cfg(feature = "serde")]
const NON_OBJECT_FILLINGS: [(&str, &str, &str, &str, &str, WrittenAsNoObject); 28] = [
    (
        "Bag<String>",
        "Bag",
        "T",
        MEMBER,
        "String",
        merged("string"),
    ),
    ("Bag<u32>", "Bag", "T", MEMBER, "u32", merged("integer")),
    ("Bag<f64>", "Bag", "T", MEMBER, "f64", merged("number")),
    ("Bag<bool>", "Bag", "T", MEMBER, "bool", merged("boolean")),
    ("Bag<char>", "Bag", "T", MEMBER, "char", merged("string")),
    (
        "Bag<Vec<u8>>",
        "Bag",
        "T",
        MEMBER,
        "Vec<u8>",
        merged("array"),
    ),
    (
        "Bag<(u8, u8)>",
        "Bag",
        "T",
        MEMBER,
        "(u8, u8)",
        merged("array"),
    ),
    (
        "Bag<()>",
        "Bag",
        "T",
        MEMBER,
        "()",
        WrittenAsNoObject::Unit { beside_tag: false },
    ),
    (
        "Bag<Option<String>>",
        "Bag",
        "T",
        MEMBER,
        "Option<String>",
        merged("string"),
    ),
    ("Bag<Code>", "Bag", "T", MEMBER, "Code", merged("string")),
    ("Bag<Tally>", "Bag", "T", MEMBER, "Tally", merged("integer")),
    (
        "Bag<CodeAlias>",
        "Bag",
        "T",
        MEMBER,
        "CodeAlias",
        merged("string"),
    ),
    (
        "Bag<TextOrCrate>",
        "Bag",
        "T",
        MEMBER,
        "TextOrCrate",
        WrittenAsNoObject::TextMember { beside_tag: false },
    ),
    (
        "Vec<Bag<String>>",
        "Bag",
        "T",
        MEMBER,
        "String",
        merged("string"),
    ),
    ("Wrap<String>", "Wrap", "T", "", "String", merged("string")),
    (
        "Tagged<String>",
        "Tagged",
        "T",
        TAG,
        "String",
        merged("string"),
    ),
    (
        "Waybill<String>",
        "Waybill",
        "Held",
        TAG,
        "String",
        beside_kind("string"),
    ),
    (
        "Waybill<Vec<u8>>",
        "Waybill",
        "Held",
        TAG,
        "Vec<u8>",
        beside_kind("array"),
    ),
    (
        "Waybill<Code>",
        "Waybill",
        "Held",
        TAG,
        "Code",
        beside_kind("string"),
    ),
    (
        "Waybill<()>",
        "Waybill",
        "Held",
        TAG,
        "()",
        WrittenAsNoObject::Unit { beside_tag: true },
    ),
    (
        "Waybill<TextOrCrate>",
        "Waybill",
        "Held",
        TAG,
        "TextOrCrate",
        WrittenAsNoObject::TextMember { beside_tag: true },
    ),
    (
        "Waybill<Option<Pallet>>",
        "Waybill",
        "Held",
        TAG,
        "Option<Pallet>",
        WrittenAsNoObject::Optional,
    ),
    (
        "Waybill<Option<Crate>>",
        "Waybill",
        "Held",
        TAG,
        "Option<Crate>",
        WrittenAsNoObject::Optional,
    ),
    (
        "Waybill<MaybeCrate>",
        "Waybill",
        "Held",
        TAG,
        "MaybeCrate",
        WrittenAsNoObject::Optional,
    ),
    (
        "Waybill<CrateMaybe>",
        "Waybill",
        "Held",
        TAG,
        "CrateMaybe",
        WrittenAsNoObject::Optional,
    ),
    (
        "Outer<String>",
        "Waybill",
        "Held",
        TAG,
        "String",
        beside_kind("string"),
    ),
    (
        "Outer<Option<Crate>>",
        "Waybill",
        "Held",
        TAG,
        "Option<Crate>",
        WrittenAsNoObject::Optional,
    ),
    (
        "Outer<MaybeCrate>",
        "Waybill",
        "Held",
        TAG,
        "MaybeCrate",
        WrittenAsNoObject::Optional,
    ),
];

/// What a refusal says of a parameter flattened beside `id`.
#[cfg(feature = "serde")]
const MEMBER: &str = " beside `id`";

/// What a refusal says of a parameter flattened or held beside the tag `kind`.
#[cfg(feature = "serde")]
const TAG: &str = " beside the tag `kind`";

/// What serde writes a value filling a type parameter as, where the build refuses it.
#[cfg(feature = "serde")]
#[derive(Clone, Copy)]
enum WrittenAsNoObject {
    /// An `Option`, held by the lone slot of the variant `Held` of `Waybill`.
    Optional,
    /// A union whose first member is text, flattened or beside the tag `kind`.
    TextMember { beside_tag: bool },
    /// `()`, flattened beside members of `Bag` or beside the tag `kind`.
    Unit { beside_tag: bool },
    /// A value its schema describes by a JSON keyword, flattened or beside the tag `kind`.
    Whole {
        beside_tag: bool,
        keyword: &'static str,
    },
}

#[cfg(feature = "serde")]
impl WrittenAsNoObject {
    /// What the refusal says of a value serde writes so.
    fn words(self) -> String {
        let beside_kind = "which cannot sit beside the tag: `#[serde(tag = \"kind\")]` with no \
                           `content` writes the variant's data as members of the object the tag \
                           is written in, and only a value serde writes as an object has members \
                           to put there.";
        let merged = "which has no members to merge, and what serde writes for it does not join \
                      the object being written.";
        match self {
            Self::Optional => format!(
                "a value serde writes as an optional, {beside_kind} serde refuses every write of \
                 it at run time \u{2014} `cannot serialize tagged newtype variant Waybill::Held \
                 containing an optional`."
            ),
            Self::TextMember { beside_tag: true } => format!(
                "a value that writes a union member that is not an object \u{2014} its branch 1 \
                 describes a `string`, {beside_kind}"
            ),
            Self::TextMember { beside_tag: false } => "a value that writes a union member that \
                 is not an object \u{2014} its branch 1 describes a `string`, which has no \
                 members to merge, and what serde writes for that member does not join the \
                 object being written."
                .to_owned(),
            Self::Unit { beside_tag: true } => "a value serde writes as `()` \u{2014} its schema \
                 describes a `null`: beside the tag serde writes the variant as the tag alone, \
                 and reads it back, while the TypeScript type and the Zod schema of the variant \
                 intersect the tag's object with `null`, which admits no value."
                .to_owned(),
            Self::Unit { beside_tag: false } => "a value serde writes as `()` \u{2014} its schema \
                 describes a `null`: serde writes no key for it, and reads the object back, while \
                 the TypeScript type and the Zod schema of `Bag` intersect the object with \
                 `null`, which admits no value."
                .to_owned(),
            Self::Whole {
                beside_tag,
                keyword,
            } => format!(
                "a value that is not written as an object \u{2014} its schema describes a \
                 `{keyword}`, {}",
                if beside_tag { beside_kind } else { merged }
            ),
        }
    }
}

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

/// The refusal of a map flattened beside `id` in the field `field` of `holder`.
#[cfg(feature = "serde")]
fn beside_id(holder: &str, field: &str) -> String {
    format!(
        "model_schema: type `{holder}`: field `{field}`: `#[serde(flatten)]` cannot hold a map \
         beside other members \u{2014} serde writes the map's entries at one level with `id`, and \
         no TypeScript type describes both. Write the map under a key of its own."
    )
}

/// The refusal of `filling` filling the type parameter `T` that `Letter` flattens beside `id`, in
/// the place `subject` names.
#[cfg(feature = "serde")]
fn filling_letter(subject: &str, filling: &str) -> String {
    format!(
        "model_schema: {subject}: `Letter` flattens its type parameter `T` beside `id`, and \
         `{filling}` fills it with a map \u{2014} serde writes the map's entries at one level \
         with `id`, and no TypeScript type describes both. Write the map under a key of its own."
    )
}

/// What `#[model_schema(default_types(T = ...))]` refuses `source` with, `filling` being what `T`
/// is declared at.
#[cfg(feature = "serde")]
fn refused_with_default(filling: &str, source: &str) -> Vec<String> {
    refusals(super::super::exec_model_schema(
        syn::parse_str(&format!("default_types(T = {filling})")).unwrap(),
        syn::parse_str(&format!("#[derive(Serialize, Deserialize)] {source}")).unwrap(),
    ))
}

/// What `#[model_schema(default_types(parameter = String))]` refuses `source` with, written as it
/// is.
#[cfg(feature = "serde")]
fn generic(parameter: &str, source: &str) -> Vec<String> {
    generic_at(parameter, "String", source)
}

/// What `#[model_schema(default_types(parameter = filling))]` refuses `source` with, written as it
/// is.
#[cfg(feature = "serde")]
fn generic_at(parameter: &str, filling: &str, source: &str) -> Vec<String> {
    refusals(super::super::exec_model_schema(
        syn::parse_str(&format!("default_types({parameter} = {filling})")).unwrap(),
        syn::parse_str(source).unwrap(),
    ))
}

/// What a generic `source` deriving both of serde's traits is refused with, its one type parameter
/// `T` declared at `String`.
#[cfg(feature = "serde")]
fn refused_generic(source: &str) -> Vec<String> {
    generic("T", &format!("#[derive(Serialize, Deserialize)] {source}"))
}

/// What a generic `source` deriving both of serde's traits, which flattens its one type parameter
/// `T` or holds it beside a tag, is refused with, `T` declared at `Plain`, a struct.
#[cfg(feature = "serde")]
fn refused_flattening(source: &str) -> Vec<String> {
    refused_with_default("Plain", source)
}

#[cfg(feature = "serde")]
#[test]
fn an_untagged_enum_with_a_map_among_its_variants_is_refused_flattened_beside_a_member() {
    let _: Vec<String> = refused("pub struct Captioned { pub name: String }");
    for scores in [
        "#[serde(untagged)] pub enum Scores { Captioned(Captioned), Tallied(HashMap<String, i32>) }",
        "#[serde(untagged)] pub enum Scores { Captioned(Captioned), \
         Tallied { #[serde(flatten)] counts: HashMap<String, i32> } }",
    ] {
        assert_eq!(refused(scores), NONE, "{scores}");
        assert_eq!(
            refused(
                "pub struct Scoreboard { pub id: String, #[serde(flatten)] pub scores: Scores }"
            ),
            vec![beside_id("Scoreboard", "scores")],
            "{scores}"
        );
        assert_eq!(
            refused("pub struct Scored { #[serde(flatten)] pub scores: Scores }"),
            NONE,
            "{scores}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn an_enum_that_writes_no_map_as_its_own_entries_builds_flattened_beside_a_member() {
    let _: Vec<String> = refused("pub struct Captioned { pub name: String }");
    let _: Vec<String> = refused("pub struct Counted { pub total: u32 }");
    for (name, enum_source) in [
        (
            "Plainly",
            "#[serde(untagged)] pub enum Plainly { Captioned(Captioned), Counted(Counted) }",
        ),
        (
            "Named",
            "pub enum Named { Captioned(Captioned), Tallied(HashMap<String, i32>) }",
        ),
        (
            "Adjacent",
            "#[serde(tag = \"kind\", content = \"held\")] pub enum Adjacent { \
             Captioned(Captioned), Tallied(HashMap<String, i32>) }",
        ),
    ] {
        assert_eq!(refused(enum_source), NONE, "{name}");
        assert_eq!(
            refused(&format!(
                "pub struct NamedBoard {{ pub id: String, #[serde(flatten)] pub scores: {name} }}"
            )),
            NONE,
            "{name}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_generic_type_written_as_its_argument_is_refused_flattened_beside_a_member_at_a_map() {
    let _: Vec<String> = refused("pub struct Plain { pub text: String }");
    let _: Vec<String> = refused("pub struct Captioned { pub name: String }");
    assert_eq!(
        refused_generic("#[serde(transparent)] pub struct Wrap<T>(T);"),
        NONE
    );
    assert_eq!(generic("T", "type Held<T> = T;"), NONE);
    assert_eq!(
        refused_generic("#[serde(untagged)] pub enum Either<T> { Captioned(Captioned), Other(T) }"),
        NONE
    );
    assert_eq!(
        refused_flattening(
            "pub struct Loose<T> { #[serde(flatten, skip_serializing_if = \"Option::is_none\")] \
             pub body: Option<T> }"
        ),
        NONE
    );
    for flattened in [
        "Wrap<HashMap<String, i32>>",
        "Held<HashMap<String, i32>>",
        "Either<HashMap<String, i32>>",
        "Loose<HashMap<String, i32>>",
        "Option<Wrap<Box<HashMap<String, i32>>>>",
        "Wrap<Box<Option<HashMap<String, i32>>>>",
        "Wrap<Held<HashMap<String, i32>>>",
    ] {
        assert_eq!(
            refused(&format!(
                "pub struct Crate {{ #[serde(flatten)] pub body: {flattened}, pub id: String }}"
            )),
            vec![beside_id("Crate", "body")],
            "{flattened}"
        );
    }
    for flattened in [
        "Wrap<Plain>",
        "Held<Plain>",
        "Either<Plain>",
        "Loose<Plain>",
    ] {
        assert_eq!(
            refused(&format!(
                "pub struct Crate {{ #[serde(flatten)] pub body: {flattened}, pub id: String }}"
            )),
            NONE,
            "{flattened}"
        );
    }
    assert_eq!(
        refused("pub struct Basket { #[serde(flatten)] pub body: Wrap<HashMap<String, i32>> }"),
        NONE
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_flattened_beside_a_member_is_refused_where_a_map_fills_it() {
    let _: Vec<String> = refused("pub struct Plain { pub text: String }");
    assert_eq!(
        refused_flattening(
            "pub struct Letter<T> { #[serde(flatten)] pub body: T, pub id: String }"
        ),
        NONE
    );
    for (letter, filling) in [
        ("Letter<HashMap<String, i32>>", "HashMap<String, i32>"),
        ("Vec<Letter<HashMap<String, i32>>>", "HashMap<String, i32>"),
        (
            "Option<Letter<BTreeMap<String, u8>>>",
            "BTreeMap<String, u8>",
        ),
        (
            "HashMap<String, Letter<Box<HashMap<String, i32>>>>",
            "Box<HashMap<String, i32>>",
        ),
    ] {
        assert_eq!(
            refused(&format!("pub struct Post {{ pub letter: {letter} }}")),
            vec![filling_letter("type `Post`: field `letter`", filling)],
            "{letter}"
        );
    }
    assert_eq!(
        refused("pub struct Post { pub letter: Letter<Plain> }"),
        NONE
    );
    assert_eq!(
        generic_at(
            "T",
            "Plain",
            "#[derive(Serialize, Deserialize)] pub enum Envelope<T> { \
             Sealed { #[serde(flatten)] body: T, id: String }, \
             Open { #[serde(flatten)] body: T, note: String } }"
        ),
        NONE
    );
    assert_eq!(
        refused("pub struct Mailed { pub envelope: Envelope<HashMap<String, i32>> }").len(),
        1
    );
    assert_eq!(
        refused("pub enum Posted { Sent { letter: (u8, Letter<HashMap<String, i32>>) } }"),
        vec![filling_letter(
            "type `Posted`: variant `Sent`, field `letter`",
            "HashMap<String, i32>"
        )]
    );
    assert_eq!(
        refusals(expanded("type Mailbag = Letter<HashMap<String, i32>>;")),
        vec![filling_letter("type `Mailbag`", "HashMap<String, i32>")]
    );
    assert_eq!(
        refused(
            "pub struct Skipped { #[serde(skip)] pub letter: Option<Letter<HashMap<String, i32>>> }"
        ),
        NONE
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_flattened_beside_a_member_is_refused_at_a_default_that_is_a_map() {
    assert_eq!(
        refused_with_default(
            "HashMap<String, i32>",
            "pub struct Letter<T> { #[serde(flatten)] pub body: T, pub id: String }"
        ),
        vec![filling_letter(
            "type `Letter`: `default_types` entry `T`",
            "HashMap<String, i32>"
        )]
    );
    assert_eq!(
        refused_with_default(
            "Plain",
            "pub struct Letter<T> { #[serde(flatten)] pub body: T, pub id: String }"
        ),
        NONE
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_parameter_handed_on_to_a_type_that_flattens_it_is_refused_where_a_map_fills_it() {
    let _: Vec<String> = refused("pub struct Plain { pub text: String }");
    let _: Vec<String> = refused_generic("#[serde(transparent)] pub struct Wrap<T>(T);");
    assert_eq!(
        refused_flattening(
            "pub struct Letter<T> { #[serde(flatten)] pub body: T, pub id: String }"
        ),
        NONE
    );
    assert_eq!(
        generic_at(
            "U",
            "Plain",
            "#[derive(Serialize, Deserialize)] pub struct Bag<U> { pub letters: \
             Vec<Letter<Box<Option<U>>>> }"
        ),
        NONE
    );
    assert_eq!(
        refused("pub struct Sack { pub bag: Bag<HashMap<String, i32>> }"),
        vec![filling_letter(
            "type `Sack`: field `bag`",
            "HashMap<String, i32>"
        )]
    );
    assert_eq!(refused("pub struct Sack { pub bag: Bag<Plain> }"), NONE);
    assert_eq!(
        refused("pub struct Post { pub letter: Letter<Wrap<HashMap<String, i32>>> }"),
        vec![filling_letter(
            "type `Post`: field `letter`",
            "Wrap<HashMap<String, i32>>"
        )]
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_flattened_beside_a_tag_is_refused_where_a_map_fills_it() {
    assert_eq!(
        refused_flattening(
            "#[serde(tag = \"kind\")] pub struct Hull<T> { #[serde(flatten)] pub body: T }"
        ),
        NONE
    );
    let beside_kind = |subject: &str| {
        format!(
            "model_schema: type `{subject}`: `Hull` flattens its type parameter `T` beside the \
             tag `kind`, and `HashMap<String, i32>` fills it with a map \u{2014} serde writes \
             the tag and the map's entries at one level, and no TypeScript type describes both. \
             Write the map under a key of its own."
        )
    };
    assert_eq!(
        refused("pub struct Dock { pub hull: Hull<HashMap<String, i32>> }"),
        vec![beside_kind("Dock`: field `hull")]
    );
    assert_eq!(
        refused(
            "pub struct Slip { #[serde(flatten)] pub hull: Hull<HashMap<String, i32>>, \
             pub id: String }"
        ),
        vec![beside_kind("Slip`: field `hull")]
    );
}

/// The README quotes each refusal of a map that reaches a flattened member through an untagged
/// enum, a generic brand or a type parameter as the build answers it: the type parameter's in the
/// list of what an upgrade asks of a crate as well.
#[cfg(feature = "serde")]
#[test]
fn the_readme_states_the_refusals_of_a_map_reached_another_way_as_the_build_answers_them() {
    let _: Vec<String> = refused("pub struct Captioned { pub name: String }");
    let _: Vec<String> = refused(
        "#[serde(untagged)] pub enum Scores { Captioned(Captioned), \
         Tallied(HashMap<String, i32>) }",
    );
    let _: Vec<String> = refused_generic("#[serde(transparent)] pub struct Wrap<T>(T);");
    let _: Vec<String> = refused_flattening(
        "pub struct Letter<T> { #[serde(flatten)] pub body: T, pub id: String }",
    );
    for (source, quoted) in [
        (
            "pub struct Scoreboard { pub id: String, #[serde(flatten)] pub scores: Scores }",
            1,
        ),
        (
            "pub struct Crate { #[serde(flatten)] pub body: Wrap<HashMap<String, i32>>, \
             pub id: String }",
            1,
        ),
        (
            "pub struct Post { pub letter: Letter<HashMap<String, i32>> }",
            2,
        ),
    ] {
        let messages = refused(source);
        assert_eq!(messages.len(), 1, "{source}: {messages:?}");
        let answered = format!("error: {}", messages[0]);
        assert_eq!(
            include_str!("../../../README.md")
                .matches(&answered)
                .count(),
            quoted,
            "the README no longer quotes: {answered}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_generic_type_declared_below_its_filling_is_not_looked_into() {
    assert_eq!(
        refused("pub struct Post { pub letter: LetterBelow<HashMap<String, i32>> }"),
        NONE
    );
    assert_eq!(
        refused(
            "pub struct Crate { #[serde(flatten)] pub body: WrapBelow<HashMap<String, i32>>, \
             pub id: String }"
        ),
        NONE
    );
    assert_eq!(
        refused_flattening(
            "pub struct LetterBelow<T> { #[serde(flatten)] pub body: T, pub id: String }"
        ),
        NONE
    );
    assert_eq!(
        refused_generic("#[serde(transparent)] pub struct WrapBelow<T>(T);"),
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

/// The refusal of the lone slot of the variant `Counted` of `holder`, written `slot`, under the tag
/// `kind`.
#[cfg(feature = "serde")]
fn slot_beside_kind(holder: &str, slot: &str) -> String {
    format!(
        "model_schema: type `{holder}`: variant `Counted`, field `0`: `{slot}` writes a map, which \
         cannot sit beside the tag `kind` \u{2014} serde writes the tag and the map's entries at \
         one level, and no TypeScript type describes both. Name a `content` key so the map gets an \
         object of its own, or write the map under a key of its own."
    )
}

#[cfg(feature = "serde")]
#[test]
fn a_lone_slot_that_writes_a_map_is_refused_under_a_tag_alone() {
    assert_eq!(
        refused("pub struct Tally { #[serde(flatten)] counts: BTreeMap<String, u8> }"),
        NONE
    );
    assert_eq!(
        refusals(expanded("type TallyCounts = BTreeMap<String, u8>;")),
        NONE
    );
    assert_eq!(
        refused("#[serde(transparent)] pub struct TallyBrand(pub BTreeMap<String, u8>);"),
        NONE
    );
    assert_eq!(
        refused("pub struct TallySlot(pub HashMap<String, u8>);"),
        NONE
    );
    assert_eq!(
        refused(
            "#[serde(untagged)] pub enum TallyEither { Named { name: String }, \
             Counted(HashMap<String, u8>) }"
        ),
        NONE
    );
    assert_eq!(
        refused_generic("#[serde(transparent)] pub struct TallyWrap<T>(T);"),
        NONE
    );
    for (holder, slot) in [
        ("Filled", "Tally"),
        ("Aliased", "TallyCounts"),
        ("Branded", "TallyBrand"),
        ("Slotted", "TallySlot"),
        ("Either", "TallyEither"),
        ("Boxed", "Box<Tally>"),
        ("Wrapped", "TallyWrap<HashMap<String, i32>>"),
        ("WrappedTally", "TallyWrap<Tally>"),
    ] {
        assert_eq!(
            refused(&format!(
                "#[serde(tag = \"kind\")] pub enum {holder} {{ Counted({slot}), Empty }}"
            )),
            vec![slot_beside_kind(holder, slot)],
            "{slot}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_lone_slot_that_writes_a_map_builds_where_serde_writes_no_tag_beside_it() {
    assert_eq!(
        refused("pub struct Tally { #[serde(flatten)] counts: BTreeMap<String, u8> }"),
        NONE
    );
    assert_eq!(refused("pub struct ShelfMark { pub label: String }"), NONE);
    for source in [
        "pub enum Named { Counted(Tally), Empty }",
        "#[serde(tag = \"kind\", content = \"held\")] pub enum Adjacent { Counted(Tally), Empty }",
        "#[serde(untagged)] pub enum Loose { Counted(Tally), Named { name: String } }",
        "#[serde(tag = \"kind\")] pub enum Keyed { Counted(ShelfMark), Empty }",
        "#[serde(tag = \"kind\")] pub enum Unwritten { Counted(#[serde(skip)] Tally), Empty }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_lone_slot_declared_below_its_enum_is_not_looked_into() {
    assert_eq!(
        refused("#[serde(tag = \"kind\")] pub enum Filled { Counted(TallyBelow), Empty }"),
        NONE
    );
    assert_eq!(
        refused("pub struct TallyBelow { #[serde(flatten)] counts: BTreeMap<String, u8> }"),
        NONE
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_in_a_lone_slot_under_a_tag_alone_is_refused_where_a_map_fills_it() {
    let _: Vec<String> = refused("pub struct Plain { pub text: String }");
    assert_eq!(
        refused_generic("#[serde(transparent)] pub struct TallyWrap<T>(T);"),
        NONE
    );
    assert_eq!(
        refused_flattening("#[serde(tag = \"kind\")] pub enum Hold<T> { Counted(TallyWrap<T>) }"),
        NONE
    );
    assert_eq!(
        refused_flattening("#[serde(tag = \"kind\")] pub enum Bare<T> { Counted(T) }"),
        NONE
    );
    let beside_kind = |subject: &str, flattens: &str| {
        format!(
            "model_schema: type `{subject}`: `{flattens}` flattens its type parameter `T` beside \
             the tag `kind`, and `HashMap<String, i32>` fills it with a map \u{2014} serde writes \
             the tag and the map's entries at one level, and no TypeScript type describes both. \
             Write the map under a key of its own."
        )
    };
    assert_eq!(
        refused("pub struct Dock { pub hold: Hold<HashMap<String, i32>> }"),
        vec![beside_kind("Dock`: field `hold", "Hold")]
    );
    assert_eq!(
        refused("pub struct Pier { pub bare: Option<Bare<HashMap<String, i32>>> }"),
        vec![beside_kind("Pier`: field `bare", "Bare")]
    );
    assert_eq!(
        refused_with_default(
            "HashMap<String, i32>",
            "#[serde(tag = \"kind\")] pub enum Stowed<T> { Counted(TallyWrap<T>) }"
        ),
        vec![beside_kind("Stowed`: `default_types` entry `T", "Stowed")]
    );
    assert_eq!(
        refused("pub struct Quay { pub hold: Hold<Plain>, pub bare: Bare<Plain> }"),
        NONE
    );
    assert_eq!(
        refused_generic(
            "#[serde(tag = \"kind\", content = \"held\")] pub enum Crated<T> { \
             Counted(TallyWrap<T>) }"
        ),
        NONE
    );
    assert_eq!(
        refused("pub struct Wharf { pub crated: Crated<HashMap<String, i32>> }"),
        NONE
    );
}

/// The refusal of `filling` filling the type parameter `parameter` that `flattens` flattens
/// `beside` a key, in the place `subject` names, `named` where the filling is an externally tagged
/// enum the advice names.
#[cfg(feature = "serde")]
fn filled_entry(
    subject: &str,
    flattens: &str,
    parameter: &str,
    beside: &str,
    filling: &str,
    named: bool,
) -> String {
    let advice = if named {
        "Name the enum where the type parameter stands, which every surface describes by its \
         entries, or write it under a key of its own."
    } else {
        "Write it under a key of its own."
    };
    format!(
        "model_schema: {subject}: `{flattens}` flattens its type parameter `{parameter}`{beside}, \
         and `{filling}` fills it with a value serde writes a variant of there as an entry of its \
         name over null \u{2014} the TypeScript type and the Zod schema of `{flattens}`, written \
         once for every filling, describe that variant as its bare name. {advice}"
    )
}

/// Declares the types the fillings of a flattened type parameter are read against: an externally
/// tagged enum with a unit variant and one without, a struct, a plain enum, an untagged enum with a
/// member over the first, a brand and an alias of it, and the types that flatten a parameter.
#[cfg(feature = "serde")]
fn declare_entry_fillings() {
    for declared in [
        "pub enum Pallet { Loose, Stacked { layers: u32 } }",
        "pub enum Stacking { Stacked { layers: u32 } }",
        "pub struct Crate { pub label: String }",
        "pub enum Plainly { One, Two }",
        "#[serde(untagged)] pub enum Either { Pal(Pallet), Boxed(Crate) }",
        "#[serde(transparent)] pub struct PalletBrand(Pallet);",
    ] {
        assert_eq!(refused(declared), NONE, "{declared}");
    }
    assert_eq!(refusals(expanded("type PalletAlias = Pallet;")), NONE);
    assert_eq!(
        generic_at(
            "Held",
            "Crate",
            "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum Waybill<Held> \
             { Empty, Held(Held) }"
        ),
        NONE
    );
    assert_eq!(
        refused_with_default(
            "Crate",
            "pub struct Bag<T> { pub id: u32, #[serde(flatten)] pub inner: T }"
        ),
        NONE
    );
    assert_eq!(
        refused_with_default(
            "Crate",
            "pub struct Wrap<T> { #[serde(flatten)] pub inner: T }"
        ),
        NONE
    );
    assert_eq!(
        generic_at(
            "U",
            "Crate",
            "#[derive(Serialize, Deserialize)] pub struct Outer<U> { pub bill: Waybill<U> }"
        ),
        NONE
    );
    assert_eq!(
        refused_with_default(
            "Crate",
            "#[serde(tag = \"kind\")] pub struct Tagged<T> { #[serde(flatten)] pub inner: T }"
        ),
        NONE
    );
}

/// A type parameter flattened, beside a member, beside a tag or alone, and one held by the lone
/// slot of a variant under a tag alone, is refused where a value serde writes a variant of as an
/// entry of its name over `null` fills it: an externally tagged enum with a unit variant, itself,
/// under an `Option` or through an alias or a brand, a plain enum, and an untagged enum with a
/// member over one.
#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_flattened_or_beside_a_tag_is_refused_where_a_variant_written_as_an_entry_fills_it()
 {
    declare_entry_fillings();
    let tag = " beside the tag `kind`";
    let member = " beside `id`";
    for (written, flattens, parameter, beside, filling, named) in [
        ("Waybill<Pallet>", "Waybill", "Held", tag, "Pallet", true),
        (
            "Vec<Waybill<Pallet>>",
            "Waybill",
            "Held",
            tag,
            "Pallet",
            true,
        ),
        ("Outer<Pallet>", "Waybill", "Held", tag, "Pallet", true),
        (
            "Waybill<PalletAlias>",
            "Waybill",
            "Held",
            tag,
            "PalletAlias",
            true,
        ),
        (
            "Waybill<PalletBrand>",
            "Waybill",
            "Held",
            tag,
            "PalletBrand",
            true,
        ),
        ("Waybill<Plainly>", "Waybill", "Held", tag, "Plainly", false),
        ("Bag<Pallet>", "Bag", "T", member, "Pallet", true),
        (
            "Bag<Option<Pallet>>",
            "Bag",
            "T",
            member,
            "Option<Pallet>",
            true,
        ),
        ("Bag<Either>", "Bag", "T", member, "Either", false),
        (
            "Tagged<Option<Pallet>>",
            "Tagged",
            "T",
            tag,
            "Option<Pallet>",
            true,
        ),
        ("Wrap<Pallet>", "Wrap", "T", "", "Pallet", true),
        ("Wrap<Plainly>", "Wrap", "T", "", "Plainly", false),
    ] {
        assert_eq!(
            refused(&format!("pub struct Manifest {{ pub held: {written} }}")),
            vec![filled_entry(
                "type `Manifest`: field `held`",
                flattens,
                parameter,
                beside,
                filling,
                named
            )],
            "{written}"
        );
    }
    assert_eq!(
        refusals(expanded("type Bill = Waybill<Pallet>;")),
        vec![filled_entry(
            "type `Bill`",
            "Waybill",
            "Held",
            tag,
            "Pallet",
            true
        )]
    );
    assert_eq!(
        refused_with_default(
            "Pallet",
            "pub struct Sack<T> { #[serde(flatten)] pub inner: T }"
        ),
        vec![filled_entry(
            "type `Sack`: `default_types` entry `T`",
            "Sack",
            "T",
            "",
            "Pallet",
            true
        )]
    );
}

/// Every other filling of those parameters builds: an enum whose every variant holds a value, a
/// struct, a map flattened alone, and a type declared below.
#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_flattened_or_beside_a_tag_builds_at_every_other_filling() {
    declare_entry_fillings();
    for built in [
        "Waybill<Stacking>",
        "Waybill<Crate>",
        "Waybill<PalletBelow>",
        "Bag<Crate>",
        "Bag<Stacking>",
        "Wrap<Stacking>",
        "Wrap<HashMap<String, u8>>",
        "Outer<Crate>",
    ] {
        assert_eq!(
            refused(&format!("pub struct Manifest {{ pub held: {built} }}")),
            NONE,
            "{built}"
        );
    }
}

/// The refusal of `filling` filling the type parameter `parameter` that `flattens` flattens
/// `beside` a key, in the place `subject` names, with a value serde writes as `written` says.
#[cfg(feature = "serde")]
fn filled_non_object(
    subject: &str,
    flattens: &str,
    parameter: &str,
    beside: &str,
    filling: &str,
    written: &str,
) -> String {
    format!(
        "model_schema: {subject}: `{flattens}` flattens its type parameter `{parameter}`{beside}, \
         and `{filling}` fills it with {written} Write it under a key of its own."
    )
}

/// A value its schema describes by `keyword`, flattened beside members of `Bag`.
#[cfg(feature = "serde")]
const fn merged(keyword: &'static str) -> WrittenAsNoObject {
    WrittenAsNoObject::Whole {
        beside_tag: false,
        keyword,
    }
}

/// A value its schema describes by `keyword`, held beside the tag `kind`.
#[cfg(feature = "serde")]
const fn beside_kind(keyword: &'static str) -> WrittenAsNoObject {
    WrittenAsNoObject::Whole {
        beside_tag: true,
        keyword,
    }
}

/// Declares, beside the types the fillings of a flattened type parameter are read against, those
/// serde writes as no object: a brand over text, a struct of one slot over a number, an alias of
/// the brand, an untagged enum with a member over text, and a struct of one slot and an alias over
/// an `Option` of a struct.
#[cfg(feature = "serde")]
fn declare_non_object_fillings() {
    declare_entry_fillings();
    for declared in [
        "#[serde(transparent)] pub struct Code(String);",
        "pub struct Tally(pub u32);",
        "#[serde(untagged)] pub enum TextOrCrate { Text(String), Boxed(Crate) }",
        "pub struct MaybeCrate(pub Option<Crate>);",
    ] {
        assert_eq!(refused(declared), NONE, "{declared}");
    }
    assert_eq!(refusals(expanded("type CodeAlias = Code;")), NONE);
    assert_eq!(refusals(expanded("type CrateMaybe = Option<Crate>;")), NONE);
}

/// A type parameter flattened, beside a member, beside a tag or alone, and one held by the lone
/// slot of a variant under a tag alone, is refused where a value serde writes as no object fills
/// it, as the same value flattened or held there is refused, and the lone slot's where an `Option`
/// fills it, of which serde refuses every write there: in a member, in an alias and in a
/// `default_types` entry.
#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_flattened_or_beside_a_tag_is_refused_where_a_value_written_as_no_object_fills_it()
 {
    declare_non_object_fillings();
    for (written, flattens, parameter, beside, filling, as_written) in NON_OBJECT_FILLINGS {
        assert_eq!(
            refused(&format!("pub struct Manifest {{ pub held: {written} }}")),
            vec![filled_non_object(
                "type `Manifest`: field `held`",
                flattens,
                parameter,
                beside,
                filling,
                &as_written.words()
            )],
            "{written}"
        );
    }
    assert_eq!(
        refusals(expanded("type Sacked = Bag<String>;")),
        vec![filled_non_object(
            "type `Sacked`",
            "Bag",
            "T",
            MEMBER,
            "String",
            &merged("string").words()
        )]
    );
    assert_eq!(
        refused_with_default(
            "String",
            "pub struct Sack<T> { pub id: u32, #[serde(flatten)] pub inner: T }"
        ),
        vec![filled_non_object(
            "type `Sack`: `default_types` entry `T`",
            "Sack",
            "T",
            MEMBER,
            "String",
            &merged("string").words()
        )]
    );
}

/// A value serde writes as an object fills those parameters, and an `Option` of one fills a
/// parameter flattened, beside a member or beside a struct's own tag: serde writes `None` there as
/// no key, and reads it back. A holder's own parameter, and a type declared below, build.
#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_flattened_or_beside_a_tag_builds_at_an_object_or_an_option_of_one_flattened() {
    declare_non_object_fillings();
    for built in [
        "Bag<Crate>",
        "Bag<Option<Crate>>",
        "Wrap<Option<Crate>>",
        "Tagged<Crate>",
        "Tagged<Option<Crate>>",
        "Waybill<Crate>",
        "Waybill<CodeBelow>",
        "Bag<CodeBelow>",
        "Outer<Crate>",
    ] {
        assert_eq!(
            refused(&format!("pub struct Manifest {{ pub held: {built} }}")),
            NONE,
            "{built}"
        );
    }
    assert_eq!(
        generic_at(
            "U",
            "Crate",
            "#[derive(Serialize, Deserialize)] pub struct Handed<U> { pub bag: Bag<U>, pub bill: \
             Waybill<U> }"
        ),
        NONE
    );
}

/// The README quotes the refusal of a type parameter flattened beside a member filled at text, and
/// of one held by the lone slot of a variant under a tag alone filled at an `Option`, as the build
/// answers each, in its section and in the list of what an upgrade asks of a crate.
#[cfg(feature = "serde")]
#[test]
fn the_readme_states_the_refusal_of_a_filling_written_as_no_object_as_the_build_answers_it() {
    declare_non_object_fillings();
    for source in [
        "pub struct Holders { pub text: Bag<String> }",
        "pub struct Billed { pub bill: Waybill<Option<Pallet>> }",
    ] {
        let messages = refused(source);
        assert_eq!(messages.len(), 1, "{source}: {messages:?}");
        let answered = format!("error: {}", messages[0]);
        assert_eq!(
            include_str!("../../../README.md")
                .matches(&answered)
                .count(),
            2,
            "the README no longer quotes: {answered}"
        );
    }
}

/// The README quotes the refusal of a type parameter beside a tag filled at an externally tagged
/// enum with a unit variant as the build answers it, in its section and in the list of what an
/// upgrade asks of a crate.
#[cfg(feature = "serde")]
#[test]
fn the_readme_states_the_refusal_of_a_filling_written_as_an_entry_as_the_build_answers_it() {
    assert_eq!(
        refused("pub enum Pallet { Loose, Stacked { layers: u32 } }"),
        NONE
    );
    assert_eq!(
        generic_at(
            "Held",
            "Crate",
            "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum Waybill<Held> \
             { Empty, Held(Held) }"
        ),
        NONE
    );
    let messages = refused("pub struct Manifest { pub bill: Waybill<Pallet> }");
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

/// serde neither writes nor reads a variant under `#[serde(skip)]`, so what it holds sits beside
/// no tag and no member: each shape refused there in a variant under a tag alone, or beside a
/// member under any tagging, builds. Under `skip_serializing` alone serde still reads the variant,
/// and under `skip_deserializing` alone still writes it: both are refused as before.
#[cfg(feature = "serde")]
#[test]
fn a_variant_serde_neither_writes_nor_reads_is_refused_nothing_it_holds() {
    assert_eq!(
        refused("pub struct Tally { #[serde(flatten)] counts: BTreeMap<String, u8> }"),
        NONE,
        "a struct of one flattened map builds"
    );
    let held = [
        ("Text", "Hidden(String)"),
        (
            "Map",
            "Spare { #[serde(flatten)] counts: BTreeMap<String, u8> }",
        ),
        ("Slot", "Counted(Tally)"),
    ];
    for (shape, variant) in held {
        for (attribute, refused_count) in [
            ("skip", 0),
            ("skip_serializing", 1),
            ("skip_deserializing", 1),
        ] {
            let holder = format!("Skipped{shape}{}", attribute.replace('_', ""));
            let source = format!(
                "#[serde(tag = \"kind\")] pub enum {holder} {{ Open {{ note: String }}, \
                 #[serde({attribute})] {variant} }}"
            );
            assert_eq!(refused(&source).len(), refused_count, "{source}");
        }
    }
    for source in [
        "pub enum SkippedBeside { Open { note: String }, #[serde(skip)] Spare { \
         #[serde(flatten)] counts: BTreeMap<String, u8>, note: String } }",
        "#[serde(untagged)] pub enum SkippedLoose { Open { note: String }, #[serde(skip)] \
         Spare { #[serde(flatten)] counts: BTreeMap<String, u8>, note: String } }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
    assert_eq!(
        refused_generic(
            "#[serde(tag = \"kind\")] pub enum SkippedHold<T> { Open { note: String }, \
             #[serde(skip)] Counted(T) }"
        ),
        NONE,
        "a skipped variant over the type parameter builds"
    );
    assert_eq!(
        refused("pub struct SkippedDock { pub hold: SkippedHold<HashMap<String, i32>> }"),
        NONE,
        "a map fills a parameter no variant serde writes holds"
    );
    // serde's derive refuses a tuple variant under a tag alone, skipped or not.
    assert_eq!(
        refused(
            "#[serde(tag = \"kind\")] pub enum SkippedPair { Open { note: String }, \
             #[serde(skip)] Pair(String, u8) }"
        )
        .len(),
        1,
        "a skipped tuple variant is refused under a tag alone"
    );
}

/// The README quotes the refusal of a lone slot that writes a map beside a tag as the build answers
/// it: where an internally tagged enum is described, and in the list of what an upgrade asks of a
/// crate.
#[cfg(feature = "serde")]
#[test]
fn the_readme_states_the_refusal_of_a_slot_that_writes_a_map_beside_a_tag_as_the_build_answers_it()
{
    let _: Vec<String> =
        refused("pub struct Tally { #[serde(flatten)] counts: BTreeMap<String, u8> }");
    let messages = refused("#[serde(tag = \"kind\")] pub enum Filled { Counted(Tally) }");
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

/// What every refusal of a key below a flattened member closes with, where it names `member`.
#[cfg(feature = "serde")]
fn keys_below(member: &str) -> String {
    format!(
        "serde reads every key below a flattened member as text, and reads no number and no `bool` \
         from text, so no entry of that map reads back. Key the map by a `String`, or write the \
         {member} under a key of its own."
    )
}

/// The refusal of the member `field` flattens, where `held` names what it holds and `subject`
/// leads it.
#[cfg(feature = "serde")]
fn keyed_by(subject: &str, field: &str, held: &str) -> String {
    format!(
        "model_schema: {subject}field `{field}`: `#[serde(flatten)]` cannot hold {held} \
         \u{2014} {}",
        keys_below("field")
    )
}

#[cfg(feature = "serde")]
#[test]
fn a_map_flattened_with_a_key_serde_reads_no_entry_of_is_refused_naming_the_key() {
    let _: Vec<String> = refused("#[serde(transparent)] pub struct Fathoms(pub u32);");
    for (rest, key) in [
        ("BTreeMap<isize, u8>", "isize"),
        ("HashMap<bool, u8>", "bool"),
        ("Option<BTreeMap<u32, u8>>", "u32"),
        ("Box<HashMap<i64, String>>", "i64"),
        ("BTreeMap<Box<u16>, u8>", "Box<u16>"),
        ("BTreeMap<Fathoms, u8>", "Fathoms"),
        ("BTreeMap<String, Vec<HashMap<u8, u8>>>", "u8"),
    ] {
        let attributes = if rest.starts_with("Option") {
            "#[serde(flatten, default, skip_serializing_if = \"Option::is_none\")]"
        } else {
            "#[serde(flatten)]"
        };
        assert_eq!(
            refused(&format!(
                "pub struct Soundings {{ {attributes} pub rest: {rest} }}"
            )),
            vec![keyed_by(
                "type `Soundings`: ",
                "rest",
                &format!("a map keyed by `{key}`")
            )],
            "{rest}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_variant_that_flattens_a_map_keyed_by_a_number_is_refused_naming_the_variant() {
    assert_eq!(
        refused("pub enum Sounding { Lines { #[serde(flatten)] depths: BTreeMap<u32, u8> } }"),
        vec![keyed_by(
            "type `Sounding`: variant `Lines`, ",
            "depths",
            "a map keyed by `u32`"
        )]
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_declared_above_that_holds_a_map_keyed_by_a_number_is_refused_flattened() {
    for declared in [
        "#[serde(transparent)] pub struct Depths(pub BTreeMap<u32, u8>);",
        "pub struct Slotted(pub Option<HashMap<bool, u8>>);",
        "pub struct Charted { pub marks: Vec<BTreeMap<i16, u8>> }",
    ] {
        assert_eq!(refused(declared), NONE, "{declared}");
    }
    assert_eq!(
        refused_generic("#[serde(transparent)] pub struct Over<T>(pub T);"),
        NONE
    );
    assert_eq!(refusals(expanded("type Ladder = BTreeMap<i32, u8>;")), NONE);
    for (written, held) in [
        (
            "Depths",
            "`Depths`, which holds a map keyed by a number or a `bool` (field `0`, a map keyed by \
             `u32`)",
        ),
        (
            "Option<Slotted>",
            "`Slotted`, which holds a map keyed by a number or a `bool` (field `0`, a map keyed \
             by `bool`)",
        ),
        (
            "Charted",
            "`Charted`, which holds a map keyed by a number or a `bool` (field `marks`, a map \
             keyed by `i16`)",
        ),
        (
            "Ladder",
            "`Ladder`, which holds a map keyed by a number or a `bool` (a map keyed by `i32`)",
        ),
        (
            "Over<BTreeMap<u8, u8>>",
            "`Over<BTreeMap<u8, u8>>`, which holds a map keyed by a number or a `bool` (a map \
             keyed by `u8`)",
        ),
    ] {
        let attributes = if written.starts_with("Option") {
            "#[serde(flatten, default, skip_serializing_if = \"Option::is_none\")]"
        } else {
            "#[serde(flatten)]"
        };
        assert_eq!(
            refused(&format!(
                "pub struct Plumbing {{ {attributes} pub lead: {written} }}"
            )),
            vec![keyed_by("type `Plumbing`: ", "lead", held)],
            "{written}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_map_flattened_with_a_key_serde_reads_from_text_builds() {
    let _: Vec<String> = refused("pub enum Bearing { North, South }");
    let _: Vec<String> = refused("#[serde(transparent)] pub struct Mark(pub String);");
    let _: Vec<String> = refused("pub struct Counted { pub by_number: BTreeMap<u32, u8> }");
    for source in [
        "pub struct Lettered { #[serde(flatten)] pub rest: BTreeMap<char, u8> }",
        "pub struct Charted { #[serde(flatten)] pub rest: BTreeMap<String, u8> }",
        "pub struct Dated { #[serde(flatten)] pub rest: BTreeMap<NaiveDate, u8> }",
        "pub struct Headed { #[serde(flatten)] pub rest: BTreeMap<Bearing, u8> }",
        "pub struct Marked { #[serde(flatten)] pub rest: BTreeMap<Mark, u8> }",
        "pub struct Unflattened { pub rest: BTreeMap<u32, u8> }",
        "pub struct HeldUnflattened { pub counted: Counted }",
        "pub struct Unread { #[serde(flatten, skip_deserializing)] pub rest: BTreeMap<u32, u8> }",
        "pub struct ReadByHand { #[serde(flatten, deserialize_with = \"read_lines\")] \
         pub rest: BTreeMap<u32, u8> }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
    assert_eq!(
        refused_generic("pub struct Keyed<T> { #[serde(flatten)] pub rest: BTreeMap<T, u8> }"),
        NONE
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_declared_below_its_holder_is_not_looked_into_for_a_number_key() {
    assert_eq!(
        refused("pub struct Unseen { #[serde(flatten)] pub lead: DepthsBelow }"),
        NONE
    );
    assert_eq!(
        refused("pub struct DepthsBelow { pub marks: BTreeMap<u32, u8> }"),
        NONE
    );
}

/// The README quotes the refusal as the build answers it.
#[cfg(feature = "serde")]
#[test]
fn the_readme_states_the_refusal_of_a_map_keyed_by_a_number_as_the_build_answers_it() {
    let _: Vec<String> =
        refused_generic("pub struct Rows<T> { #[serde(flatten)] pub rest: BTreeMap<T, u8> }");
    for source in [
        "pub struct Soundings { #[serde(flatten)] pub rest: BTreeMap<isize, u8> }",
        "pub struct Dock { pub rows: Rows<u32> }",
    ] {
        let messages = refused(source);
        assert_eq!(messages.len(), 1, "{source}: {messages:?}");
        let answered = format!("error: {}", messages[0]);
        assert!(
            include_str!("../../../README.md").contains(&answered),
            "the README no longer quotes: {answered}"
        );
    }
}

/// The refusal of `filling` filling the parameter `T` `reads` reads below a flattened member,
/// as the key of a map where `keying`, and `filled_with` after "fills it".
#[cfg(feature = "serde")]
fn read_below(
    subject: &str,
    reads: &str,
    keying: bool,
    filling: &str,
    filled_with: &str,
) -> String {
    let as_key = if keying { "as the key of a map " } else { "" };
    format!(
        "model_schema: {subject}`{reads}` reads its type parameter `T` {as_key}below \
         `#[serde(flatten)]`, and `{filling}` fills it{filled_with} \u{2014} {}",
        keys_below("member")
    )
}

#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_that_keys_a_map_below_a_flatten_is_refused_where_a_number_fills_it() {
    assert_eq!(
        refused_generic("pub struct Rows<T> { #[serde(flatten)] pub rest: BTreeMap<T, u8> }"),
        NONE
    );
    assert_eq!(
        refused_generic("pub struct Hand<T> { pub rows: Rows<T> }"),
        NONE
    );
    let keying =
        |subject: &str, reads: &str, filling: &str| read_below(subject, reads, true, filling, "");
    assert_eq!(
        refused("pub struct Dock { pub rows: Rows<u32> }"),
        vec![keying("type `Dock`: field `rows`: ", "Rows", "u32")]
    );
    assert_eq!(
        refused("pub struct Pier { pub rows: Option<Vec<Rows<bool>>> }"),
        vec![keying("type `Pier`: field `rows`: ", "Rows", "bool")]
    );
    assert_eq!(
        refused("pub struct Slip { pub hand: Hand<u8> }"),
        vec![keying("type `Slip`: field `hand`: ", "Rows", "u8")]
    );
    assert_eq!(
        refusals(expanded("type Ledger = Rows<u16>;")),
        vec![keying("type `Ledger`: ", "Rows", "u16")]
    );
    assert_eq!(
        refused_with_default(
            "u32",
            "pub struct Tally<T> { #[serde(flatten)] pub rest: BTreeMap<T, u8> }"
        ),
        vec![keying(
            "type `Tally`: `default_types` entry `T`: ",
            "Tally",
            "u32"
        )]
    );
    // Flattened, the member is refused whole, once.
    assert_eq!(
        refused("pub struct Quay { #[serde(flatten)] pub rows: Rows<Box<i64>> }").len(),
        1
    );
    for source in [
        "pub struct Wharf { pub rows: Rows<String> }",
        "pub struct Mole { pub rows: Rows<char> }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
    assert_eq!(
        refused_generic("pub struct Jetty<T> { pub rows: Rows<T> }"),
        NONE
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_read_below_a_flatten_is_refused_where_its_filling_holds_a_number_key() {
    assert_eq!(
        refused_flattening("pub struct Spread<T> { #[serde(flatten)] pub rest: T }"),
        NONE
    );
    assert_eq!(refused_generic("pub struct Inner<T> { pub held: T }"), NONE);
    assert_eq!(
        refused_generic("pub struct Wrapper<T> { #[serde(flatten)] pub inner: Inner<T> }"),
        NONE
    );
    assert_eq!(
        refused("pub struct Charted { pub marks: Vec<BTreeMap<i16, u8>> }"),
        NONE
    );
    let below = |subject: &str, reads: &str, filling: &str, filled_with: &str| {
        read_below(subject, reads, false, filling, filled_with)
    };
    assert_eq!(
        refused("pub struct Dock { pub spread: Spread<BTreeMap<u32, u8>> }"),
        vec![below(
            "type `Dock`: field `spread`: ",
            "Spread",
            "BTreeMap<u32, u8>",
            " with a map keyed by `u32`"
        )]
    );
    assert_eq!(
        refused("pub struct Berth { pub wrapper: Wrapper<Vec<HashMap<bool, u8>>> }"),
        vec![below(
            "type `Berth`: field `wrapper`: ",
            "Wrapper",
            "Vec<HashMap<bool, u8>>",
            " with a map keyed by `bool`"
        )]
    );
    assert_eq!(
        refused("pub struct Moor { pub spread: Spread<Charted> }"),
        vec![below(
            "type `Moor`: field `spread`: ",
            "Spread",
            "Charted",
            ", which holds a map keyed by a number or a `bool` (field `marks`, a map keyed by \
             `i16`)"
        )]
    );
    assert_eq!(
        refused_generic("pub struct Keyed<T> { pub spread: Spread<BTreeMap<T, u8>> }"),
        NONE
    );
    assert_eq!(
        refused("pub struct Pier { pub keyed: Keyed<i8> }"),
        vec![read_below(
            "type `Pier`: field `keyed`: ",
            "Keyed",
            true,
            "i8",
            ""
        )]
    );
    for source in [
        "pub struct Wharf { pub spread: Spread<BTreeMap<String, u8>> }",
        "pub struct Mole { pub wrapper: Wrapper<u32> }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
}

/// The refusal of a member of a variant serde reads from the copy of the payload it buffers, where
/// `subject` names the member and `held` what it holds: `tagged` for an enum under a tag alone, an
/// untagged enum otherwise.
#[cfg(feature = "serde")]
fn buffered(subject: &str, held: &str, tagged: bool) -> String {
    let (enum_is, closing) = if tagged {
        (
            "an enum under a tag alone",
            "Key the map by a `String`, or name a `content` key: serde writes the tag ahead of \
             the content, and reads the content that follows it without a copy.",
        )
    } else {
        ("an untagged enum", "Key the map by a `String`.")
    };
    format!(
        "model_schema: {subject}: {enum_is} cannot hold {held} \u{2014} serde reads {enum_is} \
         from a copy of the payload it buffers, where every key is text, and reads no number and \
         no `bool` from text, so no entry of that map reads back. {closing}"
    )
}

#[cfg(feature = "serde")]
#[test]
fn an_untagged_enum_or_one_under_a_tag_alone_with_a_map_keyed_by_a_number_is_refused() {
    let _: Vec<String> = refused("#[serde(transparent)] pub struct Fathoms(pub u32);");
    for (written, key) in [
        ("BTreeMap<u32, u8>", "u32"),
        ("Option<HashMap<bool, u8>>", "bool"),
        ("Vec<BTreeMap<i64, String>>", "i64"),
        ("Box<BTreeMap<Fathoms, u8>>", "Fathoms"),
        ("BTreeMap<String, BTreeMap<u8, u8>>", "u8"),
    ] {
        assert_eq!(
            refused(&format!(
                "#[serde(untagged)] pub enum Soundings {{ Lines({written}), Named {{ depths: \
                 {written} }} }}"
            )),
            vec![
                buffered(
                    "type `Soundings`: variant `Lines`, field `0`",
                    &format!("a map keyed by `{key}`"),
                    false
                ),
                buffered(
                    "type `Soundings`: variant `Named`, field `depths`",
                    &format!("a map keyed by `{key}`"),
                    false
                ),
            ],
            "{written}"
        );
        assert_eq!(
            refused(&format!(
                "#[serde(tag = \"kind\")] pub enum Plumbed {{ Named {{ depths: {written} }} }}"
            )),
            vec![buffered(
                "type `Plumbed`: variant `Named`, field `depths`",
                &format!("a map keyed by `{key}`"),
                true
            )],
            "{written}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn an_enum_serde_buffers_that_holds_a_type_declared_above_with_such_a_map_is_refused() {
    let _: Vec<String> = refused("pub struct Charted { pub marks: Vec<BTreeMap<i16, u8>> }");
    let _: Vec<String> = refused("pub struct Keyed<T> { pub marks: BTreeMap<T, u8> }");
    let _: Vec<String> = refused_generic("pub struct Over<T> { pub held: T }");
    let charted = "`Charted`, which holds a map keyed by a number or a `bool` (field `marks`, a \
                   map keyed by `i16`)";
    assert_eq!(
        refused("#[serde(untagged)] pub enum Leadline { Charted(Charted), Plain(String) }"),
        vec![buffered(
            "type `Leadline`: variant `Charted`, field `0`",
            charted,
            false
        )]
    );
    assert_eq!(
        refused("#[serde(tag = \"kind\")] pub enum Fathomed { Charted(Charted) }"),
        vec![buffered(
            "type `Fathomed`: variant `Charted`, field `0`",
            charted,
            true
        )]
    );
    assert_eq!(
        refused("#[serde(untagged)] pub enum Overheld { Held { over: Over<BTreeMap<u8, u8>> } }"),
        vec![buffered(
            "type `Overheld`: variant `Held`, field `over`",
            "`Over<BTreeMap<u8, u8>>`, which holds a map keyed by a number or a `bool` (a map \
             keyed by `u8`)",
            false
        )]
    );
    assert_eq!(
        refused("#[serde(untagged)] pub enum Keying { Keyed(Keyed<u32>) }"),
        vec![buffered(
            "type `Keying`: variant `Keyed`, field `0`",
            "`Keyed<u32>`, which holds a map keyed by a number or a `bool` (field `marks`, a map \
             keyed by `u32`)",
            false
        )]
    );
}

/// The refusal of `filling` filling the parameter `T` that `reads` reads in an enum serde buffers,
/// as the key of a map where `keying`, and `filled_with` after "fills it".
#[cfg(feature = "serde")]
fn read_buffered(subject: &str, reads: &str, keying: bool, filling: &str) -> String {
    let as_key = if keying { "as the key of a map " } else { "" };
    let filled_with = if keying {
        String::new()
    } else {
        format!(" with a map keyed by `{}`", &filling[9..12])
    };
    format!(
        "model_schema: {subject}`{reads}` reads its type parameter `T` {as_key}in an untagged \
         enum, and `{filling}` fills it{filled_with} \u{2014} serde reads an untagged enum from a \
         copy of the payload it buffers, where every key is text, and reads no number and no \
         `bool` from text, so no entry of that map reads back. Key the map by a `String`."
    )
}

#[cfg(feature = "serde")]
#[test]
fn a_type_parameter_an_enum_serde_buffers_reads_is_refused_where_a_number_fills_it() {
    assert_eq!(
        refused_generic("#[serde(untagged)] pub enum Lines<T> { Keyed(BTreeMap<T, u8>) }"),
        NONE
    );
    assert_eq!(
        refused_generic("#[serde(untagged)] pub enum OneOrMore<T> { One(T), More(Vec<T>) }"),
        NONE
    );
    assert_eq!(
        refused_generic("pub struct Handed<T> { pub lines: Lines<T> }"),
        NONE
    );
    assert_eq!(
        refused("pub struct Dock { pub lines: Option<Lines<u32>> }"),
        vec![read_buffered(
            "type `Dock`: field `lines`: ",
            "Lines",
            true,
            "u32"
        )]
    );
    assert_eq!(
        refused("pub struct Slip { pub handed: Handed<bool> }"),
        vec![read_buffered(
            "type `Slip`: field `handed`: ",
            "Lines",
            true,
            "bool"
        )]
    );
    assert_eq!(
        refused("pub struct Pier { pub some: OneOrMore<BTreeMap<u16, u8>> }"),
        vec![read_buffered(
            "type `Pier`: field `some`: ",
            "OneOrMore",
            false,
            "BTreeMap<u16, u8>"
        )]
    );
    for source in [
        "pub struct Wharf { pub lines: Lines<String> }",
        "pub struct Mole { pub lines: Lines<char> }",
        "pub struct Quay { pub some: OneOrMore<BTreeMap<String, u8>> }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn an_enum_serde_reads_such_a_map_in_builds() {
    for source in [
        "pub enum Bearing { Lines { depths: BTreeMap<u32, u8> }, Held(BTreeMap<bool, u8>) }",
        "#[serde(tag = \"kind\", content = \"body\")] pub enum Hauled { Lines { depths: \
         BTreeMap<u32, u8> }, Held(BTreeMap<bool, u8>) }",
        "#[serde(untagged)] pub enum Lettered { Lines(BTreeMap<char, u8>), Named(BTreeMap<String, \
         u8>) }",
        "#[serde(untagged)] pub enum Unread { Lines { #[serde(skip_deserializing)] depths: \
         BTreeMap<u32, u8>, count: u8 } }",
        "#[serde(untagged)] pub enum ReadByHand { Lines { #[serde(deserialize_with = \
         \"read_lines\")] depths: BTreeMap<u32, u8> } }",
        "#[serde(untagged)] pub enum Skipped { Text(String), #[serde(skip_deserializing)] \
         Lines(BTreeMap<u32, u8>) }",
        "#[serde(untagged)] pub enum Below { Lines(DepthsBelow) }",
    ] {
        assert_eq!(refused(source), NONE, "{source}");
    }
    assert_eq!(
        refused_generic("#[serde(untagged)] pub enum Keyed<T> { Lines(BTreeMap<T, u8>) }"),
        NONE
    );
}

/// The lone slot of a variant under a tag alone that is a map keyed by a number is refused beside
/// the tag, once.
#[cfg(feature = "serde")]
#[test]
fn a_lone_map_beside_a_tag_is_refused_as_a_map_and_not_for_its_key() {
    let messages = refused("#[serde(tag = \"kind\")] pub enum Lined { Lines(BTreeMap<u32, u8>) }");
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        !messages[0].contains("buffers"),
        "the lone map is refused for its key: {messages:?}"
    );
}

/// The README quotes the refusal as the build answers it.
#[cfg(feature = "serde")]
#[test]
fn the_readme_states_the_refusal_of_a_map_keyed_by_a_number_in_a_buffered_enum() {
    for source in [
        "#[serde(untagged)] pub enum Leadlines { Lines(BTreeMap<u32, u8>) }",
        "#[serde(tag = \"kind\")] pub enum Plumbed { Named { depths: BTreeMap<u32, u8> } }",
    ] {
        let messages = refused(source);
        assert_eq!(messages.len(), 1, "{source}: {messages:?}");
        let answered = format!("error: {}", messages[0]);
        assert!(
            include_str!("../../../README.md").contains(&answered),
            "the README no longer quotes: {answered}"
        );
    }
}
