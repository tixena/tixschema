//! The JSON Schema of each map keyed by a value serde writes as text of its own: its values under
//! every key, and its keys by that text, held to what serde and Zod read of each payload.

use serde_json::{Value, json};

use super::documents::admits;
use super::keyed_maps::{
    Baggywrinkle, Futtock, Gudgeon, Marlinspike, PAYLOADS, Scuttle, Trunnel,
    assert_serde_reads_what_zod_reads, serde_reads,
};

/// Each model a payload is read as, beside its JSON Schema.
const DOCUMENTS: [(&str, Written); 5] = [
    ("Marlinspike", Marlinspike::json_schema),
    ("Trunnel", Trunnel::json_schema),
    ("Scuttle", Scuttle::json_schema),
    ("Baggywrinkle", Baggywrinkle::json_schema),
    ("Gudgeon", Gudgeon::json_schema),
];

/// A model's JSON Schema.
type Written = fn() -> Value;

/// The JSON Schema of the model a payload is read as.
fn document(model: &str) -> Value {
    let (_, written) = DOCUMENTS.iter().find(|(name, _)| *name == model).unwrap();
    written()
}

#[test]
fn serde_reads_what_zod_reads_but_where_they_part() {
    assert_serde_reads_what_zod_reads();
}

/// The JSON Schema reads each payload Zod reads: what serde reads, and what serde parts from Zod
/// on.
#[test]
fn the_json_schema_admits_each_payload_zod_reads() {
    for payload in &PAYLOADS {
        assert_eq!(
            admits(
                &document(payload.read_as),
                &serde_json::from_str(payload.json).unwrap()
            ),
            serde_reads(payload) || payload.parted,
            "the JSON Schema parts from Zod on `{}`: {}",
            payload.name,
            document(payload.read_as)
        );
    }
}

/// A map under a key describes its values, and names its keys by the text serde writes them as.
#[test]
fn a_map_under_a_key_describes_its_values_and_names_its_keys() {
    let counts = |names: Value| {
        json!({
            "type": "object",
            "additionalProperties": { "type": "integer" },
            "propertyNames": names,
        })
    };
    let one_character = json!({ "type": "string", "minLength": 1_u8, "maxLength": 1_u8 });
    assert_eq!(
        Marlinspike::json_schema()["properties"],
        json!({
            "counts": counts(json!({ "type": "string", "pattern": "^-?[0-9]+(\\.0+)?$" })),
            "flags": counts(json!({ "type": "string", "pattern": "^(true|false)$" })),
            "letters": counts(one_character.clone()),
            "turns": counts(one_character),
        })
    );
    assert_eq!(
        Gudgeon::json_schema()["properties"]["days"],
        counts(json!({ "type": "string", "format": "date" }))
    );
}

/// A brand over a `char` is text of one character, as a `char` is, and refuses text of two, as
/// serde does.
#[test]
fn a_brand_over_a_char_is_text_of_one_character() {
    let document = Futtock::json_schema();
    assert_eq!(
        document,
        json!({ "type": "string", "minLength": 1_u8, "maxLength": 1_u8 })
    );
    assert!(admits(&document, &json!("t")), "{document}");
    assert!(!admits(&document, &json!("tt")), "{document}");
    assert_eq!(
        serde_json::from_str::<Futtock>(r#""tt""#)
            .unwrap_err()
            .to_string(),
        "invalid value: string \"tt\", expected a character at line 1 column 4"
    );
}
