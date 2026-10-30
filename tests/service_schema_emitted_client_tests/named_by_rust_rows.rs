//! The payloads each emitted client reads the models of `named_by_rust_models.rs` from, and what
//! the serde read prints of each: the value written back with its keys sorted, or `refused` with
//! each message where `validate()` judges the payload.

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::named_by_rust_models::{
    Bough, Carriage, CarriageAdjacent, CarriageExternal, CarriageUntagged, Fork, Hauler, Hitch,
    Rope, Spur, Tip,
};

/// The rows of a field written `Self`.
pub const SELF_ROWS: [Row; 8] = [
    held(
        "a tree two levels deep",
        "Bough",
        r#"{"label":"root","twigs":[{"label":"a","twigs":[{"label":"b","twigs":[]}]}]}"#,
    ),
    held(
        "a tree broken in a child and in a grandchild",
        "Bough",
        r#"{"label":"root","twigs":[{"label":"","twigs":[{"label":"","twigs":[]}]}]}"#,
    ),
    held(
        "a map of itself",
        "Spur",
        r#"{"label":"s","offshoots":{"east":{"label":"o","offshoots":{}}}}"#,
    ),
    held(
        "a map of itself, broken",
        "Spur",
        r#"{"label":"s","offshoots":{"east":{"label":"","offshoots":{}}}}"#,
    ),
    held(
        "a variant that holds itself",
        "Fork",
        r#"{"kind":"Split","halves":[{"kind":"Leaf","label":"a"},{"kind":"Split","halves":[]}]}"#,
    ),
    held(
        "a variant that holds itself, broken",
        "Fork",
        r#"{"kind":"Split","halves":[{"kind":"Leaf","label":""}]}"#,
    ),
    held(
        "a generic type that holds itself",
        "Rope",
        r#"{"links":[{"links":[],"strand":2}],"strand":1}"#,
    ),
    unjudged(
        "a generic type that holds itself, with a wrong strand",
        "Rope",
        r#"{"links":[{"links":[],"strand":"x"}],"strand":1}"#,
    ),
];

/// The rows of an `Option` over a `Box` of `Self`.
pub const TIP_ROWS: [Row; 5] = [
    held(
        "an optional box of itself, empty",
        "Tip",
        r#"{"label":"s","next":null}"#,
    ),
    held(
        "an optional box of itself",
        "Tip",
        r#"{"label":"s","next":{"label":"n","next":null}}"#,
    ),
    held(
        "an optional box of itself, three links",
        "Tip",
        r#"{"label":"a","next":{"label":"b","next":{"label":"c","next":{"label":"d","next":null}}}}"#,
    ),
    held(
        "an optional box of itself, broken",
        "Tip",
        r#"{"label":"s","next":{"label":"","next":null}}"#,
    ),
    held(
        "an optional box of itself, broken in the third link",
        "Tip",
        r#"{"label":"a","next":{"label":"b","next":{"label":"","next":null}}}"#,
    ),
];

/// The rows of a model declared below the model that holds it.
pub const BELOW_ROWS: [Row; 6] = [
    held(
        "a holder and its trailer, both whole",
        "Hitch",
        r#"{"hook":"h","trailer":{"plate":"p"}}"#,
    ),
    held(
        "a holder and its trailer, both broken",
        "Hitch",
        r#"{"hook":"","trailer":{"plate":""}}"#,
    ),
    held(
        "a trailer alone broken",
        "Hitch",
        r#"{"hook":"h","trailer":{"plate":""}}"#,
    ),
    held(
        "an optional and a list of trailers, whole",
        "Hauler",
        r#"{"name":"n","spare":{"tag":"s"},"trailers":[{"tag":"a"},{"tag":"b"}]}"#,
    ),
    held(
        "an optional and a list of trailers, all broken",
        "Hauler",
        r#"{"name":"","spare":{"tag":""},"trailers":[{"tag":"a"},{"tag":""}]}"#,
    ),
    held(
        "no trailer in the optional",
        "Hauler",
        r#"{"name":"n","spare":null,"trailers":[]}"#,
    ),
];

