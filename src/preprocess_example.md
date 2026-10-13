```rust
# extern crate bson2 as bson;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::{model_schema, model_schema_prop};

#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
    pub item_code_list: Vec<String>,
    pub fragile_item_codes: Vec<String>,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

// A field's function takes the payload of the field, and answers the payload the field is read from.
fn trimmed(payload: Value) -> Value {
    match payload {
        Value::String(text) => Value::String(text.trim().to_owned()),
        other => other,
    }
}

// A type's function takes the whole payload, ahead of every field's.
fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

fn main() {
    // Row 1: the label is read without the spaces at its ends.
    let parcel: Parcel = serde_json::from_str(
        r#"{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    )
    .unwrap();
    assert_eq!(parcel.label, "A");

    // Row 2: the bound judges the label the function answered.
    let refused = serde_json::from_str::<Parcel>(
        r#"{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    )
    .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "'label': too short: minimum length is 1, got 0"
    );

    // Row 3: the type's function gives the payload the key it leaves out.
    let parcel: Parcel = serde_json::from_str(r#"{"label":"A","itemCodeList":["x"]}"#).unwrap();
    assert!(parcel.fragile_item_codes.is_empty());

    // Row 4: the same payload as a BSON document.
    let stored = bson::doc! { "label": "A", "itemCodeList": ["x"] };
    let parcel: Parcel = bson::from_document(stored).unwrap();
    assert!(parcel.fragile_item_codes.is_empty());

    // Row 5: a type that holds a parcel reads it through the parcel's functions.
    let manifest: Manifest = serde_json::from_str(
        r#"{"parcel":{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}}"#,
    )
    .unwrap();
    assert_eq!(manifest.parcel.label, "A");

    // A key the payload does not hold is read as serde reads it: no function is called for it.
    let missing = serde_json::from_str::<Parcel>(r#"{"itemCodeList":["x"]}"#).unwrap_err();
    assert_eq!(missing.to_string(), "missing field `label`");

    // A value built in code is never read, so nothing corrects it.
    let built = Parcel {
        label: "  A  ".to_owned(),
        item_code_list: Vec::new(),
        fragile_item_codes: Vec::new(),
    };
    assert_eq!(built.label, "  A  ");
}
```
