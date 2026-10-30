//! A check of a model held by a generic holder under the holder's own type parameter, or by a
//! recursive one under `Self`, is listed as one `Invalid` at its full path, as it is below a
//! member with a concrete filling.

use core::fmt::Display;
use std::collections::HashMap;
use truck_schema::DecodeWith as _;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

/// A checked model with a type parameter.
#[model_schema(enable = ["decode_with"], default_types(CodeType = String), check = ["withinTote"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned"))]
struct Tote<CodeType: Display> {
    fragile_item_codes: Vec<String>,
    item_code_list: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<CodeType>,
}

/// Holds a tote under its own type parameter, itself, in a map and in an `Option`.
#[model_schema(enable = ["decode_with"], default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned"))]
struct Truck<CodeType: Display> {
    #[serde(default)]
    bays: HashMap<String, Tote<CodeType>>,
    load: Tote<CodeType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spare: Option<Tote<CodeType>>,
}

/// Holds a tote with a concrete filling.
#[model_schema(enable = ["decode_with"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Van {
    load: Tote<String>,
}

/// Holds a list of totes under its own type parameter, held to a count of its own.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(enable = ["decode_with"], default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned"))]
struct Convoy<CodeType: Display> {
    #[model_schema_prop(minLength = 3)]
    loads: Vec<Tote<CodeType>>,
}

/// [`Convoy`] with a concrete filling.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(enable = ["decode_with"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct StringConvoy {
    #[model_schema_prop(minLength = 3)]
    loads: Vec<Tote<String>>,
}

/// A recursive checked type.
#[model_schema(enable = ["decode_with"], check = ["withinCarton"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Carton {
    cartons: Vec<Self>,
    fragile_item_codes: Vec<String>,
    item_code_list: Vec<String>,
}

/// A model with a parameter whose member carries a bound, which the walk lists.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(enable = ["decode_with"], default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned"))]
struct SealedTote<CodeType: Display> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    code: Option<CodeType>,
    #[model_schema_prop(minLength = 1)]
    seal: String,
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(enable = ["decode_with"], default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned"))]
struct SealedTruck<CodeType: Display> {
    load: SealedTote<CodeType>,
}

/// A recursive checked type whose list of itself is held to a count.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(enable = ["decode_with"], check = ["withinStack"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Stack {
    fragile_item_codes: Vec<String>,
    item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 2)]
    stacks: Vec<Self>,
}

