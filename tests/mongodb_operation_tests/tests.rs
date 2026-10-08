//! The operations `#[model_schema(decode_with)]` adds under `mongodb`. Two binaries compile this
//! module, each with the name `bson` bound to one major version of the library and a MongoDB
//! driver built for it, so every check is compiled against both.
//!
//! `live` runs the operations against a real collection, and stands down where no server is
//! named. `offline` holds what needs no server: the error, the read of a row once the driver has
//! handed it over, a row refused before it is sent, and what a read holds before it is awaited.

mod live;
mod offline;
mod shadowing;
mod uncalled;

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use invoice_schema::{Expected, Filter, Issue, OperationError, Path, Resolution, Segment};

/// The `_id` of the row every check seeds.
const SEEDED_ID: &str = "6a7cc592ca0574e6efdfe217";

/// A nested model that declares no `_id` of its own.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Customer {
    name: String,
    open_invoices: u32,
}

/// A row, with the `_id` MongoDB stores every row under.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Invoice {
    customer: Customer,
    #[serde(rename = "_id")]
    id: ObjectId,
    number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paid_at: Option<String>,
    status: InvoiceStatus,
    total: u32,
}

#[model_schema(decode_with)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
enum InvoiceStatus {
    Draft,
    Paid,
    PastDue,
}

/// A filter every row of the collection matches.
fn every_row() -> Filter<Invoice> {
    Filter::raw(doc! {})
}

/// What the seeded row reads as.
fn invoice() -> Invoice {
    Invoice {
        customer: Customer {
            name: "Acme".to_owned(),
            open_invoices: 3_u32,
        },
        id: seeded_id(),
        number: "INV-0042".to_owned(),
        paid_at: None,
        status: InvoiceStatus::PastDue,
        total: 1_250_u32,
    }
}

/// Whether `refused` is the row [`unreadable`] stores, told by its `_id` and its one issue.
fn names_the_unreadable_row(refused: &OperationError) -> bool {
    matches!(
        refused,
        OperationError::Unreadable { row, issues }
            if *row == format!("ObjectId(\"{SEEDED_ID}\")")
                && matches!(
                    issues.as_slice(),
                    [Issue::Invalid {
                        path,
                        expected: Expected::U32,
                        found: Bson::String(text),
                        reason: _reason,
                    }] if *path == open_invoices() && text == "3"
                )
    )
}

/// The invoice numbered `at`, past due, under an `_id` of its own and a total that grows with
/// its number.
fn numbered(at: u32) -> Invoice {
    let id = format!("{seeded}{at:08x}", seeded = &SEEDED_ID[..16]);
    Invoice {
        id: ObjectId::parse_str(id).unwrap(),
        number: format!("INV-{at:04}"),
        total: at * 100_u32,
        ..invoice()
    }
}

/// Where the seeded row holds its customer's open invoices.
fn open_invoices() -> Path {
    Path(vec![
        Segment::Key("customer".to_owned()),
        Segment::Key("openInvoices".to_owned()),
    ])
}

/// The row as the type writes it.
fn readable() -> Document {
    doc! {
        "customer": { "name": "Acme", "openInvoices": 3_i64 },
        "_id": seeded_id(),
        "number": "INV-0042",
        "status": "PastDue",
        "total": 1_250_i64,
    }
}

fn seeded_id() -> ObjectId {
    ObjectId::parse_str(SEEDED_ID).unwrap()
}

/// `row` as the document the type writes for it.
fn stored(row: &Invoice) -> Document {
    row.serialize(bson::Serializer::new())
        .unwrap()
        .as_document()
        .unwrap()
        .clone()
}

/// The row as an older writer left it, with `openInvoices: "3"`.
fn unreadable() -> Document {
    doc! {
        "customer": { "name": "Acme", "openInvoices": "3" },
        "_id": seeded_id(),
        "number": "INV-0042",
        "status": "PastDue",
        "total": 1_250_i64,
    }
}

/// A whole number an older writer stored as text; a negative count is refused.
fn whole_number_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::U32,
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let settled = text
        .trim()
        .parse::<i64>()
        .is_ok_and(|number| number >= 0_i64 && path.set_in_document(raw, Bson::Int64(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}
