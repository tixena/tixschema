//! An `Option` inside a covered sequence wrapper and an `Option` around one are two different
//! values on the wire, and each surface has to say which it describes.
//!
//! Every expectation here is read off what serde writes: a `None` the wrapper holds is a `null`
//! among the array's items (items admit `null`, the array does not); a `None` around the wrapper
//! replaces the whole array (the array admits `null`, its items do not). A set's array holds its
//! items distinct, which a `Vec`'s does not.

use alloc::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tixschema::model_schema;

/// The check Zod writes on the array of a set, which refuses an array that repeats an item.
#[cfg(feature = "zod")]
const REPEAT_CHECK: &str = ".check((ctx) => {
  const seen = new Set<string>();
  for (const item of ctx.value) {
    const text = JSON.stringify(item);
    if (seen.has(text)) {
      ctx.issues.push({ code: \"custom\", message: \"repeats an item: a set holds each item \
                            once\", input: ctx.value, continue: true });
      return;
    }
    seen.add(text);
  }
})";

/// The `Option` inside the wrapper. Serde always writes the array, so the key is always present and
/// the `None` reaches the wire as a `null` among the items.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ElementNullFields {
    set_items: BTreeSet<Option<u32>>,
    vec_items: Vec<Option<u32>>,
}

/// The `Option` around the wrapper. In field position a `None` writes either an absent key or a bare
/// `null` under it, and the generated contract now admits both — dropping the key still satisfies the
/// guard, so this shape keeps the omission.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct SlotNullFields {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    set_items: Option<BTreeSet<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vec_items: Option<Vec<u32>>,
}

/// The same four nestings in the two slots that cannot be dropped, plus the one holding both
/// `Option`s at once — the levels are independent, and neither swallows the other.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NullNestingSlots {
    map_both: HashMap<String, Option<Vec<Option<u32>>>>,
    map_set_element: HashMap<String, BTreeSet<Option<u32>>>,
    map_set_slot: HashMap<String, Option<BTreeSet<u32>>>,
    map_vec_element: HashMap<String, Vec<Option<u32>>>,
    map_vec_slot: HashMap<String, Option<Vec<u32>>>,
    map_wrapped: HashMap<String, Vec<Option<BTreeSet<Option<u32>>>>>,
    tuple_both: (String, Option<Vec<Option<u32>>>),
    tuple_set_element: (String, BTreeSet<Option<u32>>),
    tuple_set_slot: (String, Option<BTreeSet<u32>>),
    tuple_vec_element: (String, Vec<Option<u32>>),
    tuple_vec_slot: (String, Option<Vec<u32>>),
}

/// The member an `Option` field written with a `skip_serializing_if` renders as. The attribute
/// decides the wire, not the spelling, and none of these fields carries `ts_optional`.
#[cfg(feature = "typescript")]
fn omitted_member(name: &str, ts_type: &str) -> String {
    format!("{name}: {ts_type} | undefined;")
}

fn element_null_fields() -> ElementNullFields {
    ElementNullFields {
        set_items: BTreeSet::from([None]),
        vec_items: vec![None],
    }
}

fn slot_null_fields() -> SlotNullFields {
    SlotNullFields {
        set_items: None,
        vec_items: None,
    }
}

fn null_nesting_slots() -> NullNestingSlots {
    NullNestingSlots {
        map_both: HashMap::from([("k".to_owned(), Some(vec![None]))]),
        map_set_element: HashMap::from([("k".to_owned(), BTreeSet::from([None]))]),
        map_set_slot: HashMap::from([("k".to_owned(), None)]),
        map_vec_element: HashMap::from([("k".to_owned(), vec![None])]),
        map_vec_slot: HashMap::from([("k".to_owned(), None)]),
        map_wrapped: HashMap::from([("k".to_owned(), vec![Some(BTreeSet::from([None])), None])]),
        tuple_both: ("t".to_owned(), Some(vec![None])),
        tuple_set_element: ("t".to_owned(), BTreeSet::from([None])),
        tuple_set_slot: ("t".to_owned(), None),
        tuple_vec_element: ("t".to_owned(), vec![None]),
        tuple_vec_slot: ("t".to_owned(), None),
    }
}

/// A `None` the wrapper holds is a `null` inside the array, and the array is written either way.
#[test]
fn test_an_option_inside_a_wrapper_writes_a_null_among_the_items() {
    let payload = serde_json::to_value(element_null_fields()).unwrap();
    assert_eq!(payload["set_items"], serde_json::json!([null]));
    assert_eq!(payload["vec_items"], serde_json::json!([null]));
}

#[test]
fn test_an_option_around_a_wrapper_stands_for_the_whole_array() {
    let payload = serde_json::to_value(slot_null_fields()).unwrap();
    assert_eq!(payload, serde_json::json!({}));
}

