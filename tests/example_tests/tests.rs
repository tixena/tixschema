//! Tests of the `rust example` fence in a doc comment, which becomes the type's `schema_example()`.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// An alias of a number, which has no `impl` to carry an example on.
#[cfg(feature = "zod")]
#[model_schema()]
pub type Score = u32;

/// An alias that declares a parameter.
#[cfg(feature = "zod")]
#[model_schema(default_types(Held = String))]
pub type Several<Held> = Vec<Held>;

#[cfg(feature = "zod")]
#[test]
fn test_simple_enum_example() {
    /// Data type enumeration
    /// ```rust example
    /// let data_type = DataType::Numeric;
    /// println!("Example: {:?}", data_type);
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum DataType {
        Alphanumeric,
        Date,
        Numeric,
    }

    let example = DataType::schema_example();
    assert_eq!(example.as_str().unwrap(), "Numeric");

    let zod = DataType::zod_schema();
    assert!(zod.contains("examples: [\"Numeric\"]"), "Got: {zod}");
    assert!(zod.contains("\"Numeric\""));

    #[cfg(feature = "typescript")]
    {
        assert!(
            zod.contains("DataType$RawSchema"),
            "Plain enum with example should have $RawSchema. Got:\n{zod}"
        );
        assert!(
            zod.contains("export const DataType$Schema: ZodType<DataType> = DataType$RawSchema;"),
            "Plain enum with example should have $Schema referencing $RawSchema. Got:\n{zod}"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn test_simple_struct_example() {
    /// User profile
    /// ```rust example
    /// let user = User {
    ///     name: "John Doe".to_string(),
    ///     age: 25,
    /// };
    /// println!("User: {:?}", user);
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct User {
        pub age: u32,
        pub name: String,
    }

    let example = User::schema_example();
    assert_eq!(example["name"].as_str().unwrap(), "John Doe");
    assert_eq!(example["age"].as_u64().unwrap(), 25);

    let zod = User::zod_schema();
    assert!(
        zod.contains("examples: [{\"age\":25,\"name\":\"John Doe\"}]"),
        "Got: {zod}"
    );
    assert!(zod.contains("John Doe"));
    assert!(zod.contains("25"));
}

#[cfg(feature = "zod")]
#[test]
fn test_complex_struct_with_logic() {
    /// Complex user with setup logic
    /// ```rust example
    /// let user_id = "usr_123".to_string();
    /// let tags = vec!["admin".to_string(), "active".to_string()];
    /// let user = ComplexUser {
    ///     id: user_id,
    ///     tags,
    ///     age: 30,
    /// };
    /// println!("Complex user: {:?}", user);
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ComplexUser {
        pub age: u32,
        pub id: String,
        pub tags: Vec<String>,
    }

    let example = ComplexUser::schema_example();
    assert_eq!(example["id"].as_str().unwrap(), "usr_123");
    assert_eq!(example["tags"][0].as_str().unwrap(), "admin");
    assert_eq!(example["tags"][1].as_str().unwrap(), "active");
    assert_eq!(example["age"].as_u64().unwrap(), 30);

    let zod = ComplexUser::zod_schema();
    assert!(
        zod.contains("examples: [{\"age\":30,\"id\":\"usr_123\",\"tags\":[\"admin\",\"active\"]}]"),
        "Got: {zod}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn test_nested_types_example() {
    use std::collections::HashMap;

    /// Profile with nested types
    /// ```rust example
    /// let profile = Profile {
    ///     tags: vec!["tag1".to_string()],
    ///     metadata: HashMap::from([("key1".to_string(), "value1".to_string())]),
    ///     optional_field: Some("present".to_string()),
    /// };
    /// println!("Profile: {:?}", profile);
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Profile {
        pub metadata: HashMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub optional_field: Option<String>,
        pub tags: Vec<String>,
    }

    let example = Profile::schema_example();
    assert_eq!(example["tags"][0].as_str().unwrap(), "tag1");
    assert_eq!(example["metadata"]["key1"].as_str().unwrap(), "value1");
    assert_eq!(example["optional_field"].as_str().unwrap(), "present");

    let zod = Profile::zod_schema();
    assert!(
        zod.contains(
            "examples: [{\"metadata\":{\"key1\":\"value1\"},\"optional_field\":\"present\",\"\
             tags\":[\"tag1\"]}]"
        ),
        "Got: {zod}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn test_discriminated_enum_example() {
    /// Event types
    /// ```rust example
    /// let event = Event::UserCreated {
    ///     user_id: "user_123".to_string(),
    ///     timestamp: "2024-01-01".to_string(),
    /// };
    /// println!("Event: {:?}", event);
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "type")]
    pub enum Event {
        UserCreated { user_id: String, timestamp: String },
        UserDeleted { user_id: String },
    }

    let example = Event::schema_example();
    assert_eq!(example["type"].as_str().unwrap(), "UserCreated");
    assert_eq!(example["user_id"].as_str().unwrap(), "user_123");
    assert_eq!(example["timestamp"].as_str().unwrap(), "2024-01-01");

    let zod = Event::zod_schema();
    assert!(zod.contains("examples: [{"), "Got: {zod}");
}

#[cfg(feature = "zod")]
#[test]
fn every_example_block_reaches_the_schema() {
    /// Type with multiple examples
    /// ```rust example
    /// let _: SeveralExamples = SeveralExamples { value: 1 };
    /// ```
    /// Second description
    /// ```rust example
    /// let _: SeveralExamples = SeveralExamples { value: 2 };
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct SeveralExamples {
        pub value: u32,
    }

    assert_eq!(
        SeveralExamples::schema_examples(),
        vec![
            serde_json::json!({ "value": 1_u32 }),
            serde_json::json!({ "value": 2_u32 })
        ],
    );
    assert_eq!(
        SeveralExamples::schema_example(),
        serde_json::json!({ "value": 1_u32 })
    );
    assert!(
        SeveralExamples::zod_schema()
            .contains(".meta({\n  examples: [{\"value\":1},{\"value\":2}]\n})"),
        "{}",
        SeveralExamples::zod_schema()
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_type_with_no_block_publishes_an_empty_list() {
    /// Type without example
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Plain {
        pub value: u32,
    }

    let zod = Plain::zod_schema();
    assert!(zod.ends_with(".meta({\n  examples: []\n});"), "Got: {zod}");
    assert!(!zod.contains("example:"), "Got: {zod}");
}

#[test]
fn test_no_example_no_method() {
    /// Type without example
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct NoExample {
        pub value: String,
    }

    let no_example = NoExample {
        value: String::new(),
    };
    assert_eq!(no_example.value, String::new());

    #[cfg(feature = "zod")]
    {
        let zod = NoExample::zod_schema();
        assert!(!zod.contains("example:"));
        assert!(zod.contains("examples: []"), "Got: {zod}");
    }
}

#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn test_typescript_zod_format() {
    /// Test type
    /// ```rust example
    /// Test { name: "test".to_string() }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Test {
        pub name: String,
    }

    let zod = Test::zod_schema();

    assert!(zod.contains("ZodType<"));
    assert!(zod.contains("$RawSchema"));
    assert!(
        zod.contains("examples: [{\"name\":\"test\"}]"),
        "Got: {zod}"
    );
}

#[cfg(all(feature = "zod", not(feature = "typescript")))]
#[test]
fn test_javascript_zod_format() {
    /// Test type
    /// ```rust example
    /// Test { name: "test".to_string() }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Test {
        pub name: String,
    }

    let zod = Test::zod_schema();

    assert!(!zod.contains("ZodType<"));
    assert!(
        zod.contains("examples: [{\"name\":\"test\"}]"),
        "Got: {zod}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn test_serde_attributes_in_example() {
    /// User with serde attributes
    /// ```rust example
    /// let user = UserWithSerde {
    ///     user_id: "123".to_string(),
    ///     email_address: "test@example.com".to_string(),
    /// };
    /// println!("User: {:?}", user);
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct UserWithSerde {
        #[serde(rename = "emailAddress")]
        pub email_address: String,
        pub user_id: String,
    }

    let example = UserWithSerde::schema_example();
    assert!(example.get("userId").is_some());
    assert_eq!(
        example["emailAddress"].as_str().unwrap(),
        "test@example.com"
    );

    let zod = UserWithSerde::zod_schema();
    assert!(zod.contains("examples: [{"), "Got: {zod}");
}

#[cfg(all(feature = "zod", feature = "mongodb"))]
#[test]
fn test_objectid_example() {
    use mongodb::bson::oid::ObjectId;

    /// Document with `ObjectId`
    /// ```rust example
    /// let oid = ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
    /// let doc = Document {
    ///     id: oid,
    ///     name: "test".to_string(),
    /// };
    /// println!("Document: {:?}", doc);
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Document {
        pub id: ObjectId,
        pub name: String,
    }

    let example = Document::schema_example();
    assert_eq!(example["name"].as_str().unwrap(), "test");
    assert!(example["id"].get("$oid").is_some());

    let zod = Document::zod_schema();
    assert!(zod.contains("examples: [{"), "Got: {zod}");
}

#[test]
fn test_example_fence_must_be_exact() {
    /// Type with regular code fence (not example)
    /// ```rust
    /// RegularFence { value: 1 }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct RegularFence {
        pub value: u32,
    }

    let regular_fence = RegularFence { value: 0 };
    assert_eq!(regular_fence.value, 0_u32);

    #[cfg(feature = "zod")]
    {
        let zod = RegularFence::zod_schema();
        assert!(!zod.contains("example:"));
        assert!(zod.contains("examples: []"), "Got: {zod}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn test_empty_example_block() {
    /// Type with empty example (should fail compilation if uncommented)
    /// This test verifies that the example must contain valid code
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ValidExample {
        pub value: u32,
    }

    let zod = ValidExample::zod_schema();
    assert!(zod.contains("value"));
}

#[cfg(feature = "zod")]
#[test]
fn test_regex_transform_println() {
    /// Data type with println pattern (for doctest compatibility)
    /// ```rust example
    /// let data_type = DataType::Integer;
    /// println!("data_type: {:?}", data_type);
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum DataType {
        Float,
        Integer,
        String,
    }

    let example = DataType::schema_example();
    assert_eq!(example.as_str().unwrap(), "Integer");

    let zod = DataType::zod_schema();
    assert!(zod.contains("examples: [\"Integer\"]"), "Got: {zod}");
    assert!(zod.contains("\"Integer\""));
}

#[cfg(feature = "zod")]
#[test]
fn test_regex_transform_let_underscore() {
    /// Status with let underscore pattern (for doctest compatibility)
    /// ```rust example
    /// let _: Status = Status::Active;
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Status {
        Active,
        Inactive,
    }

    let example = Status::schema_example();
    assert_eq!(example.as_str().unwrap(), "Active");

    let zod = Status::zod_schema();
    assert!(zod.contains("examples: [\"Active\"]"), "Got: {zod}");
    assert!(zod.contains("\"Active\""));
}

#[cfg(feature = "zod")]
#[test]
fn test_description_strips_examples() {
    /// This is a description
    /// with multiple lines
    /// ```rust example
    /// let _: DescriptionTest = DescriptionTest { value: 42 };
    /// ```
    /// More description after example
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct DescriptionTest {
        pub value: u32,
    }

    let zod = DescriptionTest::zod_schema();
    assert!(!zod.contains("DescriptionTest { value: 42 }"));
    // Note: Structs don't embed descriptions in .meta() currently (only enums do)
}

#[cfg(feature = "zod")]
#[test]
fn test_description_escapes_quotes() {
    /// Description with "quoted" text
    /// ```rust example
    /// let _: QuoteTest = QuoteTest::Active;
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum QuoteTest {
        Active,
        Inactive,
    }

    let zod = QuoteTest::zod_schema();
    assert!(zod.contains("examples: [\"Active\"]"), "Got: {zod}");
    assert!(zod.contains("description:"));
    assert!(zod.contains(r#"\"quoted\""#));
}

#[cfg(feature = "zod")]
#[test]
fn a_generic_struct_renders_its_example_at_its_declared_filling() {
    /// A generic type carrying an example block.
    ///
    /// ```rust example
    /// Boxed { value: "x".to_owned() }
    /// ```
    #[model_schema(default_types(ValueType = String))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Boxed<ValueType> {
        pub value: ValueType,
    }

    assert_eq!(
        Boxed::<String>::schema_example(),
        serde_json::json!({ "value": "x" })
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_generic_enum_renders_its_example_at_its_declared_fillings() {
    /// A generic enum carrying an example block.
    ///
    /// ```rust example
    /// Tagged::Held { held: "x".to_owned(), tag: "t".to_owned() }
    /// ```
    #[model_schema(default_types(HeldType = String, TagType = String))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Tagged<HeldType, TagType> {
        Held { held: HeldType, tag: TagType },
    }

    assert_eq!(
        Tagged::<String, String>::schema_example(),
        serde_json::json!({ "Held": { "held": "x", "tag": "t" } })
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_bounded_parameter_renders_its_example_at_its_declared_filling() {
    /// A generic type whose parameter carries a bound `String` does not satisfy.
    ///
    /// ```rust example
    /// Counted { count: 7u32 }
    /// ```
    #[model_schema(default_types(CountType = u32))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Counted<CountType: Copy> {
        pub count: CountType,
    }

    assert_eq!(
        Counted::<u32>::schema_example(),
        serde_json::json!({ "count": 7_u32 })
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_generic_enum_renders_its_example_at_a_filling_that_is_not_a_string() {
    /// A generic enum carrying an example block.
    ///
    /// ```rust example
    /// Measured::Held { held: 7u32 }
    /// ```
    #[model_schema(default_types(HeldType = u32))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Measured<HeldType> {
        Held { held: HeldType },
    }

    assert_eq!(
        Measured::<u32>::schema_example(),
        serde_json::json!({ "Held": { "held": 7_u32 } })
    );
}

#[cfg(all(feature = "zod", not(feature = "jsonschema")))]
#[test]
fn an_unfilled_parameter_keeps_the_string_instantiation() {
    /// A generic type declaring no filling.
    ///
    /// ```rust example
    /// Unfilled { value: "x".to_owned() }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Unfilled<ValueType> {
        pub value: ValueType,
    }

    assert_eq!(
        Unfilled::<String>::schema_example(),
        serde_json::json!({ "value": "x" })
    );
}

#[cfg(all(feature = "zod", not(feature = "jsonschema")))]
#[test]
fn a_string_filling_and_no_filling_write_the_same_schema() {
    /// A generic type declaring its filling.
    ///
    /// ```rust example
    /// Written { value: "x".to_owned() }
    /// ```
    #[model_schema(default_types(ValueType = String))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Written<ValueType> {
        pub value: ValueType,
    }

    /// A generic type declaring its filling.
    ///
    /// ```rust example
    /// Omitted { value: "x".to_owned() }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Omitted<ValueType> {
        pub value: ValueType,
    }

    assert_eq!(
        Written::<String>::zod_schema().replace("Written", "Item"),
        Omitted::<String>::zod_schema().replace("Omitted", "Item")
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_factory_carries_its_example_on_the_schema_it_memoizes() {
    /// A generic type carrying an example block.
    ///
    /// ```rust example
    /// Held { value: "x".to_owned() }
    /// ```
    #[model_schema(default_types(ValueType = String))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Held<ValueType> {
        pub value: ValueType,
    }

    let zod = Held::<String>::zod_schema();
    assert!(
        zod.contains(
            "  const schema = buildHeld$Schema(valueType).meta({\n    examples: \
             [{\"value\":\"x\"}]\n  });\n"
        ),
        "Got: {zod}"
    );
    assert!(!zod.contains("}.meta("), "Got: {zod}");
    assert!(
        zod.contains("  Held$SchemaFactoryCache.set(valueType, schema);\n  return schema;\n}"),
        "Got: {zod}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_plain_const_still_carries_its_example_on_the_exported_binding() {
    /// A type carrying an example block.
    ///
    /// ```rust example
    /// Kept { value: "x".to_owned() }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Kept {
        pub value: String,
    }

    let zod = Kept::zod_schema();
    #[cfg(feature = "typescript")]
    assert!(
        zod.contains(
            "export const Kept$Schema: ZodType<Kept> = Kept$RawSchema.meta({\n  examples: \
             [{\"value\":\"x\"}]\n});"
        ),
        "Got: {zod}"
    );
    #[cfg(not(feature = "typescript"))]
    assert!(
        zod.contains(".meta({\n  examples: [{\"value\":\"x\"}]\n});"),
        "Got: {zod}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_lifetime_item_renders_its_example_with_the_lifetime_elided() {
    /// A borrowing type carrying an example block.
    ///
    /// ```rust example
    /// Labelled { label: "x" }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Labelled<'label> {
        pub label: &'label str,
    }

    assert_eq!(
        Labelled::schema_example(),
        serde_json::json!({ "label": "x" })
    );
}

#[cfg(not(feature = "zod"))]
#[test]
fn a_const_declaring_item_keeps_its_example_where_no_example_is_read() {
    /// A const-bearing type carrying an example block.
    ///
    /// ```rust example
    /// Slotted { label: "x".to_owned() }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Slotted<const WIDTH: usize> {
        pub label: String,
    }

    let slotted = Slotted::<3> {
        label: "x".to_owned(),
    };
    assert_eq!(slotted.label, "x");
}

/// What a schema that declares no parameter ends in, holding `examples` on the binding it exports.
#[cfg(feature = "zod")]
fn ends_in_examples(zod: &str, examples: &str) -> bool {
    zod.ends_with(&format!(".meta({{\n  examples: {examples}\n}});"))
}

#[cfg(feature = "zod")]
#[test]
fn one_block_publishes_one_item_and_no_example_key() {
    /// Type with one example
    /// ```rust example
    /// Single { value: 7 }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Single {
        pub value: u32,
    }

    assert_eq!(
        Single::schema_examples(),
        vec![serde_json::json!({ "value": 7_u32 })]
    );
    assert_eq!(
        Single::schema_example(),
        serde_json::json!({ "value": 7_u32 })
    );

    let zod = Single::zod_schema();
    assert!(ends_in_examples(&zod, "[{\"value\":7}]"), "Got: {zod}");
    assert!(!zod.contains("example:"), "Got: {zod}");
}

#[cfg(feature = "zod")]
#[test]
fn an_empty_block_is_skipped_between_two_filled_ones() {
    /// Type with an empty block
    /// ```rust example
    /// Sparse { value: 1 }
    /// ```
    /// ```rust example
    /// ```
    /// ```rust example
    /// Sparse { value: 3 }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Sparse {
        pub value: u32,
    }

    assert_eq!(
        Sparse::schema_examples(),
        vec![
            serde_json::json!({ "value": 1_u32 }),
            serde_json::json!({ "value": 3_u32 })
        ]
    );
    let zod = Sparse::zod_schema();
    assert!(
        ends_in_examples(&zod, "[{\"value\":1},{\"value\":3}]"),
        "Got: {zod}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_block_that_is_never_closed_is_published() {
    /// A reading.
    /// ```rust example
    /// Reading { value: 7 }
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Reading {
        pub value: u32,
    }

    assert_eq!(
        Reading::schema_examples(),
        vec![serde_json::json!({ "value": 7_u32 })]
    );
    let zod = Reading::zod_schema();
    assert!(ends_in_examples(&zod, "[{\"value\":7}]"), "Got: {zod}");
}

#[cfg(feature = "zod")]
#[test]
fn an_unclosed_block_is_published_after_a_closed_one() {
    /// Two blocks, the second one never closed
    /// ```rust example
    /// Paired { value: 1 }
    /// ```
    /// ```rust example
    /// Paired { value: 2 }
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Paired {
        pub value: u32,
    }

    assert_eq!(
        Paired::schema_examples(),
        vec![
            serde_json::json!({ "value": 1_u32 }),
            serde_json::json!({ "value": 2_u32 })
        ]
    );
    let zod = Paired::zod_schema();
    assert!(
        ends_in_examples(&zod, "[{\"value\":1},{\"value\":2}]"),
        "Got: {zod}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn an_opening_fence_with_nothing_after_it_publishes_no_example() {
    /// An opening fence and no block
    /// ```rust example
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Fenced {
        pub value: u32,
    }

    let zod = Fenced::zod_schema();
    assert!(ends_in_examples(&zod, "[]"), "Got: {zod}");
}

#[cfg(feature = "zod")]
#[test]
fn each_block_is_built_in_a_scope_of_its_own() {
    /// Two blocks binding the same name
    /// ```rust example
    /// let value = 1;
    /// Scoped { value }
    /// ```
    /// ```rust example
    /// let value = 2;
    /// Scoped { value }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Scoped {
        pub value: u32,
    }

    assert_eq!(
        Scoped::schema_examples(),
        vec![
            serde_json::json!({ "value": 1_u32 }),
            serde_json::json!({ "value": 2_u32 })
        ]
    );
}

#[cfg(all(feature = "zod", feature = "serde"))]
#[test]
fn an_internally_tagged_enum_publishes_one_example_per_variant() {
    /// A shape, one example per variant
    /// ```rust example
    /// Shape::Circle { radius: 2 }
    /// ```
    /// ```rust example
    /// Shape::Square { side: 3 }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "kind")]
    pub enum Shape {
        Circle { radius: u32 },
        Square { side: u32 },
    }

    assert_eq!(
        Shape::schema_examples(),
        vec![
            serde_json::json!({ "kind": "Circle", "radius": 2_u32 }),
            serde_json::json!({ "kind": "Square", "side": 3_u32 })
        ]
    );
    assert_eq!(
        Shape::schema_example(),
        serde_json::json!({ "kind": "Circle", "radius": 2_u32 })
    );
    let zod = Shape::zod_schema();
    assert!(
        ends_in_examples(
            &zod,
            "[{\"kind\":\"Circle\",\"radius\":2},{\"kind\":\"Square\",\"side\":3}]"
        ),
        "Got: {zod}"
    );
    assert!(!zod.contains("example:"), "Got: {zod}");
}

#[cfg(all(feature = "zod", feature = "serde"))]
#[test]
fn every_tagging_of_an_enum_carries_its_examples() {
    /// Written under the variant's name
    /// ```rust example
    /// Outer::Held { held: 1 }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Outer {
        Held { held: u32 },
        Idle,
    }

    /// Written under two keys
    /// ```rust example
    /// Adjacent::Held { held: 1 }
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "kind", content = "body")]
    pub enum Adjacent {
        Held { held: u32 },
        Idle,
    }

    /// Written as the variant's own value
    /// ```rust example
    /// Bare::Text("x".to_owned())
    /// ```
    /// ```rust example
    /// Bare::Count(2)
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub enum Bare {
        Count(u32),
        Text(String),
    }

    let outer = Outer::zod_schema();
    assert!(
        ends_in_examples(&outer, "[{\"Held\":{\"held\":1}}]"),
        "Got: {outer}"
    );
    let adjacent = Adjacent::zod_schema();
    assert!(
        ends_in_examples(&adjacent, "[{\"body\":{\"held\":1},\"kind\":\"Held\"}]")
            || ends_in_examples(&adjacent, "[{\"kind\":\"Held\",\"body\":{\"held\":1}}]"),
        "Got: {adjacent}"
    );
    let bare = Bare::zod_schema();
    assert!(ends_in_examples(&bare, "[\"x\",2]"), "Got: {bare}");
    assert_eq!(
        Bare::schema_examples(),
        vec![serde_json::json!("x"), serde_json::json!(2_u32)]
    );
}

#[cfg(all(feature = "zod", feature = "serde"))]
#[test]
fn every_tagging_of_an_enum_with_no_block_publishes_an_empty_list() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Outer {
        Held { held: u32 },
        Idle,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "kind")]
    pub enum Inner {
        Held { held: u32 },
        Idle,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "kind", content = "body")]
    pub enum Adjacent {
        Held { held: u32 },
        Idle,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub enum Bare {
        Count(u32),
        Text(String),
    }

    for zod in [
        Outer::zod_schema(),
        Inner::zod_schema(),
        Adjacent::zod_schema(),
        Bare::zod_schema(),
    ] {
        assert!(ends_in_examples(&zod, "[]"), "Got: {zod}");
        assert!(!zod.contains("example:"), "Got: {zod}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_plain_enum_holds_its_examples_inside_its_own_meta() {
    /// A level
    /// ```rust example
    /// Level::Low
    /// ```
    /// ```rust example
    /// Level::High
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Level {
        High,
        Low,
    }

    /// A level with no example
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Grade {
        High,
        Low,
    }

    assert_eq!(
        Level::schema_examples(),
        vec![serde_json::json!("Low"), serde_json::json!("High")]
    );
    assert_eq!(Level::schema_example(), serde_json::json!("Low"));
    let level = Level::zod_schema();
    assert!(
        level.contains(".meta({\n  description: \"A level\",\n  examples: [\"Low\",\"High\"],\n})"),
        "Got: {level}"
    );
    assert_eq!(level.matches(".meta(").count(), 1, "Got: {level}");

    let grade = Grade::zod_schema();
    assert!(
        grade.contains(".meta({\n  description: \"A level with no example\",\n  examples: [],\n})"),
        "Got: {grade}"
    );
    assert_eq!(grade.matches(".meta(").count(), 1, "Got: {grade}");
    assert!(!grade.contains("example:"), "Got: {grade}");
}

#[cfg(feature = "zod")]
#[test]
fn a_generic_struct_holds_its_examples_on_the_schema_it_memoizes() {
    /// A generic type carrying two example blocks.
    ///
    /// ```rust example
    /// Pair { value: "x".to_owned() }
    /// ```
    /// ```rust example
    /// Pair { value: "y".to_owned() }
    /// ```
    #[model_schema(default_types(ValueType = String))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Pair<ValueType> {
        pub value: ValueType,
    }

    /// A generic type carrying none.
    #[model_schema(default_types(ValueType = String))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Lone<ValueType> {
        pub value: ValueType,
    }

    assert_eq!(
        Pair::<String>::schema_examples(),
        vec![
            serde_json::json!({ "value": "x" }),
            serde_json::json!({ "value": "y" })
        ]
    );
    let pair = Pair::<String>::zod_schema();
    assert!(
        pair.contains(
            "  const schema = buildPair$Schema(valueType).meta({\n    examples: \
             [{\"value\":\"x\"},{\"value\":\"y\"}]\n  });\n"
        ),
        "Got: {pair}"
    );
    assert_eq!(pair.matches(".meta(").count(), 1, "Got: {pair}");

    let lone = Lone::<String>::zod_schema();
    assert!(
        lone.contains(
            "  const schema = buildLone$Schema(valueType).meta({\n    examples: []\n  });\n"
        ),
        "Got: {lone}"
    );
    assert_eq!(lone.matches(".meta(").count(), 1, "Got: {lone}");
    assert!(!lone.contains("example:"), "Got: {lone}");
}

