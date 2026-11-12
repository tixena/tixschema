//! Tests of the JSON Schema each field type describes as.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tixschema::model_schema;

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Address {
    city: String,
    street: String,
}

/// `u64` is part of this sweep, so the item is gated out entirely when the Swift or Kotlin target
/// refuses that width, rather than swapped to a signed width.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AllNumericTypes {
    f32: f32,
    f64: f64,
    i16: i16,
    i32: i32,
    i64: i64,
    i8: i8,
    isize: isize,
    u16: u16,
    u32: u32,
    u64: u64,
    u8: u8,
    usize: usize,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ArrayFields {
    numbers: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optional_array: Option<Vec<String>>,
    tags: Vec<String>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct BasicStruct {
    active: bool,
    age: u32,
    name: String,
    score: f64,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ComplexCollections {
    map_of_arrays: HashMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optional_map: Option<HashMap<String, i32>>,
}

#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CustomFieldRename {
    #[serde(rename = "customName")]
    field_name: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct EmptyStruct;

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct MapFields {
    counts: HashMap<String, i32>,
    metadata: HashMap<String, String>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct OptionalFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    optional_bool: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optional_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optional_string: Option<String>,
    required: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Person {
    address: Address,
    name: String,
    previous_addresses: Vec<Address>,
}

#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct RenamedFields {
    user_email: String,
    user_name: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum Status {
    Active,
    Inactive,
    Pending,
}

#[test]
fn test_basic_struct_json_schema() {
    let schema = BasicStruct::json_schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);

    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("name"));
    assert!(properties.contains_key("age"));
    assert!(properties.contains_key("score"));
    assert!(properties.contains_key("active"));

    assert_eq!(properties["name"]["type"], "string");
    assert_eq!(properties["age"]["type"], "integer");
    assert_eq!(properties["score"]["type"], "number");
    assert_eq!(properties["active"]["type"], "boolean");

    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 4);
    assert!(required.contains(&Value::String("name".to_owned())));
    assert!(required.contains(&Value::String("age".to_owned())));
    assert!(required.contains(&Value::String("score".to_owned())));
    assert!(required.contains(&Value::String("active".to_owned())));
}

#[test]
fn test_optional_fields_not_in_required() {
    let schema = OptionalFields::json_schema();

    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("required"));
    assert!(properties.contains_key("optional_string"));
    assert!(properties.contains_key("optional_number"));
    assert!(properties.contains_key("optional_bool"));

    assert_eq!(
        properties["optional_string"],
        serde_json::json!({ "anyOf": [{ "type": "string" }, { "type": "null" }] })
    );
    assert_eq!(
        properties["optional_number"],
        serde_json::json!({ "anyOf": [{ "type": "integer" }, { "type": "null" }] })
    );
    assert_eq!(
        properties["optional_bool"],
        serde_json::json!({ "anyOf": [{ "type": "boolean" }, { "type": "null" }] })
    );

    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 1);
    assert!(required.contains(&Value::String("required".to_owned())));
    assert!(!required.contains(&Value::String("optional_string".to_owned())));
    assert!(!required.contains(&Value::String("optional_number".to_owned())));
    assert!(!required.contains(&Value::String("optional_bool".to_owned())));
}

#[test]
fn test_vec_fields_generate_array_schemas() {
    let schema = ArrayFields::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    assert_eq!(properties["tags"]["type"], "array");
    assert_eq!(properties["tags"]["items"]["type"], "string");

    assert_eq!(properties["numbers"]["type"], "array");
    assert_eq!(properties["numbers"]["items"]["type"], "integer");

    assert_eq!(
        properties["optional_array"],
        serde_json::json!({
            "anyOf": [
                { "type": "array", "items": { "type": "string" } },
                { "type": "null" }
            ]
        })
    );

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&Value::String("tags".to_owned())));
    assert!(required.contains(&Value::String("numbers".to_owned())));
    assert!(!required.contains(&Value::String("optional_array".to_owned())));
}

#[test]
fn test_hashmap_generates_object_with_additional_properties() {
    let schema = MapFields::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    assert_eq!(properties["metadata"]["type"], "object");
    assert_eq!(
        properties["metadata"]["additionalProperties"]["type"],
        "string"
    );

    assert_eq!(properties["counts"]["type"], "object");
    assert_eq!(
        properties["counts"]["additionalProperties"]["type"],
        "integer"
    );
}