#[test]
fn test_each_slot_writes_the_null_its_own_option_puts_there() {
    let payload = serde_json::to_value(null_nesting_slots()).unwrap();
    for field in ["map_set_element", "map_vec_element"] {
        assert_eq!(payload[field]["k"], serde_json::json!([null]), "{field}");
    }
    for field in ["map_set_slot", "map_vec_slot"] {
        assert_eq!(payload[field]["k"], serde_json::json!(null), "{field}");
    }
    assert_eq!(payload["map_both"]["k"], serde_json::json!([null]));
    assert_eq!(
        payload["map_wrapped"]["k"],
        serde_json::json!([[null], null])
    );
    for field in ["tuple_set_element", "tuple_vec_element"] {
        assert_eq!(payload[field][1], serde_json::json!([null]), "{field}");
    }
    for field in ["tuple_set_slot", "tuple_vec_slot"] {
        assert_eq!(payload[field][1], serde_json::json!(null), "{field}");
    }
    assert_eq!(payload["tuple_both"][1], serde_json::json!([null]));
}

#[cfg(feature = "jsonschema")]
fn integer_or_null() -> serde_json::Value {
    serde_json::json!({ "anyOf": [{ "type": "integer" }, { "type": "null" }] })
}

#[cfg(feature = "jsonschema")]
fn array_of(items: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "type": "array", "items": items })
}

#[cfg(feature = "jsonschema")]
fn or_null(base: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "anyOf": [base, { "type": "null" }] })
}

/// The array of a set, whose items are distinct.
#[cfg(feature = "jsonschema")]
fn set_of(items: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "type": "array", "items": items, "uniqueItems": true })
}

/// `schema` with the `uniqueItems` of each set taken out: what a set describes beside what its
/// `Vec` twin describes.
#[cfg(feature = "jsonschema")]
fn without_unique_items(schema: &serde_json::Value) -> serde_json::Value {
    match schema {
        serde_json::Value::Object(members) => serde_json::Value::Object(
            members
                .iter()
                .filter(|(key, _)| key.as_str() != "uniqueItems")
                .map(|(key, member)| (key.clone(), without_unique_items(member)))
                .collect(),
        ),
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(without_unique_items).collect())
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => schema.clone(),
    }
}

#[test]
#[cfg(feature = "jsonschema")]
fn test_an_element_null_field_describes_as_an_array_of_nullable_items() {
    let schema = ElementNullFields::json_schema();
    assert_eq!(
        schema["properties"]["set_items"],
        set_of(&integer_or_null())
    );
    assert_eq!(
        schema["properties"]["vec_items"],
        array_of(&integer_or_null())
    );
    for field in ["set_items", "vec_items"] {
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(field)),
            "{field} must stay required: {schema}"
        );
    }
}

