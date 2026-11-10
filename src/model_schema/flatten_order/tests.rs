//! The decision table of flattened members, every sequence of member classes against the rule
//! serde was measured to follow for it, and the class each member kind of that measure earns here.

use std::collections::HashMap;

use super::{
    ABSORBS, Class, Culprit, EMPTY, ENTRY, NO_OBJECT, OPTION_EMPTY, PLAIN, REFUSES, REST,
    TEXT_KEYS, UNSEEN, Verdict, errors, judged_levels, record, takes_only, verdict,
};
use crate::model_schema::ModelSchemaArgs;

const CLASSES: [fn() -> Class; 5] = [
    || Class::Taker,
    || Class::Open,
    || {
        Class::Refusing(vec![Culprit {
            by_hand: false,
            holds_more: false,
            name: "Tagged".to_owned(),
        }])
    },
    || Class::Absorbing,
    || Class::Unseen("the type parameter `T`".to_owned()),
];

/// One member of each measured kind, each declared once, in the order a holder flattens them.
const MEMBERS: &str = r#"
    #[derive(Serialize, Deserialize)]
    pub struct Inner { pub b: u32 }
    #[derive(Serialize, Deserialize)]
    pub struct Pin { pub a: u32 }
    #[derive(Serialize, Deserialize)]
    pub struct Peg { pub c: u32 }
    #[derive(Serialize, Deserialize)]
    pub enum External { Rect { w: u32 }, Dot, Wrap(Inner) }
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "k", content = "c")]
    pub enum Adjacent { Rect { w: u32 }, Dot, Wrap(Inner) }
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind")]
    pub enum Tagged { Rect { w: u32 }, Dot, Wrap(Inner) }
    #[derive(Serialize, Deserialize)]
    pub struct Lining { pub l: u32 }
    #[derive(Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum Untagged { Round { u: u32 }, Held(Lining) }
    #[derive(Serialize, Deserialize)]
    pub struct FlatPin { pub own: u32, #[serde(flatten)] pub pin: Pin }
    #[derive(Serialize, Deserialize)]
    pub struct FlatTagged { pub own: u32, #[serde(flatten)] pub tagged: Tagged }
    #[derive(Serialize, Deserialize)]
    pub struct FlatOnly { #[serde(flatten)] pub pin: Pin, #[serde(flatten)] pub peg: Peg }
    #[derive(Serialize, Deserialize)]
    pub struct FlatMap { #[serde(flatten)] pub entries: BTreeMap<String, u32> }
"#;

/// The rule of the decision table a holder earns, and its verdict, written from the rules as the
/// measure states them rather than from [`verdict`].
fn rule_of(classes: &[Class], closed_by_hand: bool, leftover: bool) -> (&'static str, Verdict) {
    let rest: Vec<usize> = (0..classes.len())
        .filter(|&at| classes[at] != Class::Taker)
        .collect();
    let last = classes.len().saturating_sub(1);
    let position =
        |wanted: fn(&Class) -> bool| rest.iter().copied().find(|&at| wanted(&classes[at]));
    let absorbing = position(|class| *class == Class::Absorbing);
    let unseen = position(|class| matches!(class, Class::Unseen(_)));
    let refusing = position(|class| matches!(class, Class::Refusing(_)));
    if closed_by_hand && let Some(&first) = rest.first() {
        return (
            "R11a strict holder of a member that reads every key",
            Verdict::ClosedByHand(first),
        );
    }
    if rest.is_empty() {
        return ("R1 only takers", Verdict::Closable);
    }
    let sources = rest.len() + usize::from(leftover);
    if rest.len() == 1 && unseen.is_some() {
        let only = rest[0];
        return if only == last {
            ("type parameter, last", Verdict::Open)
        } else {
            ("type parameter, not last", Verdict::Reorder(only))
        };
    }
    if sources == 1 {
        let only = rest[0];
        let is_last = only == last;
        if classes[only] == Class::Open {
            return ("R2 one open rest member", Verdict::Open);
        }
        if absorbing.is_some() {
            return if is_last {
                ("R5 one map, last", Verdict::Open)
            } else {
                ("R6 one map, not last", Verdict::Reorder(only))
            };
        }
        return if is_last {
            ("R3 one refusing rest member, last", Verdict::Open)
        } else {
            (
                "R4 one refusing rest member, not last",
                Verdict::Reorder(only),
            )
        };
    }
    if let Some(at) = absorbing {
        return (
            "R9 map beside another rest member",
            Verdict::ChangeShape(at),
        );
    }
    if let Some(at) = unseen {
        return (
            "type parameter beside another rest member",
            Verdict::ChangeShape(at),
        );
    }
    if let Some(at) = refusing {
        return (
            "R8 two or more rest members, one refusing",
            Verdict::Nonstrict(at),
        );
    }
    ("R7 two or more rest members, all open", Verdict::Open)
}

/// Every sequence of `length` classes.
fn sequences(length: usize) -> Vec<Vec<Class>> {
    let mut all: Vec<Vec<Class>> = vec![Vec::new()];
    for _ in 0..length {
        all = all
            .into_iter()
            .flat_map(|held| {
                CLASSES.iter().map(move |class| {
                    let mut longer = held.clone();
                    longer.push(class());
                    longer
                })
            })
            .collect();
    }
    all
}

#[test]
fn every_sequence_of_member_classes_earns_the_verdict_of_its_rule() {
    let mut hits: HashMap<&str, usize> = HashMap::new();
    for length in 1..=4 {
        for classes in sequences(length) {
            for closed_by_hand in [false, true] {
                for leftover in [false, true] {
                    let (rule, expected) = rule_of(&classes, closed_by_hand, leftover);
                    *hits.entry(rule).or_default() += 1;
                    assert_eq!(
                        verdict(&classes, closed_by_hand, leftover),
                        expected,
                        "{rule}: {classes:?}, closed by hand {closed_by_hand}, leftover {leftover}"
                    );
                }
            }
        }
    }
    let mut rules: Vec<&str> = hits.keys().copied().collect();
    rules.sort_unstable();
    assert_eq!(
        rules,
        [
            "R1 only takers",
            "R11a strict holder of a member that reads every key",
            "R2 one open rest member",
            "R3 one refusing rest member, last",
            "R4 one refusing rest member, not last",
            "R5 one map, last",
            "R6 one map, not last",
            "R7 two or more rest members, all open",
            "R8 two or more rest members, one refusing",
            "R9 map beside another rest member",
            "type parameter beside another rest member",
            "type parameter, last",
            "type parameter, not last",
        ],
        "a rule no sequence reached"
    );
}

#[test]
fn a_holder_that_leaves_a_key_of_its_own_counts_it_beside_a_refusing_member() {
    let refusing = CLASSES[2]();
    assert_eq!(
        verdict(&[Class::Taker, refusing.clone()], false, false),
        Verdict::Open
    );
    assert_eq!(
        verdict(&[Class::Taker, refusing], false, true),
        Verdict::Nonstrict(1)
    );
    assert_eq!(
        verdict(&[Class::Taker, Class::Open], false, true),
        Verdict::Open
    );
}

/// Records each item of `source`, `nonstrict` naming those written `#[model_schema(nonstrict)]`,
/// and answers the class of each member the last item flattens, then its compile errors.
fn classes_of(source: &str, nonstrict: &[&str]) -> (Vec<Class>, Vec<String>) {
    let file: syn::File = syn::parse_str(source).unwrap();
    let (last, above) = file.items.split_last().unwrap();
    for item in above {
        let named = if let syn::Item::Struct(item_struct) = item {
            item_struct.ident.to_string()
        } else if let syn::Item::Enum(item_enum) = item {
            item_enum.ident.to_string()
        } else {
            String::new()
        };
        let args = ModelSchemaArgs {
            nonstrict: nonstrict.contains(&named.as_str()),
            ..ModelSchemaArgs::default()
        };
        record(item, &args);
    }
    let args = ModelSchemaArgs::default();
    let classes = judged_levels(last, &args)
        .into_iter()
        .flat_map(|(_, members)| members.into_iter().map(|member| member.class))
        .collect();
    let refused = errors(last, &args)
        .iter()
        .map(ToString::to_string)
        .collect();
    (classes, refused)
}

/// A holder of one flattened `ty`, below [`MEMBERS`].
fn holding(ty: &str) -> String {
    format!(
        "{MEMBERS}\n#[derive(Serialize, Deserialize)]\npub struct Holder {{ #[serde(flatten)] pub \
         held: {ty} }}"
    )
}

fn refusing(names: &[&str]) -> Class {
    Class::Refusing(
        names
            .iter()
            .map(|name| Culprit {
                by_hand: false,
                holds_more: false,
                name: (*name).to_owned(),
            })
            .collect(),
    )
}

#[test]
fn each_measured_member_kind_earns_its_measured_class() {
    let all_strict: &[&str] = &[];
    let open = [
        "Inner",
        "Lining",
        "Pin",
        "Peg",
        "Tagged",
        "Untagged",
        "FlatPin",
        "FlatTagged",
        "FlatOnly",
    ];
    let cases: [(&str, &[&str], Class); 22] = [
        ("Pin", all_strict, Class::Taker),
        ("Pin", &open, Class::Taker),
        ("Option<Pin>", all_strict, Class::Taker),
        ("Option<Pin>", &open, Class::Taker),
        ("External", all_strict, Class::Taker),
        ("External", &open, Class::Taker),
        ("Adjacent", all_strict, Class::Taker),
        ("Adjacent", &open, Class::Taker),
        ("Tagged", all_strict, refusing(&["Tagged", "Inner"])),
        ("Tagged", &open, Class::Open),
        ("Tagged", &["Tagged"], refusing(&["Inner"])),
        ("Tagged", &["Inner"], refusing(&["Tagged"])),
        ("Untagged", all_strict, refusing(&["Untagged", "Lining"])),
        ("Untagged", &open, Class::Open),
        ("FlatPin", all_strict, refusing(&["FlatPin"])),
        ("FlatPin", &open, Class::Open),
        ("FlatPin", &["FlatPin"], Class::Open),
        ("FlatTagged", all_strict, refusing(&["Tagged", "Inner"])),
        ("FlatTagged", &open, Class::Open),
        ("FlatOnly", all_strict, refusing(&["FlatOnly"])),
        ("FlatOnly", &open, Class::Open),
        ("BTreeMap<String, u32>", all_strict, Class::Absorbing),
    ];
    for (ty, nonstrict, expected) in cases {
        let (classes, _) = classes_of(&holding(ty), nonstrict);
        assert_eq!(classes, [expected], "{ty} with {nonstrict:?} nonstrict");
    }
    let (classes, _) = classes_of(&holding("FlatMap"), all_strict);
    assert_eq!(classes, [Class::Absorbing], "a struct of one flattened map");
}

#[test]
fn a_member_the_expansion_cannot_see_earns_the_strictest_class() {
    let (classes, _) = classes_of(
        "#[derive(Serialize, Deserialize)]\npub struct Holder<T> { #[serde(flatten)] pub held: T }",
        &[],
    );
    assert_eq!(
        classes,
        [Class::Unseen("the type parameter `T`".to_owned())]
    );
    let (below, _) = classes_of(
        "#[derive(Serialize, Deserialize)]\npub struct Holder { #[serde(flatten)] pub held: \
         Below }",
        &[],
    );
    assert_eq!(
        below,
        [Class::Unseen(
            "`Below`, declared below the type that flattens it, in another crate, or with no \
             `#[model_schema]`"
                .to_owned()
        )]
    );
}

/// The one compile error `holder`, below [`MEMBERS`], earns.
fn refusal(holder: &str, nonstrict: &[&str]) -> String {
    let (_, refused) = classes_of(&format!("{MEMBERS}\n{holder}"), nonstrict);
    assert_eq!(refused.len(), 1, "{holder}: {refused:?}");
    refused[0].clone()
}

#[test]
fn a_refusing_member_ahead_of_another_is_refused_with_the_order_that_reads_back() {
    let refused = refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Holder { #[serde(flatten)] pub shape: \
         Tagged, #[serde(flatten)] pub pin: Pin }",
        &[],
    );
    assert!(
        refused.contains(
            "model_schema: type `Holder` flattens `shape` (`Tagged`) ahead of `pin`: `Tagged` \
             reads every key still left and refuses one it does not declare, so serde refuses \
             the keys of `pin` that it wrote itself. Declare `shape` after every other flattened \
             member."
        ),
        "{refused}"
    );
}

#[test]
fn two_members_that_read_every_key_name_each_type_to_mark_nonstrict() {
    let refused = refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Holder { #[serde(flatten)] pub shape: \
         Tagged, #[serde(flatten)] pub other: Untagged }",
        &["Untagged", "Lining"],
    );
    assert!(
        refused.contains(
            "model_schema: type `Holder` flattens `shape` (`Tagged`) and `other` (`Untagged`), \
             and each reads every key still left: `Tagged` refuses a key it does not declare, so \
             it refuses the keys of `other` (`Untagged`) in every order. Mark `Tagged` and \
             `Inner` `#[model_schema(nonstrict)]`."
        ),
        "{refused}"
    );
}

#[test]
fn a_holder_closed_by_hand_over_a_member_that_reads_every_key_is_refused() {
    let refused = refusal(
        "#[derive(Serialize, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct Holder { \
         #[serde(flatten)] pub shape: Tagged }",
        &["Tagged", "Inner"],
    );
    assert!(
        refused.contains(
            "model_schema: type `Holder` is written `#[serde(deny_unknown_fields)]` and \
             flattens `shape` (`Tagged`)"
        ),
        "{refused}"
    );
}

#[test]
fn two_sources_of_one_key_are_refused_naming_the_key() {
    let refused = refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Holder { pub a: u32, #[serde(flatten)] \
         pub pin: Pin }",
        &[],
    );
    assert!(
        refused.contains(
            "model_schema: type `Holder` has field `a` and flattened `pin` (`Pin`), which both \
             write the key `a`"
        ),
        "{refused}"
    );
}

#[test]
fn a_holder_whose_members_read_back_earns_no_error() {
    for (holder, nonstrict) in [
        (
            "#[derive(Serialize, Deserialize)]\npub struct Holder { #[serde(flatten)] pub pin: \
             Pin, #[serde(flatten)] pub shape: Tagged }",
            &[][..],
        ),
        (
            "#[derive(Serialize, Deserialize)]\npub struct Holder { #[serde(flatten)] pub shape: \
             Tagged, #[serde(flatten)] pub other: Untagged }",
            &["Tagged", "Untagged", "Inner", "Lining"][..],
        ),
        (
            "#[derive(Serialize, Deserialize)]\npub struct Holder { #[serde(flatten)] pub ext: \
             External, #[serde(flatten)] pub adj: Adjacent, #[serde(flatten)] pub pin: \
             Option<Pin> }",
            &[][..],
        ),
    ] {
        let (_, refused) = classes_of(&format!("{MEMBERS}\n{holder}"), nonstrict);
        assert!(refused.is_empty(), "{holder}: {refused:?}");
    }
}

/// The expansion of `source` under the arguments `args`, as text.
fn expansion(args: &str, source: &str) -> String {
    super::super::exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    )
    .to_string()
}

#[test]
fn a_braced_struct_of_no_member_refuses_a_key_unless_it_is_nonstrict() {
    let source = "#[derive(Serialize, Deserialize)] pub struct Hollow {}";
    let strict = expansion("", source);
    assert!(strict.contains("unknown_field"), "{strict}");
    assert!(strict.contains("derive (Serialize)"), "{strict}");
    let open = expansion("nonstrict", source);
    assert!(!open.contains("unknown_field"), "{open}");
    assert!(open.contains("derive (Serialize , Deserialize)"), "{open}");
}

/// The one compile error `source`, whose last item is the holder, earns: every item above it
/// recorded strict.
fn sole_refusal(source: &str) -> String {
    let (_, refused) = classes_of(source, &[]);
    assert_eq!(refused.len(), 1, "{source}: {refused:?}");
    refused[0].clone()
}

#[test]
fn an_option_of_a_map_is_refused_naming_the_map_without_the_option() {
    let refused = sole_refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Locker { #[serde(default, flatten, \
         skip_serializing_if = \"Option::is_none\")] pub counts: Option<BTreeMap<String, u8>> }",
    );
    assert!(
        refused.contains(
            "model_schema: type `Locker` flattens `counts` (`Option<BTreeMap<String, u8>>`): \
             serde writes `None` as no key, and reads an object that holds no entry of \
             `BTreeMap<String, u8>` as `Some` of a map of no entry. Flatten the map without the \
             `Option`, which serde reads as a map of no entry where none is, or write `counts` \
             under a key of its own."
        ),
        "{refused}"
    );
}

#[test]
fn an_option_of_a_struct_of_defaulted_members_is_refused_as_read_back_as_some() {
    let refused = sole_refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Inked { #[serde(default)] pub colour: u32 \
         }\n#[derive(Serialize, Deserialize)]\npub struct Stamped { pub at: u32, #[serde(default, \
         flatten, skip_serializing_if = \"Option::is_none\")] pub ink: Option<Inked> }",
    );
    assert!(
        refused.contains(
            "model_schema: type `Stamped` flattens `ink` (`Option<Inked>`): `Inked` reads from \
             an object that holds none of its keys, so serde writes `None` as no key and reads \
             it back as `Some`. Give `Inked` a member serde requires, flatten `Inked` without the \
             `Option`, or write `ink` under a key of its own."
        ),
        "{refused}"
    );
}

#[test]
fn a_member_that_reads_a_key_another_member_writes_is_refused_in_either_order() {
    let refused = sole_refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Inked { #[serde(alias = \"at\")] pub \
         colour: u32 }\n#[derive(Serialize, Deserialize)]\npub struct Dated { pub at: u32 }\n\
         #[derive(Serialize, Deserialize)]\npub struct Stamped { #[serde(flatten)] pub ink: \
         Inked, #[serde(flatten)] pub date: Dated }",
    );
    assert!(
        refused.contains(
            "model_schema: type `Stamped` has flattened `ink` (`Inked`) and flattened `date` \
             (`Dated`): flattened `ink` (`Inked`) reads the key `at`, which flattened `date` \
             (`Dated`) writes, and serde hands it to flattened `ink` (`Inked`), so flattened \
             `date` (`Dated`) reads back without it. Rename the key in one of them, or write \
             `ink` or `date` under a key of its own."
        ),
        "{refused}"
    );
    let reordered = sole_refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Inked { #[serde(alias = \"at\")] pub \
         colour: u32 }\n#[derive(Serialize, Deserialize)]\npub struct Dated { pub at: u32 }\n\
         #[derive(Serialize, Deserialize)]\npub struct Stamped { #[serde(flatten)] pub date: \
         Dated, #[serde(flatten)] pub ink: Inked }",
    );
    assert!(
        reordered.contains(
            "reads the key `at`, which flattened `date` (`Dated`) writes, and serde reads it as \
             flattened `date` (`Dated`)'s, and the Kotlin and Swift reads hand it to both, which \
             then refuse what serde wrote."
        ),
        "{reordered}"
    );
}

