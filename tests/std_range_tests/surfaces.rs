//! What TypeScript, Zod and the JSON Schema write for each range: the object of the members serde
//! writes, each bound rendered as its type renders anywhere else.

/// A range of dates, and the date it holds written alone.
#[cfg(feature = "chrono")]
mod dated {
    use core::ops::Range;

    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    use super::member_text;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Timesheet {
        pub at: DateTime<Utc>,
        pub shift: Range<DateTime<Utc>>,
    }

    #[test]
    fn a_range_of_dates_holds_the_date_as_a_date_field_writes_it() {
        let typescript = Timesheet::ts_definition();
        let typed = member_text(&typescript, "at").trim_end_matches(';');
        assert!(
            typescript.contains(&format!("shift: {{ start: {typed}; end: {typed} }};")),
            "{typescript}"
        );
        let zod = Timesheet::zod_schema();
        let parsed = member_text(&zod, "at").trim_end_matches(',');
        assert!(
            zod.contains(&format!(
                "shift: z.strictObject({{ start: {parsed}, end: {parsed} }}),"
            )),
            "{zod}"
        );
        let document = Timesheet::json_schema();
        assert_eq!(
            document["properties"]["shift"]["properties"]["start"],
            document["properties"]["at"]
        );
        assert_eq!(
            document["properties"]["shift"]["properties"]["end"],
            document["properties"]["at"]
        );
        let shift = super::super::models::rostered().shift;
        let timesheet = Timesheet {
            at: shift.start,
            shift,
        };
        let read: Timesheet =
            serde_json::from_str(&serde_json::to_string(&timesheet).unwrap()).unwrap();
        assert_eq!(read, timesheet);
    }
}

/// A range of ids, and the id it holds written alone.
#[cfg(feature = "mongodb")]
mod identified {
    use core::ops::RangeInclusive;

    use bson::oid::ObjectId;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    use super::member_text;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Shelving {
        pub id: ObjectId,
        pub ids: RangeInclusive<ObjectId>,
    }

    #[test]
    fn a_range_of_ids_holds_the_id_as_an_id_field_writes_it() {
        let typescript = Shelving::ts_definition();
        let typed = member_text(&typescript, "id").trim_end_matches(';');
        assert!(
            typescript.contains(&format!("ids: {{ start: {typed}; end: {typed} }};")),
            "{typescript}"
        );
        let zod = Shelving::zod_schema();
        let parsed = member_text(&zod, "id").trim_end_matches(',');
        assert!(
            zod.contains(&format!(
                "ids: z.strictObject({{ start: {parsed}, end: {parsed} }}),"
            )),
            "{zod}"
        );
        let document = Shelving::json_schema();
        assert_eq!(
            document["properties"]["ids"]["properties"]["start"],
            document["properties"]["id"]
        );
        let low = ObjectId::new();
        let shelving = Shelving {
            id: low,
            ids: low..=low,
        };
        let read: Shelving =
            serde_json::from_str(&serde_json::to_string(&shelving).unwrap()).unwrap();
        assert_eq!(read, shelving);
    }
}

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::documents::{Refusal, admits, refusal};
use super::models::{
    Compass, Course, Detour, Excursion, Frontier, Itinerary, Milestone, Positioned, Signposted,
    Spanned, Stride, Waymark, Waypost, judged_payloads, read_payloads, refused_payloads,
};

/// Each model a payload is read as: its name, its JSON Schema, and whether serde reads a JSON as
/// it.
type Described = (&'static str, fn() -> Value, fn(&str) -> bool);

/// The JSON Schema of a range of integers that writes `members`.
fn integer_range(members: &[&str]) -> Value {
    let properties: serde_json::Map<String, Value> = members
        .iter()
        .map(|member| ((*member).to_owned(), json!({ "type": "integer" })))
        .collect();
    json!({
        "type": "object",
        "properties": properties,
        "required": members,
        "additionalProperties": false,
    })
}

/// Whether serde reads `json` as a `T`.
fn reads<T>(json: &str) -> bool
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).is_ok()
}

