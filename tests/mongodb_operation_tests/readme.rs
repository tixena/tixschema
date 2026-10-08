//! The README's "MongoDB Operations and Typed Filters" section, held as one text with the code
//! that compiles. This module holds the model the section's examples share; `paths` holds what
//! the section shows of the typed paths, and `operations` what it shows of the operations.
//!
//! This crate's lints order a struct's fields alphabetically and the README orders them for
//! reading, so `Invoice` is held member by member, each one whole.

mod operations;
mod paths;
// `IpAddr` is a type no schema surface and no client emitter describes.
#[cfg(not(any(
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema",
    feature = "dart",
    feature = "swift",
    feature = "kotlin"
)))]
mod stored_forms;

use std::collections::HashMap;

use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The compiler errors the README quotes, each beside the file whose doc comments record it under
/// the run that earned it.
const QUOTED_ERRORS: [(&str, &str); 12] = [
    (
        QUERY_EMITTER,
        "error[E0599]: no method named `unset` found for struct `invoice_schema::Field<Root, V>` in the current scope",
    ),
    (
        QUERY_EMITTER,
        "error[E0277]: the trait bound `order_schema::Filter<Order>: AsRef<PhantomData<Invoice>>` is not satisfied",
    ),
    (
        QUERY_EMITTER,
        "error[E0277]: the trait bound `bson::Document: AsRef<PhantomData<Invoice>>` is not satisfied",
    ),
    (
        QUERY_EMITTER,
        "error[E0277]: the trait bound `Update<Invoice>: AsRef<PhantomData<Invoice>>` is not satisfied",
    ),
    (
        QUERY_EMITTER,
        "error[E0277]: the trait bound `invoice_schema::Filter<Invoice>: AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not satisfied",
    ),
    (
        QUERY_EMITTER,
        "error[E0080]: evaluation panicked: a typed MongoDB path holds at most 8 keys",
    ),
    (
        PATHS_EMITTER,
        "error[E0015]: cannot perform non-const deref coercion on `invoice_schema::Model<Invoice, Customer, customer_schema::MongoFields<Invoice>>` in constants",
    ),
    (
        PATHS_EMITTER,
        "error[E0308]: `?` operator has incompatible types",
    ),
    (PATHS_EMITTER, "error[E0308]: mismatched types"),
    (PATHS_EMITTER, "expected `InvoiceStatus`, found `&str`"),
    (
        OPERATIONS_EMITTER,
        "error[E0277]: the trait bound `invoice_schema::Update<Invoice>: AsRef<PhantomData<Invoice>>` is not satisfied",
    ),
    (
        OPERATIONS_EMITTER,
        "error[E0277]: the trait bound `invoice_schema::Filter<Invoice>: AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not satisfied",
    ),
];

/// The line the README lists the driver built for version 3 of the `bson` library with.
const SHOWN_DRIVER_FOR_3: &str = r#"mongodb = { version = "3.9", default-features = false, features = ["bson-3", "compat-3-3-0", "rustls-tls", "dns-resolver"] }"#;

/// The emitters whose doc comments record a refusal beside the run that earned it.
const OPERATIONS_EMITTER: &str = include_str!("../../src/features/recovering_decode/operations.rs");
const PATHS_EMITTER: &str = include_str!("../../src/features/recovering_decode/fields.rs");
const QUERY_EMITTER: &str = include_str!("../../src/features/recovering_decode/query.rs");

