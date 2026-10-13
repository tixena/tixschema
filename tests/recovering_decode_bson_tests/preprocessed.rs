//! A type that names a `preprocess` reads a BSON document as it reads a JSON text: each function
//! is handed its payload as a `serde_json::Value`, which the document crosses to through serde.
//! No bound is held in a build with no schema surface, so the one test of a bound is split.

use core::cell::RefCell;

use bson::oid::ObjectId;
use bson::spec::BinarySubtype;
use bson::{Binary, Bson, Document, doc};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::built_against_version_2;
use crate::read_document;

thread_local! {
    /// Each payload the function of a stored row was handed on this thread, in order.
    static HANDED: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
}

/// A row whose type names no function.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PlainRow {
    #[serde(
        deserialize_with = "read_stored_date",
        serialize_with = "write_stored_date"
    )]
    at: DateTime<Utc>,
    count: i32,
    id: ObjectId,
    label: String,
    total: i64,
    weight: f64,
}

/// A number, under a type that names no function.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PlainWeight {
    weight: f64,
}

/// The parcel of the documents, stored as a row.
#[model_schema(decode_with, preprocess = ["withoutFragile"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredParcel {
    fragile_item_codes: Vec<String>,
    item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    label: String,
}

/// The row of [`PlainRow`] under a type whose function records what it is handed, with an id
/// and a label that each name a function of their own.
#[model_schema(decode_with, preprocess = ["asHanded"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct StoredRow {
    #[serde(
        deserialize_with = "read_stored_date",
        serialize_with = "write_stored_date"
    )]
    at: DateTime<Utc>,
    count: i32,
    #[model_schema_prop(preprocess = ["asItIs"])]
    id: ObjectId,
    #[model_schema_prop(preprocess = ["trimmed"])]
    label: String,
    total: i64,
    weight: f64,
}

/// The number of [`PlainWeight`] under a type that names a function.
#[model_schema(decode_with, preprocess = ["asItIs"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct StoredWeight {
    weight: f64,
}

fn as_handed(payload: Value) -> Value {
    HANDED.with(|handed| handed.borrow_mut().push(payload.clone()));
    payload
}

const fn as_it_is(payload: Value) -> Value {
    payload
}

fn read_stored_date<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    bson::DateTime::deserialize(deserializer).map(bson::DateTime::to_chrono)
}

fn trimmed(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

fn write_stored_date<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    bson::DateTime::from_chrono(*value).serialize(serializer)
}

/// What this major's own read of a document says of a refusal whose text is `refused`.
fn as_this_major_tells(refused: &str) -> String {
    if built_against_version_2() {
        refused.to_owned()
    } else {
        format!("BSON error. Kind: A deserialization-related error occurred. Message: {refused}.")
    }
}

fn id() -> ObjectId {
    ObjectId::parse_str("0123456789abcdef01234567").unwrap()
}

/// A row that holds an id, a date, a text and a number of each kind.
fn row() -> Document {
    doc! {
        "at": bson::DateTime::from_millis(1_700_000_000_000),
        "count": 7_i32,
        "id": id(),
        "label": "  A  ",
        "total": 7_i64,
        "weight": 1.5_f64,
    }
}

fn parcel(label: &str, fragile: &[&str]) -> StoredParcel {
    StoredParcel {
        fragile_item_codes: fragile.iter().map(|&code| code.to_owned()).collect(),
        item_code_list: vec!["x".to_owned()],
        label: label.to_owned(),
    }
}

/// An id and a date are read to the values a type that names no function reads them to, by this
/// major's own read of a document and through `bson::Deserializer`. It answers the same in a
/// build with no schema surface: no bound is declared here.
#[test]
fn a_row_is_read_to_the_values_a_type_that_names_no_function_reads() {
    let plain = read_document::<PlainRow>(row()).unwrap();
    let stored = read_document::<StoredRow>(row()).unwrap();
    assert_eq!(
        stored,
        StoredRow {
            at: plain.at,
            count: plain.count,
            id: plain.id,
            label: "A".to_owned(),
            total: plain.total,
            weight: plain.weight,
        }
    );
    assert_eq!(plain.id, id());
    assert_eq!(plain.at.timestamp_millis(), 1_700_000_000_000);
    assert_eq!(
        StoredRow::deserialize(bson::Deserializer::new(Bson::Document(row())))
            .map_err(|refused| refused.to_string()),
        Ok(stored)
    );
}

