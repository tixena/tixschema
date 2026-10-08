//! `from_value_piped`, and the pipe it runs: resolvers answer one issue at a time, in order, each
//! handed only what the ones before it left `NotTouched`, and a refused read holds only the issues
//! none settled.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use invoice_schema::{Expected, Issue, Path, Resolution, Segment};

use super::lines;

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
fn bare_number(reason: &str) -> Issue<Value> {
    Issue::Invalid {
        path: Path(vec![Segment::Key("number".to_owned())]),
        expected: Expected::String,
        found: Value::String("42".to_owned()),
        reason: reason.to_owned(),
    }
}

/// A customer an older writer stored as JSON text: parsed into an object.
fn customer_as_text(raw: &mut Value, issue: &Issue<Value>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::Model("Customer"),
        found: Value::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let Ok(parsed @ Value::Object(_)) = serde_json::from_str::<Value>(text) else {
        return Resolution::Rejected;
    };
    if path.set_in_value(raw, parsed) {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

/// A whole number an older writer stored as text; a negative count is refused.
fn whole_number_as_text(raw: &mut Value, issue: &Issue<Value>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::U32,
        found: Value::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let settled = text
        .trim()
        .parse::<i64>()
        .is_ok_and(|number| number >= 0_i64 && path.set_in_value(raw, Value::from(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

/// A number an older writer stored bare: written as the type writes it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn number_without_prefix(raw: &mut Value, issue: &Issue<Value>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::String,
        found: Value::String(bare),
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
        .is_ok_and(|number| path.set_in_value(raw, Value::from(format!("INV-{number:04}"))));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

/// A resolver that notes the key of every issue it is handed in `seen`, settles the one at
/// `settled` and rejects the one at `rejected`.
fn noting<'seen>(
    seen: &'seen Mutex<Vec<String>>,
    settled: Option<&'seen str>,
    rejected: Option<&'seen str>,
) -> impl Fn(&mut Value, &Issue<Value>) -> Resolution + Sync + 'seen {
    move |_raw, issue| {
        let key = if let Issue::Unknown {
            path,
            found: _found,
        } = issue
        {
            path.to_string()
        } else {
            String::new()
        };
        let answer = if settled == Some(key.as_str()) {
            Resolution::Settled
        } else if rejected == Some(key.as_str()) {
            Resolution::Rejected
        } else {
            Resolution::NotTouched
        };
        seen.lock().unwrap().push(key);
        answer
    }
}

/// A key no type declares, as a read lists it.
fn unknown_at(key: &str) -> Issue<Value> {
    Issue::Unknown {
        path: Path(vec![Segment::Key(key.to_owned())]),
        found: Value::Bool(true),
    }
}

#[test]
fn two_resolvers_settle_a_value() {
    let stored = json!({
        "customer": { "name": "Acme", "openInvoices": "3" },
        "number": "INV-0042",
    });
    let read = Invoice::from_value_piped(stored, &[&customer_as_text, &whole_number_as_text]);
    assert_eq!(
        read,
        Ok(Invoice {
            customer: Customer {
                name: "Acme".to_owned(),
                open_invoices: 3_u32,
            },
            number: "INV-0042".to_owned(),
        })
    );
}

#[test]
fn an_issue_a_resolver_rejects_refuses_the_read_and_is_the_one_it_holds() {
    let stored = json!({
        "customer": { "name": "Acme", "openInvoices": "-3" },
        "number": "INV-0042",
    });
    let refused =
        Invoice::from_value_piped(stored, &[&customer_as_text, &whole_number_as_text]).unwrap_err();
    assert_eq!(
        lines(&refused),
        [
            "customer.openInvoices: invalid: expected U32, found String(\"-3\"): invalid type: \
             string \"-3\", expected u32"
        ]
    );
}

/// serde reads a name that breaks its bound, and the read hands the resolvers the bound's issue.
/// A build with no schema surface publishes no validator, and lists nothing.
#[test]
fn a_broken_bound_is_handed_to_the_resolvers() {
    let stored = json!({
        "customer": { "name": "", "openInvoices": 3_u32 },
        "number": "INV-0042",
    });
    let handed = Mutex::new(Vec::new());
    let noted = |_raw: &mut Value, issue: &Issue<Value>| {
        handed.lock().unwrap().push(issue.clone());
        Resolution::NotTouched
    };
    let read = Invoice::from_value_piped(stored, &[&noted]);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        let nameless = Issue::Invalid {
            path: Path(vec![
                Segment::Key("customer".to_owned()),
                Segment::Key("name".to_owned()),
            ]),
            expected: Expected::String,
            found: Value::String(String::new()),
            reason: "too short: minimum length is 1, got 0".to_owned(),
        };
        let refused = read.unwrap_err();
        assert_eq!(*handed.lock().unwrap(), refused.issues);
        assert_eq!(refused.issues, [nameless]);
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(read.unwrap().customer.name, "");
        assert!(handed.lock().unwrap().is_empty());
    }
}

/// A number that breaks two bounds is two issues at one path, each holding the value as it was
/// stored. The resolver is handed both and answers each, its repair already in the raw value when
/// it is handed the second.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_resolver_answers_both_issues_of_a_value_that_breaks_two_bounds() {
    let stored = json!({
        "customer": { "name": "Acme", "openInvoices": 3_u32 },
        "number": "42",
    });
    let handed = Mutex::new(Vec::new());
    let noted = |raw: &mut Value, issue: &Issue<Value>| {
        handed
            .lock()
            .unwrap()
            .push((raw["number"].clone(), issue.clone()));
        number_without_prefix(raw, issue)
    };
    let read = Invoice::from_value_piped(stored, &[&noted]);
    assert_eq!(read.unwrap().number, "INV-0042");
    assert_eq!(
        *handed.lock().unwrap(),
        [
            (
                json!("42"),
                bare_number("too short: minimum length is 8, got 2")
            ),
            (
                json!("INV-0042"),
                bare_number("does not match pattern '^INV-[0-9]+$'")
            ),
        ]
    );
}

/// The pipe counts issues, never paths. A resolver that repairs only a number still held bare
/// leaves the second issue `NotTouched`, and that issue refuses the read of the repaired value.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_issue_left_not_touched_at_a_repaired_value_refuses_the_read() {
    let stored = json!({
        "customer": { "name": "Acme", "openInvoices": 3_u32 },
        "number": "42",
    });
    let while_bare = |raw: &mut Value, issue: &Issue<Value>| {
        if raw["number"] == "42" {
            number_without_prefix(raw, issue)
        } else {
            Resolution::NotTouched
        }
    };
    let refused = Invoice::from_value_piped(stored, &[&while_bare]).unwrap_err();
    assert_eq!(
        refused.issues,
        [bare_number("does not match pattern '^INV-[0-9]+$'")]
    );
}

/// Each resolver is handed only the issues every one before it left `NotTouched`, once each. What
/// none settled comes back in the order it was found, whichever resolver answered it first.
#[test]
fn the_pipe_hands_a_resolver_only_what_the_earlier_ones_left_not_touched() {
    let issues = [
        unknown_at("a"),
        unknown_at("b"),
        unknown_at("c"),
        unknown_at("d"),
    ];
    let (first_seen, second_seen, third_seen) = (
        Mutex::new(Vec::new()),
        Mutex::new(Vec::new()),
        Mutex::new(Vec::new()),
    );
    let first = noting(&first_seen, None, Some("c"));
    let second = noting(&second_seen, Some("a"), None);
    let third = noting(&third_seen, None, None);
    let mut raw = json!({});
    let left = invoice_schema::unsettled(&mut raw, &issues, &[&first, &second, &third]);
    assert_eq!(*first_seen.lock().unwrap(), ["a", "b", "c", "d"]);
    assert_eq!(*second_seen.lock().unwrap(), ["a", "b", "d"]);
    assert_eq!(*third_seen.lock().unwrap(), ["b", "d"]);
    assert_eq!(left, [unknown_at("b"), unknown_at("c"), unknown_at("d")]);
}

#[test]
fn the_pipe_answers_every_issue_where_no_resolver_settles_one() {
    let issues = [unknown_at("a"), unknown_at("b")];
    let mut raw = json!({});
    assert_eq!(invoice_schema::unsettled(&mut raw, &issues, &[]), issues);
    let seen = Mutex::new(Vec::new());
    let touching_nothing = noting(&seen, None, None);
    assert_eq!(
        invoice_schema::unsettled(&mut raw, &issues, &[&touching_nothing]),
        issues
    );
    assert_eq!(*seen.lock().unwrap(), ["a", "b"]);
}

/// A resolver repairs the value the pipe is handed, and the issue it settled is not among the
/// ones the pipe answers.
#[test]
fn the_pipe_leaves_out_a_settled_issue_and_keeps_its_repair() {
    let mut raw = json!({
        "customer": "{\"name\":\"Acme\",\"openInvoices\":3}",
        "legacy": true,
        "number": "INV-0042",
    });
    let found =
        Invoice::from_value_with(raw.clone(), |_raw, _found| invoice_schema::Verdict::Reject)
            .unwrap_err()
            .issues;
    assert_eq!(found.len(), 2);
    let left = invoice_schema::unsettled(
        &mut raw,
        &found,
        &[&customer_as_text, &whole_number_as_text],
    );
    assert_eq!(left, [unknown_at("legacy")]);
    assert_eq!(
        raw,
        json!({
            "customer": { "name": "Acme", "openInvoices": 3_u32 },
            "legacy": true,
            "number": "INV-0042",
        })
    );
}
