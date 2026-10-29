//! A generic model whose container writes `#[serde(bound)]` puts that bound, and no more, on the
//! MongoDB layer and the recovering reads, as serde's derive does for its own impls.

use core::marker::PhantomData;

use bson::doc;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;

pub trait Extension {}

pub struct NoSerde;
impl Extension for NoSerde {}
impl Extension for String {}

#[derive(Deserialize)]
pub struct ReadOnly;
impl Extension for ReadOnly {}

#[derive(Serialize)]
pub struct WriteOnly;
impl Extension for WriteOnly {}

#[model_schema(enable = ["decode_with", "mongodb"], default_types(E = String))]
#[derive(Deserialize, Serialize)]
#[serde(bound = "")]
pub struct Holder<E: Extension> {
    #[serde(skip)]
    pub kind: PhantomData<E>,
    pub title: String,
}

#[model_schema(enable = ["decode_with", "mongodb"], default_types(E = String))]
#[derive(Deserialize, Serialize)]
#[serde(bound(serialize = ""))]
pub struct WrittenFree<E: Extension> {
    #[serde(skip)]
    pub kind: PhantomData<E>,
    pub title: String,
}

#[model_schema(enable = ["decode_with", "mongodb"], default_types(E = String))]
#[derive(Deserialize, Serialize)]
#[serde(bound(deserialize = ""))]
pub struct ReadFree<E: Extension> {
    #[serde(skip)]
    pub kind: PhantomData<E>,
    pub title: String,
}

#[model_schema(enable = ["decode_with", "mongodb"], default_types(E = String))]
#[derive(Deserialize, Serialize)]
#[serde(bound(
    deserialize = "E: serde::Deserialize<'de>",
    serialize = "E: serde::Serialize"
))]
pub struct Tagged<E: Extension> {
    pub kind: E,
    pub title: String,
}

#[test]
fn a_parameter_serde_never_writes_needs_no_serde_for_the_paths_or_the_reads() {
    let paths = Holder::<NoSerde>::MONGO_FIELDS;
    assert_eq!(
        shown(paths.title().eq("t".to_owned()).unwrap()),
        r#"{ "title": { "$eq": "t" } }"#
    );
    let read_value = Holder::<NoSerde>::from_value_piped(serde_json::json!({ "title": "t" }), &[]);
    assert_eq!(read_value.unwrap().title, "t");
    let read_document = Holder::<NoSerde>::from_bson_piped(doc! { "title": "t" }, &[]);
    assert_eq!(read_document.unwrap().title, "t");
}

#[test]
fn a_written_serialize_side_leaves_the_reads_and_the_paths_to_their_own_sides() {
    let paths = WrittenFree::<ReadOnly>::MONGO_FIELDS;
    assert_eq!(
        shown(paths.title().eq("t".to_owned()).unwrap()),
        r#"{ "title": { "$eq": "t" } }"#
    );
    let read_value =
        WrittenFree::<ReadOnly>::from_value_piped(serde_json::json!({ "title": "t" }), &[]);
    assert_eq!(read_value.unwrap().title, "t");
}

#[test]
fn a_written_deserialize_side_leaves_the_reads_free_of_the_parameter() {
    let read_value =
        ReadFree::<WriteOnly>::from_value_piped(serde_json::json!({ "title": "t" }), &[]);
    assert_eq!(read_value.unwrap().title, "t");
    let read_document = ReadFree::<WriteOnly>::from_bson_piped(doc! { "title": "t" }, &[]);
    assert_eq!(read_document.unwrap().title, "t");
    let paths = ReadFree::<WriteOnly>::MONGO_FIELDS;
    assert_eq!(
        shown(paths.title().eq("t".to_owned()).unwrap()),
        r#"{ "title": { "$eq": "t" } }"#
    );
}

#[test]
fn a_parameter_held_in_a_written_field_takes_the_bounds_the_container_writes() {
    let paths = Tagged::<String>::MONGO_FIELDS;
    assert_eq!(
        shown(paths.title().eq("t".to_owned()).unwrap()),
        r#"{ "title": { "$eq": "t" } }"#
    );
    let read_value =
        Tagged::<String>::from_value_piped(serde_json::json!({ "kind": "k", "title": "t" }), &[]);
    assert_eq!(read_value.unwrap().kind, "k");
    let read_document = Tagged::<String>::from_bson_piped(doc! { "kind": "k", "title": "t" }, &[]);
    assert_eq!(read_document.unwrap().title, "t");
}
