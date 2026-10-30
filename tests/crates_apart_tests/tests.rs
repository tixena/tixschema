//! An issue inside a type of another crate, reported at its full path, and the types of a crate
//! whose types are never stored, read through that crate's own library and held by a stored type
//! here.

use core::fmt::Debug;
use invoice_schema::DecodeWith as _;

use bson::{Bson, doc};
use invoice_lookup_schema::DecodeWith as _;
use invoice_lookups::{InvoiceLookup, Parcel, invoice_lookup_schema};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use stored_customers::Customer;
use tixschema::model_schema;

// The JSON schema of a type holding a `Customer` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use stored_customers::customer_schema;

/// The manifest of the crate whose types list `"decode_with"` alone.
const LOOKUPS_MANIFEST: &str = include_str!("../crates_apart/invoice_lookups/Cargo.toml");

/// The root of the crate whose types list `"decode_with"` alone.
const LOOKUPS_ROOT: &str = include_str!("../crates_apart/invoice_lookups/src/lib.rs");

/// A stored invoice of this crate, billing a customer of another.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub customer: Customer,
    pub number: String,
}

/// A stored log of this crate, holding a lookup of a crate whose types list `"decode_with"` alone.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LookupLog {
    pub lookup: InvoiceLookup,
    pub number: String,
}

/// Each issue among `handed` that is `Invalid`, as its path, its expected type and the value it
/// holds, every other one as its `Debug` text.
fn listed<V>(
    handed: &[invoice_schema::Issue<V>],
) -> Vec<Result<(String, invoice_schema::Expected, V), String>>
where
    V: Clone + Debug,
{
    handed
        .iter()
        .map(|issue| {
            if let invoice_schema::Issue::Invalid {
                path,
                expected,
                found,
                reason: _reason,
            } = issue
            {
                Ok((path.to_string(), expected.clone(), found.clone()))
            } else {
                Err(format!("{issue:?}"))
            }
        })
        .collect()
}

#[test]
fn an_issue_inside_a_type_of_another_crate_is_reported_at_its_full_path_from_bson() {
    let mut handed: Vec<invoice_schema::Issue<Bson>> = Vec::new();
    let read = Invoice::from_bson_with(
        doc! { "customer": { "name": "Ada", "openInvoices": "3" }, "number": "INV-1" },
        |_raw, found| {
            handed = found.to_vec();
            invoice_schema::Verdict::Reject
        },
    );
    assert_eq!(
        listed(&handed),
        [Ok((
            "customer.openInvoices".to_owned(),
            invoice_schema::Expected::U32,
            Bson::String("3".to_owned())
        ))]
    );
    assert_eq!(read.map_err(|refused| refused.issues.len()), Err(1));
}

#[test]
fn an_issue_inside_a_type_of_another_crate_is_reported_at_its_full_path_from_json() {
    let mut handed: Vec<invoice_schema::Issue<Value>> = Vec::new();
    let read = Invoice::from_value_with(
        json!({ "customer": { "name": "Ada", "openInvoices": "3" }, "number": "INV-1" }),
        |_raw, found| {
            handed = found.to_vec();
            invoice_schema::Verdict::Reject
        },
    );
    assert_eq!(
        listed(&handed),
        [Ok((
            "customer.openInvoices".to_owned(),
            invoice_schema::Expected::U32,
            json!("3")
        ))]
    );
    assert_eq!(read.map_err(|refused| refused.issues.len()), Err(1));
}

#[test]
fn a_value_holding_a_type_of_another_crate_reads_where_nothing_is_wrong() {
    let read = Invoice::from_bson_with(
        doc! { "customer": { "name": "Ada", "openInvoices": 3_i32 }, "number": "INV-1" },
        |_raw, _found| invoice_schema::Verdict::Reject,
    );
    assert_eq!(
        read.ok(),
        Some(Invoice {
            customer: Customer {
                name: "Ada".to_owned(),
                open_invoices: 3,
            },
            number: "INV-1".to_owned(),
        })
    );
}

#[test]
fn the_crate_whose_types_list_decode_with_alone_lists_no_mongodb_and_writes_one_initializer() {
    let lists_mongodb = LOOKUPS_MANIFEST
        .lines()
        .filter_map(|line| line.split_once('=').map(|(name, _value)| name.trim()))
        .any(|name| name == "mongodb");
    assert!(!lists_mongodb, "{LOOKUPS_MANIFEST}");
    assert!(
        LOOKUPS_ROOT.contains("tixschema::decode_with_foundation!();")
            && !LOOKUPS_ROOT.contains("mongodb_foundation"),
        "{LOOKUPS_ROOT}"
    );
}

#[test]
fn a_stored_type_reads_a_type_of_a_crate_whose_types_list_decode_with_alone_from_bson() {
    let row = doc! {
        "lookup": { "number": "INV-1", "parcels": [{ "label": "fragile" }] },
        "number": "L-1",
    };
    let read = LookupLog::from_bson_with(row, |_raw, _found| lookup_log_schema::Verdict::Reject);
    assert_eq!(
        read.ok(),
        Some(LookupLog {
            lookup: InvoiceLookup {
                number: "INV-1".to_owned(),
                parcels: vec![Parcel {
                    label: "fragile".to_owned(),
                }],
            },
            number: "L-1".to_owned(),
        })
    );
    let mut handed: Vec<lookup_log_schema::Issue<Bson>> = Vec::new();
    let broken = doc! {
        "lookup": { "number": "INV-1", "parcels": [{ "label": 7_i32 }] },
        "number": "L-1",
    };
    let refused = LookupLog::from_bson_with(broken, |_raw, found| {
        handed = found.to_vec();
        lookup_log_schema::Verdict::Reject
    });
    assert_eq!(
        listed(&handed),
        [Ok((
            "lookup.parcels[0].label".to_owned(),
            invoice_schema::Expected::String,
            Bson::Int32(7)
        ))]
    );
    assert_eq!(refused.map_err(|unread| unread.issues.len()), Err(1));
}

#[test]
fn a_type_of_a_crate_that_names_no_bson_is_read_through_that_crates_library() {
    let read = InvoiceLookup::from_value_with(
        json!({ "number": "INV-1", "parcels": [{ "label": "fragile" }] }),
        |_raw, _found| invoice_lookup_schema::Verdict::Reject,
    );
    assert_eq!(
        read.ok(),
        Some(InvoiceLookup {
            number: "INV-1".to_owned(),
            parcels: vec![Parcel {
                label: "fragile".to_owned(),
            }],
        })
    );
}

#[test]
fn an_issue_inside_a_type_of_a_crate_that_names_no_bson_is_reported_at_its_full_path() {
    let mut paths = Vec::new();
    let read = InvoiceLookup::from_value_with(
        json!({ "number": "INV-1", "parcels": [{ "label": 7_i32 }] }),
        |_raw, found| {
            paths = found
                .iter()
                .map(|issue| {
                    if let invoice_lookup_schema::Issue::Invalid {
                        path,
                        expected: _expected,
                        found: _found,
                        reason: _reason,
                    } = issue
                    {
                        path.to_string()
                    } else {
                        format!("{issue:?}")
                    }
                })
                .collect();
            invoice_lookup_schema::Verdict::Reject
        },
    );
    assert_eq!(paths, ["parcels[0].label"]);
    assert_eq!(read.map_err(|refused| refused.issues.len()), Err(1));
}
