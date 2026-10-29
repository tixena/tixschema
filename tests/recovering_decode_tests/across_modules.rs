//! A type of one module holding a type of another, both read through the one support library
//! the crate writes: an issue inside the held type is reported at its full path, and the
//! callback's and the resolvers' types of the two modules are one.

mod billing;
mod customers;

use serde_json::{Value, json};

use self::billing::{Invoice, invoice_schema};
use self::customers::{Customer, customer_schema};

/// An invoice whose customer holds its count of open invoices as text.
fn stored() -> Value {
    json!({ "customer": { "name": "Ada", "openInvoices": "3" }, "number": "INV-1" })
}

/// Settles a count of open invoices an older writer stored as text, wherever it sits: one
/// resolver, written against one module's `Issue`, for the reads of both modules' types.
fn count_as_text(
    raw: &mut Value,
    issue: &customer_schema::Issue<Value>,
) -> customer_schema::Resolution {
    use customer_schema::Stored as _;
    let customer_schema::Issue::Invalid {
        path,
        expected: customer_schema::Expected::U32,
        found: Value::String(text),
        reason: _reason,
    } = issue
    else {
        return customer_schema::Resolution::NotTouched;
    };
    if text
        .parse::<u32>()
        .is_ok_and(|count| raw.set_at(path, json!(count)))
    {
        customer_schema::Resolution::Settled
    } else {
        customer_schema::Resolution::Rejected
    }
}

/// Each issue among `handed`, which are all `Invalid`, as its path, its expected type and the
/// value it holds.
fn invalid_values(
    handed: &[customer_schema::Issue<Value>],
) -> Vec<(String, customer_schema::Expected, Value)> {
    let invalid: Vec<(String, customer_schema::Expected, Value)> = handed
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
fn an_issue_inside_a_type_of_another_module_is_reported_at_its_full_path() {
    // The issues of `Invoice` are held as `Customer`'s module names them: one type.
    let mut handed: Vec<customer_schema::Issue<Value>> = Vec::new();
    let read = Invoice::from_value_with(stored(), |_raw, found| {
        handed = found.to_vec();
        customer_schema::Verdict::Reject
    });
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
    let settled: &[invoice_schema::Resolver<'_, Value, Value>] = &[&count_as_text];
    assert_eq!(
        Invoice::from_value_piped(stored(), settled).ok(),
        Some(Invoice {
            customer: Customer {
                name: "Ada".to_owned(),
                open_invoices: 3,
            },
            number: "INV-1".to_owned(),
        })
    );
    assert_eq!(
        Customer::from_value_piped(json!({ "name": "Ada", "openInvoices": "x" }), settled)
            .map_err(|refused| refused.to_string()),
        Err(
            r#"openInvoices: invalid: expected U32, found String("x"): invalid type: string "x", expected u32"#
                .to_owned()
        )
    );
}
