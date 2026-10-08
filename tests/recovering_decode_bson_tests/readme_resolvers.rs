//! The README's resolvers example, held as one text with the code that compiles, and run.
//!
//! Two of the lines it prints end in the `bson` library's own wording, which its two major
//! versions write differently: the README shows both, and each binary holds the run to its own.

use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use invoice_schema::{Expected, Issue, Resolution};

use crate::BSON_MAJOR;

/// The inner type the README declares, character for character.
const DECLARED_CUSTOMER: &str = r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}"#;

/// The line the README brings the resolvers' types into scope with.
const DECLARED_IMPORT: &str = "use invoice_schema::{Expected, Issue, Resolution};";

/// The outer type the README declares, character for character.
const DECLARED_INVOICE: &str = "#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub customer: Customer,
    pub number: String,
}";

/// The README's two resolvers, the rows it stores and the read it makes of each, character for
/// character.
const DECLARED_RESOLVERS: [&str; 4] = [
    r#"/// A customer an older writer stored as JSON text: parsed into a document.
fn customer_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::Model("Customer"),
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) else {
        return Resolution::Rejected;
    };
    // Each major version of the library names its own function for this; both have the serializer.
    let Ok(Bson::Document(customer)) = parsed.serialize(bson::Serializer::new()) else {
        return Resolution::Rejected;
    };
    if path.set_in_document(raw, Bson::Document(customer)) {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}"#,
    "/// A whole number an older writer stored as text; a negative count is refused.
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
}",
    r#"/// Five rows older writers left.
fn stored_rows() -> [Document; 5] {
    [
        // a count stored as text
        doc! {
            "customer": { "name": "Acme", "openInvoices": "3" },
            "number": "INV-0042",
        },
        // a count stored as text that is no count
        doc! {
            "customer": { "name": "Acme", "openInvoices": "-3" },
            "number": "INV-0042",
        },
        // a key no type declares
        doc! {
            "customer": { "name": "Acme", "openInvoices": 3 },
            "legacy": true,
            "number": "INV-0042",
        },
        // a customer stored as text, beside a key no type declares
        doc! {
            "customer": "{\"name\":\"Acme\",\"openInvoices\":3}",
            "legacy": true,
            "number": "INV-0042",
        },
        // a customer stored as text, whose count is text too
        doc! {
            "customer": "{\"name\":\"Acme\",\"openInvoices\":\"3\"}",
            "number": "INV-0042",
        },
    ]
}"#,
    r#"/// One stored row read with both resolvers, told in one line.
fn told(row: Document) -> String {
    match Invoice::from_bson_piped(row, &[&customer_as_text, &whole_number_as_text]) {
        Ok(invoice) => format!("read as {invoice:?}"),
        Err(refused) => format!("refused: {refused}"),
    }
}"#,
];

/// What the README shows each stored row told as, under version 2 of the `bson` library.
const SHOWN_UNDER_2: [&str; 5] = [
    r#"read as Invoice { customer: Customer { name: "Acme", open_invoices: 3 }, number: "INV-0042" }"#,
    r#"refused: customer.openInvoices: invalid: expected U32, found String("-3"): invalid type: string "-3", expected u32"#,
    "refused: legacy: unknown: found Boolean(true)",
    "refused: legacy: unknown: found Boolean(true)",
    r#"refused: customer.openInvoices: invalid: expected U32, found String("3"): invalid type: string "3", expected u32"#,
];

/// The two refusals that end in the library's own wording, as version 3 writes them.
const SHOWN_UNDER_3: [&str; 2] = [
    r#"refused: customer.openInvoices: invalid: expected U32, found String("-3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "-3", expected u32."#,
    r#"refused: customer.openInvoices: invalid: expected U32, found String("3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "3", expected u32."#,
];

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub customer: Customer,
    pub number: String,
}

/// A customer an older writer stored as JSON text: parsed into a document.
fn customer_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::Model("Customer"),
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) else {
        return Resolution::Rejected;
    };
    // Each major version of the library names its own function for this; both have the serializer.
    let Ok(Bson::Document(customer)) = parsed.serialize(bson::Serializer::new()) else {
        return Resolution::Rejected;
    };
    if path.set_in_document(raw, Bson::Document(customer)) {
        Resolution::Settled
    } else {
        Resolution::Rejected
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

/// Five rows older writers left.
fn stored_rows() -> [Document; 5] {
    [
        // a count stored as text
        doc! {
            "customer": { "name": "Acme", "openInvoices": "3" },
            "number": "INV-0042",
        },
        // a count stored as text that is no count
        doc! {
            "customer": { "name": "Acme", "openInvoices": "-3" },
            "number": "INV-0042",
        },
        // a key no type declares
        doc! {
            "customer": { "name": "Acme", "openInvoices": 3 },
            "legacy": true,
            "number": "INV-0042",
        },
        // a customer stored as text, beside a key no type declares
        doc! {
            "customer": "{\"name\":\"Acme\",\"openInvoices\":3}",
            "legacy": true,
            "number": "INV-0042",
        },
        // a customer stored as text, whose count is text too
        doc! {
            "customer": "{\"name\":\"Acme\",\"openInvoices\":\"3\"}",
            "number": "INV-0042",
        },
    ]
}

/// One stored row read with both resolvers, told in one line.
fn told(row: Document) -> String {
    match Invoice::from_bson_piped(row, &[&customer_as_text, &whole_number_as_text]) {
        Ok(invoice) => format!("read as {invoice:?}"),
        Err(refused) => format!("refused: {refused}"),
    }
}

fn readme() -> &'static str {
    include_str!("../../README.md")
}

/// This file, read back so a pinned declaration can be held against the one that compiles.
fn source() -> &'static str {
    include_str!("readme_resolvers.rs")
}

/// A declaration the README shows and this module compiles, held to being one text: it appears
/// here twice, as the constant the README is searched for and as what the compiler reads.
fn assert_declared_and_documented(pinned: &str) {
    assert_eq!(
        source().matches(pinned).count(),
        2,
        "this is pinned but no longer declared here character for character:\n{pinned}"
    );
    assert!(
        readme().contains(pinned),
        "the README no longer declares this verbatim:\n{pinned}"
    );
}

#[test]
fn the_readme_declares_the_resolvers_that_compile_here() {
    for pinned in [DECLARED_CUSTOMER, DECLARED_IMPORT, DECLARED_INVOICE]
        .into_iter()
        .chain(DECLARED_RESOLVERS)
    {
        assert_declared_and_documented(pinned);
    }
}

#[test]
fn the_readme_shows_what_each_stored_row_is_told_as() {
    let told: Vec<String> = stored_rows().into_iter().map(told).collect();
    let shown = if BSON_MAJOR == 2 {
        SHOWN_UNDER_2
    } else {
        let [read, _rejected, unknown, settled, _uncovered] = SHOWN_UNDER_2;
        let [rejected, uncovered] = SHOWN_UNDER_3;
        [read, rejected, unknown, settled, uncovered]
    };
    assert_eq!(told, shown);
    for block in [SHOWN_UNDER_2.join("\n"), SHOWN_UNDER_3.join("\n")] {
        assert!(
            readme().contains(&block),
            "the README no longer shows these lines verbatim:\n{block}"
        );
    }
}
