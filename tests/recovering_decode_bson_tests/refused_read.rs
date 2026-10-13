//! A BSON document whose value breaks a bound or a check is refused as it is read, with what
//! `validate()` answers of the value: through each major's own read of a document, and through
//! `bson::Deserializer`.

use core::fmt::Debug;
use std::collections::HashMap;
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use std::path::PathBuf;

use bson::{Bson, doc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::{built_against_version_2, written};
use crate::read_document;

#[model_schema(check = ["withinDockedParcel"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DockedParcel {
    fragile_item_codes: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    item_label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DockedManifest {
    boxed_parcel: Box<DockedParcel>,
    first_parcel: DockedParcel,
    parcel_list: Vec<DockedParcel>,
    parcels_by_dock: HashMap<String, DockedParcel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spare_parcel: Option<DockedParcel>,
}

/// A brand that names a check alone.
#[model_schema(check = ["dockedInUpperCase"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct DockCode(String);

/// A brand held to a bound as well. A brand's bound is read with its schema surface, so a build
/// with none writes none.
#[cfg_attr(
    any(feature = "typescript", feature = "zod", feature = "jsonschema"),
    model_schema(minLength = 2, check = ["labelledInUpperCase"])
)]
#[cfg_attr(
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema")),
    model_schema(check = ["labelledInUpperCase"])
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct DockLabel(String);

/// A brand over a list, which publishes no `Display`: its length counts the items.
#[cfg_attr(
    any(feature = "typescript", feature = "zod", feature = "jsonschema"),
    model_schema(no_display, minLength = 2, check = ["dockedOnce"])
)]
#[cfg_attr(
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema")),
    model_schema(no_display, check = ["dockedOnce"])
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct DockCodes(Vec<String>);

/// A brand over a number.
#[model_schema(check = ["dockedAboveZero"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct DockCount(i32);

/// A brand over a path, measured by the text serde writes for it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(no_display, minLength = 3)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct DockDir(PathBuf);

/// A brand written with a named field.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(maxLength = 3)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct DockSlug {
    text: String,
}

/// A brand with a parameter, whose bound is its declared default's.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(minLength = 2, default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct DockTag<CodeType>(CodeType);

/// Holds a label in each place a holder reads a model unjudged.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DockedShelf {
    boxed_label: Box<DockLabel>,
    first_label: DockLabel,
    label_list: Vec<DockLabel>,
    labels_by_dock: HashMap<String, DockLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spare_label: Option<DockLabel>,
}

fn within_docked_parcel(value: &DockedParcel) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| code.as_str() != "x")
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

fn sound() -> DockedParcel {
    DockedParcel {
        fragile_item_codes: vec!["x".to_owned()],
        item_label: "A".to_owned(),
    }
}

fn broken() -> DockedParcel {
    DockedParcel {
        fragile_item_codes: vec!["x".to_owned(), "y".to_owned()],
        item_label: String::new(),
    }
}

fn manifest_of(held: &DockedParcel) -> DockedManifest {
    DockedManifest {
        boxed_parcel: Box::new(held.clone()),
        first_parcel: held.clone(),
        parcel_list: vec![sound(), held.clone()],
        parcels_by_dock: HashMap::from([("north".to_owned(), held.clone())]),
        spare_parcel: Some(held.clone()),
    }
}

fn docked_in_upper_case(value: &DockCode) -> Result<(), Vec<String>> {
    if value.0.chars().any(char::is_lowercase) {
        return Err(vec!["a dock code is upper case".to_owned()]);
    }
    Ok(())
}

fn labelled_in_upper_case(value: &DockLabel) -> Result<(), Vec<String>> {
    if value.0.chars().any(char::is_lowercase) {
        return Err(vec!["a dock label is upper case".to_owned()]);
    }
    Ok(())
}

fn docked_above_zero(value: &DockCount) -> Result<(), Vec<String>> {
    if value.0 <= 0_i32 {
        return Err(vec!["a dock count is above zero".to_owned()]);
    }
    Ok(())
}

fn docked_once(value: &DockCodes) -> Result<(), Vec<String>> {
    let repeated: Vec<String> = value
        .0
        .iter()
        .enumerate()
        .filter(|(index, code)| value.0[..*index].contains(code))
        .map(|(index, code)| format!("'{index}': {code} is docked twice"))
        .collect();
    if repeated.is_empty() {
        Ok(())
    } else {
        Err(repeated)
    }
}

