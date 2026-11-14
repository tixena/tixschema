//! A value behind a type parameter is read unjudged by the holder that writes what fills the
//! parameter, and judged once with the rest of the holder, each message at its full path.

/// The types the guide shows a holder judging what fills a parameter with.
mod guide {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Parcel {
        #[model_schema_prop(minLength = 1)]
        pub item_label: String,
    }

    #[model_schema(default_types(Item = String))]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Page<Item> {
        pub items: Vec<Item>,
        #[model_schema_prop(minLength = 1)]
        pub page_title: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Shelf {
        pub first_page: Page<Parcel>,
    }
}

use core::fmt::Debug;
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A parcel that keeps every rule.
const SOUND: &str = r#"{"fragileItemCodes":[],"itemCodeList":[],"itemLabel":"a"}"#;

/// A parcel that breaks its bound and its check.
const BROKEN: &str = r#"{"fragileItemCodes":["y"],"itemCodeList":[],"itemLabel":""}"#;

/// A parcel that breaks its bound alone.
const UNLABELLED: &str = r#"{"fragileItemCodes":[],"itemCodeList":[],"itemLabel":""}"#;

/// A model with a bound and a check.
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

/// A string brand with a bound and a check.
#[model_schema(minLength = 2, check = ["labelInUpperCase"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Label(String);

/// A generic struct with a rule of its own.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<Item> {
    pub items: Vec<Item>,
    #[model_schema_prop(minLength = 1)]
    pub page_title: String,
}

/// A generic enum with no rule of its own.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Slot<Item> {
    Empty,
    Filled { held: Item, spares: Vec<Item> },
    Single(Item),
}

/// A generic struct that holds another at its own parameter, and a map of it.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cabinet<Held> {
    pub by_dock: HashMap<String, Held>,
    pub page: Page<Held>,
}

/// Two parameters.
#[model_schema(default_types(First = String, Second = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pair<First, Second> {
    pub first: First,
    pub second: Vec<Second>,
}

/// A generic type that holds itself, written as `Self`, with a check of its own.
#[model_schema(default_types(Id = String), check = ["atMostTwoBranches"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Branch<Id> {
    pub branches: Vec<Self>,
    pub id: Id,
}

/// An untagged generic enum, whose variant serde chooses by what reads.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Either<Item> {
    Labelled(Item),
    Plain(String),
}

/// A brand over its own parameter.
#[model_schema(default_types(Inner = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Wrapper<Inner>(Inner);

/// A generic type no `#[model_schema]` was written on.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Foreign<Item> {
    pub held: Vec<Item>,
}

/// The holder of each shape, every member written with what fills the parameters.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shelf {
    pub binder: Page<Page<Parcel>>,
    pub branch: Branch<Parcel>,
    pub cabinet: Cabinet<Parcel>,
    pub either: Either<Label>,
    pub first_page: Page<Parcel>,
    pub foreign: Foreign<Parcel>,
    pub labels: Page<Label>,
    pub lists: Page<Vec<Parcel>>,
    pub pair: Pair<Parcel, Label>,
    pub slot: Slot<Parcel>,
    pub slots: Page<Slot<Parcel>>,
    pub stack: Vec<Page<Parcel>>,
    pub words: Page<String>,
    pub wrapped: Wrapper<Parcel>,
}

fn within_parcel(value: &ParcelDraft) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() {
        Ok(())
    } else {
        Err(outside)
    }
}

fn label_in_upper_case(value: &Label) -> Result<(), Vec<String>> {
    if value.as_inner().chars().any(char::is_lowercase) {
        return Err(vec!["a label is upper case".to_owned()]);
    }
    Ok(())
}

fn at_most_two_branches<Id>(value: &BranchDraft<Id>) -> Result<(), Vec<String>> {
    if value.branches.len() > 2 {
        return Err(vec!["'branches': a branch holds at most two".to_owned()]);
    }
    Ok(())
}