#[test]
fn a_variant_of_an_untagged_enum_that_reads_every_object_ahead_of_another_is_refused() {
    let refused = sole_refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Pin { pub a: u32 }\n\
         #[derive(Serialize, Deserialize)]\n#[serde(untagged)]\npub enum Shelving { Any { \
         #[serde(flatten)] extra: BTreeMap<String, u32> }, Pinned(Pin) }",
    );
    assert!(
        refused.contains(
            "model_schema: type `Shelving`: variant `Any` of the untagged `Shelving` reads every \
             object, as every member it flattens reads from an object that holds none of its \
             keys, and it refuses no key it does not declare, so serde never reads variant \
             `Pinned`, declared after it, which serde writes as an object. Declare `Any` after \
             every other variant."
        ),
        "{refused}"
    );
    let (_, reordered) = classes_of(
        "#[derive(Serialize, Deserialize)]\npub struct Pin { pub a: u32 }\n\
         #[derive(Serialize, Deserialize)]\n#[serde(untagged)]\npub enum Shelving { Pinned(Pin), \
         Any { #[serde(flatten)] extra: BTreeMap<String, u32> } }",
        &[],
    );
    assert_eq!(
        reordered,
        Vec::<String>::new(),
        "declared last, it reads back"
    );
}

/// A member whose keys are those of a type parameter is refused, the parameter written bare, under
/// an `Option` or a `Box`, in a variant, or through a generic untagged enum that holds it, and a
/// generic type that holds its parameter under a key is flattened as any struct is.
#[test]
fn a_member_whose_keys_are_those_of_a_type_parameter_is_refused() {
    let either: syn::Item = syn::parse_str(
        "#[derive(Serialize, Deserialize)]\n#[serde(untagged)]\npub enum Either<T> { Count { \
         count: u32 }, Held(T) }",
    )
    .unwrap();
    let page: syn::Item =
        syn::parse_str("#[derive(Serialize, Deserialize)]\npub struct Page<T> { pub item: T }")
            .unwrap();
    record(&either, &ModelSchemaArgs::default());
    record(&page, &ModelSchemaArgs::default());
    for (holder, member, written) in [
        ("Stamped", "record", "T"),
        ("Maybe", "record", "Option<T>"),
        ("Boxed", "record", "Box<T>"),
        ("Shelf", "either", "Either<T>"),
    ] {
        let refused = sole_refusal(&format!(
            "#[derive(Serialize, Deserialize)]\npub struct {holder}<T> {{ pub label: String, \
             #[serde(flatten)] pub {member}: {written} }}"
        ));
        assert!(
            refused.contains(&format!(
                "type `{holder}` flattens `{member}` (`{written}`), whose keys are those of a type \
                 parameter"
            )),
            "{refused}"
        );
    }
    let refused = sole_refusal(
        "#[derive(Serialize, Deserialize)]\npub enum Entry<T> { Stamped { created_at: String, \
         #[serde(flatten)] record: T } }",
    );
    assert!(
        refused.contains("type `Entry` flattens `record` (`T`), whose keys are those of a type"),
        "{refused}"
    );
    let (_, listing) = classes_of(
        "#[derive(Serialize, Deserialize)]\npub struct Listing<T> { pub label: String, \
         #[serde(flatten)] pub page: Page<T> }",
        &[],
    );
    assert_eq!(listing, Vec::<String>::new());
}

