//! Structs that hold no key on the wire: the message the macro declares for an operation of no
//! argument, and a unit struct and a struct whose every member serde skips, each strict and
//! `nonstrict`. Each is read from `{}`, and a strict one refuses a key beside it where a
//! `nonstrict` one reads past it, on every surface.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::belfry_service::RingBellRequest;

/// The payloads every surface reads each type from.
pub const PAYLOADS: [&str; 2] = ["{}", r#"{"stray":1}"#];

/// Each type's name, and whether it is strict.
pub const TYPES: [(&str, bool); 5] = [
    ("RingBellRequest", true),
    ("Knell", true),
    ("OpenKnell", false),
    ("Unheld", true),
    ("OpenUnheld", false),
];

/// A unit struct, which crosses the wire as `{}`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Knell;

/// A unit struct that reads past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct OpenKnell;

/// A struct whose one member serde neither writes nor reads, which crosses the wire as `{}`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Unheld {
    #[serde(skip)]
    pub held: u32,
}

/// A struct whose one member serde neither writes nor reads, reading past a key beside it.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct OpenUnheld {
    #[serde(skip)]
    pub held: u32,
}

/// Every row a surface is handed: each type with each payload.
pub fn rows() -> Vec<(&'static str, &'static str)> {
    TYPES
        .iter()
        .flat_map(|(name, _)| PAYLOADS.map(|payload| (*name, payload)))
        .collect()
}

/// What every surface must answer of each row: `{}` where it reads it, `refused` where it does
/// not. A strict type refuses the stray key, and a `nonstrict` one reads past it.
pub fn expected() -> Vec<String> {
    TYPES
        .iter()
        .flat_map(|(_, strict)| {
            PAYLOADS.map(|payload| {
                if *strict && payload != "{}" {
                    "refused".to_owned()
                } else {
                    "{}".to_owned()
                }
            })
        })
        .collect()
}

/// What serde writes back of `json` read as `T`, or `refused`.
fn serde_told<T>(json: &str) -> String
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| serde_json::to_string(&read).unwrap(),
    )
}

/// The JSON Schema of the type `name` names.
pub fn json_schema_of(name: &str) -> serde_json::Value {
    match name {
        "RingBellRequest" => RingBellRequest::json_schema(),
        "Knell" => Knell::json_schema(),
        "OpenKnell" => OpenKnell::json_schema(),
        "Unheld" => Unheld::json_schema(),
        _ => OpenUnheld::json_schema(),
    }
}

/// The TypeScript type and the Zod schema of every type.
pub fn ts_definitions() -> Vec<String> {
    vec![
        RingBellRequest::ts_definition(),
        RingBellRequest::zod_schema(),
        Knell::ts_definition(),
        Knell::zod_schema(),
        OpenKnell::ts_definition(),
        OpenKnell::zod_schema(),
        Unheld::ts_definition(),
        Unheld::zod_schema(),
        OpenUnheld::ts_definition(),
        OpenUnheld::zod_schema(),
    ]
}

#[test]
fn serde_reads_each_empty_struct_from_an_empty_object_and_a_strict_one_refuses_a_key() {
    let told: Vec<String> = rows()
        .into_iter()
        .map(|(name, json)| match name {
            "RingBellRequest" => serde_told::<RingBellRequest>(json),
            "Knell" => serde_told::<Knell>(json),
            "OpenKnell" => serde_told::<OpenKnell>(json),
            "Unheld" => serde_told::<Unheld>(json),
            _ => serde_told::<OpenUnheld>(json),
        })
        .collect();
    assert_eq!(told, expected());
    let read_past: OpenUnheld = serde_json::from_str(PAYLOADS[1]).unwrap();
    assert_eq!(read_past.held, 0);
    let read: Unheld = serde_json::from_str(PAYLOADS[0]).unwrap();
    assert_eq!(read.held, 0);
}

#[test]
fn the_json_schema_of_each_empty_struct_accepts_what_serde_reads() {
    let told: Vec<String> = rows()
        .into_iter()
        .map(|(name, json)| {
            let validator = jsonschema::validator_for(&json_schema_of(name)).unwrap();
            if validator.is_valid(&serde_json::from_str(json).unwrap()) {
                "{}".to_owned()
            } else {
                "refused".to_owned()
            }
        })
        .collect();
    assert_eq!(told, expected());
}

#[test]
fn the_typescript_type_of_each_empty_struct_is_an_object_of_no_key() {
    for (name, _) in TYPES {
        let written = ts_definitions()
            .into_iter()
            .find(|text| text.contains(&format!("export type {name}")))
            .unwrap();
        assert!(
            written.contains(&format!("export type {name}Draft = Record<string, never>;"))
                || written.contains(&format!("export type {name} = Record<string, never>;")),
            "{written}"
        );
    }
}
