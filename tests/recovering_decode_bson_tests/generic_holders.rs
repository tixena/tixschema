//! A check of a model held by a generic holder under the holder's own type parameter, or by a
//! recursive one under `Self`, is listed in a BSON document as one `Invalid` at its full path, as
//! it is below a member with a concrete filling.

use core::fmt::Display;
use truck_schema::DecodeWith as _;

use bson::{Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A checked model with a type parameter.
#[model_schema(enable = ["decode_with", "mongodb"], default_types(CodeType = String), check = ["withinTote"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned"))]
struct Tote<CodeType: Display> {
    fragile_item_codes: Vec<String>,
    item_code_list: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<CodeType>,
}

/// Holds a tote under its own type parameter.
#[model_schema(enable = ["decode_with", "mongodb"], default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned"))]
struct Truck<CodeType: Display> {
    load: Tote<CodeType>,
}

/// Holds a tote with a concrete filling.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Van {
    load: Tote<String>,
}

/// Holds a list of totes under its own type parameter, held to a count of its own.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(enable = ["decode_with", "mongodb"], default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned"))]
struct Convoy<CodeType: Display> {
    #[model_schema_prop(minLength = 3)]
    loads: Vec<Tote<CodeType>>,
}

/// [`Convoy`] with a concrete filling.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct StringConvoy {
    #[model_schema_prop(minLength = 3)]
    loads: Vec<Tote<String>>,
}

/// A recursive checked type.
#[model_schema(enable = ["decode_with", "mongodb"], check = ["withinCarton"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Carton {
    cartons: Vec<Self>,
    fragile_item_codes: Vec<String>,
    item_code_list: Vec<String>,
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

fn within_carton(value: &Carton) -> Result<(), Vec<String>> {
    outside(&value.item_code_list, &value.fragile_item_codes)
}

/// A tote whose second fragile code is outside of its list.
fn broken() -> Document {
    doc! { "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] }
}

/// The line the check of [`broken`] is listed as, held at `path`.
fn listed_at(path: &str) -> String {
    format!("{path}: invalid: expected Unknown, found String(\"y\"): y is outside of itemCodeList")
}

/// One line per issue, as `Unrecovered` displays them.
#[rustversion::since(1.98)]
fn lines<T, E>(read: Result<T, E>) -> Vec<String>
where
    E: ToString,
{
    read.err()
        .map_or_default(|refused| refused.to_string().lines().map(str::to_owned).collect())
}

#[rustversion::before(1.98)]
fn lines<T, E>(read: Result<T, E>) -> Vec<String>
where
    E: ToString,
{
    read.err()
        .map(|refused| refused.to_string().lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

#[test]
fn a_check_held_under_the_holders_type_parameter_is_listed_at_its_path() {
    let read = Truck::<String>::from_bson_with(doc! { "load": broken() }, |_, _| {
        truck_schema::Verdict::Reject
    });
    assert_eq!(lines(read), [listed_at("load.fragileItemCodes[1]")]);
}

#[test]
fn a_check_held_under_a_list_of_self_is_listed_at_its_path() {
    let inner = doc! { "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"], "cartons": [] };
    let read = Carton::from_bson_with(
        doc! { "itemCodeList": ["x"], "fragileItemCodes": [], "cartons": [inner] },
        |_, _| carton_schema::Verdict::Reject,
    );
    assert_eq!(lines(read), [listed_at("cartons[0].fragileItemCodes[1]")]);
}

#[test]
fn a_check_held_with_a_concrete_filling_is_listed_at_its_path_as_it_was() {
    let read = Van::from_bson_with(doc! { "load": broken() }, |_, _| {
        van_schema::Verdict::Reject
    });
    assert_eq!(lines(read), [listed_at("load.fragileItemCodes[1]")]);
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_counted_list_held_under_the_holders_type_parameter_lists_the_issues_of_a_concrete_filling() {
    let sound = doc! { "itemCodeList": ["x"], "fragileItemCodes": ["x"] };
    let stored = doc! { "loads": [sound, broken()] };
    let generic = lines(Convoy::<String>::from_bson_with(stored.clone(), |_, _| {
        convoy_schema::Verdict::Reject
    }));
    let concrete = lines(StringConvoy::from_bson_with(stored, |_, _| {
        string_convoy_schema::Verdict::Reject
    }));
    assert_eq!(generic, concrete);
    assert_eq!(generic.len(), 2, "the count and the check: {generic:?}");
    assert!(
        generic.contains(&listed_at("loads[1].fragileItemCodes[1]")),
        "got: {generic:?}"
    );
}