/// The function of a type is handed the document as the JSON value serde reads from it: an id
/// and a date as the objects extended JSON writes them, and every number as a JSON number. It
/// answers the same in a build with no schema surface.
#[test]
fn a_function_is_handed_the_document_as_a_json_value() {
    HANDED.with(|handed| handed.borrow_mut().clear());
    assert_eq!(
        read_document::<StoredRow>(row()).map(|read| read.count),
        Ok(7_i32)
    );
    assert_eq!(
        HANDED.with(|handed| handed.borrow().clone()),
        [json!({
            "at": { "$date": "2023-11-14T22:13:20Z" },
            "count": 7_i32,
            "id": { "$oid": "0123456789abcdef01234567" },
            "label": "  A  ",
            "total": 7_i64,
            "weight": 1.5_f64,
        })]
    );
}

/// What each read of a document answers for `stored`, and how often the callback of the read
/// that takes one ran: this major's own read, the callback read, the piped read and the read of
/// a stored row.
fn every_read(stored: &Document) -> ([Option<StoredParcel>; 4], u32) {
    let mut calls = 0_u32;
    let with = StoredParcel::from_bson_with(stored.clone(), |_, _| {
        calls += 1;
        stored_parcel_schema::Verdict::Reject
    });
    (
        [
            read_document::<StoredParcel>(stored.clone()).ok(),
            with.ok(),
            StoredParcel::from_bson_piped(stored.clone(), &[]).ok(),
            StoredParcel::mongo_read_row(stored.clone(), &[]).ok(),
        ],
        calls,
    )
}

/// Rows 1 and 4 of the sample, by each read of a document: a label is read without the spaces
/// at its ends, and a document that holds no `fragileItemCodes` with an empty list. It answers
/// the same in a build with no schema surface: neither row breaks the bound.
#[test]
fn a_document_is_read_through_each_function_by_every_read() {
    let row_1 = doc! { "label": "  A  ", "itemCodeList": ["x"], "fragileItemCodes": ["x"] };
    assert_eq!(
        every_read(&row_1),
        ([(); 4].map(|()| Some(parcel("A", &["x"]))), 0)
    );
    let row_3 = doc! { "label": "A", "itemCodeList": ["x"] };
    assert_eq!(
        every_read(&row_3),
        ([(); 4].map(|()| Some(parcel("A", &[]))), 0)
    );
}

/// The callback is handed the document the function of the type corrected, with each number in
/// the type the crossing gives it: the list the function added is there, and the number stored
/// as an `Int32` is an `Int64`. It answers the same in a build with no schema surface.
#[test]
fn the_callback_is_handed_the_document_the_function_of_the_type_corrected() {
    let mut handed: Option<Document> = None;
    let mut listed: Vec<String> = Vec::new();
    let read = StoredParcel::from_bson_with(
        doc! { "label": "A", "itemCodeList": 5_i32 },
        |document, found| {
            handed = Some(document.clone());
            for issue in found {
                if let stored_parcel_schema::Issue::Invalid {
                    path,
                    expected: _expected,
                    found: held,
                    reason: _reason,
                } = issue
                {
                    listed.push(format!("{path} holds {held}"));
                }
            }
            document.insert("itemCodeList", vec!["x"]);
            stored_parcel_schema::Verdict::Fixed
        },
    );
    assert_eq!(
        handed,
        Some(doc! { "fragileItemCodes": [], "itemCodeList": 5_i64, "label": "A" })
    );
    assert_eq!(listed, ["itemCodeList holds 5"]);
    assert_eq!(read.ok(), Some(parcel("A", &[])));
}

