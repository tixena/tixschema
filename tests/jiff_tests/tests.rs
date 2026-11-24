//! Tests of the jiff date and time types on every surface.

/// An epoch-seconds hook, written by hand, to prove `as_number` holds back its injected
/// millisecond hook where a field already reads itself through one of its own.
mod epoch_seconds {
    use jiff::Timestamp;
    use serde::de::Error as _;
    use serde::{Deserialize as _, Deserializer, Serializer};

    pub fn serialize<S>(dt: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(dt.as_second())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = i64::deserialize(deserializer)?;
        Timestamp::from_second(secs)
            .ok()
            .ok_or_else(|| D::Error::custom("bad timestamp"))
    }
}

/// An epoch-milliseconds hook, written by hand, to read or to write a date in one direction alone.
mod epoch_millis {
    use jiff::Timestamp;
    use jiff::fmt::serde::timestamp::millisecond::required as ts_milliseconds;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(dt: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ts_milliseconds::serialize(dt, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        ts_milliseconds::deserialize(deserializer)
    }
}

/// The guide's `Event`, declared as `docs/guide/jiff.md` declares it.
mod guide_event {
    use jiff::{
        Timestamp,
        civil::{Date, DateTime, Time},
    };
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Event {
        pub created_at: Timestamp,
        pub date: Date,
        #[model_schema_prop(as_number)]
        pub epoch_ms: Timestamp,
        pub local_datetime: DateTime,
        pub name: String,
        pub time: Time,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub updated_at: Option<Timestamp>,
    }
}

use jiff::civil::{Date, DateTime, Time, date, time};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ZonedDateTime};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use tixschema::model_schema;

use serde::de::DeserializeOwned;

use super::documents::admits;
use super::untagged_millis_models::{
    MILLIS_ROWS, MillisNamed, MillisNamedDraft, MillisOrFlag, MillisOrFlagDraft, serde_written,
};
use super::untagged_millis_models::{MillisLater, MillisMaybe};