/// The hidden constant the expansion of `source` publishes.
fn published_bits(source: &str) -> u16 {
    let expanded = expansion("", source);
    let digits: String = expanded
        .split_once("__TIXSCHEMA_FLATTENED : u16 = ")
        .map(|(_, after)| after.chars().take_while(char::is_ascii_digit).collect())
        .unwrap_or_default();
    digits.parse().unwrap()
}

#[test]
fn each_kind_publishes_the_bits_a_holder_reads_of_it() {
    for (source, bits) in [
        ("pub struct Pin { pub a: u32 }", REFUSES),
        (
            "pub struct Loose { #[serde(default)] pub a: u32 }",
            REFUSES | EMPTY,
        ),
        // Read whole, as a newtype variant's payload, it refuses every key beside its entry.
        (
            "pub enum Plain { Calm, Rough { x: u32 } }",
            REFUSES | UNSEEN | ENTRY,
        ),
        ("pub struct Pair(u32, u32);", REST | NO_OBJECT),
        (
            "#[serde(tag = \"kind\")] pub enum Tagged { Rect { w: u32 } }",
            REST | REFUSES,
        ),
        (
            "pub struct Counts { #[serde(flatten)] pub c: BTreeMap<String, u32> }",
            REST | ABSORBS | EMPTY,
        ),
    ] {
        assert_eq!(
            published_bits(&format!("#[derive(Serialize, Deserialize)] {source}")),
            bits,
            "{source}"
        );
    }
}