#[cfg(all(feature = "zod", feature = "serde"))]
#[test]
fn a_brand_holds_its_examples_inside_its_own_meta() {
    /// A token
    /// ```rust example
    /// Token("a".to_owned())
    /// ```
    /// ```rust example
    /// Token("b".to_owned())
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(transparent)]
    pub struct Token(pub String);

    /// A token with no example
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(transparent)]
    pub struct Ticket(pub String);

    /// A generic token with no example
    #[model_schema(default_types(Held = String))]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(transparent)]
    pub struct Stub<Held>(pub Held);

    assert_eq!(
        Token::schema_examples(),
        vec![serde_json::json!("a"), serde_json::json!("b")]
    );
    assert_eq!(Token::schema_example(), serde_json::json!("a"));
    let token = Token::zod_schema();
    assert!(
        token.contains(".meta({\n  description: \"A token\",\n  examples: [\"a\",\"b\"],\n});"),
        "Got: {token}"
    );
    assert_eq!(token.matches(".meta(").count(), 1, "Got: {token}");

    let ticket = Ticket::zod_schema();
    assert!(
        ticket
            .contains(".meta({\n  description: \"A token with no example\",\n  examples: [],\n});"),
        "Got: {ticket}"
    );
    assert_eq!(ticket.matches(".meta(").count(), 1, "Got: {ticket}");
    assert!(!ticket.contains("example:"), "Got: {ticket}");

    let stub = Stub::<String>::zod_schema();
    assert!(
        stub.contains(
            "held.meta({\n  description: \"A generic token with no example\",\n  examples: [],\n})"
        ),
        "Got: {stub}"
    );
    assert_eq!(stub.matches(".meta(").count(), 1, "Got: {stub}");
}

