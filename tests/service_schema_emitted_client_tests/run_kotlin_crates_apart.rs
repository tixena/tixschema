//! The Kotlin of two crates compiled into one package: each helper an emitter numbers carries its
//! crate's name after its number, so neither crate declares a name the other does.

#![cfg(feature = "kotlin")]

use invoice_lookups::{ParcelWeight, parcel_weight_kotlin};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// A weight this crate reads through a number helper of its own.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StampWeight {
    pub grams: u32,
}

/// The name of every number helper `kotlin` declares.
fn number_helpers(kotlin: &str) -> Vec<String> {
    Regex::new(r"object (KotlinNumber\w+) :").map_or_else(
        |_| Vec::new(),
        |pattern| {
            pattern
                .captures_iter(kotlin)
                .map(|found| found[1].to_owned())
                .collect()
        },
    )
}

#[test]
fn the_kotlin_of_two_crates_compiles_in_one_package() {
    let ours = stamp_weight_kotlin::kotlin_definition();
    let theirs = parcel_weight_kotlin::kotlin_definition();
    let (our_helpers, their_helpers) = (number_helpers(&ours), number_helpers(&theirs));
    assert!(
        !our_helpers.is_empty()
            && our_helpers
                .iter()
                .all(|named| named.ends_with("ServiceSchemaEmittedClientTests")),
        "{ours}"
    );
    assert!(
        !their_helpers.is_empty()
            && their_helpers
                .iter()
                .all(|named| named.ends_with("InvoiceLookups")),
        "{theirs}"
    );
    let main = "fun main() {\n\
         println(Json.encodeToString(StampWeight.serializer(), \
         Json.decodeFromString(StampWeight.serializer(), \"\"\"{\"grams\":3}\"\"\")))\n\
         println(Json.encodeToString(ParcelWeight.serializer(), \
         Json.decodeFromString(ParcelWeight.serializer(), \"\"\"{\"grams\":4}\"\"\")))\n\
         }\n";
    let serde_read = [
        serde_json::to_string(&serde_json::from_str::<StampWeight>(r#"{"grams":3}"#).unwrap()),
        serde_json::to_string(&serde_json::from_str::<ParcelWeight>(r#"{"grams":4}"#).unwrap()),
    ]
    .map(Result::unwrap);
    let source = [KOTLIN_IMPORTS, &ours, &theirs, main];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, format!("{}\n{}\n", serde_read[0], serde_read[1]));
}