#[test]
fn a_holder_of_a_member_declared_below_asks_its_constant_once_for_each_refusal() {
    let expanded = expansion(
        "",
        "#[derive(Serialize, Deserialize)] pub struct Parcel { pub id: u32, #[serde(flatten)] \
         pub mark: MarkBelow, #[serde(default, flatten, skip_serializing_if = \
         \"Option::is_none\")] pub ink: Option<InkBelow> }",
    );
    assert!(expanded.contains("deny_unknown_fields"), "{expanded}");
    for said in [
        "serde writes `MarkBelow` as no object, which it cannot flatten into `Parcel`",
        "`MarkBelow` is an externally tagged enum with a unit variant",
        "`InkBelow` reads from an object that holds none of its keys",
        "Mark `Parcel` `#[model_schema(nonstrict)]`",
    ] {
        assert!(expanded.contains(said), "no `{said}` in: {expanded}");
    }
    for message in expanded.split("\"model_schema: ").skip(1) {
        let text = message.split('"').next().unwrap_or_default();
        assert_eq!(
            expanded.matches(text).count(),
            1,
            "asked more than once: {text}"
        );
    }
}

#[test]
fn a_seen_member_that_reads_every_key_beside_an_unseen_part_leaves_its_holder_open() {
    let later: syn::Item = syn::parse_str(
        "#[derive(Serialize, Deserialize)]\n#[serde(untagged)]\npub enum Later { Obj { a: String \
         }, Held(HeldBelow) }",
    )
    .unwrap();
    let holder: syn::ItemStruct = syn::parse_str(
        "#[derive(Serialize, Deserialize)]\npub struct Holder { #[serde(flatten)] pub m: Later }",
    )
    .unwrap();
    let nonstrict = ModelSchemaArgs {
        nonstrict: true,
        ..ModelSchemaArgs::default()
    };
    record(&later, &nonstrict);
    assert!(!takes_only(&holder.fields, &holder.generics));
    let (classes, refused) = classes_of(
        "#[derive(Serialize, Deserialize)]\npub struct Holder<T> { #[serde(flatten)] pub m: T }",
        &[],
    );
    assert_eq!(
        classes,
        [Class::Unseen("the type parameter `T`".to_owned())]
    );
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(
        UNSEEN & (REST | REFUSES | ABSORBS | EMPTY | ENTRY | NO_OBJECT),
        0
    );
}

