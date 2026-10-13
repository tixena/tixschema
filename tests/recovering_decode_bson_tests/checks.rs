//! A check a type or a field names is the recovering read's to list, in a BSON document as in a
//! JSON value: serde builds the value, the checks run on it, and each message is one `Invalid`
//! at the place it names in the document.

use std::collections::HashMap;

use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(decode_with, check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckedParcel {
    fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    label: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct CheckedManifest {
    parcel: CheckedParcel,
}

/// A parcel held in a list, in an `Option` and in a map.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckedDepot {
    #[serde(default)]
    parcel_list: Vec<CheckedParcel>,
    #[serde(default)]
    parcels_by_bay: HashMap<String, CheckedParcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spare_parcel: Option<CheckedParcel>,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all_fields = "camelCase")]
enum CheckedExternal {
    Rail(#[model_schema_prop(check = ["knownLine"])] String),
    Road {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
enum CheckedInternal {
    Road {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]
enum CheckedAdjacent {
    Rail(#[model_schema_prop(check = ["knownLine"])] String),
    Road {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
enum CheckedUntagged {
    Road {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
}

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

fn within_parcel(value: &CheckedParcel) -> Result<(), Vec<String>> {
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

fn known_line(value: &str) -> Result<(), Vec<String>> {
    if value == "unknown" {
        return Err(vec!["a line is known".to_owned()]);
    }
    Ok(())
}

fn short_list(value: &[String]) -> Result<(), Vec<String>> {
    value.get(2).map_or(Ok(()), |third| {
        Err(vec![format!("'2': {third} is one code too many")])
    })
}

fn duplicated() -> Document {
    doc! { "label": "A", "itemCodeList": ["x", "x"], "fragileItemCodes": ["x"] }
}

fn outside() -> Document {
    doc! { "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] }
}

fn sound() -> Document {
    doc! { "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x"] }
}

/// One line per issue, as `Unrecovered` displays them.
fn lines<T, E>(read: Result<T, E>) -> Vec<String>
where
    E: ToString,
{
    read.err()
        .map(|refused| refused.to_string().lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// Reads `stored` as a parcel with a callback that rejects, and answers every issue it listed.
fn listed(stored: Document) -> Vec<String> {
    lines(CheckedParcel::from_bson_with(stored, |_, _| {
        checked_parcel_schema::Verdict::Reject
    }))
}

/// A resolver that takes the value a check refused out of the document.
fn dropping_what_is_outside(
    raw: &mut Document,
    issue: &checked_parcel_schema::Issue<Bson>,
) -> checked_parcel_schema::Resolution {
    let checked_parcel_schema::Issue::Invalid {
        path,
        expected: checked_parcel_schema::Expected::Unknown,
        found: _found,
        reason: _reason,
    } = issue
    else {
        return checked_parcel_schema::Resolution::NotTouched;
    };
    if path.remove_from_document(raw) {
        checked_parcel_schema::Resolution::Settled
    } else {
        checked_parcel_schema::Resolution::Rejected
    }
}

#[test]
fn a_broken_check_is_one_invalid_at_the_place_its_message_names() {
    assert_eq!(
        listed(duplicated()),
        [
            "itemCodeList: invalid: expected Unknown, found Array([String(\"x\"), String(\"x\")]): \
             duplicate item code x"
        ]
    );
    assert_eq!(
        listed(outside()),
        [
            "fragileItemCodes[1]: invalid: expected Unknown, found String(\"y\"): y is outside of \
          itemCodeList"
        ]
    );
    let held = CheckedManifest::from_bson_with(doc! { "parcel": outside() }, |_, _| {
        checked_manifest_schema::Verdict::Reject
    });
    assert_eq!(
        lines(held),
        [
            "parcel.fragileItemCodes[1]: invalid: expected Unknown, found String(\"y\"): y is \
          outside of itemCodeList"
        ]
    );
    let read =
        CheckedParcel::from_bson_with(sound(), |_, _| checked_parcel_schema::Verdict::Reject);
    assert_eq!(read.unwrap().label, "A");
}

#[test]
fn the_issue_of_a_check_holds_its_path_its_value_and_its_reason() {
    let mut handed: Vec<String> = Vec::new();
    let read = CheckedParcel::from_bson_with(outside(), |_, found| {
        handed.extend(found.iter().map(|issue| format!("{issue:?}")));
        checked_parcel_schema::Verdict::Reject
    });
    assert!(read.is_err(), "the callback rejected");
    assert_eq!(
        handed,
        [
            "Invalid { path: Path([Key(\"fragileItemCodes\"), Index(1)]), expected: Unknown, found: \
          String(\"y\"), reason: \"y is outside of itemCodeList\" }"
        ]
    );
}

#[test]
fn the_piped_read_lists_a_broken_check_as_the_callback_read_does() {
    assert_eq!(
        lines(CheckedParcel::from_bson_piped(outside(), &[])),
        listed(outside()),
        "no resolver settled it"
    );
    let dropped = CheckedParcel::from_bson_piped(outside(), &[&dropping_what_is_outside]);
    assert_eq!(dropped.unwrap().fragile_item_codes, ["x"]);
    let held = CheckedManifest::from_bson_piped(doc! { "parcel": duplicated() }, &[]);
    assert_eq!(
        lines(held),
        [
            "parcel.itemCodeList: invalid: expected Unknown, found Array([String(\"x\"), \
             String(\"x\")]): duplicate item code x"
        ]
    );
}

#[test]
fn a_callback_repairs_a_check_through_the_path_of_its_issue() {
    let read = CheckedParcel::from_bson_with(outside(), |raw, found| {
        for issue in found {
            if let checked_parcel_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                assert!(
                    path.remove_from_document(raw),
                    "the path leads to the value"
                );
            }
        }
        checked_parcel_schema::Verdict::Fixed
    });
    assert_eq!(read.unwrap().fragile_item_codes, ["x"]);
}

#[test]
fn a_fixed_verdict_that_leaves_a_check_broken_fails_the_read() {
    let read =
        CheckedParcel::from_bson_with(outside(), |_, _| checked_parcel_schema::Verdict::Fixed);
    assert_eq!(lines(read), listed(outside()));
}

/// No value is built from a document serde refuses, so no check runs on it: the first list holds
/// serde's issue alone, and the check's issue is listed once the document reads.
#[test]
fn no_check_runs_until_serde_reads_the_value() {
    let mut first: Vec<String> = Vec::new();
    let stored = doc! { "label": 5_i32, "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] };
    let read = CheckedParcel::from_bson_with(stored, |raw, found| {
        first.extend(found.iter().map(|issue| {
            if let checked_parcel_schema::Issue::Invalid {
                path,
                expected,
                found: held,
                reason: _reason,
            } = issue
            {
                format!("{path}: expected {expected:?}, found {held:?}")
            } else {
                format!("{issue:?}")
            }
        }));
        raw.insert("label", "A");
        checked_parcel_schema::Verdict::Fixed
    });
    assert_eq!(first, ["label: expected String, found Int32(5)"]);
    assert_eq!(lines(read), listed(outside()));
}

#[test]
fn a_held_model_lists_its_checks_under_the_place_it_is_held_at() {
    let stored = doc! {
        "parcelList": [sound(), outside()],
        "parcelsByBay": { "north": outside(), "south": sound() },
        "spareParcel": duplicated(),
    };
    let read = CheckedDepot::from_bson_with(stored, |_, _| checked_depot_schema::Verdict::Reject);
    assert_eq!(
        lines(read),
        [
            "parcelList[1].fragileItemCodes[1]: invalid: expected Unknown, found String(\"y\"): y \
             is outside of itemCodeList",
            "parcelsByBay.north.fragileItemCodes[1]: invalid: expected Unknown, found \
             String(\"y\"): y is outside of itemCodeList",
            "spareParcel.itemCodeList: invalid: expected Unknown, found Array([String(\"x\"), \
             String(\"x\")]): duplicate item code x",
        ]
    );
}

#[test]
fn a_members_check_is_listed_where_serde_writes_the_member_under_each_tagging() {
    let long = || vec!["a", "b", "c"];
    let refused = "invalid: expected Unknown, found String(\"c\"): c is one code too many";
    let external =
        CheckedExternal::from_bson_with(doc! { "Road": { "roadStops": long() } }, |_, _| {
            checked_external_schema::Verdict::Reject
        });
    assert_eq!(lines(external), [format!("Road.roadStops[2]: {refused}")]);
    let by_rail = CheckedExternal::from_bson_with(doc! { "Rail": "unknown" }, |_, _| {
        checked_external_schema::Verdict::Reject
    });
    assert_eq!(
        lines(by_rail),
        ["Rail: invalid: expected Unknown, found String(\"unknown\"): a line is known"]
    );
    let internal =
        CheckedInternal::from_bson_with(doc! { "kind": "Road", "roadStops": long() }, |_, _| {
            checked_internal_schema::Verdict::Reject
        });
    assert_eq!(lines(internal), [format!("roadStops[2]: {refused}")]);
    let adjacent = CheckedAdjacent::from_bson_with(
        doc! { "kind": "Road", "detail": { "roadStops": long() } },
        |_, _| checked_adjacent_schema::Verdict::Reject,
    );
    assert_eq!(lines(adjacent), [format!("detail.roadStops[2]: {refused}")]);
    let content =
        CheckedAdjacent::from_bson_with(doc! { "kind": "Rail", "detail": "unknown" }, |_, _| {
            checked_adjacent_schema::Verdict::Reject
        });
    assert_eq!(
        lines(content),
        ["detail: invalid: expected Unknown, found String(\"unknown\"): a line is known"]
    );
    let untagged = CheckedUntagged::from_bson_with(doc! { "roadStops": long() }, |_, _| {
        checked_untagged_schema::Verdict::Reject
    });
    assert_eq!(lines(untagged), [format!("roadStops[2]: {refused}")]);
}

/// A bound of a member and a check of it are listed at one path, the bound first.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_bound_and_a_check_of_one_value_are_two_issues() {
    let stored = doc! { "label": "", "itemCodeList": ["x", "x"], "fragileItemCodes": ["y"] };
    assert_eq!(
        listed(stored),
        [
            "label: invalid: expected String, found String(\"\"): too short: minimum length is 1, \
             got 0",
            "itemCodeList: invalid: expected Unknown, found Array([String(\"x\"), String(\"x\")]): \
             duplicate item code x",
            "fragileItemCodes[0]: invalid: expected Unknown, found String(\"y\"): y is outside of \
             itemCodeList",
        ]
    );
}
