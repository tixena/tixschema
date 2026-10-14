```rust
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::{model_schema, model_schema_prop};

tixschema::decode_with_foundation!();

#[model_schema(enable = ["decode_with"], check = ["withinParcel"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    pub fragile_item_codes: Vec<String>,
}

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

// A field's check takes the field's own value, and names no path.
fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

// A type's check takes the whole value, and leads each message with the path of what it refuses.
fn within_parcel(value: &Parcel) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() { Ok(()) } else { Err(outside) }
}

fn main() {
    let parcel = |label: &str, codes: &[&str], fragile: &[&str]| Parcel {
        label: label.to_string(),
        item_code_list: codes.iter().map(|code| code.to_string()).collect(),
        fragile_item_codes: fragile.iter().map(|code| code.to_string()).collect(),
    };
    let lines = |answered: Result<(), Vec<String>>| answered.err().unwrap_or_default();

    assert!(parcel("A", &["x"], &["x"]).validate().is_ok());
    assert_eq!(
        lines(parcel("", &["x"], &["x"]).validate()),
        ["'label': too short: minimum length is 1, got 0"]
    );
    assert_eq!(
        lines(parcel("A", &["x", "x"], &["x"]).validate()),
        ["'itemCodeList': duplicate item code x"]
    );
    assert_eq!(
        lines(parcel("A", &["x"], &["x", "y"]).validate()),
        ["'fragileItemCodes.1': y is outside of itemCodeList"]
    );
    // Every bound and every check runs, and none stops another.
    assert_eq!(
        lines(parcel("", &["x", "x"], &["y"]).validate()),
        [
            "'label': too short: minimum length is 1, got 0",
            "'itemCodeList': duplicate item code x",
            "'fragileItemCodes.0': y is outside of itemCodeList",
        ]
    );
    // A holder runs the `validate()` of what it holds, under the key it holds it at.
    let manifest = Manifest { parcel: parcel("A", &["x"], &["x", "y"]) };
    assert_eq!(
        lines(manifest.validate()),
        ["'parcel.fragileItemCodes.1': y is outside of itemCodeList"]
    );

    // The recovering read lists a broken check as an issue, at the place its message names.
    let read = Parcel::from_value_with(
        json!({ "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] }),
        |_, issues| {
            assert_eq!(
                format!("{:?}", issues[0]),
                r#"Invalid { path: Path([Key("fragileItemCodes"), Index(1)]), expected: Unknown, found: String("y"), reason: "y is outside of itemCodeList" }"#
            );
            parcel_schema::Verdict::Reject
        },
    );
    assert!(read.is_err());
}
```
