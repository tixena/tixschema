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

/// The payload serde writes for each place that is refused, in the order of `PLACES`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
const WRITTEN: &str = r#"Package, unlabelled: {"itemLabel":""}
Docket, empty and numbered zero: ["",0]
Bundle, unnamed and unlabelled: ["",{"itemLabel":""}]
Measured, unnamed and unlabelled after a skipped slot: ["",{"itemLabel":""}]
Lone, empty: ""
Strapped, an unlabelled parcel: {"itemLabel":""}
Crane, an unlabelled strapped load: {"strappedLoad":{"itemLabel":""}}
Hoist, an empty code: {"code":""}
Freight, boxed unlabelled: {"Boxed":{"itemLabel":""}}
Freight, crated unlabelled: {"Crated":{"cratedParcel":{"itemLabel":""}}}
Freight, loose and unlabelled: {"Loose":{"freightLabel":""}}
Freight, split unnamed and unlabelled: {"Split":["",{"itemLabel":""}]}
Freight, stacked with one unlabelled: {"Stacked":[{"itemLabel":"A"},{"itemLabel":""}]}
Freight, stacked three high: {"Stacked":[{"itemLabel":"A"},{"itemLabel":"A"},{"itemLabel":"A"}]}
Seal, stamped with nothing: {"Stamped":""}
Seal, signed by no one: {"Signed":{"signedBy":""}}
Haul, boxed unlabelled: {"kind":"Boxed","held":{"itemLabel":""}}
Haul, crated unlabelled: {"kind":"Crated","held":{"cratedParcel":{"itemLabel":""}}}
Haul, loose and unlabelled: {"kind":"Loose","held":{"haulLabel":""}}
Haul, split unnamed and unlabelled: {"kind":"Split","held":["",{"itemLabel":""}]}
Haul, stamped with nothing: {"kind":"Stamped","held":""}
Lift, boxed unlabelled: {"kind":"Boxed","itemLabel":""}
Lift, crated unlabelled: {"kind":"Crated","cratedParcel":{"itemLabel":""}}
Lift, loose and unlabelled: {"kind":"Loose","liftLabel":""}
Load, labelled with nothing: {"loadLabel":"","loadParcels":[{"itemLabel":"A"},{"itemLabel":""}]}
Load, an empty text: ""
Load, an unlabelled whole: {"itemLabel":""}
Mooring, anchored with no code: {"atAnchor":{"anchorCode":""}}
Mooring, an unlabelled parcel in dry dock: {"dry_dock":{"itemLabel":""}}
Departure, by road from an unnamed origin: {"ByRoad":{"originName":""}}
"#;

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

/// Without `serde` the tag a variant's members are written beside is not read, and a member is
/// under the variant's own name.
#[test]
fn every_shape_runs_its_checks() {
    #[cfg(feature = "serde")]
    let shapes = SHAPES;
    #[cfg(not(feature = "serde"))]
    let shapes = SHAPES.replace(
        "    'stops': duplicate item code a",
        "    'Air.stops': duplicate item code a",
    );
    assert_eq!(report(&shape_cases()), shapes);
}

/// A message names a value by the steps serde writes on the way to it: a key, a position among
/// several slots, the variant's name or the content key, and nothing for the value itself.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn every_place_is_named_as_the_payload_holds_it() {
    use core::fmt::Write as _;

    use super::models::places::{PLACES, place_cases};

    let cases = place_cases();
    let mut written = String::new();
    for held in cases.iter().filter(|held| held.answered.is_err()) {
        writeln!(written, "{}: {}", held.name, held.payload).unwrap();
    }
    assert_eq!(written, WRITTEN);
    assert_eq!(report(&cases), PLACES);
}