/// `Invoice` as the README declares it: what stands above its members, then each member in the
/// README's order.
const DECLARED_INVOICE: [&str; 12] = [
    r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {"#,
    r#"    #[serde(rename = "_id")]
    pub id: ObjectId,"#,
    r#"    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing: Option<Address>,"#,
    "    pub customer: Customer,",
    "    pub details: HashMap<String, String>,",
    "    pub items: Vec<LineItem>,",
    "    pub number: String,",
    r#"    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,"#,
    "    pub payment: Payment,",
    "    pub status: InvoiceStatus,",
    "    pub tags: Vec<String>,",
    "    pub total: f64,",
];

/// The models `Invoice` holds, as the README declares them above it, character for character.
const DECLARED_MODELS: [&str; 5] = [
    r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub city: String,
    pub postal_code: String,
}"#,
    r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub address: Address,
    pub name: String,
    pub open_invoices: u32,
}"#,
    r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvoiceStatus {
    Draft,
    Paid,
    PastDue,
}"#,
    "#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct LineItem {
    pub price: f64,
    pub quantity: u32,
    pub sku: String,
}",
    r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Payment {
    Card { last4: String },
    Cash,
}"#,
];

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub city: String,
    pub postal_code: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub address: Address,
    pub name: String,
    pub open_invoices: u32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvoiceStatus {
    Draft,
    Paid,
    PastDue,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct LineItem {
    pub price: f64,
    pub quantity: u32,
    pub sku: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Payment {
    Card { last4: String },
    Cash,
}

// Below every model it holds: a field typed with a model declared above reaches that model's own
// paths, and one typed with a model declared below is one whole value.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing: Option<Address>,
    pub customer: Customer,
    pub details: HashMap<String, String>,
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub items: Vec<LineItem>,
    pub number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
    pub payment: Payment,
    pub status: InvoiceStatus,
    pub tags: Vec<String>,
    pub total: f64,
}

/// A declaration the README shows and `source` compiles, held to being one text: it appears in
/// `source` twice, as the constant the README is searched for and as what the compiler reads.
fn assert_declared_and_documented(source: &str, pinned: &str) {
    assert_eq!(
        source.matches(pinned).count(),
        2,
        "this is pinned but no longer declared in its file character for character:\n{pinned}"
    );
    assert_documented(pinned);
}

/// A text the README shows, held to being in it verbatim.
fn assert_documented(shown: &str) {
    assert!(
        readme().contains(shown),
        "the README no longer shows this verbatim:\n{shown}"
    );
}

/// An invoice of the customer named `customer`, past due, with nothing the checks do not name.
fn invoice(number: &str, customer: &str, total: f64) -> Invoice {
    Invoice {
        billing: None,
        customer: Customer {
            address: Address {
                city: "Moca".to_owned(),
                postal_code: "56000".to_owned(),
            },
            name: customer.to_owned(),
            open_invoices: 1,
        },
        details: HashMap::new(),
        id: ObjectId::new(),
        items: Vec::new(),
        number: number.to_owned(),
        paid_at: None,
        payment: Payment::Cash,
        status: InvoiceStatus::PastDue,
        tags: Vec::new(),
        total,
    }
}

fn readme() -> &'static str {
    include_str!("../../README.md")
}

/// This file, read back so a pinned declaration can be held against the one that compiles.
fn source() -> &'static str {
    include_str!("readme.rs")
}

/// `bson3/` is built with the driver line the README gives for version 3.
#[test]
fn the_readme_lists_the_driver_this_repository_builds_for_version_3() {
    assert!(
        include_str!("../../bson3/Cargo.toml").contains(SHOWN_DRIVER_FOR_3),
        "`bson3/Cargo.toml` no longer lists the driver as the README does"
    );
    assert_documented(SHOWN_DRIVER_FOR_3);
}

#[test]
fn the_readme_quotes_errors_recorded_beside_the_runs_that_earned_them() {
    for (emitter, quoted) in QUOTED_ERRORS {
        assert!(
            emitter.contains(quoted),
            "no doc comment of the emitter records this error any more:\n{quoted}"
        );
        assert_documented(quoted);
    }
}

#[test]
fn the_readme_declares_the_model_that_compiles_here() {
    for pinned in DECLARED_MODELS.into_iter().chain(DECLARED_INVOICE) {
        assert_declared_and_documented(source(), pinned);
    }
    assert_documented(&format!("{}\n}}", DECLARED_INVOICE.join("\n")));
}