#[test]
#[cfg(feature = "jsonschema")]
fn test_a_slot_null_field_describes_as_the_array_it_writes_when_present() {
    let schema = SlotNullFields::json_schema();
    let integer = serde_json::json!({ "type": "integer" });
    assert_eq!(
        schema["properties"]["set_items"],
        or_null(&set_of(&integer))
    );
    assert_eq!(
        schema["properties"]["vec_items"],
        or_null(&array_of(&integer))
    );
    for field in ["set_items", "vec_items"] {
        assert!(
            !schema["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(field)),
            "{field} must not be required: {schema}"
        );
    }
}

#[test]
#[cfg(feature = "jsonschema")]
fn test_each_slot_describes_the_null_its_own_option_writes() {
    let properties = NullNestingSlots::json_schema()["properties"].clone();
    let integer = serde_json::json!({ "type": "integer" });
    let plain_array = array_of(&integer);
    let nullable_items = array_of(&integer_or_null());
    let nullable_set = set_of(&integer_or_null());

    assert_eq!(
        properties["map_set_element"]["additionalProperties"],
        nullable_set
    );
    assert_eq!(
        properties["map_vec_element"]["additionalProperties"],
        nullable_items
    );
    assert_eq!(
        properties["map_set_slot"]["additionalProperties"],
        or_null(&set_of(&integer))
    );
    assert_eq!(
        properties["map_vec_slot"]["additionalProperties"],
        or_null(&plain_array)
    );
    assert_eq!(
        properties["map_both"]["additionalProperties"],
        or_null(&nullable_items)
    );

    assert_eq!(
        properties["tuple_set_element"]["prefixItems"][1],
        nullable_set
    );
    assert_eq!(
        properties["tuple_vec_element"]["prefixItems"][1],
        nullable_items
    );
    assert_eq!(
        properties["tuple_set_slot"]["prefixItems"][1],
        or_null(&set_of(&integer))
    );
    assert_eq!(
        properties["tuple_vec_slot"]["prefixItems"][1],
        or_null(&plain_array)
    );
    assert_eq!(
        properties["tuple_both"]["prefixItems"][1],
        or_null(&nullable_items)
    );

    // A wrapper under an array is two levels, and each one answers for the `Option` written at it:
    // the inner array's items, then the inner array itself, both inside the outer array.
    assert_eq!(
        properties["map_wrapped"]["additionalProperties"],
        array_of(&or_null(&nullable_set))
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_the_typescript_surface_puts_the_null_at_the_level_it_was_written() {
    for spelling in [
        "set_items: Array<number | null>;",
        "vec_items: Array<number | null>;",
    ] {
        let definition = ElementNullFields::ts_definition();
        assert!(definition.contains(spelling), "Got: {definition}");
    }
    for spelling in [
        omitted_member("set_items", "Array<number>"),
        omitted_member("vec_items", "Array<number>"),
    ] {
        let definition = SlotNullFields::ts_definition();
        assert!(definition.contains(&spelling), "Got: {definition}");
    }
    let definition = NullNestingSlots::ts_definition();
    for spelling in [
        "map_both: Partial<Record<string, Array<number | null> | null>>;",
        "map_wrapped: Partial<Record<string, Array<Array<number | null> | null>>>;",
        "map_set_element: Partial<Record<string, Array<number | null>>>;",
        "map_set_slot: Partial<Record<string, Array<number> | null>>;",
        "map_vec_element: Partial<Record<string, Array<number | null>>>;",
        "map_vec_slot: Partial<Record<string, Array<number> | null>>;",
        "tuple_both: [string, Array<number | null> | null];",
        "tuple_set_element: [string, Array<number | null>];",
        "tuple_set_slot: [string, Array<number> | null];",
        "tuple_vec_element: [string, Array<number | null>];",
        "tuple_vec_slot: [string, Array<number> | null];",
    ] {
        assert!(definition.contains(spelling), "Got: {definition}");
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_the_zod_surface_puts_the_nullable_at_the_level_it_was_written() {
    let element_null = ElementNullFields::zod_schema();
    for spelling in [
        format!(
            "set_items: z.array(z.nullable(z.number().int().min(0).max(4294967295))){REPEAT_CHECK},"
        ),
        "vec_items: z.array(z.nullable(z.number().int().min(0).max(4294967295))),".to_owned(),
    ] {
        assert!(element_null.contains(&spelling), "Got: {element_null}");
    }
    let slot_null = SlotNullFields::zod_schema();
    for spelling in [
        format!(
            "set_items: z.union([z.null().transform(() => undefined), \
             z.array(z.number().int().min(0).max(4294967295)){REPEAT_CHECK}, z.undefined()]).prefault(undefined),"
        ),
        "vec_items: z.union([z.null().transform(() => undefined), z.array(z.number().int().min(0).max(4294967295)), \
         z.undefined()]).prefault(undefined),"
            .to_owned(),
    ] {
        assert!(slot_null.contains(&spelling), "Got: {slot_null}");
    }
    let schema = NullNestingSlots::zod_schema();
    for spelling in [
        "map_both: z.record(z.string(), z.nullable(z.array(z.nullable(z.number().int().min(0).max(4294967295))))),"
            .to_owned(),
        format!(
            "map_wrapped: z.record(z.string(), \
             z.array(z.nullable(z.array(z.nullable(z.number().int().min(0).max(4294967295))){REPEAT_CHECK}))),"
        ),
        format!(
            "map_set_element: z.record(z.string(), \
             z.array(z.nullable(z.number().int().min(0).max(4294967295))){REPEAT_CHECK}),"
        ),
        format!(
            "map_set_slot: z.record(z.string(), z.nullable(z.array(z.number().int().min(0).max(4294967295)){REPEAT_CHECK})),"
        ),
        "map_vec_element: z.record(z.string(), z.array(z.nullable(z.number().int().min(0).max(4294967295)))),".to_owned(),
        "map_vec_slot: z.record(z.string(), z.nullable(z.array(z.number().int().min(0).max(4294967295)))),".to_owned(),
        "tuple_both: z.tuple([z.string(), z.nullable(z.array(z.nullable(z.number().int().min(0).max(4294967295))))]),"
            .to_owned(),
        format!(
            "tuple_set_element: z.tuple([z.string(), \
             z.array(z.nullable(z.number().int().min(0).max(4294967295))){REPEAT_CHECK}]),"
        ),
        format!(
            "tuple_set_slot: z.tuple([z.string(), z.nullable(z.array(z.number().int().min(0).max(4294967295)){REPEAT_CHECK})]),"
        ),
        "tuple_vec_element: z.tuple([z.string(), z.array(z.nullable(z.number().int().min(0).max(4294967295)))]),"
            .to_owned(),
        "tuple_vec_slot: z.tuple([z.string(), z.nullable(z.array(z.number().int().min(0).max(4294967295)))]),".to_owned(),
    ] {
        assert!(schema.contains(&spelling), "{spelling} in: {schema}");
    }
}

#[test]
#[cfg(feature = "jsonschema")]
fn test_a_set_nesting_describes_as_the_vec_nesting_it_writes() {
    let properties = NullNestingSlots::json_schema()["properties"].clone();
    for (set_field, vec_field) in [
        ("map_set_element", "map_vec_element"),
        ("map_set_slot", "map_vec_slot"),
        ("tuple_set_element", "tuple_vec_element"),
        ("tuple_set_slot", "tuple_vec_slot"),
    ] {
        assert_eq!(
            without_unique_items(&properties[set_field]),
            properties[vec_field],
            "{set_field} against {vec_field}"
        );
    }
    assert_eq!(
        without_unique_items(&ElementNullFields::json_schema()["properties"]["set_items"]),
        ElementNullFields::json_schema()["properties"]["vec_items"]
    );
}