/// A shelf that holds `held` in every place it holds a label, after a sound one in its list.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn shelf_of(held: &DockLabel) -> DockedShelf {
    DockedShelf {
        boxed_label: Box::new(held.clone()),
        first_label: held.clone(),
        label_list: vec![DockLabel("OK".to_owned()), held.clone()],
        labels_by_dock: HashMap::from([("north".to_owned(), held.clone())]),
        spare_label: Some(held.clone()),
    }
}

/// What this major's own read of a document says of a refusal whose text is `refused`.
fn told_by_this_major(refused: &str) -> String {
    if built_against_version_2() {
        refused.to_owned()
    } else {
        format!("BSON error. Kind: A deserialization-related error occurred. Message: {refused}.")
    }
}

/// Holds the read of the BSON value serde writes for `value` to what `validate()` answers of
/// `value`: an error with every message where it reports any, and the value where it reports none.
fn read_as_validated<T>(value: &T, validated: Result<(), Vec<String>>)
where
    T: Debug + DeserializeOwned + PartialEq + Serialize,
{
    let stored = value.serialize(bson::Serializer::new()).unwrap();
    let read =
        T::deserialize(bson::Deserializer::new(stored)).map_err(|refused| refused.to_string());
    match validated {
        Ok(()) => assert_eq!(read.as_ref(), Ok(value)),
        Err(reported) => assert_eq!(read, Err(told_by_this_major(&reported.join("; ")))),
    }
}

#[test]
fn a_document_that_breaks_a_bound_and_a_check_is_refused_with_what_validate_answers() {
    let refused = broken().validate().unwrap_err().join("; ");
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        refused,
        "'itemLabel': too short: minimum length is 1, got 0; \
         'fragileItemCodes.1': y is outside of itemCodeList"
    );
    // A build with no schema surface holds no bound, and the check alone answers.
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    assert_eq!(
        refused,
        "'fragileItemCodes.1': y is outside of itemCodeList"
    );
    assert_eq!(
        read_document::<DockedParcel>(doc! { "itemLabel": "", "fragileItemCodes": ["x", "y"] }),
        Err(told_by_this_major(&refused))
    );
    assert_eq!(
        DockedParcel::deserialize(bson::Deserializer::new(Bson::Document(written(&broken()))))
            .map_err(|error| error.to_string()),
        Err(told_by_this_major(&refused))
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_document_is_refused_with_every_message_of_every_model_it_holds_at_its_full_path() {
    let held = manifest_of(&broken());
    let refused = held.validate().unwrap_err().join("; ");
    assert_eq!(
        refused,
        "'boxedParcel.itemLabel': too short: minimum length is 1, got 0; \
         'boxedParcel.fragileItemCodes.1': y is outside of itemCodeList; \
         'firstParcel.itemLabel': too short: minimum length is 1, got 0; \
         'firstParcel.fragileItemCodes.1': y is outside of itemCodeList; \
         'parcelList.1.itemLabel': too short: minimum length is 1, got 0; \
         'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList; \
         'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0; \
         'parcelsByDock.north.fragileItemCodes.1': y is outside of itemCodeList; \
         'spareParcel.itemLabel': too short: minimum length is 1, got 0; \
         'spareParcel.fragileItemCodes.1': y is outside of itemCodeList"
    );
    assert_eq!(
        read_document::<DockedManifest>(written(&held)),
        Err(told_by_this_major(&refused))
    );
}

/// A holder that names no check publishes no `validate()` in a build with no schema surface, and
/// the first broken parcel it holds refuses the read, with its own message.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_document_is_refused_by_the_first_broken_model_where_its_holder_publishes_no_validate() {
    let refused = read_document::<DockedManifest>(written(&manifest_of(&broken()))).unwrap_err();
    assert!(
        refused.contains("'fragileItemCodes.1': y is outside of itemCodeList"),
        "got: {refused}"
    );
}

#[test]
fn a_document_that_keeps_every_rule_is_read_and_written_back_as_it_was() {
    let held = manifest_of(&sound());
    let document = written(&held);
    let read = read_document::<DockedManifest>(document.clone());
    assert_eq!(read.as_ref(), Ok(&held));
    assert_eq!(read.map(|value| written(&value)), Ok(document));
}

