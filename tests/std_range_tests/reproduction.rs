//! The reproduction: a field of `Range<usize>`, with `Range` brought in by `use`. Swift and Kotlin
//! refuse a `usize` wherever it is written, so the model is declared where neither is on.

use alloc::collections::BTreeMap;
use core::ops::Range;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::documents::admits;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub span: Range<usize>,
    pub value: String,
}

/// A range of `usize` in an `Option`, a list and a map.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Passages {
    pub keyed: BTreeMap<String, Range<usize>>,
    pub listed: Vec<Range<usize>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<Range<usize>>,
}

#[test]
fn a_range_of_usize_is_the_object_in_an_option_a_list_and_a_map() {
    let object = "{ start: number; end: number }";
    let typescript = Passages::ts_definition();
    for member in [
        format!("keyed: Partial<Record<string, {object}>>;"),
        format!("listed: Array<{object}>;"),
        format!("maybe: {object} | undefined;"),
    ] {
        assert!(typescript.contains(&member), "{member} in: {typescript}");
    }
    for held in [true, false] {
        let passages = Passages {
            keyed: BTreeMap::from([("a".to_owned(), 1..2)]),
            listed: vec![3..4, 7..9],
            maybe: held.then_some(5..6),
        };
        let json = serde_json::to_value(&passages).unwrap();
        assert!(admits(&Passages::json_schema(), &json), "{json}");
        assert_eq!(serde_json::from_value::<Passages>(json).unwrap(), passages);
    }
}

#[test]
fn a_range_of_usize_compiles_and_is_described_as_its_start_and_end() {
    assert!(
        Text::ts_definition().contains("span: { start: number; end: number };"),
        "{}",
        Text::ts_definition()
    );
    assert!(
        Text::zod_schema().contains(
            "span: z.strictObject({ start: z.number().int().min(0), end: z.number().int().min(0) }),"
        ),
        "{}",
        Text::zod_schema()
    );
    let value = Text {
        span: 8..13,
        value: "word".to_owned(),
    };
    let json = serde_json::to_value(&value).unwrap();
    assert_eq!(
        json["span"],
        serde_json::json!({ "start": 8_usize, "end": 13_usize })
    );
    assert_eq!(serde_json::from_value::<Text>(json).unwrap(), value);
    assert_eq!(
        Text::json_schema()["properties"]["span"]["required"],
        serde_json::json!(["start", "end"])
    );
}
