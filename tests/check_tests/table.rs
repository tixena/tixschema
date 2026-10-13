//! The shared table of cases and one value per shape, as `validate()` answers them in this build.

use serde_json::Value;

use super::models::{SHAPES, manifest, parcel_cases, report, shape_cases};

/// The payload of each case of the shared table.
const PAYLOADS: [&str; 6] = [
    r#"{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    r#"{"label":"","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    r#"{"label":"A","itemCodeList":["x","x"],"fragileItemCodes":["x"]}"#,
    r#"{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x","y"]}"#,
    r#"{"label":"","itemCodeList":["x","x"],"fragileItemCodes":["y"]}"#,
    r#"{"parcel":{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x","y"]}}"#,
];

/// serde writes a parcel's keys in the order of its members, so the two are compared as values.
#[test]
fn each_value_is_the_one_its_payload_describes() {
    let mut written: Vec<Value> = parcel_cases()
        .iter()
        .map(|held| serde_json::from_str(&held.payload).unwrap())
        .collect();
    written.push(serde_json::to_value(manifest()).unwrap());
    let described: Vec<Value> = PAYLOADS
        .iter()
        .map(|payload| serde_json::from_str(payload).unwrap())
        .collect();
    assert_eq!(written, described);
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn validate_answers_the_shared_table() {
    use super::models::{TABLE, manifest_case};

    let mut cases = parcel_cases();
    cases.push(manifest_case());
    assert_eq!(report(&cases), TABLE);
}

/// With `serde` and no schema surface no bound is held, and a member goes by the key serde
/// writes.
#[cfg(all(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
#[test]
fn validate_runs_the_checks_alone_where_no_bound_is_published() {
    assert_eq!(
        report(&parcel_cases()),
        "\
case 1: passes
case 2: passes
case 3: refused
    'itemCodeList': duplicate item code x
case 4: refused
    'fragileItemCodes.1': y is outside of itemCodeList
case 5: refused
    'itemCodeList': duplicate item code x
    'fragileItemCodes.0': y is outside of itemCodeList
"
    );
}

/// Without `serde` no attribute of serde's is read, so a member goes by its own name.
#[cfg(not(feature = "serde"))]
#[test]
fn validate_runs_the_checks_alone_without_serde() {
    assert_eq!(
        report(&parcel_cases()),
        "\
case 1: passes
case 2: passes
case 3: refused
    'item_code_list': duplicate item code x
case 4: refused
    'fragileItemCodes.1': y is outside of itemCodeList
case 5: refused
    'item_code_list': duplicate item code x
    'fragileItemCodes.0': y is outside of itemCodeList
"
    );
}

#[test]
fn every_shape_runs_its_checks() {
    assert_eq!(report(&shape_cases()), SHAPES);
}