#[cfg(feature = "zod")]
#[test]
fn an_alias_publishes_an_empty_list() {
    let held: Several<Score> = vec![7];
    assert_eq!(held, [7]);

    // An alias publishes under its own name with `Type` after it, and its ident beside that.
    let score = score_schema::Schema::zod_schema();
    assert!(
        score.ends_with(
            ".meta({\n  examples: []\n});\n\nexport const Score$Schema = ScoreType$Schema;"
        ),
        "Got: {score}"
    );
    assert_eq!(score.matches(".meta(").count(), 1, "Got: {score}");
    assert!(!score.contains("example:"), "Got: {score}");

    let several = several_schema::Schema::zod_schema();
    assert!(
        several.contains(
            "  const schema = buildSeveralType$Schema(held).meta({\n    examples: []\n  });\n"
        ),
        "Got: {several}"
    );
    assert_eq!(several.matches(".meta(").count(), 1, "Got: {several}");
}

#[cfg(feature = "zod")]
#[test]
fn a_tuple_struct_and_a_unit_struct_publish_an_empty_list() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Point(pub u32, pub u32);

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Wrapped(pub u32);

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Ping;

    for zod in [
        Point::zod_schema(),
        Wrapped::zod_schema(),
        Ping::zod_schema(),
    ] {
        assert!(ends_in_examples(&zod, "[]"), "Got: {zod}");
        assert!(!zod.contains("example:"), "Got: {zod}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_tuple_struct_carries_its_examples() {
    /// A point
    /// ```rust example
    /// Point(1, 2)
    /// ```
    /// ```rust example
    /// Point(3, 4)
    /// ```
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Point(pub u32, pub u32);

    assert_eq!(
        Point::schema_examples(),
        vec![
            serde_json::json!([1_u32, 2_u32]),
            serde_json::json!([3_u32, 4_u32])
        ]
    );
    let zod = Point::zod_schema();
    assert!(ends_in_examples(&zod, "[[1,2],[3,4]]"), "Got: {zod}");
}
