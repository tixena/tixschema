//! A BSON document whose value breaks a bound or a check is refused as it is read, with what
//! `validate()` answers of the value: through each major's own read of a document, and through
//! `bson::Deserializer`.

use std::collections::HashMap;

use bson::{Bson, doc};
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

/// What this major's own read of a document says of a refusal whose text is `refused`.
fn told_by_this_major(refused: &str) -> String {
    if built_against_version_2() {
        refused.to_owned()
    } else {
        format!("BSON error. Kind: A deserialization-related error occurred. Message: {refused}.")
    }
}

#[test]
fn a_document_that_breaks_a_bound_and_a_check_is_refused_with_what_validate_answers() {
    let refused = broken().validate().unwrap_err().join("; ");
    assert_eq!(
        refused,
        "'itemLabel': too short: minimum length is 1, got 0; \
         'fragileItemCodes.1': y is outside of itemCodeList"
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

#[test]
fn a_document_that_keeps_every_rule_is_read_and_written_back_as_it_was() {
    let held = manifest_of(&sound());
    let document = written(&held);
    let read = read_document::<DockedManifest>(document.clone());
    assert_eq!(read.as_ref(), Ok(&held));
    assert_eq!(read.map(|value| written(&value)), Ok(document));
}