#[cfg(any(feature = "dart", feature = "swift"))]
#[test]
fn a_member_whose_type_flattens_one_declared_below_it_is_refused_beside_another_key() {
    let refused = sole_refusal(
        "#[derive(Serialize, Deserialize)]\npub struct Mid { pub m: u32, #[serde(flatten)] pub \
         below: MidBelow }\n#[derive(Serialize, Deserialize)]\npub struct Top { pub t: u32, \
         #[serde(flatten)] pub mid: Mid }",
    );
    assert!(
        refused.contains(
            "model_schema: type `Top` flattens `mid` (`Mid`) beside other keys, and `Mid` holds \
             `MidBelow`, which was declared below `Mid`, in another crate, or with no \
             `#[model_schema]`: the Dart and Swift reads of `Mid` were written before they could \
             see which keys `MidBelow` reads, and hand it the keys beside `mid` that serde does \
             not. Declare `MidBelow` above `Mid`, or write `mid` under a key of its own."
        ),
        "{refused}"
    );
}

#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
#[test]
fn a_closed_holder_of_a_generic_member_declared_below_it_is_warned_unless_nonstrict() {
    let source = "#[derive(Serialize, Deserialize)] pub struct Haulage { pub label: String, \
                  #[serde(flatten)] pub haulier: Haulier<String> }";
    let strict = expansion("", source);
    assert!(
        strict.contains(
            "type `Haulage` flattens `haulier` (`Haulier<String>`), a generic type declared below \
             `Haulage` or in another crate: the Dart, Kotlin and Swift reads of `Haulage` cannot \
             learn which keys it takes, so they read past a key no member declares, which the \
             Rust read refuses. Avoid this shape where you can: declare `Haulier` above \
             `Haulage`, write `haulier` under a key of its own, or mark `Haulage` \
             `#[model_schema(nonstrict)]` so that every surface reads past the key."
        ),
        "{strict}"
    );
    assert!(strict.contains("deprecated"), "{strict}");
    let open = expansion("nonstrict", source);
    assert!(!open.contains("TIXSCHEMA_READS_PAST_A_STRAY_KEY"), "{open}");
}

