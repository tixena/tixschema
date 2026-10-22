//! A model type named `Value`, declared below an alias that names it: the check written beside the
//! alias finds the name is no `serde_json::Value`, and the alias holds the type declared here.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

/// Keys and their values.
#[model_schema()]
pub type Entries = BTreeMap<String, Value>;

/// What a key maps to.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Value {
    Entries(Entries),
    Text(String),
}

#[test]
fn an_alias_above_holds_the_type_declared_here() {
    let read: Entries =
        serde_json::from_value(json!({ "deck": "oak", "rail": { "post": "ash" } })).unwrap();
    assert_eq!(
        read,
        BTreeMap::from([
            ("deck".to_owned(), Value::Text("oak".to_owned())),
            (
                "rail".to_owned(),
                Value::Entries(BTreeMap::from([(
                    "post".to_owned(),
                    Value::Text("ash".to_owned())
                )]))
            ),
        ]),
        "each value is read as the type declared here"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_names_the_type_declared_here() {
    let ts = entries_schema::Schema::ts_definition();
    assert!(ts.contains("Partial<Record<string, Value>>"), "got: {ts}");
}