#[test]
fn test_nested_structs_present_in_schema() {
    let schema = Person::json_schema();
    let address_schema = Address::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    assert!(properties.contains_key("address"));

    assert!(properties.contains_key("previous_addresses"));
    assert_eq!(properties["previous_addresses"]["type"], "array");

    assert_eq!(address_schema["type"], "object");
    let address_properties = address_schema["properties"].as_object().unwrap();
    assert!(address_properties.contains_key("street"));
    assert!(address_properties.contains_key("city"));
}

#[test]
fn test_plain_enum_generates_string_enum() {
    let schema = Status::json_schema();

    assert_eq!(schema["type"], "string");

    let enum_values = schema["enum"].as_array().unwrap();
    assert_eq!(enum_values.len(), 3);
    assert!(enum_values.contains(&Value::String("Active".to_owned())));
    assert!(enum_values.contains(&Value::String("Inactive".to_owned())));
    assert!(enum_values.contains(&Value::String("Pending".to_owned())));
}

#[test]
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
fn test_integer_types_use_integer_schema() {
    let schema = AllNumericTypes::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    assert_eq!(properties["u8"]["type"], "integer");
    assert_eq!(properties["u16"]["type"], "integer");
    assert_eq!(properties["u32"]["type"], "integer");
    assert_eq!(properties["u64"]["type"], "integer");
    assert_eq!(properties["i8"]["type"], "integer");
    assert_eq!(properties["i16"]["type"], "integer");
    assert_eq!(properties["i32"]["type"], "integer");
    assert_eq!(properties["i64"]["type"], "integer");
    assert_eq!(properties["usize"]["type"], "integer");
    assert_eq!(properties["isize"]["type"], "integer");
}

#[test]
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
fn test_float_types_use_number_schema() {
    let schema = AllNumericTypes::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    assert_eq!(properties["f32"]["type"], "number");
    assert_eq!(properties["f64"]["type"], "number");
}

#[test]
#[cfg(feature = "serde")]
fn test_serde_rename_all_affects_property_names() {
    let schema = RenamedFields::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    assert!(properties.contains_key("userName"));
    assert!(properties.contains_key("userEmail"));

    assert!(!properties.contains_key("user_name"));
    assert!(!properties.contains_key("user_email"));

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&Value::String("userName".to_owned())));
    assert!(required.contains(&Value::String("userEmail".to_owned())));
}

#[test]
#[cfg(feature = "serde")]
fn test_serde_field_rename() {
    let schema = CustomFieldRename::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    assert!(properties.contains_key("customName"));
    assert!(!properties.contains_key("field_name"));

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&Value::String("customName".to_owned())));
}

#[test]
fn test_complex_nested_collections() {
    let schema = ComplexCollections::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    assert_eq!(properties["map_of_arrays"]["type"], "object");
    let map_of_arrays_additional = properties["map_of_arrays"]["additionalProperties"]
        .as_object()
        .unwrap();
    assert_eq!(map_of_arrays_additional["type"], "array");
    assert_eq!(map_of_arrays_additional["items"]["type"], "string");

    assert_eq!(properties["optional_map"]["anyOf"][0]["type"], "object");
    assert_eq!(
        properties["optional_map"]["anyOf"][0]["additionalProperties"]["type"],
        "integer"
    );
    assert_eq!(properties["optional_map"]["anyOf"][1]["type"], "null");

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&Value::String("map_of_arrays".to_owned())));
    assert!(!required.contains(&Value::String("optional_map".to_owned())));
}

#[test]
#[cfg(feature = "mongodb")]
fn test_objectid_generates_proper_schema() {
    use mongodb::bson::oid::ObjectId;

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct Document {
        #[serde(skip_serializing_if = "Option::is_none")]
        author_id: Option<ObjectId>,
        id: ObjectId,
        tag_ids: Vec<ObjectId>,
    }

    let schema = Document::json_schema();

    let properties = schema["properties"].as_object().unwrap();

    let id_prop = &properties["id"];
    assert_eq!(id_prop["type"], "object");
    assert_eq!(id_prop["properties"]["$oid"]["type"], "string");
    assert_eq!(id_prop["required"][0], "$oid");
    assert_eq!(id_prop["additionalProperties"], false);

    let author_prop = &properties["author_id"];
    assert_eq!(author_prop["anyOf"][0]["type"], "object");
    assert_eq!(
        author_prop["anyOf"][0]["properties"]["$oid"]["type"],
        "string"
    );
    assert_eq!(author_prop["anyOf"][1]["type"], "null");

    let tags_prop = &properties["tag_ids"];
    assert_eq!(tags_prop["type"], "array");
    let items = &tags_prop["items"];
    assert_eq!(items["type"], "object");
    assert_eq!(items["properties"]["$oid"]["type"], "string");
    assert_eq!(items["required"][0], "$oid");
    assert_eq!(items["additionalProperties"], false);
}