#[test]
fn each_shape_no_holder_may_flatten_publishes_its_bit() {
    for (source, bit) in [
        ("pub enum Colour { Red, Blue }", PLAIN),
        (
            "pub struct Shelves { pub by_shelf: BTreeMap<u32, u32> }",
            TEXT_KEYS,
        ),
        (
            "pub struct MaybeTally(pub Option<BTreeMap<String, u32>>);",
            OPTION_EMPTY,
        ),
    ] {
        let bits = published_bits(&format!("#[derive(Serialize, Deserialize)] {source}"));
        assert_eq!(bits & bit, bit, "{source}: {bits}");
    }
    let plain = published_bits("#[derive(Serialize, Deserialize)] pub struct Pin { pub a: u32 }");
    assert_eq!(plain & (PLAIN | TEXT_KEYS | OPTION_EMPTY), 0);
}

#[test]
fn each_type_publishes_the_keys_a_holder_sees_it_write_or_read() {
    let expanded = expansion(
        "",
        "#[derive(Serialize, Deserialize)] pub struct Ink { pub hue: String, #[serde(alias = \
         \"shade\")] pub depth: u32 }",
    );
    assert!(
        expanded.contains(
            "__TIXSCHEMA_KEYS : & 'static [& 'static str] = & [\"hue\" , \"depth\" , \"shade\"]"
        ),
        "{expanded}"
    );
    let tagged = expansion(
        "",
        "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum Mark { Line { \
         length: u32 }, Inked(InkBelowMark) }",
    );
    assert!(
        tagged.contains("< InkBelowMark > :: __TIXSCHEMA_KEYS"),
        "{tagged}"
    );
}