fn outside(list: &[String], fragile: &[String]) -> Result<(), Vec<String>> {
    let outside: Vec<String> = fragile
        .iter()
        .enumerate()
        .filter(|(_, code)| !list.contains(code))
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

fn within_tote<CodeType>(value: &Tote<CodeType>) -> Result<(), Vec<String>>
where
    CodeType: Display,
{
    outside(&value.item_code_list, &value.fragile_item_codes)
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn within_stack(value: &Stack) -> Result<(), Vec<String>> {
    outside(&value.item_code_list, &value.fragile_item_codes)
}

fn within_carton(value: &Carton) -> Result<(), Vec<String>> {
    outside(&value.item_code_list, &value.fragile_item_codes)
}

/// A tote whose second fragile code is outside of its list.
fn broken() -> Value {
    json!({ "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] })
}

/// The issue the check of [`broken`] is listed as, held at `path`.
fn listed_at(path: &str) -> String {
    format!(
        "Invalid {{ path: Path([{path}]), expected: Unknown, found: String(\"y\"), reason: \"y is \
         outside of itemCodeList\" }}"
    )
}

/// The issues `stored` is listed with as it is read as a `Truck<String>`.
fn listed_by_truck(stored: Value) -> String {
    let mut listed = String::new();
    let read = Truck::<String>::from_value_with(stored, |_, issues| {
        listed = format!("{issues:?}");
        truck_schema::Verdict::Reject
    });
    read.unwrap_err();
    listed
}

#[test]
fn a_check_held_under_the_holders_type_parameter_is_listed_at_its_path() {
    assert_eq!(
        listed_by_truck(json!({ "load": broken() })),
        format!(
            "[{}]",
            listed_at(r#"Key("load"), Key("fragileItemCodes"), Index(1)"#)
        )
    );
}

#[test]
fn a_check_held_under_a_list_of_self_is_listed_at_its_path() {
    let inner = json!({ "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"], "cartons": [] });
    let mut listed = String::new();
    let read = Carton::from_value_with(
        json!({ "itemCodeList": ["x"], "fragileItemCodes": [], "cartons": [inner] }),
        |_, issues| {
            listed = format!("{issues:?}");
            carton_schema::Verdict::Reject
        },
    );
    read.unwrap_err();
    assert_eq!(
        listed,
        format!(
            "[{}]",
            listed_at(r#"Key("cartons"), Index(0), Key("fragileItemCodes"), Index(1)"#)
        )
    );
}

#[test]
fn a_check_held_under_the_holders_type_parameter_in_an_option_and_in_a_map_is_listed_at_its_path() {
    let sound = json!({ "itemCodeList": ["x"], "fragileItemCodes": ["x"] });
    assert_eq!(
        listed_by_truck(json!({ "load": sound, "spare": broken() })),
        format!(
            "[{}]",
            listed_at(r#"Key("spare"), Key("fragileItemCodes"), Index(1)"#)
        )
    );
    assert_eq!(
        listed_by_truck(json!({ "load": sound, "bays": { "north": broken() } })),
        format!(
            "[{}]",
            listed_at(r#"Key("bays"), Key("north"), Key("fragileItemCodes"), Index(1)"#)
        )
    );
}

#[test]
fn a_check_held_with_a_concrete_filling_is_listed_at_its_path_as_it_was() {
    let mut listed = String::new();
    let read = Van::from_value_with(json!({ "load": broken() }), |_, issues| {
        listed = format!("{issues:?}");
        van_schema::Verdict::Reject
    });
    read.unwrap_err();
    assert_eq!(
        listed,
        format!(
            "[{}]",
            listed_at(r#"Key("load"), Key("fragileItemCodes"), Index(1)"#)
        )
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_bound_held_under_the_holders_type_parameter_is_listed_at_its_path_as_it_was() {
    let mut listed = String::new();
    let read =
        SealedTruck::<String>::from_value_with(json!({ "load": { "seal": "" } }), |_, issues| {
            listed = format!("{issues:?}");
            sealed_truck_schema::Verdict::Reject
        });
    read.unwrap_err();
    assert_eq!(
        listed,
        "[Invalid { path: Path([Key(\"load\"), Key(\"seal\")]), expected: String, found: \
         String(\"\"), reason: \"too short: minimum length is 1, got 0\" }]"
    );
}

/// A plain read of the holder names the broken check under the holder's path.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_plain_read_of_the_holder_names_the_check_at_its_full_path() {
    assert_eq!(
        serde_json::from_value::<Truck<String>>(json!({ "load": broken() }))
            .unwrap_err()
            .to_string(),
        "'load.fragileItemCodes.1': y is outside of itemCodeList"
    );
}

/// A payload that holds fewer totes than the list's count asks, the second of which breaks its
/// check.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn too_few_and_broken() -> Value {
    let sound = json!({ "itemCodeList": ["x"], "fragileItemCodes": ["x"] });
    json!({ "loads": [sound, broken()] })
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_counted_list_held_under_the_holders_type_parameter_lists_the_issues_of_a_concrete_filling() {
    let mut generic = String::new();
    Convoy::<String>::from_value_with(too_few_and_broken(), |_, issues| {
        generic = format!("{issues:?}");
        convoy_schema::Verdict::Reject
    })
    .unwrap_err();
    let mut concrete = String::new();
    StringConvoy::from_value_with(too_few_and_broken(), |_, issues| {
        concrete = format!("{issues:?}");
        string_convoy_schema::Verdict::Reject
    })
    .unwrap_err();
    assert_eq!(generic, concrete);
    assert!(
        generic.contains("too short: minimum length is 3, got 2"),
        "the count of the list is listed: {generic}"
    );
    assert!(
        generic.contains(&listed_at(
            r#"Key("loads"), Index(1), Key("fragileItemCodes"), Index(1)"#
        )),
        "the check of the second tote is listed at its path: {generic}"
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_counted_list_of_self_lists_its_count_and_the_check_of_the_stack_it_holds() {
    let mut inner = broken();
    inner["stacks"] = json!([]);
    let mut listed = String::new();
    Stack::from_value_with(
        json!({ "itemCodeList": [], "fragileItemCodes": [], "stacks": [inner] }),
        |_, issues| {
            listed = format!("{issues:?}");
            stack_schema::Verdict::Reject
        },
    )
    .unwrap_err();
    assert!(
        listed.contains("too short: minimum length is 2, got 1"),
        "the count of the list is listed: {listed}"
    );
    assert!(
        listed.contains(&listed_at(
            r#"Key("stacks"), Index(0), Key("fragileItemCodes"), Index(1)"#
        )),
        "the check of the stack it holds is listed at its path: {listed}"
    );
}
