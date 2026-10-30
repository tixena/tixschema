//! A type of one module holding a type of another, both read through the support libraries the
//! crate writes once: an issue inside the held type is reported at its full path, from a BSON
//! document and from a JSON value, and the callback's, the resolvers' and the query's types of the
//! two modules are one.

mod billing;
mod customers;

use core::fmt::Debug;
use invoice_schema::DecodeWith as _;

use bson::{Bson, Document, doc};
use serde_json::{Value, json};

use self::billing::{Invoice, invoice_schema};
use self::customers::{Customer, customer_schema};

/// An invoice whose customer holds its count of open invoices as text.
fn stored() -> Document {
    doc! { "customer": { "name": "Ada", "openInvoices": "3" }, "number": "INV-1" }
}

/// What every read of [`stored`] settles to.
fn settled() -> Invoice {
    Invoice {
        customer: Customer {
            name: "Ada".to_owned(),
            open_invoices: 3,
        },
        number: "INV-1".to_owned(),
    }
}

/// Settles a count of open invoices an older writer stored as text, wherever it sits: one
/// resolver, written against one module's `Issue`, for the reads of both modules' types.
fn count_as_text(
    raw: &mut Document,
    issue: &customer_schema::Issue<Bson>,
) -> customer_schema::Resolution {
    use customer_schema::Stored as _;
    let customer_schema::Issue::Invalid {
        path,
        expected: customer_schema::Expected::U32,
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return customer_schema::Resolution::NotTouched;
    };
    if text
        .parse::<i64>()
        .is_ok_and(|count| raw.set_at(path, Bson::Int64(count)))
    {
        customer_schema::Resolution::Settled
    } else {
        customer_schema::Resolution::Rejected
    }
}

/// Each issue among `handed`, which are all `Invalid`, as its path, its expected type and the
/// value it holds.
fn invalid_values<V>(
    handed: &[customer_schema::Issue<V>],
) -> Vec<(String, customer_schema::Expected, V)>
where
    V: Clone + Debug,
{
    let invalid: Vec<(String, customer_schema::Expected, V)> = handed
        .iter()
        .filter_map(|issue| {
            if let customer_schema::Issue::Invalid {
                path,
                expected,
                found,
                reason: _reason,
            } = issue
            {
                Some((path.to_string(), expected.clone(), found.clone()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(invalid.len(), handed.len(), "got: {handed:?}");
    invalid
}

#[test]
fn an_issue_inside_a_type_of_another_module_is_reported_at_its_full_path_from_bson() {
    // The issues of `Invoice` are held as `Customer`'s module names them: one type.
    let mut handed: Vec<customer_schema::Issue<Bson>> = Vec::new();
    let read = Invoice::from_bson_with(stored(), |_raw, found| {
        handed = found.to_vec();
        customer_schema::Verdict::Reject
    });
    assert_eq!(
        invalid_values(&handed),
        [(
            "customer.openInvoices".to_owned(),
            invoice_schema::Expected::U32,
            Bson::String("3".to_owned())
        )]
    );
    assert_eq!(read.map_err(|refused| refused.issues.len()), Err(1));
}

#[test]
fn an_issue_inside_a_type_of_another_module_is_reported_at_its_full_path_from_json() {
    let mut handed: Vec<customer_schema::Issue<Value>> = Vec::new();
    let read = Invoice::from_value_with(
        json!({ "customer": { "name": "Ada", "openInvoices": "3" }, "number": "INV-1" }),
        |_raw, found| {
            handed = found.to_vec();
            customer_schema::Verdict::Reject
        },
    );
    assert_eq!(
        invalid_values(&handed),
        [(
            "customer.openInvoices".to_owned(),
            invoice_schema::Expected::U32,
            json!("3")
        )]
    );
    assert_eq!(read.map_err(|refused| refused.issues.len()), Err(1));
}

#[test]
fn one_resolver_serves_the_types_of_both_modules() {
    let resolvers: &[invoice_schema::Resolver<'_, Document, Bson>] = &[&count_as_text];
    assert_eq!(
        Invoice::from_bson_piped(stored(), resolvers).ok(),
        Some(settled())
    );
    assert_eq!(
        Customer::from_bson_piped(doc! { "name": "Ada", "openInvoices": "3" }, resolvers).ok(),
        Some(settled().customer)
    );
}

#[test]
fn a_filter_over_the_held_type_nests_under_the_holders_path() {
    let filter: Document = Invoice::MONGO_FIELDS
        .customer()
        .nest(Customer::MONGO_FIELDS.name().eq("Ada".to_owned()).unwrap())
        .into();
    assert_eq!(filter, doc! { "customer.name": { "$eq": "Ada" } });
}
