//! `from_bson_piped`: resolvers run in order over every issue a read finds, once, each answering
//! one issue, and a refused read holds only the issues none settled.

use core::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use invoice_schema::{Expected, Issue, Path, Resolution, Segment};

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Customer {
    #[model_schema_prop(minLength = 1)]
    name: String,
    open_invoices: u32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Invoice {
    customer: Customer,
    #[model_schema_prop(minLength = 8, pattern = "^INV-[0-9]+$")]
    number: String,
}

/// What a read lists at `number` for the bare `42`, under the bound whose sentence is `reason`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn bare_number(reason: &str) -> Issue<Bson> {
    Issue::Invalid {
        path: Path(vec![Segment::Key("number".to_owned())]),
        expected: Expected::String,
        found: Bson::String("42".to_owned()),
        reason: reason.to_owned(),
    }
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

/// A number an older writer stored bare: written as the type writes it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn number_without_prefix(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::String,
        found: Bson::String(bare),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    if path.to_string() != "number" {
        return Resolution::NotTouched;
    }
    let settled = bare
        .parse::<u32>()
        .is_ok_and(|number| path.set_in_document(raw, Bson::String(format!("INV-{number:04}"))));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

fn invoice() -> Invoice {
    Invoice {
        customer: Customer {
            name: "Acme".to_owned(),
            open_invoices: 3_u32,
        },
        number: "INV-0042".to_owned(),
    }
}

/// The key `legacy`, which no type declares, as a read lists it.
fn legacy() -> Issue<Bson> {
    Issue::Unknown {
        path: Path(vec![Segment::Key("legacy".to_owned())]),
        found: Bson::Boolean(true),
    }
}

#[test]
fn two_resolvers_settle_a_row() {
    let stored = doc! {
        "customer": { "name": "Acme", "openInvoices": "3" },
        "number": "INV-0042",
    };
    let read = Invoice::from_bson_piped(stored, &[&customer_as_text, &whole_number_as_text]);
    assert_eq!(read, Ok(invoice()));
}

#[test]
fn an_issue_a_resolver_rejects_refuses_the_read_and_is_the_one_it_holds() {
    let stored = doc! {
        "customer": { "name": "Acme", "openInvoices": "-3" },
        "number": "INV-0042",
    };
    let refused =
        Invoice::from_bson_piped(stored, &[&customer_as_text, &whole_number_as_text]).unwrap_err();
    assert_eq!(refused.issues.len(), 1);
    let shown = refused.to_string();
    assert!(
        shown.starts_with("customer.openInvoices: invalid: expected U32, found String(\"-3\"): "),
        "got: {shown}"
    );
}

#[test]
fn an_issue_no_resolver_knows_is_the_one_a_refused_read_holds() {
    let stored = doc! {
        "customer": { "name": "Acme", "openInvoices": 3_i64 },
        "legacy": true,
        "number": "INV-0042",
    };
    let refused =
        Invoice::from_bson_piped(stored, &[&customer_as_text, &whole_number_as_text]).unwrap_err();
    assert_eq!(refused.issues, [legacy()]);
}

#[test]
fn a_settled_issue_is_not_among_the_ones_a_refused_read_holds() {
    let stored = doc! {
        "customer": "{\"name\":\"Acme\",\"openInvoices\":3}",
        "legacy": true,
        "number": "INV-0042",
    };
    let refused =
        Invoice::from_bson_piped(stored, &[&customer_as_text, &whole_number_as_text]).unwrap_err();
    assert_eq!(refused.issues, [legacy()]);
}

#[test]
fn a_broken_bound_is_handed_to_the_resolvers() {
    let stored = doc! {
        "customer": { "name": "", "openInvoices": 3_i64 },
        "number": "INV-0042",
    };
    let handed = Mutex::new(Vec::new());
    let noted = |_raw: &mut Document, issue: &Issue<Bson>| {
        handed.lock().unwrap().push(issue.clone());
        Resolution::NotTouched
    };
    let read = Invoice::from_bson_piped(stored, &[&noted]);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        let nameless = Issue::Invalid {
            path: Path(vec![
                Segment::Key("customer".to_owned()),
                Segment::Key("name".to_owned()),
            ]),
            expected: Expected::String,
            found: Bson::String(String::new()),
            reason: "too short: minimum length is 1, got 0".to_owned(),
        };
        let refused = read.unwrap_err();
        assert_eq!(*handed.lock().unwrap(), refused.issues);
        assert_eq!(refused.issues, [nameless]);
        assert_eq!(
            refused.to_string(),
            "customer.name: invalid: expected String, found String(\"\"): too short: minimum \
             length is 1, got 0"
        );
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(read.unwrap().customer.name, "");
        assert!(handed.lock().unwrap().is_empty());
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_resolver_answers_both_issues_of_a_value_that_breaks_two_bounds() {
    let stored = doc! {
        "customer": { "name": "Acme", "openInvoices": 3_i64 },
        "number": "42",
    };
    let handed = Mutex::new(Vec::new());
    let noted = |raw: &mut Document, issue: &Issue<Bson>| {
        handed
            .lock()
            .unwrap()
            .push((raw.get_str("number").unwrap().to_owned(), issue.clone()));
        number_without_prefix(raw, issue)
    };
    let read = Invoice::from_bson_piped(stored, &[&noted]);
    assert_eq!(read, Ok(invoice()));
    assert_eq!(
        *handed.lock().unwrap(),
        [
            (
                "42".to_owned(),
                bare_number("too short: minimum length is 8, got 2")
            ),
            (
                "INV-0042".to_owned(),
                bare_number("does not match pattern '^INV-[0-9]+$'")
            ),
        ]
    );
}

/// The pipe counts issues, never paths.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_issue_left_not_touched_at_a_repaired_value_refuses_the_read() {
    let stored = doc! {
        "customer": { "name": "Acme", "openInvoices": 3_i64 },
        "number": "42",
    };
    let while_bare = |raw: &mut Document, issue: &Issue<Bson>| {
        if matches!(raw.get_str("number"), Ok("42")) {
            number_without_prefix(raw, issue)
        } else {
            Resolution::NotTouched
        }
    };
    let refused = Invoice::from_bson_piped(stored, &[&while_bare]).unwrap_err();
    assert_eq!(
        refused.issues,
        [bare_number("does not match pattern '^INV-[0-9]+$'")]
    );
}

#[test]
fn a_later_resolver_is_never_handed_an_issue_an_earlier_one_settled() {
    let stored = doc! {
        "customer": { "name": "Acme", "openInvoices": "3" },
        "number": "INV-0042",
    };
    let handed = AtomicU32::new(0);
    let rejecting = |_raw: &mut Document, _issue: &Issue<Bson>| {
        handed.fetch_add(1, Ordering::Relaxed);
        Resolution::Rejected
    };
    let read = Invoice::from_bson_piped(stored.clone(), &[&whole_number_as_text, &rejecting]);
    assert_eq!(read, Ok(invoice()));
    assert_eq!(handed.load(Ordering::Relaxed), 0);

    let refused =
        Invoice::from_bson_piped(stored, &[&rejecting, &whole_number_as_text]).unwrap_err();
    assert_eq!(refused.issues.len(), 1);
    assert_eq!(handed.load(Ordering::Relaxed), 1);
}

#[test]
fn an_issue_a_repair_uncovers_refuses_the_read() {
    let stored = doc! {
        "customer": "{\"name\":\"Acme\",\"openInvoices\":\"3\"}",
        "number": "INV-0042",
    };
    let handed = AtomicU32::new(0);
    let counted = |raw: &mut Document, issue: &Issue<Bson>| {
        handed.fetch_add(1, Ordering::Relaxed);
        whole_number_as_text(raw, issue)
    };
    let refused = Invoice::from_bson_piped(stored, &[&customer_as_text, &counted]).unwrap_err();
    assert_eq!(refused.issues.len(), 1);
    let shown = refused.to_string();
    assert!(
        shown.starts_with("customer.openInvoices: invalid: expected U32, found String(\"3\"): "),
        "got: {shown}"
    );
    assert_eq!(handed.load(Ordering::Relaxed), 0);
}
