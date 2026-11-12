//! What the schema surfaces say of a strict type and of a `nonstrict` one.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
pub struct Closed {
    pub name: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
pub struct HoldsAnOpenStruct {
    #[serde(flatten)]
    pub open: Open,
    pub title: String,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Open {
    pub name: String,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "side")]
pub enum OpenSide {
    Load { weight: u32 },
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "load")]
pub enum OpenAdjacent {
    Load { weight: u32 },
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
pub enum OpenExternal {
    Square { side: u32 },
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum OpenShape {
    Square { side: u32 },
}

fn properties_of(schema: &Value) -> Vec<&Value> {
    let mut objects = Vec::new();
    let mut pending = vec![schema];
    while let Some(held) = pending.pop() {
        match held {
            Value::Object(entries) => {
                if let Some(open) = entries.get("additionalProperties") {
                    objects.push(open);
                }
                pending.extend(entries.values());
            }
            Value::Array(items) => pending.extend(items),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    objects
}

#[test]
fn a_nonstrict_type_writes_an_open_object_in_json_schema_and_a_strict_one_a_closed_object() {
    assert_eq!(Closed::json_schema()["additionalProperties"], false);
    assert_eq!(Open::json_schema()["additionalProperties"], true);
    assert_eq!(
        properties_of(&OpenSide::json_schema()),
        vec![&Value::Bool(true)]
    );
    assert_eq!(
        properties_of(&OpenAdjacent::json_schema()),
        vec![&Value::Bool(true), &Value::Bool(true)]
    );
    assert_eq!(
        properties_of(&OpenShape::json_schema()),
        vec![&Value::Bool(true)]
    );
}

#[test]
fn the_wrapper_of_an_externally_tagged_variant_stays_closed_around_an_open_content() {
    let schema = OpenExternal::json_schema();
    let mut closed = properties_of(&schema);
    closed.sort_by_key(|held| held.as_bool());
    assert_eq!(closed, vec![&Value::Bool(false), &Value::Bool(true)]);
}

#[cfg(feature = "zod")]
#[test]
fn a_nonstrict_type_writes_z_object_and_a_strict_one_z_strict_object() {
    assert!(Closed::zod_schema().contains("z.strictObject({"));
    assert!(!Closed::zod_schema().contains("z.object("));
    for open in [
        Open::zod_schema(),
        OpenSide::zod_schema(),
        OpenAdjacent::zod_schema(),
        OpenShape::zod_schema(),
    ] {
        assert!(open.contains("z.object("), "{open}");
        assert!(!open.contains("strictObject"), "{open}");
    }
    let external = OpenExternal::zod_schema();
    assert!(external.contains("z.strictObject({\n  \"Square\": z.object({"));
}

#[cfg(feature = "typescript")]
#[test]
fn the_typescript_type_of_a_nonstrict_type_is_the_type_of_a_strict_one() {
    let closed = Closed::ts_definition().replace("Closed", "T");
    let open = Open::ts_definition().replace("Open", "T");
    assert_eq!(closed, open);
}

#[test]
fn a_nonstrict_struct_flattened_into_a_strict_holder_leaves_the_holder_closed() {
    let schema = HoldsAnOpenStruct::json_schema();
    assert_eq!(schema["additionalProperties"], false);
    let mut properties: Vec<&str> = schema["properties"]
        .as_object()
        .map(|members| members.keys().map(String::as_str).collect())
        .unwrap_or_default();
    properties.sort_unstable();
    assert_eq!(properties, ["name", "title"]);
}