/// The epoch milliseconds serde writes for `2024-01-02T03:04:05.678Z` under `as_number`.
const MILLIS: i64 = 1_704_164_645_678;

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct AllJiffTypes {
    date: Date,
    local_datetime: DateTime,
    local_timestamp: Timestamp,
    time: Time,
    utc_datetime: Timestamp,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct DateList {
    dates: Vec<Date>,
    name: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct DateMap {
    events: HashMap<String, Timestamp>,
    name: String,
}

// An enum-keyed map enumerates its keys, so every member carries the value schema outright
// instead of the single `additionalProperties` the String-keyed `DateMap` above carries.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ScheduleSlot {
    Closing,
    Opening,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct SlotSchedule {
    dates: HashMap<ScheduleSlot, Date>,
    timestamps: HashMap<ScheduleSlot, Timestamp>,
}

// A String-keyed map has no key list to enumerate, so one `additionalProperties` schema stands for
// every member — and it is the value type's own rendering, arrayed and nullable as the value is.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct JiffValueMaps {
    dates: HashMap<String, Date>,
    local_datetimes: HashMap<String, DateTime>,
    optional_dates: HashMap<String, Option<Date>>,
    times: HashMap<String, Time>,
    timestamps: HashMap<String, Timestamp>,
    windows: HashMap<String, Vec<Time>>,
}

// A jiff value keeps the rendering it has in field position however deep the map nesting goes:
// the depth belongs to the map, never to the value type.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NestedJiffMaps {
    dates_by_group: HashMap<String, HashMap<String, Date>>,
    timestamps_by_run: HashMap<String, HashMap<String, Timestamp>>,
    window_batches: HashMap<String, Vec<HashMap<String, Time>>>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct EventWithDate {
    date: Date,
    name: String,
}

/// An enum with dates in its variants. Its content key is named: every variant wraps a scalar,
/// which serde refuses to write beside a bare tag.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum FixedValue {
    Alphanumeric(String),
    Boolean(bool),
    Date(Date),
    DateTime(Timestamp),
    Decimal(f64),
    Integer(i64),
    Time(Time),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct LocalEvent {
    local_datetime: DateTime,
    title: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct LocalTimestamp {
    id: String,
    local_time: Timestamp,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OptionalTimestamp {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<Timestamp>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Schedule {
    start_time: Time,
    task: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct TimestampedRecord {
    created_at: Timestamp,
    id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct Sample {
    #[model_schema_prop(as_number)]
    created_at: Timestamp,
    due_at: Timestamp,
    start_time: Time,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum DynamicValue {
    Native(Timestamp),
    Number(#[model_schema_prop(as_number)] Timestamp),
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct CustomEpochHook {
    #[model_schema_prop(as_number)]
    #[serde(with = "epoch_seconds")]
    stamp: Timestamp,
}

/// A date under `as_number` in an `Option`, which drops its key, and in a `nullable` one.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct HeldMillis {
    #[model_schema_prop(as_number)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    later: Option<Timestamp>,
    #[model_schema_prop(as_number, nullable)]
    settled: Option<Timestamp>,
}

/// A date under `as_number` in a named member of a variant.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
enum MillisEvent {
    Logged {
        #[model_schema_prop(as_number)]
        at: Timestamp,
    },
}

/// A date under `as_number` in a slot of a tuple struct.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct MillisPair(#[model_schema_prop(as_number)] Timestamp, u8);

/// A date under `as_number` in an untagged member read through a hook of its author's own.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
enum SecondsOrFlag {
    Flag(bool),
    Seconds(
        #[model_schema_prop(as_number)]
        #[serde(with = "epoch_seconds")]
        Timestamp,
    ),
}

/// A date under `as_number` in an untagged member that names a check.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
enum CheckedMillis {
    Flag(bool),
    Millis(#[model_schema_prop(as_number, check = ["after_epoch"])] Timestamp),
}

/// A date under `as_number` its author reads, or writes, through a function of their own.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct OneWayMillis {
    #[model_schema_prop(as_number)]
    #[serde(deserialize_with = "epoch_millis::deserialize")]
    read_by_hand: Timestamp,
    #[model_schema_prop(as_number)]
    #[serde(serialize_with = "epoch_millis::serialize")]
    written_by_hand: Timestamp,
}

/// A date under `as_number` in an `Option` that drops its key and names no `default`.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct DroppedMillis {
    #[model_schema_prop(as_number)]
    #[serde(skip_serializing_if = "Option::is_none")]
    later: Option<Timestamp>,
    #[model_schema_prop(as_number, nullable)]
    settled: Option<Timestamp>,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OptionalMillis {
    #[model_schema_prop(as_number)]
    #[serde(skip_serializing_if = "Option::is_none")]
    later: Option<Timestamp>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Appointment {
    at: ZonedDateTime,
    name: String,
}

/// The check [`CheckedMillis`] names: a date after the epoch.
fn after_epoch(value: &Timestamp) -> Result<(), Vec<String>> {
    if value.as_millisecond() > 0 {
        Ok(())
    } else {
        Err(vec!["not after the epoch".to_owned()])
    }
}

#[test]
fn test_jiff_types_constructible() {
    let date = date(2020, 1, 1);
    let time = time(0, 0, 0, 0);
    let naive_dt = date.to_datetime(time);
    let utc_dt = Timestamp::from_second(0).unwrap();
    let local_dt = utc_dt;

    let all = AllJiffTypesDraft {
        date,
        local_datetime: naive_dt,
        local_timestamp: local_dt,
        time,
        utc_datetime: utc_dt,
    };
    assert_eq!(all.date, date);
    let date_list = DateListDraft {
        dates: Vec::new(),
        name: String::new(),
    };
    assert_eq!(date_list.dates, Vec::<Date>::new());
    let date_map = DateMapDraft {
        events: HashMap::new(),
        name: String::new(),
    };
    assert!(date_map.events.is_empty());
    let slot_schedule = SlotScheduleDraft {
        dates: HashMap::from([(ScheduleSlot::Opening, date), (ScheduleSlot::Closing, date)]),
        timestamps: HashMap::new(),
    };
    assert_eq!(slot_schedule.dates.len(), 2);
    assert!(slot_schedule.timestamps.is_empty());
    let jiff_value_maps = JiffValueMapsDraft {
        dates: HashMap::from([("opening".to_owned(), date)]),
        local_datetimes: HashMap::new(),
        optional_dates: HashMap::from([("closing".to_owned(), None)]),
        times: HashMap::new(),
        timestamps: HashMap::new(),
        windows: HashMap::from([("opening".to_owned(), vec![time])]),
    };
    assert_eq!(jiff_value_maps.dates.len(), 1);
    assert_eq!(jiff_value_maps.optional_dates["closing"], None);
    assert_eq!(jiff_value_maps.windows["opening"], vec![time]);
    let nested_jiff_maps = NestedJiffMapsDraft {
        dates_by_group: HashMap::from([(
            "opening".to_owned(),
            HashMap::from([("first".to_owned(), date)]),
        )]),
        timestamps_by_run: HashMap::new(),
        window_batches: HashMap::from([(
            "opening".to_owned(),
            vec![HashMap::from([("first".to_owned(), time)])],
        )]),
    };
    assert_eq!(nested_jiff_maps.dates_by_group["opening"]["first"], date);
    assert_eq!(nested_jiff_maps.window_batches["opening"][0]["first"], time);
    assert!(nested_jiff_maps.timestamps_by_run.is_empty());
    assert!(jiff_value_maps.local_datetimes.is_empty());
    assert!(jiff_value_maps.timestamps.is_empty());
    assert!(jiff_value_maps.times.is_empty());
    let values = [
        FixedValueDraft::Alphanumeric(String::new()),
        FixedValueDraft::Boolean(false),
        FixedValueDraft::Date(date),
        FixedValueDraft::DateTime(local_dt),
        FixedValueDraft::Decimal(0.0),
        FixedValueDraft::Integer(0),
        FixedValueDraft::Time(time),
    ];
    assert_eq!(values.len(), 7);
    let local_event = LocalEventDraft {
        local_datetime: naive_dt,
        title: String::new(),
    };
    assert_eq!(local_event.title, String::new());
    let local_timestamp = LocalTimestampDraft {
        id: String::new(),
        local_time: local_dt,
    };
    assert_eq!(local_timestamp.id, String::new());
    let optional_timestamp = OptionalTimestampDraft {
        id: String::new(),
        updated_at: None,
    };
    assert!(optional_timestamp.updated_at.is_none());
}

#[test]
#[cfg(feature = "typescript")]
fn test_naive_date_typescript() {
    let ts = EventWithDate::ts_definition();
    assert!(
        ts.contains("date: string;"),
        "Date should map to string. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_naive_time_typescript() {
    let ts = Schedule::ts_definition();
    assert!(
        ts.contains("start_time: string;"),
        "Time should map to string. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_naive_datetime_typescript() {
    let ts = LocalEvent::ts_definition();
    assert!(
        ts.contains("local_datetime: string;"),
        "DateTime should map to string. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_datetime_utc_typescript() {
    let ts = TimestampedRecord::ts_definition();
    assert!(
        ts.contains("created_at: Date;"),
        "Timestamp should map to Date. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_datetime_local_typescript() {
    let ts = LocalTimestamp::ts_definition();
    assert!(
        ts.contains("local_time: Date;"),
        "Timestamp should map to Date. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_optional_datetime_typescript() {
    let ts = OptionalTimestamp::ts_definition();
    let expected = "updated_at: Date | undefined;";
    assert!(
        ts.contains(expected),
        "Option<Timestamp> should map to {expected}. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_vec_date_typescript() {
    let ts = DateList::ts_definition();
    assert!(
        ts.contains("dates: Array<string>;"),
        "Vec<Date> should map to Array<string>. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_hashmap_datetime_typescript() {
    let ts = DateMap::ts_definition();
    assert!(
        ts.contains("events: Partial<Record<string, Date>>;"),
        "HashMap<String, Timestamp> should map to Partial<Record<string, Date>>. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_all_jiff_types_typescript() {
    let ts = AllJiffTypes::ts_definition();
    assert!(
        ts.contains("date: string;"),
        "Date should be string. Got: {ts}"
    );
    assert!(
        ts.contains("time: string;"),
        "Time should be string. Got: {ts}"
    );
    assert!(
        ts.contains("local_datetime: string;"),
        "DateTime should be string. Got: {ts}"
    );
    assert!(
        ts.contains("utc_datetime: Date;"),
        "Timestamp should be Date. Got: {ts}"
    );
    assert!(
        ts.contains("local_timestamp: Date;"),
        "Timestamp should be Date. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_naive_date_zod() {
    let zod = EventWithDate::zod_schema();
    assert!(
        zod.contains("date: z.iso.date(),"),
        "Date should use z.iso.date(). Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_naive_time_zod() {
    let zod = Schedule::zod_schema();
    assert!(
        zod.contains("start_time: z.preprocess((arg) =>"),
        "Time should be wrapped in a millis-accepting preprocessor. Got: {zod}"
    );
    assert!(
        zod.contains("}, z.iso.time()),"),
        "Time preprocessor should validate with z.iso.time(). Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_naive_datetime_zod() {
    let zod = LocalEvent::zod_schema();
    assert!(
        zod.contains("local_datetime: z.preprocess((arg) => {")
            && zod.contains(
                "const sent = `${read[1]}T${read[2]}${read[3] ?? \":00\"}`; return \
                 sent.replace(/\\.(\\d*?)0+$/, (_, kept) => kept ? `.${kept}` : \"\"); }, \
                 z.string().regex(/^(?:"
            ),
        "DateTime should write the form jiff writes, then hold the text to it. Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_datetime_utc_zod() {
    let zod = TimestampedRecord::zod_schema();
    assert!(
        zod.contains(
            "created_at: z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()),"
        ),
        "Timestamp should use the date schema. Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_datetime_local_zod() {
    let zod = LocalTimestamp::zod_schema();
    assert!(
        zod.contains(
            "local_time: z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()),"
        ),
        "Timestamp should use the date schema. Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_optional_datetime_zod() {
    let zod = OptionalTimestamp::zod_schema();
    assert!(
        zod.contains(
            "updated_at: z.union([z.null().transform(() => undefined), z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()), z.undefined()]).prefault(undefined),"
        ),
        "Option<Timestamp> should use z.union([z.null().transform(() => undefined), \
         z.coerce.date(), z.undefined()]). Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_vec_date_zod() {
    let zod = DateList::zod_schema();
    assert!(
        zod.contains("dates: z.array(z.iso.date()),"),
        "Vec<Date> should use z.array(z.iso.date()). Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_hashmap_datetime_zod() {
    let zod = DateMap::zod_schema();
    assert!(
        zod.contains("events: z.record(z.string(), z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())),"),
        "HashMap<String, Timestamp> should use z.record(z.string(), <the date schema>). Got: {zod}"
    );
}

#[test]
fn test_naive_date_json_schema() {
    let schema = EventWithDate::json_schema();
    let properties = schema["properties"].as_object().unwrap();
    let date_prop = &properties["date"];
    assert_eq!(date_prop["type"], "string");
    assert_eq!(date_prop["format"], "date");
}

#[test]
fn test_naive_time_json_schema() {
    let schema = Schedule::json_schema();
    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(properties["start_time"], time_schema());
}

#[test]
fn test_naive_datetime_json_schema() {
    let schema = LocalEvent::json_schema();
    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(properties["local_datetime"], date_time_schema());
}

#[test]
fn test_datetime_utc_json_schema() {
    let schema = TimestampedRecord::json_schema();
    let properties = schema["properties"].as_object().unwrap();
    let datetime_prop = &properties["created_at"];
    assert_eq!(datetime_prop["type"], "string");
    assert_eq!(datetime_prop["format"], "date-time");
}

#[test]
fn test_vec_date_json_schema() {
    let schema = DateList::json_schema();
    let properties = schema["properties"].as_object().unwrap();
    let dates_prop = &properties["dates"];
    assert_eq!(dates_prop["type"], "array");
    assert_eq!(dates_prop["items"]["type"], "string");
    assert_eq!(dates_prop["items"]["format"], "date");
}

#[test]
fn test_enum_keyed_jiff_map_json_schema() {
    let schema = SlotSchedule::json_schema();
    let properties = schema["properties"].as_object().unwrap();

    for (field_name, expected_format) in [("dates", "date"), ("timestamps", "date-time")] {
        let field = &properties[field_name];
        assert_eq!(field["type"], "object", "in: {field}");
        assert_eq!(field["additionalProperties"], false, "in: {field}");
        let members = ScheduleSlot::enum_members();
        assert_eq!(
            field["properties"].as_object().unwrap().len(),
            members.len(),
            "in: {field}"
        );
        for member in members {
            assert_eq!(
                field["properties"][&member],
                serde_json::json!({ "type": "string", "format": expected_format }),
                "member {member} in: {field}"
            );
        }
    }
}

#[test]
fn test_string_keyed_jiff_map_json_schema() {
    let schema = JiffValueMaps::json_schema();
    let properties = schema["properties"].as_object().unwrap();

    for (field_name, expected_value_schema) in [
        (
            "dates",
            serde_json::json!({ "type": "string", "format": "date" }),
        ),
        ("local_datetimes", date_time_schema()),
        (
            "optional_dates",
            serde_json::json!({
                "anyOf": [{ "type": "string", "format": "date" }, { "type": "null" }]
            }),
        ),
        ("times", time_schema()),
        (
            "timestamps",
            serde_json::json!({ "type": "string", "format": "date-time" }),
        ),
        (
            "windows",
            serde_json::json!({ "type": "array", "items": time_schema() }),
        ),
    ] {
        let field = &properties[field_name];
        assert_eq!(field["type"], "object", "in: {field}");
        assert_eq!(
            field["additionalProperties"], expected_value_schema,
            "in: {field}"
        );
    }

    let events = &DateMap::json_schema()["properties"]["events"];
    assert_eq!(
        events["additionalProperties"],
        serde_json::json!({ "type": "string", "format": "date-time" }),
        "in: {events}"
    );
}

#[test]
fn test_nested_string_keyed_jiff_map_json_schema() {
    let schema = NestedJiffMaps::json_schema();
    let properties = schema["properties"].as_object().unwrap();

    for (field_name, expected_value_schema) in [
        (
            "dates_by_group",
            serde_json::json!({
                "type": "object",
                "additionalProperties": { "type": "string", "format": "date" }
            }),
        ),
        (
            "timestamps_by_run",
            serde_json::json!({
                "type": "object",
                "additionalProperties": { "type": "string", "format": "date-time" }
            }),
        ),
        (
            "window_batches",
            serde_json::json!({
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": time_schema()
                }
            }),
        ),
    ] {
        let field = &properties[field_name];
        assert_eq!(field["type"], "object", "in: {field}");
        assert_eq!(
            field["additionalProperties"], expected_value_schema,
            "in: {field}"
        );
    }
}

#[test]
#[cfg(all(feature = "typescript", feature = "zod"))]
fn test_nested_string_keyed_jiff_map_typescript_and_zod() {
    let ts_definition = NestedJiffMaps::ts_definition();
    for expected in [
        "dates_by_group: Partial<Record<string, Partial<Record<string, string>>>>;",
        "timestamps_by_run: Partial<Record<string, Partial<Record<string, Date>>>>;",
        "window_batches: Partial<Record<string, Array<Partial<Record<string, string>>>>>;",
    ] {
        assert!(
            ts_definition.contains(expected),
            "missing {expected}, got: {ts_definition}"
        );
    }

    let zod_schema = NestedJiffMaps::zod_schema();
    for expected in [
        "dates_by_group: z.record(z.string(), z.record(z.string(), z.iso.date())),",
        "timestamps_by_run: z.record(z.string(), z.record(z.string(), z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()))),",
    ] {
        assert!(
            zod_schema.contains(expected),
            "missing {expected}, got: {zod_schema}"
        );
    }
}

#[test]
#[cfg(feature = "typescript")]
fn test_enum_with_datetime_typescript() {
    let ts = FixedValue::ts_definition();
    assert!(
        ts.contains("FixedValue"),
        "Should generate FixedValue type. Got: {ts}"
    );
    assert!(
        ts.contains("DateTime"),
        "Should contain DateTime variant. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_enum_with_datetime_zod() {
    let zod = FixedValue::zod_schema();
    assert!(
        zod.contains("FixedValue$Schema"),
        "Should generate FixedValue$Schema. Got: {zod}"
    );
}

#[test]
fn test_jiff_compilation_smoke_test() {
    let event = EventWithDateDraft {
        name: "Test Event".to_owned(),
        date: date(2025, 11, 29),
    };

    let schedule = ScheduleDraft {
        task: "Meeting".to_owned(),
        start_time: time(14, 30, 0, 0),
    };

    let timestamp = TimestampedRecordDraft {
        id: "123".to_owned(),
        created_at: Timestamp::now(),
    };

    assert_eq!(event.name, "Test Event");
    assert_eq!(schedule.task, "Meeting");
    assert_eq!(timestamp.id, "123");
}

#[test]
#[cfg(feature = "typescript")]
fn test_sample_typescript_mixes_date_and_number() {
    let ts = Sample::ts_definition();
    assert!(
        ts.contains("due_at: Date;"),
        "default Timestamp should be Date. Got: {ts}"
    );
    assert!(
        ts.contains("created_at: number;"),
        "as_number Timestamp should be number. Got: {ts}"
    );
    assert!(
        ts.contains("start_time: string;"),
        "Time should stay string. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_sample_zod_mixes_coerce_date_and_inline_number() {
    let zod = Sample::zod_schema();
    assert!(
        zod.contains("due_at: z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()),"),
        "default Timestamp should use the date schema. Got: {zod}"
    );
    assert!(
        zod.contains("created_at: z.preprocess((arg) =>"),
        "as_number Timestamp should use an inline preprocess arrow. Got: {zod}"
    );
    assert!(
        zod.contains("arg instanceof Date) return arg.getTime();"),
        "as_number coercer should map Date -> getTime(). Got: {zod}"
    );
    assert!(
        zod.contains("}, z.number().int()),"),
        "as_number coercer should validate with z.number().int(). Got: {zod}"
    );
    assert!(
        !zod.contains("created_at: z.iso.datetime"),
        "as_number field must not use z.iso.datetime. Got: {zod}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_tuple_variant_datetime_honors_as_number() {
    let ts = DynamicValue::ts_definition();
    assert!(
        ts.contains("Date"),
        "Native variant payload should render Date. Got: {ts}"
    );
    assert!(
        ts.contains("number"),
        "Number variant payload should render number via as_number. Got: {ts}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_tuple_variant_datetime_zod_honors_as_number() {
    let zod = DynamicValue::zod_schema();
    assert!(
        zod.contains("z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())"),
        "Native variant payload should use the date schema. Got: {zod}"
    );
    assert!(
        zod.contains("}, z.number().int())"),
        "Number variant payload should use the inline as_number coercer. Got: {zod}"
    );
}

#[test]
fn test_as_number_types_constructible() {
    let dt = Timestamp::from_second(0).unwrap();
    let sample = SampleDraft {
        created_at: dt,
        due_at: dt,
        start_time: time(0, 0, 0, 0),
    };
    assert_eq!(sample.created_at, dt);
    let values = [DynamicValueDraft::Native(dt), DynamicValueDraft::Number(dt)];
    assert_eq!(values.len(), 2);
}

#[test]
fn an_as_number_field_crosses_the_wire_as_epoch_milliseconds() {
    let sample = SampleDraft {
        created_at: Timestamp::from_millisecond(1_758_000_000_000).unwrap(),
        due_at: Timestamp::from_millisecond(1_758_000_000_000).unwrap(),
        start_time: time(0, 0, 0, 0),
    };
    let json = serde_json::to_string(&sample).unwrap();
    assert!(
        json.contains(r#""created_at":1758000000000"#),
        "as_number must write epoch milliseconds. Got: {json}"
    );
    assert!(
        json.contains(r#""due_at":"2025-09-16T05:20:00Z""#),
        "a DateTime without the flag keeps jiff's string. Got: {json}"
    );
    let back: Sample = serde_json::from_str(&json).unwrap();
    assert_eq!(back, sample.clone().validate().unwrap());
    let variant = serde_json::to_string(&DynamicValueDraft::Number(sample.created_at)).unwrap();
    assert_eq!(variant, r#"{"type":"Number","value":1758000000000}"#);
}

#[test]
fn as_number_holds_back_where_the_author_wrote_their_own_hook() {
    let value = CustomEpochHookDraft {
        stamp: Timestamp::from_second(1_758_000_000).unwrap(),
    };
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(
        json, r#"{"stamp":1758000000}"#,
        "the author's own seconds hook must run, not the injected milliseconds one. Got: {json}"
    );
    let back: CustomEpochHook = serde_json::from_str(&json).unwrap();
    assert_eq!(back, value.validate().unwrap());
}

/// The JSON Schema of a `Time`: the text serde writes, its seconds written, whole. No
/// `format` names it: RFC 3339's `time` holds a zone, which serde refuses.
fn time_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "pattern": r"^(?:(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\.[0-9]*[1-9])?)$"
    })
}

/// The JSON Schema of a `DateTime`: the text serde writes, whole. No `format` names it:
/// RFC 3339's `date-time` holds a zone, which serde refuses.
fn date_time_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "pattern": concat!(
            r"^(?:(?:(?:[0-9][0-9][2468][048]|[0-9][0-9][13579][26]|[0-9][0-9]0[48]|",
            r"[02468][048]00|[13579][26]00)-02-29|[0-9]{4}-(?:(?:0[13578]|1[02])-",
            r"(?:0[1-9]|[12][0-9]|3[01])|(?:0[469]|11)-(?:0[1-9]|[12][0-9]|30)|",
            r"(?:02)-(?:0[1-9]|1[0-9]|2[0-8])))T(?:[01][0-9]|2[0-3]):[0-5][0-9]:",
            r"[0-5][0-9](?:\.[0-9]*[1-9])?)$"
        )
    })
}

/// The JSON Schema of `T` admits what serde writes of `value`, a date under `as_number` among it,
/// and refuses the payload with that date's text in place of its number, which serde refuses.
fn held_to_the_millis_serde_writes<T>(value: &T, document: &serde_json::Value)
where
    T: Serialize + DeserializeOwned,
{
    let written = serde_json::to_string(value).unwrap();
    assert!(
        written.contains(&MILLIS.to_string()),
        "serde writes no epoch milliseconds in {written}"
    );
    let as_text = written.replace(&MILLIS.to_string(), r#""2024-01-02T03:04:05.678Z""#);
    assert!(
        serde_json::from_str::<T>(&as_text).is_err(),
        "serde reads the date's text in {as_text}"
    );
    assert!(
        admits(document, &serde_json::from_str(&written).unwrap()),
        "the JSON Schema refuses {written}, which serde writes: {document}"
    );
    assert!(
        !admits(document, &serde_json::from_str(&as_text).unwrap()),
        "the JSON Schema admits {as_text}, which serde refuses: {document}"
    );
}

#[test]
fn a_date_under_as_number_is_described_as_the_integer_serde_writes() {
    let at = Timestamp::from_millisecond(MILLIS).unwrap();
    let start_time = time(0, 0, 0, 0);
    held_to_the_millis_serde_writes(
        &SampleDraft {
            created_at: at,
            due_at: at,
            start_time,
        },
        &Sample::json_schema(),
    );
    held_to_the_millis_serde_writes(&DynamicValueDraft::Number(at), &DynamicValue::json_schema());
    held_to_the_millis_serde_writes(
        &HeldMillisDraft {
            later: Some(at),
            settled: Some(at),
        },
        &HeldMillis::json_schema(),
    );
    held_to_the_millis_serde_writes(
        &MillisEventDraft::Logged { at },
        &MillisEvent::json_schema(),
    );
    held_to_the_millis_serde_writes(&MillisPairDraft(at, 1), &MillisPair::json_schema());

    let absent = serde_json::to_value(HeldMillisDraft {
        later: None,
        settled: None,
    })
    .unwrap();
    assert!(
        admits(&HeldMillis::json_schema(), &absent),
        "the JSON Schema refuses {absent}, which serde writes"
    );
}

#[test]
fn a_date_with_no_as_number_keeps_its_date_time_format() {
    let properties = &Sample::json_schema()["properties"];
    assert_eq!(
        properties["created_at"],
        serde_json::json!({ "type": "integer" }),
        "in: {properties}"
    );
    assert_eq!(
        properties["due_at"],
        serde_json::json!({ "type": "string", "format": "date-time" }),
        "in: {properties}"
    );
}

#[test]
fn the_json_schema_of_each_jiff_type_admits_what_serde_writes_and_refuses_a_zone_it_refuses() {
    let date = date(2024, 1, 5);
    let time = time(10, 20, 30, 125 * 1_000_000);
    let utc = date
        .to_datetime(time)
        .to_zoned_datetime(TimeZone::UTC)
        .unwrap()
        .timestamp();
    let written = serde_json::to_value(AllJiffTypesDraft {
        date,
        local_datetime: date.to_datetime(time),
        local_timestamp: utc,
        time,
        utc_datetime: utc,
    })
    .unwrap();
    let document = AllJiffTypes::json_schema();
    assert!(
        admits(&document, &written),
        "the JSON Schema refuses {written}, which serde writes: {document}"
    );
    for (member, zoned) in [
        ("time", "10:20:30Z"),
        ("local_datetime", "2024-01-05T10:20:30Z"),
        ("local_datetime", "2024-01-05T10:20:30.125Z"),
    ] {
        let mut payload = written.clone();
        payload[member] = serde_json::Value::String(zoned.to_owned());
        assert!(
            serde_json::from_value::<AllJiffTypes>(payload.clone()).is_err(),
            "serde reads {payload}"
        );
        assert!(
            !admits(&document, &payload),
            "the JSON Schema admits {payload}, which serde refuses: {document}"
        );
    }

    let tagged = FixedValue::json_schema();
    let slot = serde_json::to_value(FixedValueDraft::Time(time)).unwrap();
    assert!(
        admits(&tagged, &slot),
        "the JSON Schema refuses {slot}, which serde writes: {tagged}"
    );
    let zoned_slot = serde_json::json!({ "type": "Time", "value": "10:20:30Z" });
    assert!(
        serde_json::from_value::<FixedValue>(zoned_slot.clone()).is_err(),
        "serde reads {zoned_slot}"
    );
    assert!(
        !admits(&tagged, &zoned_slot),
        "the JSON Schema admits {zoned_slot}, which serde refuses: {tagged}"
    );
}

/// serde writes and reads a date under `as_number` in an untagged enum's member as the integer of
/// its epoch milliseconds, and refuses its text, as [`MILLIS_ROWS`] states.
#[test]
fn an_untagged_member_under_as_number_crosses_the_wire_as_epoch_milliseconds() {
    let at = Timestamp::from_millisecond(1_704_164_645_678).unwrap();
    assert_eq!(
        serde_json::to_string(&MillisOrFlagDraft::Millis(at)).unwrap(),
        "1704164645678"
    );
    assert_eq!(
        serde_json::to_string(&MillisNamedDraft::Logged { at }).unwrap(),
        r#"{"at":1704164645678}"#
    );
    for (read_as, json, written, _) in MILLIS_ROWS {
        assert_eq!(
            serde_written(read_as, json).as_deref(),
            written,
            "serde answers {json} read as {read_as} otherwise"
        );
    }
}

/// The JSON Schema of each enum of [`MILLIS_ROWS`] admits each payload serde reads, and refuses
/// each one it refuses.
#[test]
fn the_json_schema_of_an_untagged_member_under_as_number_admits_what_serde_reads() {
    for (read_as, json, written, _) in MILLIS_ROWS {
        let document = match read_as {
            "MillisOrFlag" => MillisOrFlag::json_schema(),
            "MillisNamed" => MillisNamed::json_schema(),
            "MillisMaybe" => MillisMaybe::json_schema(),
            _ => MillisLater::json_schema(),
        };
        let payload: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(
            admits(&document, &payload),
            written.is_some(),
            "the JSON Schema of {read_as} answers {json} otherwise than serde: {document}"
        );
    }
}

/// TypeScript and Zod describe the number serde writes for an untagged member under `as_number`.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn an_untagged_member_under_as_number_is_described_as_a_number() {
    let flag = MillisOrFlag::ts_definition();
    assert!(
        flag.contains("export type MillisOrFlagDraft = boolean | number;"),
        "Got: {flag}"
    );
    let named = MillisNamed::ts_definition();
    assert!(
        named.contains("export type MillisNamedDraft = boolean | { at: number };"),
        "Got: {named}"
    );
    let zod = MillisOrFlag::zod_schema();
    assert!(
        zod.contains(
            "z.union([z.boolean(), z.preprocess((arg) => { if (arg instanceof Date) return \
             arg.getTime(); if (typeof arg === \"string\") return Date.parse(arg); return arg; }, \
             z.number().int())])"
        ),
        "Got: {zod}"
    );
}

/// An untagged member under `as_number` read through a hook of its author's keeps it, and one
/// that names a check is read as the epoch milliseconds and held to the check.
#[test]
fn an_untagged_member_under_as_number_keeps_its_own_hook_and_its_check() {
    let at = Timestamp::from_second(1_758_000_000).unwrap();
    let written = serde_json::to_string(&SecondsOrFlagDraft::Seconds(at)).unwrap();
    assert_eq!(
        written, "1758000000",
        "the author's own seconds hook must run, not the injected milliseconds one"
    );
    assert_eq!(
        serde_json::from_str::<SecondsOrFlag>(&written).unwrap(),
        SecondsOrFlagDraft::Seconds(at).into()
    );

    let checked =
        CheckedMillisDraft::Millis(Timestamp::from_millisecond(1_758_000_000_000).unwrap());
    let millis = serde_json::to_string(&checked).unwrap();
    assert_eq!(millis, "1758000000000");
    assert_eq!(
        serde_json::from_str::<CheckedMillis>(&millis).unwrap(),
        checked.validate().unwrap()
    );
    let refused = serde_json::from_str::<CheckedMillis>("-5")
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("not after the epoch"),
        "the read does not refuse the date the check refuses: {refused}"
    );
}

/// A date under `as_number` its author reads, or writes, through a function of their own is
/// written and read as the epoch milliseconds in the other direction, through the hook the key
/// hangs there.
#[test]
fn a_date_under_as_number_with_a_hook_of_its_own_in_one_direction_is_written_as_a_number() {
    let at = Timestamp::from_millisecond(1_758_000_000_000).unwrap();
    let value = OneWayMillisDraft {
        read_by_hand: at,
        written_by_hand: at,
    };
    let written = serde_json::to_string(&value).unwrap();
    assert_eq!(
        written,
        r#"{"read_by_hand":1758000000000,"written_by_hand":1758000000000}"#
    );
    assert_eq!(
        serde_json::from_str::<OneWayMillis>(&written).unwrap(),
        value.validate().unwrap()
    );
}

/// An `Option` of a date under `as_number` serde drops the key of is read back from its absence,
/// as an `Option` read by no hook is.
#[test]
fn an_absent_key_of_an_optional_date_under_as_number_is_read_as_none() {
    let empty = DroppedMillisDraft {
        later: None,
        settled: None,
    };
    let written = serde_json::to_string(&empty).unwrap();
    assert_eq!(written, r#"{"settled":null}"#);
    assert_eq!(
        serde_json::from_str::<DroppedMillis>(&written).unwrap(),
        empty.clone().validate().unwrap()
    );
    assert_eq!(
        serde_json::from_str::<DroppedMillis>("{}").unwrap(),
        empty.validate().unwrap(),
        "an absent key of the nullable date is not read as `None`"
    );
    let at = Timestamp::from_millisecond(1_758_000_000_000).unwrap();
    assert_eq!(
        serde_json::from_str::<DroppedMillis>(r#"{"later":1758000000000}"#).unwrap(),
        DroppedMillisDraft {
            later: Some(at),
            settled: None,
        }
        .into()
    );
}

/// serde refuses a date read through jiff's millisecond hook beside a tag alone and flattened as
/// the integer it writes, in the words the README quotes for a date under `as_number`.
#[test]
fn serde_refuses_a_date_in_milliseconds_beside_a_tag_and_flattened_as_an_integer() {
    #[derive(Serialize)]
    #[serde(tag = "kind")]
    enum Stamping {
        Stamp(#[serde(with = "jiff::fmt::serde::timestamp::millisecond::required")] Timestamp),
    }
    #[derive(Serialize)]
    struct FlatStamp {
        id: u8,
        #[serde(flatten, with = "jiff::fmt::serde::timestamp::millisecond::required")]
        stamp: Timestamp,
    }
    let at = Timestamp::from_millisecond(1_704_164_645_678).unwrap();
    let readme = include_str!("../../docs/guide/jiff.md");
    for (refused, said) in [
        (
            serde_json::to_string(&Stamping::Stamp(at)),
            "cannot serialize tagged newtype variant Stamping::Stamp containing an integer",
        ),
        (
            serde_json::to_string(&FlatStamp { id: 1, stamp: at }),
            "can only flatten structs and maps (got an integer)",
        ),
    ] {
        assert_eq!(refused.unwrap_err().to_string(), said);
        assert!(
            readme.contains(&format!("`{said}`")),
            "the README no longer quotes `{said}`"
        );
    }
}

#[cfg(feature = "typescript")]
#[test]
fn the_guides_event_publishes_the_draft_the_guide_shows() {
    let guide = include_str!("../../docs/guide/jiff.md");
    let declared = guide_event::Event::ts_definition();
    let shown = "export type EventDraft = {\n  created_at: Date;\n  date: string;\n  epoch_ms: \
                 number;\n  local_datetime: string;\n  name: string;\n  time: string;\n  \
                 updated_at: Date | undefined;\n};";
    assert!(guide.contains(shown), "the guide no longer shows: {shown}");
    for member in shown.lines() {
        assert!(
            declared.lines().any(|line| line == member),
            "the generator no longer writes {member}. Got: {declared}"
        );
    }
}

#[test]
fn the_guides_event_writes_the_fraction_and_the_milliseconds_jiff_writes() {
    let event = guide_event::EventDraft {
        name: "launch".to_owned(),
        date: date(2026, 11, 20),
        time: time(8, 0, 0, 0),
        local_datetime: date(2026, 11, 20).at(8, 0, 0, 0),
        created_at: "2026-11-20T12:00:00.5Z".parse().unwrap(),
        epoch_ms: "2026-11-20T12:00:00.123Z".parse().unwrap(),
        updated_at: None,
    }
    .validate()
    .unwrap();
    let written = serde_json::to_string(&event).unwrap();
    assert_eq!(
        written,
        r#"{"created_at":"2026-11-20T12:00:00.5Z","date":"2026-11-20","epoch_ms":1795176000123,"local_datetime":"2026-11-20T08:00:00","name":"launch","time":"08:00:00"}"#
    );
    let guide = include_str!("../../docs/guide/jiff.md");
    for shown in [
        r#"`"created_at":"2026-11-20T12:00:00.5Z"`"#,
        r#"`"epoch_ms":1795176000123`"#,
    ] {
        assert!(guide.contains(shown), "the guide no longer shows {shown}");
    }
}

#[test]
fn an_optional_timestamp_under_as_number_reads_an_absent_key_as_none() {
    let read: OptionalMillis = serde_json::from_str("{}").unwrap();
    assert_eq!(read.later, None);
    let at = Timestamp::from_millisecond(1_795_176_000_123).unwrap();
    let written = serde_json::to_string(&OptionalMillisDraft { later: Some(at) }).unwrap();
    assert_eq!(written, r#"{"later":1795176000123}"#);
}

/// The texts chrono writes of an instant, a time and the epoch milliseconds, read by the jiff types
/// as the value chrono read.
#[test]
fn the_texts_chrono_wrote_read_as_the_values_chrono_read() {
    #[derive(Deserialize)]
    struct Stored {
        at: Timestamp,
        clock: Time,
        #[serde(with = "jiff::fmt::serde::timestamp::millisecond::required")]
        epoch: Timestamp,
        stamp: DateTime,
    }
    let read: Stored = serde_json::from_str(
        r#"{"at":"2024-01-05T10:20:30.500Z","clock":"10:20:30.500","epoch":1795176000123,"stamp":"2024-01-05T10:20:30.120"}"#,
    )
    .unwrap();
    assert_eq!(read.at, "2024-01-05T10:20:30.5Z".parse().unwrap());
    assert_eq!(read.clock, time(10, 20, 30, 500_000_000));
    assert_eq!(read.epoch.as_millisecond(), 1_795_176_000_123);
    assert_eq!(read.stamp, date(2024, 1, 5).at(10, 20, 30, 120_000_000));
}

/// A zoned date and time crosses as the one text jiff writes, zone named, read back and written
/// again byte for byte; the JSON Schema admits it and refuses one with no zone, which serde
/// refuses.
#[test]
fn a_zoned_date_and_time_goes_round_as_the_text_jiff_writes() {
    let text = r#"{"at":"2026-11-20T08:00:00-04:00[America/Santo_Domingo]","name":"visit"}"#;
    let read: Appointment = serde_json::from_str(text).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), text);
    let document = Appointment::json_schema();
    assert!(admits(
        &document,
        &serde_json::from_str::<serde_json::Value>(text).unwrap()
    ));
    let unzoned = serde_json::json!({ "at": "2026-11-20T08:00:00-04:00", "name": "visit" });
    serde_json::from_value::<Appointment>(unzoned.clone()).unwrap_err();
    assert!(!admits(&document, &unzoned));
}

#[cfg(feature = "typescript")]
#[test]
fn a_zoned_date_and_time_is_a_string_in_typescript() {
    assert!(
        Appointment::ts_definition().contains("  at: string;"),
        "got: {}",
        Appointment::ts_definition()
    );
}
