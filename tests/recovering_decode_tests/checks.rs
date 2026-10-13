//! A check a type or a field names is the recovering read's to list. serde builds the value, the
//! checks run on it, and each message is one `Invalid` at the place it names in the raw value.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::lines;

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

/// A parcel held in a list, in an `Option`, in a map and behind a `Box`.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckedDepot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    boxed_parcel: Option<Box<CheckedParcel>>,
    #[serde(default)]
    parcel_list: Vec<CheckedParcel>,
    #[serde(default)]
    parcels_by_bay: BTreeMap<String, CheckedParcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spare_parcel: Option<CheckedParcel>,
}

#[model_schema(decode_with, check = ["upperCase"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct CheckedCode(String);

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct CheckedLabelled {
    code: CheckedCode,
    #[serde(flatten)]
    route: CheckedRoute,
}

#[model_schema(decode_with, check = ["withARoute"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct CheckedRoute {
    stops: Vec<String>,
}

#[model_schema(decode_with, default_types(CodeType = String), check = ["withACode"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct CheckedCrate<CodeType> {
    codes: Vec<CodeType>,
    label: String,
}

#[model_schema(decode_with, check = ["withAStop"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all_fields = "camelCase")]
enum CheckedExternal {
    Held,
    Rail(#[model_schema_prop(check = ["knownLine"])] String),
    Road {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
    Sea(
        #[model_schema_prop(check = ["knownLine"])] String,
        #[model_schema_prop(check = ["shortList"])] Vec<String>,
    ),
    Whole(CheckedRoute),
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
enum CheckedInternal {
    Held,
    Road {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
    Whole(CheckedRoute),
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
    Whole(CheckedRoute),
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
enum CheckedUntagged {
    Rail(#[model_schema_prop(check = ["knownLine"])] String),
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

fn upper_case(value: &CheckedCode) -> Result<(), Vec<String>> {
    if value.0.chars().any(char::is_lowercase) {
        return Err(vec!["a code is upper case".to_owned()]);
    }
    Ok(())
}

fn with_a_route(value: &CheckedRoute) -> Result<(), Vec<String>> {
    if value.stops.is_empty() {
        return Err(vec!["'stops': a route makes at least one stop".to_owned()]);
    }
    Ok(())
}

fn with_a_code<CodeType>(value: &CheckedCrate<CodeType>) -> Result<(), Vec<String>> {
    if value.codes.is_empty() {
        return Err(vec!["'codes': a crate holds at least one code".to_owned()]);
    }
    Ok(())
}

fn with_a_stop(value: &CheckedExternal) -> Result<(), Vec<String>> {
    if let CheckedExternal::Road { road_stops } = value
        && road_stops.is_empty()
    {
        return Err(vec![
            "'Road.roadStops': a road transfer makes a stop".to_owned(),
        ]);
    }
    Ok(())
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

/// The raw value of each case of the shared table that breaks a check alone.
fn duplicated() -> Value {
    json!({ "label": "A", "itemCodeList": ["x", "x"], "fragileItemCodes": ["x"] })
}

fn outside() -> Value {
    json!({ "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] })
}

fn sound() -> Value {
    json!({ "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x"] })
}

/// A resolver that takes the value a check refused out of the raw value.
fn dropping_what_is_outside(
    raw: &mut Value,
    issue: &checked_parcel_schema::Issue<Value>,
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
    if path.remove_from_value(raw) {
        checked_parcel_schema::Resolution::Settled
    } else {
        checked_parcel_schema::Resolution::Rejected
    }
}

/// Reads `stored` as a parcel with a callback that rejects, and answers every issue it listed.
fn listed(stored: Value) -> Vec<String> {
    let read =
        CheckedParcel::from_value_with(stored, |_, _| checked_parcel_schema::Verdict::Reject);
    lines(&read.unwrap_err())
}

#[test]
fn a_broken_check_is_one_invalid_at_the_place_its_message_names() {
    assert_eq!(
        listed(duplicated()),
        [
            "itemCodeList: invalid: expected Unknown, found Array [String(\"x\"), String(\"x\")]: \
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
    let held = CheckedManifest::from_value_with(json!({ "parcel": outside() }), |_, _| {
        checked_manifest_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&held.unwrap_err()),
        [
            "parcel.fragileItemCodes[1]: invalid: expected Unknown, found String(\"y\"): y is \
          outside of itemCodeList"
        ]
    );
    let read =
        CheckedParcel::from_value_with(sound(), |_, _| checked_parcel_schema::Verdict::Reject);
    assert_eq!(read.unwrap().label, "A");
}

/// The issue a callback is handed, as the README prints it.
#[test]
fn the_issue_of_a_check_holds_its_path_its_value_and_its_reason() {
    let mut handed: Vec<String> = Vec::new();
    let read = CheckedParcel::from_value_with(outside(), |_, found| {
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

/// What the README says a read lists for a broken check is what a read lists.
#[test]
fn the_readme_shows_where_a_broken_check_is_listed() {
    let readme = include_str!("../../README.md");
    let mut shown: Vec<String> = Vec::new();
    let read = CheckedParcel::from_value_with(outside(), |_, found| {
        for issue in found {
            if let checked_parcel_schema::Issue::Invalid {
                path,
                expected: checked_parcel_schema::Expected::Unknown,
                found: _found,
                reason,
            } = issue
            {
                shown.push(format!("lists `Invalid` at `{path}`, reason `{reason}`"));
            }
        }
        checked_parcel_schema::Verdict::Reject
    });
    assert!(read.is_err(), "the callback rejected");
    let held = CheckedManifest::from_value_with(json!({ "parcel": outside() }), |_, found| {
        for issue in found {
            if let checked_manifest_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                shown.push(format!("lists it at `{path}`"));
            }
        }
        checked_manifest_schema::Verdict::Reject
    });
    assert!(held.is_err(), "the callback rejected");
    assert_eq!(shown.len(), 2, "one issue for each read: {shown:?}");
    for sentence in shown {
        assert!(
            readme.contains(&sentence),
            "the README no longer says a read `{sentence}`"
        );
    }
}

#[test]
fn the_piped_read_lists_a_broken_check_as_the_callback_read_does() {
    let unsettled = CheckedParcel::from_value_piped(outside(), &[]);
    assert_eq!(
        lines(&unsettled.unwrap_err()),
        listed(outside()),
        "no resolver settled it"
    );
    let dropped = CheckedParcel::from_value_piped(outside(), &[&dropping_what_is_outside]);
    assert_eq!(dropped.unwrap().fragile_item_codes, ["x"]);
    let held = CheckedManifest::from_value_piped(json!({ "parcel": duplicated() }), &[]);
    assert_eq!(
        lines(&held.unwrap_err()),
        [
            "parcel.itemCodeList: invalid: expected Unknown, found Array [String(\"x\"), \
             String(\"x\")]: duplicate item code x"
        ]
    );
}

#[test]
fn a_callback_repairs_a_check_through_the_path_of_its_issue() {
    let read = CheckedParcel::from_value_with(outside(), |raw, found| {
        for issue in found {
            if let checked_parcel_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                assert!(path.remove_from_value(raw), "the path leads to the value");
            }
        }
        checked_parcel_schema::Verdict::Fixed
    });
    assert_eq!(read.unwrap().fragile_item_codes, ["x"]);
}

#[test]
fn a_fixed_verdict_that_leaves_a_check_broken_fails_the_read() {
    let read =
        CheckedParcel::from_value_with(outside(), |_, _| checked_parcel_schema::Verdict::Fixed);
    assert_eq!(lines(&read.unwrap_err()), listed(outside()));
}

/// No value is built from a payload serde refuses, so no check runs on it: the first list holds
/// serde's issue alone, and the check's issue is listed once the payload reads.
#[test]
fn no_check_runs_until_serde_reads_the_value() {
    let mut first: Vec<String> = Vec::new();
    let stored = json!({ "label": 5_u32, "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] });
    let read = CheckedParcel::from_value_with(stored, |raw, found| {
        first.extend(found.iter().map(|issue| format!("{issue:?}")));
        raw["label"] = json!("A");
        checked_parcel_schema::Verdict::Fixed
    });
    assert_eq!(
        first,
        [
            "Invalid { path: Path([Key(\"label\")]), expected: String, found: Number(5), reason: \
          \"invalid type: integer `5`, expected a string\" }"
        ]
    );
    assert_eq!(lines(&read.unwrap_err()), listed(outside()));
}

#[test]
fn a_held_model_lists_its_checks_under_the_place_it_is_held_at() {
    let stored = json!({
        "boxedParcel": duplicated(),
        "parcelList": [sound(), outside()],
        "parcelsByBay": { "north": outside(), "south": sound() },
        "spareParcel": duplicated(),
    });
    let read = CheckedDepot::from_value_with(stored, |_, _| checked_depot_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "boxedParcel.itemCodeList: invalid: expected Unknown, found Array [String(\"x\"), \
             String(\"x\")]: duplicate item code x",
            "parcelList[1].fragileItemCodes[1]: invalid: expected Unknown, found String(\"y\"): y \
             is outside of itemCodeList",
            "parcelsByBay.north.fragileItemCodes[1]: invalid: expected Unknown, found \
             String(\"y\"): y is outside of itemCodeList",
            "spareParcel.itemCodeList: invalid: expected Unknown, found Array [String(\"x\"), \
             String(\"x\")]: duplicate item code x",
        ]
    );
    let empty =
        CheckedDepot::from_value_with(json!({}), |_, _| checked_depot_schema::Verdict::Reject);
    assert_eq!(empty.unwrap().parcel_list, Vec::new());
}

/// A message with no lead is at the root, and a flattened model's place is its holder's own.
#[test]
fn a_message_with_no_lead_is_at_the_root() {
    let code =
        CheckedCode::from_value_with(json!("ab"), |_, _| checked_code_schema::Verdict::Reject);
    assert_eq!(
        lines(&code.unwrap_err()),
        ["the value itself: invalid: expected Unknown, found String(\"ab\"): a code is upper case"]
    );
    let held = CheckedLabelled::from_value_with(json!({ "code": "ab", "stops": [] }), |_, _| {
        checked_labelled_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&held.unwrap_err()),
        [
            "code: invalid: expected Unknown, found String(\"ab\"): a code is upper case",
            "stops: invalid: expected Unknown, found Array []: a route makes at least one stop",
        ]
    );
}

#[test]
fn a_type_with_a_parameter_lists_its_check_at_every_filling() {
    let refused =
        "codes: invalid: expected Unknown, found Array []: a crate holds at least one code";
    let named =
        CheckedCrate::<String>::from_value_with(json!({ "codes": [], "label": "c" }), |_, _| {
            checked_crate_schema::Verdict::Reject
        });
    assert_eq!(lines(&named.unwrap_err()), [refused]);
    let counted =
        CheckedCrate::<u32>::from_value_with(json!({ "codes": [], "label": "c" }), |_, _| {
            checked_crate_schema::Verdict::Reject
        });
    assert_eq!(lines(&counted.unwrap_err()), [refused]);
    let filled =
        CheckedCrate::<u32>::from_value_with(json!({ "codes": [7_u32], "label": "c" }), |_, _| {
            checked_crate_schema::Verdict::Reject
        });
    assert_eq!(filled.unwrap().codes, [7]);
}

#[test]
fn a_members_check_is_listed_where_serde_writes_the_member_under_each_tagging() {
    let long = json!(["a", "b", "c"]);
    let external = |stored: Value| {
        let read = CheckedExternal::from_value_with(stored, |_, _| {
            checked_external_schema::Verdict::Reject
        });
        lines(&read.unwrap_err())
    };
    assert_eq!(
        external(json!({ "Road": { "roadStops": long } })),
        [
            "Road.roadStops[2]: invalid: expected Unknown, found String(\"c\"): c is one code too many"
        ]
    );
    assert_eq!(
        external(json!({ "Rail": "unknown" })),
        ["Rail: invalid: expected Unknown, found String(\"unknown\"): a line is known"]
    );
    assert_eq!(
        external(json!({ "Sea": ["unknown", long] })),
        [
            "Sea[0]: invalid: expected Unknown, found String(\"unknown\"): a line is known",
            "Sea[1][2]: invalid: expected Unknown, found String(\"c\"): c is one code too many",
        ]
    );
    assert_eq!(
        external(json!({ "Road": { "roadStops": [] } })),
        ["Road.roadStops: invalid: expected Unknown, found Array []: a road transfer makes a stop"]
    );
    assert_eq!(
        external(json!({ "Whole": { "stops": [] } })),
        ["Whole.stops: invalid: expected Unknown, found Array []: a route makes at least one stop"]
    );
    let held = CheckedExternal::from_value_with(json!("Held"), |_, _| {
        checked_external_schema::Verdict::Reject
    });
    assert_eq!(held, Ok(CheckedExternal::Held));

    let internal = |stored: Value| {
        let read = CheckedInternal::from_value_with(stored, |_, _| {
            checked_internal_schema::Verdict::Reject
        });
        lines(&read.unwrap_err())
    };
    assert_eq!(
        internal(json!({ "kind": "Road", "roadStops": long })),
        ["roadStops[2]: invalid: expected Unknown, found String(\"c\"): c is one code too many"]
    );
    assert_eq!(
        internal(json!({ "kind": "Whole", "stops": [] })),
        ["stops: invalid: expected Unknown, found Array []: a route makes at least one stop"]
    );
    let unheld = CheckedInternal::from_value_with(json!({ "kind": "Held" }), |_, _| {
        checked_internal_schema::Verdict::Reject
    });
    assert_eq!(unheld, Ok(CheckedInternal::Held));

    let adjacent = |stored: Value| {
        let read = CheckedAdjacent::from_value_with(stored, |_, _| {
            checked_adjacent_schema::Verdict::Reject
        });
        lines(&read.unwrap_err())
    };
    assert_eq!(
        adjacent(json!({ "kind": "Road", "detail": { "roadStops": long } })),
        [
            "detail.roadStops[2]: invalid: expected Unknown, found String(\"c\"): c is one code too many"
        ]
    );
    assert_eq!(
        adjacent(json!({ "kind": "Rail", "detail": "unknown" })),
        ["detail: invalid: expected Unknown, found String(\"unknown\"): a line is known"]
    );
    assert_eq!(
        adjacent(json!({ "kind": "Whole", "detail": { "stops": [] } })),
        [
            "detail.stops: invalid: expected Unknown, found Array []: a route makes at least one stop"
        ]
    );

    let untagged = |stored: Value| {
        let read = CheckedUntagged::from_value_with(stored, |_, _| {
            checked_untagged_schema::Verdict::Reject
        });
        lines(&read.unwrap_err())
    };
    assert_eq!(
        untagged(json!({ "roadStops": long })),
        ["roadStops[2]: invalid: expected Unknown, found String(\"c\"): c is one code too many"]
    );
    assert_eq!(
        untagged(json!("unknown")),
        ["the value itself: invalid: expected Unknown, found String(\"unknown\"): a line is known"]
    );
}

/// A bound of a member and a check of it are listed at one path, the bound first: the walker
/// lists the bound from the raw value, and the check runs on the value serde built.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_bound_and_a_check_of_one_value_are_two_issues() {
    let stored = json!({ "label": "", "itemCodeList": ["x", "x"], "fragileItemCodes": ["y"] });
    assert_eq!(
        listed(stored),
        [
            "label: invalid: expected String, found String(\"\"): too short: minimum length is 1, \
             got 0",
            "itemCodeList: invalid: expected Unknown, found Array [String(\"x\"), String(\"x\")]: \
             duplicate item code x",
            "fragileItemCodes[0]: invalid: expected Unknown, found String(\"y\"): y is outside of \
             itemCodeList",
        ]
    );
}
