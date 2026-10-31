//! What a type that lists `"mongodb"` has and a type that lists `"decode_with"` alone has not, by the
//! features tixschema is built with, and what a stored type may hold: a stored model in every
//! position, a generic one, leaves, and a type with no walker of its own, read whole.

#[cfg(all(
    feature = "mongodb",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
mod plain;

#[cfg(feature = "bson")]
use invoice_lookup_schema::DecodeWith as _;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Declares `Absent`, and a fallback that answers it for each name given, in the scope it is
/// written in: an item of the type's own is found ahead of the fallback's. Each test writes it in
/// its own body, so no schema module of this file reads the fallback through its glob.
macro_rules! fallback {
    ([$($constant:ident),*] [$($function:ident),*] [$($method:ident),*]) => {
        #[derive(Debug, Eq, PartialEq)]
        struct Absent;

        trait Fallback {
            $(const $constant: Absent = Absent;)*
            $(fn $function() -> Absent {
                Absent
            })*
            $(fn $method(&self) -> Absent {
                Absent
            })*
        }

        impl<T> Fallback for T {}
    };
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Customer {
    name: String,
    open_invoices: u32,
}

#[model_schema(enable = ["decode_with", "mongodb"], default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Wrapper<T> {
    held: T,
}

/// Stored: what it holds is a stored model, a leaf, or a type with no walker of its own.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Invoice {
    by_region: HashMap<String, Box<Customer>>,
    count: u32,
    customer: Customer,
    customers: Vec<Option<Customer>>,
    details: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    memo: Option<String>,
    number: String,
    shelf: ShelfTag,
    tags: Vec<String>,
    wrapped: Wrapper<String>,
}

/// Read from a request and never stored.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct InvoiceLookup {
    number: String,
}

/// No `enable`, and declared below the stored type that holds it: it has no walker, and the stored
/// type reads it whole.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ShelfTag {
    code: String,
}

/// The read of one row of a collection, by a filter over the rows of `Invoice`.
#[cfg(feature = "mongodb")]
type FindOne = for<'held> fn(
    &'held mongodb::Collection<bson::Document>,
    invoice_schema::Filter<Invoice>,
) -> invoice_schema::Read<'held, Invoice, Option<Invoice>>;

/// Takes `Invoice::find_one`, which builds only where the type has it.
#[cfg(feature = "mongodb")]
const fn found_by(_find: FindOne) {}

/// An invoice of one customer, with every other field empty.
#[cfg(feature = "bson")]
fn invoice() -> Invoice {
    let customer = Customer {
        name: "Ada".to_owned(),
        open_invoices: 1,
    };
    Invoice {
        by_region: HashMap::new(),
        count: 1,
        customer,
        customers: Vec::new(),
        details: HashMap::new(),
        memo: None,
        number: "INV-1".to_owned(),
        shelf: ShelfTag {
            code: "A-1".to_owned(),
        },
        tags: Vec::new(),
        wrapped: Wrapper {
            held: "kept".to_owned(),
        },
    }
}

/// `invoice()` as a BSON document, with `key` holding `value`.
#[cfg(feature = "bson")]
fn stored_with(key: &str, value: bson::Bson) -> bson::Document {
    let mut stored = bson::to_document(&invoice()).unwrap();
    stored.insert(key, value);
    stored
}

#[test]
fn a_lookup_has_no_mongodb_layer_in_any_build() {
    fallback!([MONGO_FIELDS] [find_one, insert_one] []);
    assert_eq!(InvoiceLookup::MONGO_FIELDS, Absent);
    assert_eq!(InvoiceLookup::find_one(), Absent);
    assert_eq!(InvoiceLookup::insert_one(), Absent);
}

/// The build with `bson`: a lookup reads from BSON, as every flagged type does.
#[cfg(feature = "bson")]
#[test]
fn under_bson_a_lookup_reads_from_bson() {
    assert_eq!(
        InvoiceLookup::from_bson_piped(bson::doc! { "number": "INV-1" }, &[]),
        Ok(InvoiceLookup {
            number: "INV-1".to_owned()
        })
    );
}

/// The build with neither `bson` nor `mongodb`: no type has a BSON read or a MongoDB layer, the
/// stored one included.
#[cfg(not(feature = "bson"))]
#[test]
fn without_bson_no_type_has_a_bson_read_or_a_mongodb_layer() {
    fallback!([MONGO_FIELDS] [decode_with_bson_issues, find_one, from_bson_piped, from_bson_with] []);
    assert_eq!(Invoice::MONGO_FIELDS, Absent);
    assert_eq!(Invoice::find_one(), Absent);
    assert_eq!(Invoice::from_bson_with(), Absent);
    assert_eq!(Invoice::from_bson_piped(), Absent);
    assert_eq!(InvoiceLookup::from_bson_with(), Absent);
    assert_eq!(InvoiceLookup::decode_with_bson_issues(), Absent);
}

/// The build with `bson` and not `mongodb`: a stored type reads from BSON, and has no operation and
/// no typed path.
#[cfg(all(feature = "bson", not(feature = "mongodb")))]
#[test]
fn under_bson_alone_a_stored_type_reads_from_bson_and_has_no_mongodb_layer() {
    fallback!([MONGO_FIELDS] [find_one] []);
    assert_eq!(Invoice::MONGO_FIELDS, Absent);
    assert_eq!(Invoice::find_one(), Absent);
    let stored = stored_with("count", bson::Bson::Int32(2));
    assert_eq!(
        Invoice::from_bson_piped(stored, &[]).map(|read| read.count),
        Ok(2)
    );
}

/// The build with `mongodb`: a stored type has the operations, the typed paths and the BSON read.
#[cfg(feature = "mongodb")]
#[test]
fn under_mongodb_a_stored_type_has_the_operations_and_the_typed_paths() {
    found_by(Invoice::find_one);
    let numbered = Invoice::MONGO_FIELDS
        .number()
        .eq("INV-1".to_owned())
        .unwrap();
    assert_eq!(
        numbered.into_document().to_string(),
        r#"{ "number": { "$eq": "INV-1" } }"#
    );
    let nested = Invoice::MONGO_FIELDS.wrapped().nest(
        Wrapper::<String>::MONGO_FIELDS
            .held()
            .eq("kept".to_owned())
            .unwrap(),
    );
    assert_eq!(
        nested.into_document().to_string(),
        r#"{ "wrapped.held": { "$eq": "kept" } }"#
    );
}

/// An issue inside a flagged model is at its full path, and a value of a type with no walker of
/// its own is read whole, one issue at its field.
#[cfg(feature = "bson")]
#[test]
fn a_stored_type_walks_each_flagged_type_and_reads_the_rest_whole() {
    let mut handed: Vec<(String, bool)> = Vec::new();
    let mut stored = stored_with("shelf", bson::Bson::Int32(7));
    stored.insert("wrapped", bson::doc! { "held": 3_i32 });
    let read = Invoice::from_bson_with(stored, |_raw, found| {
        handed = found
            .iter()
            .map(|issue| {
                if let invoice_schema::Issue::Invalid {
                    path,
                    expected,
                    found: _found,
                    reason: _reason,
                } = issue
                {
                    (
                        path.to_string(),
                        *expected == invoice_schema::Expected::Unknown,
                    )
                } else {
                    (format!("{issue:?}"), false)
                }
            })
            .collect();
        invoice_schema::Verdict::Reject
    });
    assert_eq!(
        handed,
        [
            ("shelf".to_owned(), true),
            ("wrapped.held".to_owned(), false)
        ]
    );
    assert_eq!(read.map_err(|refused| refused.issues.len()), Err(2));
}