/// The models the payloads are read as.
fn described() -> Vec<Described> {
    vec![
        ("Compass", Compass::json_schema, reads::<Compass>),
        ("Course", Course::<u32>::json_schema, reads::<Course<u32>>),
        ("Detour", Detour::json_schema, reads::<Detour>),
        ("Excursion", Excursion::json_schema, reads::<Excursion>),
        ("Frontier", Frontier::json_schema, reads::<Frontier>),
        ("Positioned", Positioned::json_schema, reads::<Positioned>),
        #[cfg(feature = "chrono")]
        (
            "Rostered",
            super::models::Rostered::json_schema,
            reads::<super::models::Rostered>,
        ),
        ("Signposted", Signposted::json_schema, reads::<Signposted>),
        ("Spanned", Spanned::json_schema, reads::<Spanned>),
        ("Stride", Stride::<u32>::json_schema, reads::<Stride<u32>>),
        ("Waypost", Waypost::json_schema, reads::<Waypost>),
    ]
}

/// The model `read_as` names.
fn model(read_as: &str) -> Described {
    described()
        .into_iter()
        .find(|(named, _, _)| *named == read_as)
        .unwrap()
}

/// The JSON Schema of the model `read_as` names.
fn document_of(read_as: &str) -> Value {
    (model(read_as).1)()
}

/// Whether serde reads `json` as the model `read_as` names.
fn serde_reads(read_as: &str, json: &str) -> bool {
    (model(read_as).2)(json)
}

#[test]
fn each_range_is_described_as_the_object_serde_writes() {
    let typescript = Spanned::ts_definition();
    for member in [
        "closed: { start: number; end: number };",
        "from: { start: number };",
        "span: { start: number; end: number };",
        "to: { end: number };",
    ] {
        assert!(typescript.contains(member), "{member} in: {typescript}");
    }
    let zod = Spanned::zod_schema();
    for member in [
        "closed: z.strictObject({ start: z.number().int().min(0).max(4294967295), end: z.number().int().min(0).max(4294967295) }),",
        "from: z.strictObject({ start: z.number().int().min(0).max(4294967295) }),",
        "span: z.strictObject({ start: z.number().int().min(0).max(4294967295), end: z.number().int().min(0).max(4294967295) }),",
        "to: z.strictObject({ end: z.number().int().min(0).max(4294967295) }),",
    ] {
        assert!(zod.contains(member), "{member} in: {zod}");
    }
    let document = Spanned::json_schema();
    for (field, members) in [
        ("closed", &["start", "end"][..]),
        ("from", &["start"][..]),
        ("span", &["start", "end"][..]),
        ("to", &["end"][..]),
    ] {
        assert_eq!(
            document["properties"][field],
            integer_range(members),
            "{field}"
        );
    }
    assert_eq!(
        serde_json::to_value(super::models::spanned()).unwrap(),
        json!({
            "closed": { "start": 1_u32, "end": 2_u32 },
            "from": { "start": 8_u32 },
            "span": { "start": 8_u32, "end": 13_u32 },
            "to": { "end": 13_u32 },
        })
    );
}

#[test]
fn a_range_is_the_object_in_every_position_that_holds_a_value() {
    let object = "{ start: number; end: number }";
    let typescript = Positioned::ts_definition();
    for member in [
        format!("filled: Itinerary<{object}>;"),
        format!("keyed: Partial<Record<string, {object}>>;"),
        format!("listed: Array<{object}>;"),
        format!("maybe: {object} | undefined;"),
        format!("paired: [{object}, string];"),
    ] {
        assert!(typescript.contains(&member), "{member} in: {typescript}");
    }
    let schema = "z.strictObject({ start: z.number().int().min(0).max(4294967295), end: z.number().int().min(0).max(4294967295) })";
    let zod = Positioned::zod_schema();
    for member in [
        format!("filled: Itinerary$SchemaFactory({schema}),"),
        format!("keyed: z.record(z.string(), {schema}),"),
        format!("listed: z.array({schema}),"),
        format!(
            "maybe: z.union([z.null().transform(() => undefined), {schema}, z.undefined()]).prefault(undefined),"
        ),
        format!("paired: z.tuple([{schema}, z.string()]),"),
    ] {
        assert!(zod.contains(&member), "{member} in: {zod}");
    }
    let range = integer_range(&["start", "end"]);
    let document = Positioned::json_schema();
    assert_eq!(
        document["properties"]["filled"]["properties"]["held"],
        range
    );
    assert_eq!(
        document["properties"]["keyed"]["additionalProperties"],
        range
    );
    assert_eq!(document["properties"]["listed"]["items"], range);
    assert_eq!(
        document["properties"]["maybe"],
        json!({ "anyOf": [range, { "type": "null" }] })
    );
    assert_eq!(
        document["properties"]["paired"]["prefixItems"][0],
        integer_range(&["start", "end"])
    );
    let required = document["required"].as_array().unwrap();
    assert!(!required.contains(&json!("maybe")), "{document}");
}