#[test]
fn a_brand_that_names_a_check_alone_is_refused_as_it_is_read() {
    let lower = DockCode("abc".to_owned());
    assert_eq!(
        lower.validate(),
        Err(vec!["a dock code is upper case".to_owned()])
    );
    read_as_validated(&lower, lower.validate());
    let upper = DockCode("ABC".to_owned());
    assert_eq!(upper.validate(), Ok(()));
    read_as_validated(&upper, upper.validate());
}

#[test]
fn a_brand_is_refused_with_every_message_validate_answers_of_the_value_read() {
    let refused = DockLabel("a".to_owned()).validate().unwrap_err().join("; ");
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        refused,
        "too short: minimum length is 2, got 1; a dock label is upper case"
    );
    // A build with no schema surface holds no bound, and the check alone answers.
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    assert_eq!(refused, "a dock label is upper case");
    for text in ["a", "A", "ab", "AB"] {
        let held = DockLabel(text.to_owned());
        read_as_validated(&held, held.validate());
    }
}

#[test]
fn a_brand_over_a_list_is_refused_as_validate_answers_of_the_list_read() {
    let codes = |written: &[&str]| DockCodes(written.iter().map(|&code| code.to_owned()).collect());
    assert_eq!(
        codes(&["a", "b", "a"]).validate(),
        Err(vec!["'2': a is docked twice".to_owned()])
    );
    for written in [&[][..], &["a"], &["a", "a"], &["a", "b"], &["a", "b", "a"]] {
        let held = codes(written);
        read_as_validated(&held, held.validate());
    }
}

#[test]
fn a_brand_over_a_number_is_refused_as_its_check_answers() {
    assert_eq!(
        DockCount(0).validate(),
        Err(vec!["a dock count is above zero".to_owned()])
    );
    for held in [DockCount(0), DockCount(7)] {
        read_as_validated(&held, held.validate());
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_over_a_path_and_one_with_a_named_field_are_refused_as_their_bounds_answer() {
    let short = DockDir(PathBuf::from("/a"));
    assert_eq!(
        short.validate(),
        Err(vec!["too short: minimum length is 3, got 2".to_owned()])
    );
    read_as_validated(&short, short.validate());
    let long = DockDir(PathBuf::from("/etc"));
    read_as_validated(&long, long.validate());

    let slug = |text: &str| DockSlug {
        text: text.to_owned(),
    };
    assert_eq!(
        slug("abcd").validate(),
        Err(vec!["too long: maximum length is 3, got 4".to_owned()])
    );
    for text in ["abcd", "abc"] {
        let held = slug(text);
        read_as_validated(&held, held.validate());
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_generic_brand_is_refused_at_its_declared_default_and_read_at_any_other_filling() {
    let short = DockTag("a".to_owned());
    assert_eq!(
        short.validate(),
        Err(vec!["too short: minimum length is 2, got 1".to_owned()])
    );
    read_as_validated(&short, short.validate());
    let long = DockTag("ab".to_owned());
    read_as_validated(&long, long.validate());
    // The bound is the declared default's, so one digit is read at another filling.
    let counted = DockTag(7_i32);
    assert_eq!(counted.validate(), Ok(()));
    read_as_validated(&counted, counted.validate());
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_document_names_every_broken_brand_it_holds_at_its_full_path() {
    let held = shelf_of(&DockLabel("a".to_owned()));
    let refused = held.validate().unwrap_err().join("; ");
    assert_eq!(
        refused,
        "'boxedLabel': too short: minimum length is 2, got 1; \
         'boxedLabel': a dock label is upper case; \
         'firstLabel': too short: minimum length is 2, got 1; \
         'firstLabel': a dock label is upper case; \
         'labelList.1': too short: minimum length is 2, got 1; \
         'labelList.1': a dock label is upper case; \
         'labelsByDock.north': too short: minimum length is 2, got 1; \
         'labelsByDock.north': a dock label is upper case; \
         'spareLabel': too short: minimum length is 2, got 1; \
         'spareLabel': a dock label is upper case"
    );
    assert_eq!(
        read_document::<DockedShelf>(written(&held)),
        Err(told_by_this_major(&refused))
    );
    let sound = shelf_of(&DockLabel("AB".to_owned()));
    assert_eq!(read_document::<DockedShelf>(written(&sound)), Ok(sound));
}