/// A build with a schema surface holds the label to its bound, on what the label's function
/// answered.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_label_its_function_emptied_is_refused_where_the_bound_is_held() {
    let row_2 = doc! { "label": "   ", "itemCodeList": ["x"], "fragileItemCodes": ["x"] };
    assert_eq!(
        read_document::<StoredParcel>(row_2.clone()),
        Err(as_this_major_tells(
            "'label': too short: minimum length is 1, got 0"
        ))
    );
    let refused = StoredParcel::from_bson_piped(row_2, &[]).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "label: invalid: expected String, found String(\"   \"): too short: minimum length is \
         1, got 0"
    );
}

/// A build with no schema surface holds no bound, and the label its function emptied is read.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_label_its_function_emptied_is_read_where_no_bound_is_held() {
    let row_2 = doc! { "label": "   ", "itemCodeList": ["x"], "fragileItemCodes": ["x"] };
    assert_eq!(
        read_document::<StoredParcel>(row_2.clone()),
        Ok(parcel("", &["x"]))
    );
    assert_eq!(
        StoredParcel::from_bson_piped(row_2, &[]).ok(),
        Some(parcel("", &["x"]))
    );
}

/// What a `serde_json::Value` cannot hold: a binary and a `Decimal128` are a byte array to
/// serde, which no JSON value is, and a number that is none reaches the function as `null`. Every
/// read of a type that names a function for itself refuses them, and a type that names none
/// reads the number. It answers the same in a build with no schema surface.
#[test]
fn a_document_that_holds_what_no_json_value_can_is_refused() {
    let no_json_value = "invalid type: byte array, expected any valid JSON value";
    let binary = Bson::Binary(Binary {
        bytes: vec![1, 2],
        subtype: BinarySubtype::Generic,
    });
    let decimal = Bson::Decimal128("1.5".parse().unwrap());
    for held in [binary, decimal] {
        let stored = doc! { "weight": held };
        assert_eq!(
            read_document::<StoredWeight>(stored.clone()),
            Err(as_this_major_tells(no_json_value))
        );
        let mut calls = 0_u32;
        let refused = StoredWeight::from_bson_with(stored, |_, _| {
            calls += 1;
            stored_weight_schema::Verdict::Reject
        })
        .unwrap_err();
        assert_eq!(
            refused.to_string(),
            format!("undescribed: {}", as_this_major_tells(no_json_value))
        );
        assert_eq!(calls, 0);
    }
    for weight in [f64::NAN, f64::INFINITY] {
        let no_number = doc! { "weight": weight };
        assert!(
            read_document::<PlainWeight>(no_number.clone())
                .is_ok_and(|read| !read.weight.is_finite())
        );
        assert_eq!(
            read_document::<StoredWeight>(no_number),
            Err(as_this_major_tells("invalid type: null, expected f64"))
        );
    }
}

/// The document the callback is handed has crossed back from the JSON value: an id and a date
/// are the BSON values they were, and a number stored as an `Int32` is an `Int64`. It answers
/// the same in a build with no schema surface: no bound is declared here.
#[test]
fn an_id_and_a_date_cross_back_to_the_values_they_were() {
    let mut stored = row();
    stored.insert("total", "seven");
    let mut handed: Option<Document> = None;
    let read = StoredRow::from_bson_with(stored, |document, _| {
        handed = Some(document.clone());
        document.insert("total", 7_i64);
        stored_row_schema::Verdict::Fixed
    });
    assert_eq!(
        handed,
        Some(doc! {
            "at": bson::DateTime::from_millis(1_700_000_000_000),
            "count": 7_i64,
            "id": id(),
            "label": "  A  ",
            "total": "seven",
            "weight": 1.5_f64,
        })
    );
    assert_eq!(
        read.ok().map(|row| (row.id, row.total)),
        Some((id(), 7_i64))
    );
}
