//! The operations `#[model_schema(decode_with)]` adds under `mongodb`. Two binaries compile this
//! module, each with the name `bson` bound to one major version of the library and a MongoDB
//! driver built for it, so every check is compiled against both.
//!
//! `live` runs the operations against a real collection, and stands down where no server is
//! named. `offline` holds what needs no server: the error, and the read of a row once the driver
//! has handed it over.

mod live;
mod offline;
mod shadowing;
mod uncalled;

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use invoice_schema::{Expected, Issue, Path, Resolution, Segment};

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
    }
}

fn seeded_id() -> ObjectId {
    ObjectId::parse_str(SEEDED_ID).unwrap()
}

/// The row as an older writer left it, with `openInvoices: "3"`.
fn unreadable() -> Document {
    doc! {
        "customer": { "name": "Acme", "openInvoices": "3" },
        "_id": seeded_id(),
        "number": "INV-0042",
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