/// The rows of a variant whose own `rename_all` names its members, under every tagging.
pub const CARRIAGE_ROWS: [Row; 21] = [
    held(
        "internal, the variant rename_all",
        "Carriage",
        r#"{"mode":"OverLand","roadName":"DR-1","axle_total":3}"#,
    ),
    held(
        "internal, the enum rename_all_fields",
        "Carriage",
        r#"{"mode":"Sea","PORT_NAME":"Haina"}"#,
    ),
    held(
        "internal, a bound broken",
        "Carriage",
        r#"{"mode":"OverLand","roadName":"","axle_total":3}"#,
    ),
    unjudged(
        "internal, the Rust name",
        "Carriage",
        r#"{"mode":"OverLand","road_name":"DR-1","axle_count":3}"#,
    ),
    unjudged(
        "internal, the enum rule on the variant",
        "Carriage",
        r#"{"mode":"OverLand","ROAD_NAME":"DR-1","axle_total":3}"#,
    ),
    held(
        "external, the variant rename_all",
        "CarriageExternal",
        r#"{"OverLand":{"roadName":"DR-1","axle_total":3}}"#,
    ),
    held(
        "external, the enum rename_all_fields",
        "CarriageExternal",
        r#"{"Sea":{"PORT_NAME":"Haina"}}"#,
    ),
    held(
        "external, a bound broken",
        "CarriageExternal",
        r#"{"OverLand":{"roadName":"","axle_total":3}}"#,
    ),
    unjudged(
        "external, the Rust name",
        "CarriageExternal",
        r#"{"OverLand":{"road_name":"DR-1","axle_count":3}}"#,
    ),
    unjudged(
        "external, the enum rule on the variant",
        "CarriageExternal",
        r#"{"OverLand":{"ROAD_NAME":"DR-1","axle_total":3}}"#,
    ),
    held(
        "adjacent, the variant rename_all",
        "CarriageAdjacent",
        r#"{"mode":"OverLand","body":{"roadName":"DR-1","axle_total":3}}"#,
    ),
    held(
        "adjacent, the enum rename_all_fields",
        "CarriageAdjacent",
        r#"{"mode":"Sea","body":{"PORT_NAME":"Haina"}}"#,
    ),
    held(
        "adjacent, a bound broken",
        "CarriageAdjacent",
        r#"{"mode":"OverLand","body":{"roadName":"","axle_total":3}}"#,
    ),
    unjudged(
        "adjacent, the Rust name",
        "CarriageAdjacent",
        r#"{"mode":"OverLand","body":{"road_name":"DR-1","axle_count":3}}"#,
    ),
    unjudged(
        "adjacent, the enum rule on the variant",
        "CarriageAdjacent",
        r#"{"mode":"OverLand","body":{"ROAD_NAME":"DR-1","axle_total":3}}"#,
    ),
    held(
        "untagged, the variant rename_all",
        "CarriageUntagged",
        r#"{"roadName":"DR-1","axle_total":3}"#,
    ),
    held(
        "untagged, the enum rename_all_fields",
        "CarriageUntagged",
        r#"{"PORT_NAME":"Haina"}"#,
    ),
    unjudged(
        "untagged, a bound broken",
        "CarriageUntagged",
        r#"{"roadName":"","axle_total":3}"#,
    ),
    unjudged(
        "untagged, the Rust name",
        "CarriageUntagged",
        r#"{"road_name":"DR-1","axle_count":3}"#,
    ),
    unjudged(
        "untagged, the enum rule on the variant",
        "CarriageUntagged",
        r#"{"ROAD_NAME":"DR-1","axle_total":3}"#,
    ),
    unjudged(
        "internal, no tag",
        "Carriage",
        r#"{"roadName":"DR-1","axle_total":3}"#,
    ),
];

/// One payload a client reads: its name, the type it is read as, the JSON, and whether the
/// messages of the refusal are compared.
pub struct Row {
    pub json: &'static str,
    pub judged: bool,
    pub name: &'static str,
    pub read_as: &'static str,
}

const fn held(name: &'static str, read_as: &'static str, json: &'static str) -> Row {
    Row {
        judged: true,
        json,
        name,
        read_as,
    }
}

const fn unjudged(name: &'static str, read_as: &'static str, json: &'static str) -> Row {
    Row {
        judged: false,
        json,
        name,
        read_as,
    }
}

/// Whether `read_as` names a type that declares a parameter, which a client reads at a 32-bit
/// integer.
pub fn declares_a_parameter(read_as: &str) -> bool {
    read_as == "Rope"
}

/// `json` with the keys of every object sorted, as the rows print a value.
pub fn sorted(json: &str) -> String {
    /// `value` with the keys of every object it holds sorted.
    fn sorting(value: Value) -> Value {
        match value {
            Value::Object(entries) => {
                let keyed: BTreeMap<String, Value> = entries
                    .into_iter()
                    .map(|(key, held)| (key, sorting(held)))
                    .collect();
                Value::Object(keyed.into_iter().collect())
            }
            Value::Array(items) => Value::Array(items.into_iter().map(sorting).collect()),
            held @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => held,
        }
    }
    serde_json::to_string(&sorting(serde_json::from_str(json).unwrap())).unwrap()
}

/// What the serde read prints of `row`, read as `T`.
fn printed<T>(row: &Row) -> String
where
    T: DeserializeOwned + Serialize,
{
    match serde_json::from_str::<T>(row.json) {
        Ok(read) => format!(
            "{}: read {}\n",
            row.name,
            sorted(&serde_json::to_string(&read).unwrap())
        ),
        Err(refused) => {
            let mut said = format!("{}: refused\n", row.name);
            if row.judged {
                let told = refused.to_string();
                let message = told
                    .rsplit_once(" at line ")
                    .map_or(told.as_str(), |(message, _)| message);
                for each in message.split("; ") {
                    let _ = writeln!(said, "    {each}");
                }
            }
            said
        }
    }
}

/// What the serde read prints of each of `rows`.
pub fn expected(rows: &[Row]) -> String {
    rows.iter()
        .map(|row| match row.read_as {
            "Bough" => printed::<Bough>(row),
            "Carriage" => printed::<Carriage>(row),
            "CarriageAdjacent" => printed::<CarriageAdjacent>(row),
            "CarriageExternal" => printed::<CarriageExternal>(row),
            "CarriageUntagged" => printed::<CarriageUntagged>(row),
            "Fork" => printed::<Fork>(row),
            "Hauler" => printed::<Hauler>(row),
            "Hitch" => printed::<Hitch>(row),
            "Rope" => printed::<Rope<i32>>(row),
            "Spur" => printed::<Spur>(row),
            "Tip" => printed::<Tip>(row),
            _ => String::new(),
        })
        .collect()
}

/// `printed` as the clients print it, each value line with its keys sorted.
pub fn normalized(printed: &str) -> String {
    let mut lines = String::new();
    for line in printed.lines() {
        match line.split_once(": read ") {
            Some((name, json)) => {
                let _ = writeln!(lines, "{name}: read {}", sorted(json));
            }
            None => {
                let _ = writeln!(lines, "{line}");
            }
        }
    }
    lines
}
