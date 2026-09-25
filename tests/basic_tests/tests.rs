#[cfg(all(
    test,
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
use serde::{Deserialize, Serialize};
#[cfg(all(test, feature = "jsonschema"))]
use serde_json::Value;
#[cfg(all(
    test,
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
use tixschema::model_schema;

#[cfg(all(
    test,
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[model_schema()]
struct BasicUser {
    age: u32,
    height: f32,
    id: String,
    is_active: bool,
    name: String,
}

#[cfg(all(
    test,
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
#[model_schema()]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
struct EmptyStruct;

/// A unit struct declaring only `Serialize` — proves the macro emits only the impl whose derive
/// was present, not the pair.
#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
#[model_schema()]
#[derive(Serialize, Debug, Clone, PartialEq)]
struct WriteOnlyMarker;

/// The read-only twin of [`WriteOnlyMarker`].
#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
#[model_schema()]
#[derive(Deserialize, Debug, Clone, PartialEq)]
struct ReadOnlyMarker;

/// A field typed as a unit struct — proves the field position round-trips `{}` too, not just the
/// bare type.
#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct HoldsMarker {
    label: String,
    marker: EmptyStruct,
}

#[cfg(all(
    test,
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct UserWithOptionals {
    #[serde(skip_serializing_if = "Option::is_none")]
    age: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,
}

#[cfg(all(
    test,
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
#[test]
fn test_basic_structs_constructible() {
    let basic = BasicUser {
        age: 0,
        height: 0.0,
        id: String::new(),
        is_active: false,
        name: String::new(),
    };
    assert_eq!(basic.id, String::new());
    let empty = EmptyStruct;
    assert_eq!(format!("{empty:?}"), "EmptyStruct");
    let optionals = UserWithOptionals {
        age: None,
        email: None,
        id: String::new(),
        name: String::new(),
        nickname: None,
    };
    assert_eq!(optionals.id, String::new());
}

#[test]
#[cfg(feature = "jsonschema")]
fn test_basic_struct_json_schema() {
    let schema = BasicUser::json_schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);

    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("id"));
    assert!(properties.contains_key("name"));
    assert!(properties.contains_key("age"));
    assert!(properties.contains_key("height"));
    assert!(properties.contains_key("is_active"));

    assert_eq!(properties["id"]["type"], "string");
    assert_eq!(properties["name"]["type"], "string");
    assert_eq!(properties["age"]["type"], "integer");
    assert_eq!(properties["height"]["type"], "number");
    assert_eq!(properties["is_active"]["type"], "boolean");

    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 5);
    assert!(required.contains(&Value::String("id".to_owned())));
    assert!(required.contains(&Value::String("name".to_owned())));
    assert!(required.contains(&Value::String("age".to_owned())));
    assert!(required.contains(&Value::String("height".to_owned())));
    assert!(required.contains(&Value::String("is_active".to_owned())));
}

#[test]
#[cfg(feature = "typescript")]
fn test_basic_struct_ts_definition() {
    let ts_definition = BasicUser::ts_definition();

    assert!(ts_definition.contains("export type BasicUser = {"));
    assert!(ts_definition.contains("id: string;"));
    assert!(ts_definition.contains("name: string;"));
    assert!(ts_definition.contains("age: number;"));
    assert!(ts_definition.contains("height: number;"));
    assert!(ts_definition.contains("is_active: boolean;"));

    assert!(!ts_definition.contains("export const BasicUser$Schema"));
    assert!(!ts_definition.contains("z.strictObject"));
    assert!(!ts_definition.contains("z.string()"));
    assert!(!ts_definition.contains("z.number()"));
    assert!(!ts_definition.contains("z.boolean()"));
}

#[test]
#[cfg(feature = "zod")]
fn test_basic_struct_zod_schema() {
    let zod_schema = BasicUser::zod_schema();

    assert!(zod_schema.contains("export const BasicUser$Schema"));
    assert!(zod_schema.contains("z.strictObject({"));
    assert!(zod_schema.contains("id: z.string()"));
    assert!(zod_schema.contains("name: z.string()"));
    assert!(zod_schema.contains("age: z.number().int()"));
    assert!(zod_schema.contains("height: z.number()"));
    assert!(zod_schema.contains("is_active: z.boolean()"));

    assert!(!zod_schema.contains("export type BasicUser"));
    assert!(!zod_schema.contains("id: string;"));
    assert!(!zod_schema.contains("age: number;"));
}

