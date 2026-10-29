```rust
# extern crate bson2 as bson;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use invoice_schema::{Expected, Issue, Resolution, Resolver, Stored as _};

tixschema::decode_with_foundation!();

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub number: String,
    pub line_count: u32,
}

/// Takes out a key the type no longer declares.
fn drop_unknown_key(raw: &mut Value, issue: &Issue<Value>) -> Resolution {
    match issue {
        Issue::Unknown { path, .. } if raw.remove_at(path) => Resolution::Settled,
        _ => Resolution::NotTouched,
    }
}

/// Reads a count an older writer stored as text, and refuses text that is no count.
fn count_from_text(raw: &mut Value, issue: &Issue<Value>) -> Resolution {
    let Issue::Invalid { path, expected: Expected::U32, found: Value::String(text), .. } = issue
    else {
        return Resolution::NotTouched;
    };
    match text.parse::<u32>() {
        Ok(count) if raw.set_at(path, Value::from(count)) => Resolution::Settled,
        _ => Resolution::Rejected,
    }
}

/// The repairs every stored invoice is read through, in order.
const STORED: &[Resolver<'static, Value, Value>] = &[&drop_unknown_key, &count_from_text];

fn main() {
    let read = Invoice::from_value_piped(
        json!({ "number": "INV-7", "lineCount": "3", "currency": "EUR" }),
        STORED,
    );
    assert_eq!(read, Ok(Invoice { number: "INV-7".to_owned(), line_count: 3 }));

    // An issue no resolver settles fails the read, and is listed at its path.
    let refused = Invoice::from_value_piped(json!({ "number": 42, "lineCount": 1 }), STORED)
        .unwrap_err();
    assert_eq!(refused.issues.len(), 1);
    assert!(matches!(
        &refused.issues[0],
        Issue::Invalid { path, .. } if path.to_string() == "number"
    ));
}
```
