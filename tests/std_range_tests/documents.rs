//! What a JSON Schema document admits, for the keywords the documents of the ranges write, the
//! keys a map names (`propertyNames`), a `pattern`, the `date` format and the names a plain enum
//! lists (`enum`).

use serde_json::Value;

/// Why a document refuses a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// An object holds a key the document does not declare, and admits no other.
    UndeclaredKey,
    /// Any other keyword refuses the value.
    Written,
}

/// Whether `document` admits `value`.
pub fn admits(document: &Value, value: &Value) -> bool {
    refusal(document, value).is_none()
}

/// Why `document` refuses `value`, and `None` where it admits it. A key no member declares is
/// told apart where it is the only thing refused.
pub fn refusal(document: &Value, value: &Value) -> Option<Refusal> {
    if let Value::Bool(admitted) = *document {
        return (!admitted).then_some(Refusal::Written);
    }
    let admitting = |keyword: &str| {
        document
            .get(keyword)
            .and_then(Value::as_array)
            .map(|branches| {
                branches
                    .iter()
                    .filter(|branch| admits(branch, value))
                    .count()
            })
    };
    let branches = |keyword: &str| {
        document
            .get(keyword)
            .and_then(Value::as_array)
            .map(Vec::len)
    };
    if admitting("anyOf") == Some(0)
        || admitting("oneOf").is_some_and(|count| count != 1)
        || admitting("allOf") != branches("allOf")
    {
        return Some(Refusal::Written);
    }
    if document
        .get("const")
        .is_some_and(|written| !same(written, value))
    {
        return Some(Refusal::Written);
    }
    if document
        .get("enum")
        .and_then(Value::as_array)
        .is_some_and(|listed| !listed.iter().any(|named| same(named, value)))
    {
        return Some(Refusal::Written);
    }
    if let Some(kind) = document.get("type").and_then(Value::as_str)
        && !of_type(kind, value)
    {
        return Some(Refusal::Written);
    }
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as u64;
        let least = document.get("minLength").and_then(Value::as_u64);
        let most = document.get("maxLength").and_then(Value::as_u64);
        if least.is_some_and(|bound| length < bound) || most.is_some_and(|bound| length > bound) {
            return Some(Refusal::Written);
        }
        if document.get("format").and_then(Value::as_str) == Some("date") && !full_date(text) {
            return Some(Refusal::Written);
        }
        if let Some(pattern) = document.get("pattern").and_then(Value::as_str)
            && !regex::Regex::new(pattern).unwrap().is_match(text)
        {
            return Some(Refusal::Written);
        }
    }
    match value {
        Value::Object(object) => object_refusal(document, object),
        Value::Array(items) => (!admits_items(document, items)).then_some(Refusal::Written),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

/// Why `document` refuses the members of `object`, and `None` where it admits them.
fn object_refusal(document: &Value, object: &serde_json::Map<String, Value>) -> Option<Refusal> {
    let properties = document.get("properties").and_then(Value::as_object);
    let required = document
        .get("required")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    if !required
        .iter()
        .filter_map(Value::as_str)
        .all(|key| object.contains_key(key))
    {
        return Some(Refusal::Written);
    }
    if let Some(names) = document.get("propertyNames")
        && !object
            .keys()
            .all(|key| admits(names, &Value::String(key.clone())))
    {
        return Some(Refusal::Written);
    }
    let mut found = None;
    for (key, held) in object {
        let refused = properties.and_then(|named| named.get(key)).map_or_else(
            || match document.get("additionalProperties") {
                Some(Value::Bool(false)) => Some(Refusal::UndeclaredKey),
                Some(Value::Bool(true)) | None => None,
                Some(rest) => refusal(rest, held),
            },
            |described| refusal(described, held),
        );
        match refused {
            Some(Refusal::Written) => return refused,
            Some(Refusal::UndeclaredKey) => found = refused,
            None => {}
        }
    }
    found
}

/// Whether `document` admits the items of `items`.
fn admits_items(document: &Value, items: &[Value]) -> bool {
    let count = items.len() as u64;
    let least = document.get("minItems").and_then(Value::as_u64);
    let most = document.get("maxItems").and_then(Value::as_u64);
    if least.is_some_and(|bound| count < bound) || most.is_some_and(|bound| count > bound) {
        return false;
    }
    let prefix = document
        .get("prefixItems")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    items.iter().enumerate().all(|(at, item)| {
        if let Some(described) = prefix.get(at) {
            return admits(described, item);
        }
        match document.get("items") {
            Some(Value::Bool(open)) => *open,
            Some(each) => admits(each, item),
            None => true,
        }
    })
}

/// Whether `text` is an RFC 3339 full-date, `YYYY-MM-DD`, which the `date` format names.
fn full_date(text: &str) -> bool {
    let shaped = text.len() == 10
        && text.char_indices().all(|(at, written)| {
            if at == 4 || at == 7 {
                written == '-'
            } else {
                written.is_ascii_digit()
            }
        });
    shaped && chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
}

/// Whether `written` and `value` are one JSON value: two numbers are one where they are equal,
/// whatever form each is written in, as `214` and `214.0` are.
fn same(written: &Value, value: &Value) -> bool {
    match (written.as_f64(), value.as_f64()) {
        (Some(left), Some(right)) => left.total_cmp(&right).is_eq(),
        _ => written == value,
    }
}

/// Whether `value` is of the JSON type `kind` names.
fn of_type(kind: &str, value: &Value) -> bool {
    match kind {
        "array" => value.is_array(),
        "boolean" => value.is_boolean(),
        "integer" => value.is_i64() || value.is_u64(),
        "null" => value.is_null(),
        "number" => value.is_number(),
        "object" => value.is_object(),
        "string" => value.is_string(),
        _ => false,
    }
}