#[test]
fn test_empty_struct_generates_valid_schema() {
    let schema = EmptyStruct::json_schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);

    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(properties.len(), 0);

    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 0);
}

#[test]
fn test_schema_has_required_structure() {
    #[model_schema(enable = ["jsonschema"])]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct Test {
        field: String,
    }

    let schema = Test::json_schema();

    assert!(schema.is_object());

    assert!(schema.get("type").is_some());
    assert!(schema.get("properties").is_some());
    assert!(schema.get("required").is_some());
    assert!(schema.get("additionalProperties").is_some());
}

/// A type that lists `"jsonschema"` answers its document, with its field's rule in it.
#[test]
fn a_type_that_lists_the_entry_answers_its_document() {
    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Signup {
        #[model_schema_prop(minLength = 3)]
        pub username: String,
    }

    assert_eq!(
        Signup::json_schema(),
        serde_json::json!({
            "additionalProperties": false,
            "properties": { "username": { "minLength": 3_u32, "type": "string" } },
            "required": ["username"],
            "type": "object"
        })
    );
}

/// A field's rustdoc is its property's `description`; a field with none has no key.
#[test]
fn a_fields_rustdoc_is_its_propertys_description() {
    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct CommentArgs {
        pub bare: String,
        /// Path to the document
        pub file: String,
        /// Names from the registry who must see this in their pending queue.
        #[serde(default)]
        pub to: Vec<String>,
    }

    let schema = CommentArgs::json_schema();
    assert_eq!(
        schema["properties"]["to"],
        serde_json::json!({
            "default": [],
            "description": "Names from the registry who must see this in their pending queue.",
            "items": { "type": "string" },
            "type": "array"
        })
    );
    assert_eq!(
        schema["properties"]["file"]["description"],
        "Path to the document"
    );
    assert!(schema["properties"]["bare"].get("description").is_none());
}

/// The lines are joined with newlines and the `rust example` block is left out.
#[test]
fn a_many_line_rustdoc_joins_its_lines_and_leaves_out_its_examples() {
    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Documented {
        /// First line.
        /// Second line.
        ///
        /// ```rust example
        /// "ignored"
        /// ```
        /// Last line.
        pub value: String,
    }

    assert_eq!(
        Documented::json_schema()["properties"]["value"]["description"],
        "First line.\nSecond line.\n\nLast line."
    );
}

/// A member typed with another model keeps its `$ref` and gains `description` beside it.
#[test]
fn a_member_that_is_a_reference_keeps_its_ref_and_gains_a_description() {
    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Holder {
        /// Where it ships.
        pub address: Address,
    }

    let property = &Holder::json_schema()["properties"]["address"];
    assert_eq!(property["description"], "Where it ships.");
    assert_eq!(property["type"], "object");
}

/// A named variant's members carry their rustdoc under every tagging.
#[test]
fn the_members_of_a_named_variant_carry_their_rustdoc_under_every_tagging() {
    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum External {
        Named {
            /// External member.
            id: String,
        },
    }

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    pub enum Internal {
        Named {
            /// Internal member.
            id: String,
        },
    }

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind", content = "data")]
    pub enum Adjacent {
        Named {
            /// Adjacent member.
            id: String,
        },
    }

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum Untagged {
        Named {
            /// Untagged member.
            id: String,
        },
    }

    for (schema, text) in [
        (External::json_schema(), "External member."),
        (Internal::json_schema(), "Internal member."),
        (Adjacent::json_schema(), "Adjacent member."),
        (Untagged::json_schema(), "Untagged member."),
    ] {
        assert!(
            schema
                .to_string()
                .contains(&format!("\"description\":\"{text}\"")),
            "{text} missing from {schema}"
        );
    }
}