#[test]
fn a_type_that_flattens_itself_publishes_no_key() {
    let first = expansion(
        "",
        "#[derive(Serialize, Deserialize)] pub struct LoopFirst { pub first: String, \
         #[serde(flatten)] pub second: LoopSecond }",
    );
    assert!(
        first.contains("< LoopSecond > :: __TIXSCHEMA_KEYS"),
        "{first}"
    );
    let second = expansion(
        "",
        "#[derive(Serialize, Deserialize)] pub struct LoopSecond { pub second: String, \
         #[serde(flatten)] pub first: Box<LoopFirst> }",
    );
    assert!(
        second.contains("__TIXSCHEMA_KEYS : & 'static [& 'static str] = & []"),
        "{second}"
    );
}

#[test]
fn a_holder_of_a_member_declared_below_compares_their_keys_in_a_const() {
    let expanded = expansion(
        "",
        "#[derive(Serialize, Deserialize)] pub struct Shipment { pub grams: u32, \
         #[serde(flatten)] pub weight: WeightBelow }",
    );
    assert!(
        expanded
            .contains("tixschema_shared_key (& [\"grams\"] , < WeightBelow > :: __TIXSCHEMA_KEYS)"),
        "{expanded}"
    );
    assert!(
        expanded.contains(
            "model_schema: type `Shipment` has field `grams` and flattened `weight` \
             (`WeightBelow`), one of them declared below"
        ),
        "{expanded}"
    );
}