fn page(items: &[&str]) -> String {
    format!(r#"{{"items":[{}],"pageTitle":"t"}}"#, items.join(","))
}

/// A shelf every member of which keeps every rule, but the ones `changed` writes.
fn shelf(changed: &[(&str, String)]) -> String {
    let mut members: Vec<(&str, String)> = vec![
        ("binder", page(&[])),
        ("branch", format!(r#"{{"branches":[],"id":{SOUND}}}"#)),
        (
            "cabinet",
            format!(r#"{{"byDock":{{}},"page":{}}}"#, page(&[])),
        ),
        ("either", r#""AB""#.to_owned()),
        ("firstPage", page(&[SOUND])),
        ("foreign", r#"{"held":[]}"#.to_owned()),
        ("labels", page(&[])),
        ("lists", page(&[])),
        ("pair", format!(r#"{{"first":{SOUND},"second":[]}}"#)),
        ("slot", r#""empty""#.to_owned()),
        ("slots", page(&[])),
        ("stack", "[]".to_owned()),
        ("words", page(&[])),
        ("wrapped", SOUND.to_owned()),
    ];
    for (key, written) in changed {
        for member in &mut members {
            if member.0 == *key {
                member.1.clone_from(written);
            }
        }
    }
    let written: Vec<String> = members
        .iter()
        .map(|(key, value)| format!(r#""{key}":{value}"#))
        .collect();
    format!("{{{}}}", written.join(","))
}

/// What `text` is refused with as it is read as a `T`, each message on a line of its own.
fn refused<T>(text: &str) -> Vec<String>
where
    T: DeserializeOwned + Debug,
{
    serde_json::from_str::<T>(text)
        .unwrap_err()
        .to_string()
        .split("; ")
        .map(ToOwned::to_owned)
        .collect()
}

/// What a shelf with `changed` written is refused with.
fn shelf_refused(changed: &[(&str, String)]) -> Vec<String> {
    refused::<Shelf>(&shelf(changed))
}

#[test]
fn a_sound_shelf_reads_and_writes_back_the_same_text() {
    let text = shelf(&[]);
    let read: Shelf = serde_json::from_str(&text).unwrap();
    let again: Shelf = serde_json::from_str(&serde_json::to_string(&read).unwrap()).unwrap();
    assert_eq!(read, again);
    ShelfDraft::from(read).validate().unwrap();
}

#[test]
fn every_broken_value_behind_a_parameter_is_named_at_its_full_path() {
    assert_eq!(
        shelf_refused(&[("firstPage", page(&[UNLABELLED, SOUND, UNLABELLED]))]),
        [
            "'firstPage.items.0.itemLabel': too short: minimum length is 1, got 0",
            "'firstPage.items.2.itemLabel': too short: minimum length is 1, got 0",
        ]
    );
    assert_eq!(
        shelf_refused(&[("firstPage", page(&[BROKEN, SOUND, BROKEN]))]),
        [
            "'firstPage.items.0.itemLabel': too short: minimum length is 1, got 0",
            "'firstPage.items.0.fragileItemCodes.0': y is outside of itemCodeList",
            "'firstPage.items.2.itemLabel': too short: minimum length is 1, got 0",
            "'firstPage.items.2.fragileItemCodes.0': y is outside of itemCodeList",
        ]
    );
}

#[test]
fn a_generic_enum_names_the_variant_and_the_member_that_holds_the_value() {
    assert_eq!(
        shelf_refused(&[(
            "slot",
            format!(r#"{{"filled":{{"held":{UNLABELLED},"spares":[{SOUND},{UNLABELLED}]}}}}"#)
        )]),
        [
            "'slot.filled.held.itemLabel': too short: minimum length is 1, got 0",
            "'slot.filled.spares.1.itemLabel': too short: minimum length is 1, got 0",
        ]
    );
}

#[test]
fn a_generic_model_filled_with_another_hands_its_check_on() {
    assert_eq!(
        shelf_refused(&[(
            "binder",
            page(&[&page(&[SOUND, UNLABELLED]), &page(&[UNLABELLED])])
        )]),
        [
            "'binder.items.0.items.1.itemLabel': too short: minimum length is 1, got 0",
            "'binder.items.1.items.0.itemLabel': too short: minimum length is 1, got 0",
        ]
    );
    assert_eq!(
        shelf_refused(&[(
            "slots",
            page(&[&format!(r#"{{"single":{UNLABELLED}}}"#), r#""empty""#])
        )]),
        ["'slots.items.0.single.itemLabel': too short: minimum length is 1, got 0"]
    );
    assert_eq!(
        shelf_refused(&[(
            "cabinet",
            format!(
                r#"{{"byDock":{{"north":{UNLABELLED}}},"page":{}}}"#,
                page(&[UNLABELLED])
            )
        )]),
        [
            "'cabinet.byDock.north.itemLabel': too short: minimum length is 1, got 0",
            "'cabinet.page.items.0.itemLabel': too short: minimum length is 1, got 0",
        ]
    );
}

#[test]
fn a_brand_behind_a_parameter_is_named_at_its_full_path() {
    assert_eq!(
        shelf_refused(&[("labels", page(&[r#""a""#, r#""OK""#, r#""b""#]))]),
        [
            "'labels.items.0': too short: minimum length is 2, got 1",
            "'labels.items.0': a label is upper case",
            "'labels.items.2': too short: minimum length is 2, got 1",
            "'labels.items.2': a label is upper case",
        ]
    );
}

#[test]
fn a_parameter_filled_with_a_primitive_leaves_the_models_own_rule() {
    assert_eq!(
        shelf_refused(&[("words", r#"{"items":["a"],"pageTitle":""}"#.to_owned())]),
        ["'words.pageTitle': too short: minimum length is 1, got 0"]
    );
}

#[test]
fn each_of_two_parameters_is_handed_its_own_check() {
    assert_eq!(
        shelf_refused(&[(
            "pair",
            format!(r#"{{"first":{UNLABELLED},"second":["OK","b"]}}"#)
        )]),
        [
            "'pair.first.itemLabel': too short: minimum length is 1, got 0",
            "'pair.second.1': too short: minimum length is 2, got 1",
            "'pair.second.1': a label is upper case",
        ]
    );
}

#[test]
fn a_list_filling_a_parameter_and_a_list_of_a_generic_model_name_each_index() {
    assert_eq!(
        shelf_refused(&[("lists", page(&[&format!("[{SOUND},{UNLABELLED}]")]))]),
        ["'lists.items.0.1.itemLabel': too short: minimum length is 1, got 0"]
    );
    assert_eq!(
        shelf_refused(&[(
            "stack",
            format!("[{},{}]", page(&[SOUND]), page(&[UNLABELLED]))
        )]),
        ["'stack.1.items.0.itemLabel': too short: minimum length is 1, got 0"]
    );
}

#[test]
fn a_generic_type_written_as_self_hands_its_check_to_each_level() {
    assert_eq!(
        shelf_refused(&[(
            "branch",
            format!(r#"{{"branches":[{{"branches":[],"id":{UNLABELLED}}}],"id":{UNLABELLED}}}"#)
        )]),
        [
            "'branch.branches.0.id.itemLabel': too short: minimum length is 1, got 0",
            "'branch.id.itemLabel': too short: minimum length is 1, got 0",
        ]
    );
}

#[test]
fn a_brand_over_its_own_parameter_hands_the_value_to_its_holder() {
    assert_eq!(
        shelf_refused(&[("wrapped", BROKEN.to_owned())]),
        [
            "'wrapped.itemLabel': too short: minimum length is 1, got 0",
            "'wrapped.fragileItemCodes.0': y is outside of itemCodeList",
        ]
    );
}

#[test]
fn an_untagged_generic_enum_chooses_as_serdes_read_of_it_chooses() {
    // `Labelled` reads any string with the unjudged reader alone, so it is chosen, and the holder
    // refuses what it holds.
    assert_eq!(
        shelf_refused(&[("either", r#""a""#.to_owned())]),
        [
            "'either': too short: minimum length is 2, got 1",
            "'either': a label is upper case",
        ]
    );
    assert_eq!(
        shelf_refused(&[("either", r#""ab""#.to_owned())]),
        ["'either': a label is upper case"]
    );
    let read: Shelf = serde_json::from_str(&shelf(&[])).unwrap();
    assert_eq!(
        serde_json::to_string(&read.either).unwrap(),
        r#""AB""#,
        "a label that keeps its rules is read as one"
    );
    assert!(matches!(&*read.either, EitherDraft::Labelled(_)));
}

#[test]
fn a_generic_type_no_model_schema_was_written_on_judges_what_it_holds_as_it_reads() {
    assert_eq!(
        shelf_refused(&[(
            "foreign",
            format!(r#"{{"held":[{UNLABELLED},{UNLABELLED}]}}"#)
        )]),
        ["'itemLabel': too short: minimum length is 1, got 0 at line 1 column 375"]
    );
}

#[test]
fn a_generic_model_read_by_itself_judges_what_fills_its_parameter_as_it_reads() {
    assert_eq!(
        refused::<Page<Parcel>>(&page(&[BROKEN, SOUND, BROKEN])),
        [
            "'itemLabel': too short: minimum length is 1, got 0",
            "'fragileItemCodes.0': y is outside of itemCodeList at line 1 column 70",
        ]
    );
}

#[test]
fn a_refusal_serde_raises_itself_is_serdes_unchanged() {
    for (written, said) in [
        (
            "5".to_owned(),
            "invalid type: integer `5`, expected struct Page",
        ),
        (
            r#"{"items":[],"pageTitle":"t","x":1}"#.to_owned(),
            "unknown field `x`, expected `items` or `pageTitle`",
        ),
        (r#"{"items":[]}"#.to_owned(), "missing field `pageTitle`"),
        (
            page(&[r#"{"fragileItemCodes":[],"itemCodeList":[],"itemLabel":5}"#]),
            "invalid type: integer `5`, expected a string",
        ),
    ] {
        let refusal = serde_json::from_str::<Shelf>(&shelf(&[("firstPage", written)]))
            .unwrap_err()
            .to_string();
        assert!(refusal.starts_with(said), "got: {refusal}");
    }
}

#[test]
fn a_value_built_in_code_is_judged_with_what_fills_its_parameters() {
    let unjudged = |label: &str| {
        Parcel::model_schema_wrap_unjudged(ParcelDraft {
            fragile_item_codes: Vec::new(),
            item_code_list: Vec::new(),
            item_label: label.to_owned(),
        })
    };
    let first_page = Page::model_schema_wrap_unjudged(PageDraft {
        items: vec![unjudged("a"), unjudged("")],
        page_title: "t".to_owned(),
    });
    assert_eq!(
        first_page.model_schema_issues_with1(&|parcel| parcel.validation_issues()),
        ["'items.1.itemLabel': too short: minimum length is 1, got 0"]
    );
    assert!(
        first_page.validation_issues().is_empty(),
        "a generic model's own `validate()` reaches nothing behind its parameter"
    );
}

#[test]
fn the_guide_shows_the_holder_judging_what_fills_a_parameter() {
    let payload = r#"{"firstPage":{"items":[{"itemLabel":""},{"itemLabel":"a"},{"itemLabel":""}],"pageTitle":"t"}}"#;
    let refusal = serde_json::from_str::<guide::Shelf>(payload)
        .unwrap_err()
        .to_string();
    let shown = format!("serde_json::from_str::<Shelf>(r#\"{payload}\"#);\n// Err: {refusal}\n");
    assert!(
        include_str!("../../docs/guide/validation.md").contains(&shown),
        "the guide no longer shows:\n{shown}"
    );
}