#[test]
fn a_range_of_the_types_own_parameter_holds_the_argument() {
    let typescript = Stride::<u32>::ts_definition();
    assert!(
        typescript.contains("span: { start: Bound; end: Bound };"),
        "{typescript}"
    );
    let zod = Stride::<u32>::zod_schema();
    for written in [
        "span: z.strictObject({ start: bound, end: bound }),",
        "Stride$SchemaFactory(z.number().int().min(0).max(4294967295))",
    ] {
        assert!(zod.contains(written), "{written} in: {zod}");
    }
    assert_eq!(
        Stride::<u32>::json_schema()["properties"]["span"],
        integer_range(&["start", "end"])
    );
    let held = Itinerary::<String>::ts_definition();
    assert!(held.contains("held: Held;"), "{held}");
}

#[test]
fn a_bound_renders_as_its_type_renders_anywhere_else() {
    let typescript = Signposted::ts_definition();
    for member in [
        "marks: { start: Milestone; end: Milestone };",
        "span: { start: Waymark; end: Waymark };",
    ] {
        assert!(typescript.contains(member), "{member} in: {typescript}");
    }
    let zod = Signposted::zod_schema();
    for member in [
        "marks: z.strictObject({ start: Milestone$Schema, end: Milestone$Schema }),",
        "span: z.strictObject({ start: Waymark$Schema, end: Waymark$Schema }),",
    ] {
        assert!(zod.contains(member), "{member} in: {zod}");
    }
    let document = Signposted::json_schema();
    for (field, bound) in [
        ("marks", Milestone::json_schema()),
        ("span", Waymark::json_schema()),
    ] {
        assert_eq!(
            document["properties"][field]["properties"]["start"], bound,
            "{field}"
        );
        assert_eq!(
            document["properties"][field]["properties"]["end"], bound,
            "{field}"
        );
    }
    let frontier = Frontier::ts_definition();
    for member in ["from: { start: Waymark };", "to: { end: Waymark };"] {
        assert!(frontier.contains(member), "{member} in: {frontier}");
    }
}

#[test]
fn the_json_schema_admits_what_serde_writes_and_refuses_what_serde_refuses() {
    for payload in read_payloads() {
        let value: Value = serde_json::from_str(&payload.json).unwrap();
        assert!(
            serde_reads(payload.read_as, &payload.json),
            "{}",
            payload.name
        );
        assert!(
            admits(&document_of(payload.read_as), &value),
            "{}: {}",
            payload.name,
            payload.json
        );
    }
    for payload in refused_payloads() {
        let value: Value = serde_json::from_str(&payload.json).unwrap();
        assert!(
            !serde_reads(payload.read_as, &payload.json),
            "{}",
            payload.name
        );
        let told = if payload.undeclared_key {
            Refusal::UndeclaredKey
        } else {
            Refusal::Written
        };
        assert_eq!(
            refusal(&document_of(payload.read_as), &value),
            Some(told),
            "{}: {}",
            payload.name,
            payload.json
        );
    }
    // A bound the type of a bound holds itself to is the document of that type's own.
    for payload in judged_payloads() {
        let value: Value = serde_json::from_str(&payload.json).unwrap();
        assert_eq!(
            admits(&document_of(payload.read_as), &value),
            serde_reads(payload.read_as, &payload.json),
            "{}: {}",
            payload.name,
            payload.json
        );
    }
}

/// The text `field` is written with in the members of `written`, up to the end of its line.
#[cfg(any(feature = "chrono", feature = "mongodb"))]
fn member_text<'written>(written: &'written str, field: &str) -> &'written str {
    written
        .lines()
        .find_map(|line| line.trim_start().strip_prefix(&format!("{field}: ")))
        .unwrap()
}
