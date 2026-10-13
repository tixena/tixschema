//! The models the `preprocess` tests read: the parcel of the documents, the types that hold it,
//! and one type per shape the key is written on. Each function is one a `preprocess` names.

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What every side prints for the sample of the documents, in a build that holds `label` to its
/// bound: the Rust read and the Zod schema alike.
pub const SAMPLE: &str = "\
row 1: read {\"fragileItemCodes\":[\"x\"],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
row 2: refused
    'label': too short: minimum length is 1, got 0
row 3: read {\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
row 5: read {\"parcel\":{\"fragileItemCodes\":[\"x\"],\"itemCodeList\":[\"x\"],\"label\":\"A\"}}
";

/// What every side prints for the positions another type reads a preprocessed type in, and for
/// each shape the key is written on: the Rust read and the Zod schema alike. A unit struct is
/// read from `{}` under the `serde` feature alone, so the table is that build's.
#[cfg(feature = "serde")]
pub const POSITIONS: &str = "\
a field: read {\"parcel\":{\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}}
a list, a map and an Option: read {\"by_dock\":{\"north\":{\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}},\"list\":[{\"fragileItemCodes\":[],\"itemCodeList\":[],\"label\":\"B\"}],\"spare\":{\"fragileItemCodes\":[],\"itemCodeList\":[],\"label\":\"C\"}}
a flattened type: read {\"dock\":\"north\",\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
a variant under a tag: read {\"kind\":\"Held\",\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
a variant beside its tag: read {\"kind\":\"Held\",\"detail\":{\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}}
a variant under its name: read {\"Held\":{\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}}
an untagged variant: read {\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
a type that reaches itself: read {\"children\":[{\"children\":[],\"id\":\"b\"}],\"id\":\"a\"}
a flattened choice: read {\"dock\":\"north\",\"plate\":\"AB 1\"}
a unit struct: read {}
a tuple struct: read [[\"x\"],2]
a brand: read \"A\"
a plain enum: read \"Express\"
an internally tagged enum: read {\"kind\":\"Air\",\"flight\":\"TX 1\"}
an externally tagged enum: read {\"Rail\":\"TX 1\"}
an adjacently tagged enum: read {\"kind\":\"Left\",\"detail\":\"door\"}
an untagged enum: read 7
a type with a parameter, at a text: read {\"codes\":[\"x\"],\"label\":\"A\"}
a type with a parameter, at a number: read {\"codes\":[7],\"label\":\"A\"}
two functions, in the order written: read {\"fragileItemCodes\":[],\"label\":\"A\"}
a check, after the function: refused
    'grams': 2000 is over the limit of 1000
a null the payload holds: read {\"label\":\"unnamed\"}
";

/// The payload of each row of the sample a JSON text holds, under the name of its row. Row 4 is
/// row 3 as a BSON document.
pub const SAMPLE_ROWS: [(&str, &str); 4] = [
    (
        "row 1",
        r#"{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    ),
    (
        "row 2",
        r#"{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    ),
    ("row 3", r#"{"label":"A","itemCodeList":["x"]}"#),
    (
        "row 5",
        r#"{"parcel":{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}}"#,
    ),
];

/// The parcel of the documents, declared above every type that holds it. This crate's lints order
/// a struct's members by name, so `label` is declared last.
#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
}

/// A type whose function writes, under `keys`, the keys of the object it was handed.
#[model_schema(preprocess = ["withItsKeys"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tagged {
    pub keys: Vec<String>,
    pub tag: String,
}

/// A choice of two objects, whose function trims the plate of the one that holds one.
#[model_schema(preprocess = ["trimmedPlate"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Carrier {
    Rail { wagon: String },
    Road { plate: String },
}

/// A unit struct.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClosedDock;

/// An adjacently tagged enum whose variant holds a parcel.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Consigned {
    Held(Parcel),
    Lost,
}

/// A type with a parameter.
#[model_schema(default_types(CodeType = String), preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    pub label: String,
}

/// A parcel flattened into the object of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Crated {
    pub dock: String,
    #[serde(flatten)]
    pub parcel: Parcel,
}

/// An internally tagged enum whose variant holds a parcel.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Delivery {
    Held(Parcel),
    Lost,
}

/// An untagged enum that holds a parcel.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Either {
    Code(String),
    Parcel(Parcel),
}

/// An enum of unit variants.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Grade {
    Express,
    Standard,
}

/// An adjacently tagged enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Handover {
    Left(String),
    Signed { by: String },
}

/// A brand.
#[model_schema(preprocess = ["unwrapped", "trimmed"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Label(pub String);

/// The type that holds the parcel of the documents under a key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

/// A type that reaches itself, and whose function gives a leaf its empty list.
#[model_schema(preprocess = ["withChildren"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Node {
    pub children: Vec<Self>,
    pub id: String,
}

/// An untagged enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Reference {
    Named { name: String },
    Numbered(u32),
}

/// A type flattened into its holder: what it reads under `keys` is what its function was handed.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pinned {
    pub dock: String,
    #[serde(flatten)]
    pub tagged: Tagged,
}

/// A choice flattened into the object of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Routed {
    pub dock: String,
    #[serde(flatten)]
    pub via: Carrier,
}

/// A parcel in a list, as a map's value and in an `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shelf {
    pub by_dock: BTreeMap<String, Parcel>,
    pub list: Vec<Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Parcel>,
}

/// An internally tagged enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Shipment {
    Air { flight: String },
    Ground,
}

/// An externally tagged enum that holds a parcel.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Slot {
    Empty,
    Held(Parcel),
}

/// A tuple struct.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stack(pub Vec<String>, pub u32);

/// An externally tagged enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Transfer {
    Held,
    Rail(String),
    Road { stops: Vec<String> },
}

/// A label whose function names it where the payload holds none.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Unnamed {
    #[model_schema_prop(preprocess = ["orUnnamed"])]
    pub label: String,
}

/// A type that names a function and a check: the check judges what the function answered.
#[model_schema(check = ["withinLimit"], preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Weighed {
    pub grams: u32,
}

/// A parcel whose two functions run in the order written: out of its envelope, then completed.
#[model_schema(preprocess = ["unwrapped", "withoutFragile"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wrapped {
    pub fragile_item_codes: Vec<String>,
    pub label: String,
}

/// One payload a type reads: the name of the case, the schema that reads it under node, the
/// payload, and what a serde read of it as a `T` prints.
#[cfg(feature = "serde")]
type Position = (&'static str, &'static str, &'static str, String);

/// The position `name`: `payload` read as a `T`, which the schema `schema` reads under node.
#[cfg(feature = "serde")]
fn position<T>(name: &'static str, schema: &'static str, payload: &'static str) -> Position
where
    T: DeserializeOwned + Serialize,
{
    (name, schema, payload, row_read::<T>(name, payload))
}

/// Every position another type reads a preprocessed type in, then every shape the key is written
/// on. A shape's payload is in an envelope, which the type's function takes it out of.
#[cfg(feature = "serde")]
pub fn positions() -> Vec<Position> {
    let parcel = r#"{"label":"  A  ","itemCodeList":["x"]}"#;
    let held: [&'static str; 7] = [
        r#"{"parcel":{"label":"  A  ","itemCodeList":["x"]}}"#,
        r#"{"by_dock":{"north":{"label":" A ","itemCodeList":["x"]}},"list":[{"label":" B ","itemCodeList":[]}],"spare":{"label":" C ","itemCodeList":[]}}"#,
        r#"{"dock":"north","label":"  A  ","itemCodeList":["x"]}"#,
        r#"{"kind":"Held","label":"  A  ","itemCodeList":["x"]}"#,
        r#"{"kind":"Held","detail":{"label":"  A  ","itemCodeList":["x"]}}"#,
        r#"{"Held":{"label":"  A  ","itemCodeList":["x"]}}"#,
        parcel,
    ];
    vec![
        position::<Manifest>("a field", "Manifest", held[0]),
        position::<Shelf>("a list, a map and an Option", "Shelf", held[1]),
        position::<Crated>("a flattened type", "Crated", held[2]),
        position::<Delivery>("a variant under a tag", "Delivery", held[3]),
        position::<Consigned>("a variant beside its tag", "Consigned", held[4]),
        position::<Slot>("a variant under its name", "Slot", held[5]),
        position::<Either>("an untagged variant", "Either", held[6]),
        position::<Node>(
            "a type that reaches itself",
            "Node",
            r#"{"id":"a","children":[{"id":"b"}]}"#,
        ),
        position::<Routed>(
            "a flattened choice",
            "Routed",
            r#"{"dock":"north","plate":"  AB 1  "}"#,
        ),
        position::<ClosedDock>("a unit struct", "ClosedDock", r#"{"payload":{}}"#),
        position::<Stack>("a tuple struct", "Stack", r#"{"payload":[["x"],2]}"#),
        position::<Label>("a brand", "Label", r#"{"payload":"  A  "}"#),
        position::<Grade>("a plain enum", "Grade", r#"{"payload":"Express"}"#),
        position::<Shipment>(
            "an internally tagged enum",
            "Shipment",
            r#"{"payload":{"kind":"Air","flight":"TX 1"}}"#,
        ),
        position::<Transfer>(
            "an externally tagged enum",
            "Transfer",
            r#"{"payload":{"Rail":"TX 1"}}"#,
        ),
        position::<Handover>(
            "an adjacently tagged enum",
            "Handover",
            r#"{"payload":{"kind":"Left","detail":"door"}}"#,
        ),
        position::<Reference>("an untagged enum", "Reference", r#"{"payload":7}"#),
        position::<Crate<String>>(
            "a type with a parameter, at a text",
            "Crate<String>",
            r#"{"payload":{"codes":["x"],"label":"A"}}"#,
        ),
        position::<Crate<u32>>(
            "a type with a parameter, at a number",
            "Crate<u32>",
            r#"{"payload":{"codes":[7],"label":"A"}}"#,
        ),
        position::<Wrapped>(
            "two functions, in the order written",
            "Wrapped",
            r#"{"payload":{"label":"A"}}"#,
        ),
        position::<Weighed>(
            "a check, after the function",
            "Weighed",
            r#"{"payload":{"grams":2000}}"#,
        ),
        position::<Unnamed>("a null the payload holds", "Unnamed", r#"{"label":null}"#),
    ]
}

/// What a serde read of `payload` as a `T` prints under `row`: the value written back, or each
/// message of the refusal on a line of its own.
fn row_read<T>(row: &str, payload: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    match serde_json::from_str::<T>(payload) {
        Ok(read) => format!("{row}: read {}\n", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let mut printed = format!("{row}: refused\n");
            for message in refused.to_string().split("; ") {
                let _ = writeln!(printed, "    {message}");
            }
            printed
        }
    }
}

/// What a serde read of each row of the sample prints: the manifest's row as a manifest, and
/// every other as a parcel.
pub fn sample_report() -> String {
    SAMPLE_ROWS
        .iter()
        .map(|&(row, payload)| {
            if row == "row 5" {
                row_read::<Manifest>(row, payload)
            } else {
                row_read::<Parcel>(row, payload)
            }
        })
        .collect()
}

/// `unnamed` for a payload that holds nothing, and any other payload as it is.
pub fn or_unnamed(payload: Value) -> Value {
    if payload.is_null() {
        Value::String("unnamed".to_owned())
    } else {
        payload
    }
}

/// An object whose `plate`, where it holds one, has no space at its ends.
pub fn trimmed_plate(mut payload: Value) -> Value {
    if let Some(Value::String(plate)) = payload.get_mut("plate") {
        *plate = plate.trim().to_owned();
    }
    payload
}

/// The payload of a text without the spaces at its ends, and any other payload as it is.
pub fn trimmed(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

/// What an envelope holds under `payload`, and a payload that is no envelope as it is.
pub fn unwrapped(payload: Value) -> Value {
    if let Value::Object(mut entries) = payload {
        entries
            .remove("payload")
            .unwrap_or_else(|| Value::Object(entries))
    } else {
        payload
    }
}

/// An object that lists, under `keys`, the keys it was handed with, in order.
pub fn with_its_keys(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        let mut keys: Vec<String> = object.keys().cloned().collect();
        keys.sort_unstable();
        object.insert(
            "keys".to_owned(),
            Value::Array(keys.into_iter().map(Value::String).collect()),
        );
    }
    payload
}

/// Refuses a weight over a kilogram.
pub fn within_limit(value: &Weighed) -> Result<(), Vec<String>> {
    if value.grams > 1000 {
        Err(vec![format!(
            "'grams': {} is over the limit of 1000",
            value.grams
        )])
    } else {
        Ok(())
    }
}

/// An object with an empty `children` where it holds none.
pub fn with_children(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("children")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

/// An object with an empty `fragileItemCodes` where it holds none.
pub fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}