#[test]
#[cfg(feature = "jsonschema")]
fn test_optional_fields_json_schema() {
    let schema = UserWithOptionals::json_schema();

    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("id"));
    assert!(properties.contains_key("name"));
    assert!(properties.contains_key("email"));
    assert!(properties.contains_key("age"));
    assert!(properties.contains_key("nickname"));

    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 2);
    assert!(required.contains(&Value::String("id".to_owned())));
    assert!(required.contains(&Value::String("name".to_owned())));
    assert!(!required.contains(&Value::String("email".to_owned())));
    assert!(!required.contains(&Value::String("age".to_owned())));
    assert!(!required.contains(&Value::String("nickname".to_owned())));
}

/// The member an `Option` field written with a `skip_serializing_if` renders as. The attribute
/// decides the wire, not the spelling, and none of these fields carries `ts_optional`.
#[cfg(feature = "typescript")]
fn omitted_member(name: &str, ts_type: &str) -> String {
    format!("{name}: {ts_type} | undefined;")
}

#[test]
#[cfg(feature = "typescript")]
fn test_optional_fields_ts_definition() {
    let ts_definition = UserWithOptionals::ts_definition();

    assert!(ts_definition.contains("id: string;"));
    assert!(ts_definition.contains("name: string;"));
    assert!(ts_definition.contains(&omitted_member("email", "string")));
    assert!(ts_definition.contains(&omitted_member("age", "number")));
    assert!(ts_definition.contains(&omitted_member("nickname", "string")));

    assert!(!ts_definition.contains("z.union([z.string(), z.undefined()])"));
    assert!(!ts_definition.contains("z.union([z.number().int(), z.undefined()])"));
}

#[test]
#[cfg(feature = "zod")]
fn test_optional_fields_zod_schema() {
    let zod_schema = UserWithOptionals::zod_schema();

    assert!(zod_schema.contains(
        "email: z.union([z.null().transform(() => undefined), z.string(), z.undefined()])"
    ));
    assert!(zod_schema.contains(
        "age: z.union([z.null().transform(() => undefined), z.number().int(), z.undefined()])"
    ));
    assert!(zod_schema.contains(
        "nickname: z.union([z.null().transform(() => undefined), z.string(), z.undefined()])"
    ));

    assert!(!zod_schema.contains("email: string | undefined;"));
    assert!(!zod_schema.contains("age: number | undefined;"));
}

#[test]
#[cfg(feature = "jsonschema")]
fn test_empty_struct_json_schema() {
    let schema = EmptyStruct::json_schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);

    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.is_empty());

    let required = schema["required"].as_array().unwrap();
    assert_eq!(required, &Vec::<Value>::new());
}

#[test]
#[cfg(feature = "typescript")]
fn test_empty_struct_ts_definition() {
    let ts_definition = EmptyStruct::ts_definition();

    assert!(ts_definition.contains("export type EmptyStruct = Record<string, never>;"));
}

#[test]
#[cfg(all(feature = "serde", feature = "typescript"))]
fn test_field_typed_as_a_unit_struct_ts_definition() {
    let ts_definition = HoldsMarker::ts_definition();

    assert!(
        ts_definition.contains("marker: EmptyStruct;"),
        "got: {ts_definition}"
    );
}

#[test]
#[cfg(all(feature = "serde", feature = "zod"))]
fn test_field_typed_as_a_unit_struct_zod_schema() {
    let zod_schema = HoldsMarker::zod_schema();

    assert!(
        zod_schema.contains("marker: EmptyStruct$Schema"),
        "got: {zod_schema}"
    );
}

#[test]
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
fn test_field_typed_as_a_unit_struct_round_trips_as_an_empty_object() {
    let value = HoldsMarker {
        label: "widget".to_owned(),
        marker: EmptyStruct,
    };
    let written = serde_json::to_string(&value).unwrap();
    assert_eq!(written, r#"{"label":"widget","marker":{}}"#);
    let read: HoldsMarker = serde_json::from_str(&written).unwrap();
    assert_eq!(read, value);
}

#[test]
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
fn test_empty_struct_writes_and_reads_an_empty_object() {
    assert_eq!(serde_json::to_string(&EmptyStruct).unwrap(), "{}");
    serde_json::from_str::<EmptyStruct>("{}").unwrap();
    serde_json::from_str::<EmptyStruct>(r#"{"extra":1}"#).unwrap();
    serde_json::from_str::<EmptyStruct>("null").unwrap_err();
}

#[test]
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
fn test_serialize_only_unit_struct_writes_an_empty_object() {
    assert_eq!(serde_json::to_string(&WriteOnlyMarker).unwrap(), "{}");
}

#[test]
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "jsonschema", feature = "zod")
))]
fn test_deserialize_only_unit_struct_reads_an_empty_object() {
    serde_json::from_str::<ReadOnlyMarker>("{}").unwrap();
    serde_json::from_str::<ReadOnlyMarker>("null").unwrap_err();
}