#[test]
fn a_flattened_option_of_a_member_that_refuses_a_key_is_refused() {
    let refused = sole_refusal(
        "#[derive(Serialize, Deserialize)]\n#[serde(tag = \"kind\")]\npub enum Mark { Line { \
         length: u32 } }\n#[derive(Serialize, Deserialize)]\npub struct Parcel { pub label: String, \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub mark: \
         Option<Mark> }",
    );
    assert!(
        refused.contains(
            "model_schema: type `Parcel` flattens `mark` (`Option<Mark>`): `Mark` reads every key \
             still left and refuses one it does not declare"
        ),
        "{refused}"
    );
    assert!(
        refused.contains(
            "Mark `Mark` `#[model_schema(nonstrict)]`, flatten `Mark` without the `Option`, or \
             write `mark` under a key of its own."
        ),
        "{refused}"
    );
    let (_, open) = classes_of(
        "#[derive(Serialize, Deserialize)]\n#[serde(tag = \"kind\")]\npub enum Mark { Line { \
         length: u32 } }\n#[derive(Serialize, Deserialize)]\npub struct Parcel { pub label: String, \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub mark: \
         Option<Mark> }",
        &["Mark"],
    );
    assert_eq!(open, Vec::<String>::new());
}

#[test]
fn an_enum_held_open_by_a_flattened_member_is_refused_where_another_level_would_refuse() {
    let mark = "#[derive(Serialize, Deserialize)]\n#[serde(tag = \"kind\")]\npub enum Mark { Line { \
                length: u32 } }\n";
    for (enum_source, level) in [
        (
            "#[serde(tag = \"form\", content = \"body\")] pub enum Label { Marked { \
             #[serde(flatten)] mark: Mark } }",
            "beside the tag `form` and the content key `body`",
        ),
        (
            "pub enum Label { Marked { #[serde(flatten)] mark: Mark }, Plain { note: String } }",
            "in variant `Plain`",
        ),
        (
            "#[serde(tag = \"form\")] pub enum Label { Marked { #[serde(flatten)] mark: Mark }, \
             Blank }",
            "beside the tag in variant `Blank`",
        ),
    ] {
        let refused = sole_refusal(&format!(
            "{mark}#[derive(Serialize, Deserialize)]\n{enum_source}"
        ));
        assert!(
            refused.contains(
                "model_schema: type `Label`: variant `Marked` flattens `mark` (`Mark`), which reads \
                 every key still left"
            ) && refused.contains(level),
            "{enum_source}: {refused}"
        );
    }
    let (_, alone) = classes_of(
        &format!(
            "{mark}#[derive(Serialize, Deserialize)]\n#[serde(tag = \"form\")] pub enum Label {{ \
             Marked {{ #[serde(flatten)] mark: Mark }} }}"
        ),
        &[],
    );
    assert_eq!(alone, Vec::<String>::new());
}
