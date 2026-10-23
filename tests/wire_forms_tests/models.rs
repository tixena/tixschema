//! The models of [`WIRE_FORMS`]: one per type the wire holds as a text, a number or a `bool`, the
//! type held by `value`, among `values` and as a key of `keyed` wherever it keys a map. A row hands
//! one value to a model, in every place the model has, and states what serde answers of it and
//! what each client sends: the payload, or nothing, where the client refuses the value before it
//! sends it. Every payload a client sends is read by serde, by the JSON Schema and by the Zod
//! schema of its model.
//!
//! `wire_forms_tests.rs` holds the serde and JSON Schema columns,
//! `generic_types_tests/declarations.rs` sends each row through the Zod schema under `node`, and
//! `service_schema_emitted_client_tests/run_{dart,swift,kotlin}_wire_forms.rs` through each client.

use alloc::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
#[cfg(feature = "mongodb")]
use mongodb::bson::oid::ObjectId;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each value a caller hands a client, and what serde and each client answer of it. Every row a
/// client sends is in the one form every target reads: a value in another form of the same value
/// is sent in that form, and one no form serde reads holds is refused.
pub const WIRE_FORMS: &[WireFormRow] = &[
    row("ConveyedFlag", "true", "read", ALL_HANDED),
    row("ConveyedLetter", r#""a""#, "read", ALL_HANDED),
    row("ConveyedLetter", "\"\u{1f600}\"", "read", ALL_HANDED),
    row(
        "ConveyedLetter",
        r#""ab""#,
        r#"invalid value: string "ab", expected a character at line 1 column 14"#,
        ALL_REFUSED,
    ),
    row(
        "ConveyedLetter",
        r#""""#,
        r#"invalid value: string "", expected a character at line 1 column 12"#,
        ALL_REFUSED,
    ),
    row("ConveyedText", r#""any text""#, "read", ALL_HANDED),
    row("ConveyedU8", "255", "read", ALL_HANDED),
    row(
        "ConveyedU8",
        "256",
        "invalid value: integer `256`, expected u8 at line 1 column 14",
        NARROW_REFUSED,
    ),
    row(
        "ConveyedU8",
        "-1",
        "invalid value: integer `-1`, expected u8 at line 1 column 13",
        NARROW_REFUSED,
    ),
    row("ConveyedU16", "65535", "read", ALL_HANDED),
    row(
        "ConveyedU16",
        "65536",
        "invalid value: integer `65536`, expected u16 at line 1 column 16",
        NARROW_REFUSED,
    ),
    row("ConveyedU32", "4294967295", "read", ALL_HANDED),
    row(
        "ConveyedU32",
        "4294967296",
        "invalid value: integer `4294967296`, expected u32 at line 1 column 21",
        NARROW_REFUSED,
    ),
    row("ConveyedI8", "-128", "read", ALL_HANDED),
    row(
        "ConveyedI8",
        "128",
        "invalid value: integer `128`, expected i8 at line 1 column 14",
        NARROW_REFUSED,
    ),
    row("ConveyedI16", "-32768", "read", ALL_HANDED),
    row(
        "ConveyedI16",
        "32768",
        "invalid value: integer `32768`, expected i16 at line 1 column 16",
        NARROW_REFUSED,
    ),
    row("ConveyedI32", "-2147483648", "read", ALL_HANDED),
    row(
        "ConveyedI32",
        "3000000000",
        "invalid value: integer `3000000000`, expected i32 at line 1 column 21",
        NARROW_REFUSED,
    ),
    row("ConveyedI64", "-9007199254740991", "read", ALL_HANDED),
    // TypeScript holds a map's key of an `i64` as a `number`, which does not hold this one.
    row(
        "ConveyedI64",
        "1152921504606846976",
        "read",
        Clients {
            typescript: Sends::Unheld,
            ..ALL_HANDED
        },
    ),
    row("ConveyedIsize", "9007199254740991", "read", ALL_HANDED),
    row(
        "ConveyedIsize",
        "-1152921504606846976",
        "read",
        Clients {
            typescript: Sends::Unheld,
            ..ALL_HANDED
        },
    ),
    row("ConveyedU64", "9007199254740991", "read", WIDE_HANDED),
    row(
        "ConveyedU64",
        "-1",
        "invalid value: integer `-1`, expected u64 at line 1 column 11",
        Clients {
            typescript: Sends::Refused,
            ..WIDE_HANDED
        },
    ),
    row("ConveyedU64", "1152921504606846976", "read", WIDE_HANDED),
    row("ConveyedU64", "18446744073709551615", "read", WIDE_HANDED),
    row("ConveyedUsize", "9007199254740991", "read", WIDE_HANDED),
    row(
        "ConveyedUsize",
        "-1",
        "invalid value: integer `-1`, expected usize at line 1 column 11",
        Clients {
            typescript: Sends::Refused,
            ..WIDE_HANDED
        },
    ),
    row("ConveyedF32", "0.5", "read", ALL_HANDED),
    row(
        "ConveyedF32",
        "1e300",
        r#"read, written back as {"value":null,"values":[null]}"#,
        NARROW_REFUSED,
    ),
    row("ConveyedF64", "0.5", "read", ALL_HANDED),
    row(
        "ConveyedF64",
        "1e300",
        r#"read, written back as {"value":1e+300,"values":[1e+300]}"#,
        Clients {
            dart: Sends::As("1e+300"),
            kotlin: Sends::As("1.0E300"),
            swift: Sends::As("1e+300"),
            typescript: Sends::As("1e+300"),
        },
    ),
    row(
        "ConveyedF64",
        "NaN",
        r#"written as {"value":null,"values":[null]}, which serde refuses: invalid type: null, expected f64 at line 1 column 13"#,
        ALL_REFUSED,
    ),
    row("ConveyedLiteralCount", "214", "read", ALL_HANDED),
    row("ConveyedLiteralFlag", "true", "read", ALL_HANDED),
    row("ConveyedLiteralText", r#""val""#, "read", ALL_HANDED),
    row("ConveyedDay", r#""2024-01-05""#, "read", ALL_HANDED),
    row(
        "ConveyedDay",
        r#""2024-1-5""#,
        r#"read, written back as {"keyed":{"2024-01-05":true},"value":"2024-01-05","values":["2024-01-05"]}"#,
        ALL_REFUSED,
    ),
    row(
        "ConveyedDay",
        r#""2023-02-29""#,
        "input is out of range at line 1 column 22",
        ALL_REFUSED,
    ),
    row(
        "ConveyedDay",
        r#""hello""#,
        "input contains invalid characters at line 1 column 17",
        ALL_REFUSED,
    ),
    row("ConveyedHour", r#""10:20:30""#, "read", ALL_HANDED),
    row("ConveyedHour", r#""10:20:30.123""#, "read", ALL_HANDED),
    row(
        "ConveyedHour",
        r#""10:20""#,
        r#"read, written back as {"keyed":{"10:20:00":true},"value":"10:20:00","values":["10:20:00"]}"#,
        Clients {
            dart: Sends::As(r#""10:20:00""#),
            kotlin: Sends::As(r#""10:20:00""#),
            swift: Sends::As(r#""10:20:00""#),
            typescript: Sends::As(r#""10:20:00""#),
        },
    ),
    row(
        "ConveyedHour",
        "37230000",
        "input is out of range at line 1 column 20",
        Clients {
            dart: Sends::Unheld,
            kotlin: Sends::Unheld,
            swift: Sends::Unheld,
            typescript: Sends::As(r#""10:20:30""#),
        },
    ),
    row(
        "ConveyedHour",
        r#""hello""#,
        "input contains invalid characters at line 1 column 17",
        ALL_REFUSED,
    ),
    row(
        "ConveyedStamp",
        r#""2024-01-05T10:20:30""#,
        "read",
        ALL_HANDED,
    ),
    row(
        "ConveyedStamp",
        r#""2024-01-05T10:20:30.123""#,
        "read",
        ALL_HANDED,
    ),
    row(
        "ConveyedStamp",
        r#""2024-01-05T10:20:30.000Z""#,
        "trailing input at line 1 column 36",
        Clients {
            dart: Sends::As(r#""2024-01-05T10:20:30.000""#),
            kotlin: Sends::As(r#""2024-01-05T10:20:30.000""#),
            swift: Sends::As(r#""2024-01-05T10:20:30.000""#),
            typescript: Sends::As(r#""2024-01-05T10:20:30.000""#),
        },
    ),
    row(
        "ConveyedStamp",
        r#""2024-01-05T10:20""#,
        "premature end of input at line 1 column 28",
        Clients {
            dart: Sends::As(r#""2024-01-05T10:20:00""#),
            kotlin: Sends::As(r#""2024-01-05T10:20:00""#),
            swift: Sends::As(r#""2024-01-05T10:20:00""#),
            typescript: Sends::As(r#""2024-01-05T10:20:00""#),
        },
    ),
    row(
        "ConveyedStamp",
        r#""2024-01-05T10:20Z""#,
        "input contains invalid characters at line 1 column 29",
        Clients {
            dart: Sends::As(r#""2024-01-05T10:20:00""#),
            kotlin: Sends::As(r#""2024-01-05T10:20:00""#),
            swift: Sends::As(r#""2024-01-05T10:20:00""#),
            typescript: Sends::As(r#""2024-01-05T10:20:00""#),
        },
    ),
    row(
        "ConveyedStamp",
        r#""2024-01-05 10:20:30""#,
        "input contains invalid characters at line 1 column 31",
        ALL_REFUSED,
    ),
    row(
        "ConveyedStamp",
        r#""hello""#,
        "input contains invalid characters at line 1 column 17",
        ALL_REFUSED,
    ),
    row(
        "ConveyedInstant",
        r#""2024-01-05T10:20:30.000Z""#,
        r#"read, written back as {"keyed":{"2024-01-05T10:20:30Z":true},"value":"2024-01-05T10:20:30Z","values":["2024-01-05T10:20:30Z"]}"#,
        ALL_HANDED,
    ),
    row(
        "ConveyedInstant",
        r#""2024-01-05T10:20:30Z""#,
        "read",
        Clients {
            dart: Sends::As(r#""2024-01-05T10:20:30.000Z""#),
            kotlin: Sends::Handed,
            swift: Sends::As(r#""2024-01-05T10:20:30.000Z""#),
            typescript: Sends::Apart(r#""2024-01-05T10:20:30.000Z""#, r#""2024-01-05T10:20:30Z""#),
        },
    ),
    row(
        "ConveyedInstant",
        r#""2024-01-05T10:20:30+02:00""#,
        r#"read, written back as {"keyed":{"2024-01-05T08:20:30Z":true},"value":"2024-01-05T08:20:30Z","values":["2024-01-05T08:20:30Z"]}"#,
        Clients {
            dart: Sends::As(r#""2024-01-05T08:20:30.000Z""#),
            kotlin: Sends::Handed,
            swift: Sends::As(r#""2024-01-05T08:20:30.000Z""#),
            typescript: Sends::Apart(
                r#""2024-01-05T08:20:30.000Z""#,
                r#""2024-01-05T10:20:30+02:00""#,
            ),
        },
    ),
    row(
        "ConveyedInstant",
        r#""2026-10-22T13:07:11.032917""#,
        "premature end of input at line 1 column 38",
        Clients {
            dart: Sends::As(r#""2026-10-22T13:07:11.032917Z""#),
            kotlin: Sends::Refused,
            swift: Sends::Unheld,
            typescript: Sends::Refused,
        },
    ),
    row(
        "ConveyedInstant",
        r#""2024-01-05T10:20Z""#,
        "input contains invalid characters at line 1 column 29",
        Clients {
            dart: Sends::As(r#""2024-01-05T10:20:00.000Z""#),
            kotlin: Sends::As(r#""2024-01-05T10:20:00Z""#),
            swift: Sends::Unheld,
            typescript: Sends::Refused,
        },
    ),
    row(
        "ConveyedInstant",
        r#""hello""#,
        "input contains invalid characters at line 1 column 17",
        Clients {
            dart: Sends::Unheld,
            kotlin: Sends::Refused,
            swift: Sends::Unheld,
            typescript: Sends::Refused,
        },
    ),
    row("ConveyedMillis", "1704164645678", "read", ALL_HANDED),
    row(
        "ConveyedMillis",
        "1.5",
        "invalid type: floating point `1.5`, expected a unix timestamp in milliseconds at line 1 \
         column 12",
        Clients {
            dart: Sends::Unheld,
            kotlin: Sends::Unheld,
            swift: Sends::Unheld,
            typescript: Sends::Refused,
        },
    ),
    row("ConveyedSlotted", r#""Daily""#, "read", ALL_HANDED),
    #[cfg(feature = "mongodb")]
    row(
        "ConveyedObjectId",
        r#"{"$oid":"0123456789abcdef01234567"}"#,
        "read",
        CONSUMERS_HANDED,
    ),
    #[cfg(feature = "mongodb")]
    row(
        "ConveyedObjectId",
        r#"{"$oid":"0123456789ABCDEF01234567"}"#,
        r#"read, written back as {"value":{"$oid":"0123456789abcdef01234567"},"values":[{"$oid":"0123456789abcdef01234567"}]}"#,
        Clients {
            typescript: Sends::Refused,
            ..CONSUMERS_HANDED
        },
    ),
];

/// Every client sends the value as it was handed.
const ALL_HANDED: Clients = Clients {
    dart: Sends::Handed,
    kotlin: Sends::Handed,
    swift: Sends::Handed,
    typescript: Sends::Handed,
};

/// Every client refuses the value before it sends it.
const ALL_REFUSED: Clients = Clients {
    dart: Sends::Refused,
    kotlin: Sends::Refused,
    swift: Sends::Refused,
    typescript: Sends::Refused,
};

/// TypeScript and Dart refuse the value, and the narrower types of Swift and Kotlin hold none.
const NARROW_REFUSED: Clients = Clients {
    dart: Sends::Refused,
    kotlin: Sends::Unheld,
    swift: Sends::Unheld,
    typescript: Sends::Refused,
};

/// TypeScript sends the value, and Dart, Swift and Kotlin refuse the type at expansion.
const WIDE_HANDED: Clients = Clients {
    dart: Sends::Unemitted,
    kotlin: Sends::Unemitted,
    swift: Sends::Unemitted,
    typescript: Sends::Handed,
};

/// TypeScript sends the value, and Dart, Swift and Kotlin hold it in the consumer's own type.
#[cfg(feature = "mongodb")]
const CONSUMERS_HANDED: Clients = Clients {
    dart: Sends::Unemitted,
    kotlin: Sends::Unemitted,
    swift: Sends::Unemitted,
    typescript: Sends::Handed,
};

/// What a client does with the value a row hands it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sends {
    /// Sends the first JSON value as `value` and among `values`, and the second as the key.
    Apart(&'static str, &'static str),
    /// Sends this JSON value in every place.
    As(&'static str),
    /// Sends it as it was handed, in every place.
    Handed,
    /// Refuses it before it sends it.
    Refused,
    /// Has no type for it: the client refuses the Rust type at expansion, or the type is the
    /// consumer's own.
    Unemitted,
    /// Holds no such value: its type cannot be written with it.
    Unheld,
}

/// The places a model holds its value in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Places {
    /// `keyed`, `value` and `values`.
    Keyed,
    /// `value` and `values`: the type keys no map.
    Listed,
    /// `value` alone.
    Lone,
}

/// One value a caller hands each client, held by the model `held_by`, and what each target
/// answers of it.
pub struct WireFormRow {
    pub dart: Sends,
    /// The value, as JSON. `NaN` is the float no JSON text writes.
    pub handed: &'static str,
    pub held_by: &'static str,
    pub kotlin: Sends,
    /// What serde answers of the model holding `handed`: `read` where it writes the payload back
    /// as it was, what it writes back otherwise, or its refusal.
    pub serde: &'static str,
    pub swift: Sends,
    pub typescript: Sends,
}

impl WireFormRow {
    /// The payload a client that `sends` sends of the row, and `None` where it sends none.
    pub fn payload(&self, sends: Sends) -> Option<String> {
        let places = places(self.held_by);
        match sends {
            Sends::Handed => Some(payload(places, self.handed, self.handed)),
            Sends::As(value) => Some(payload(places, value, value)),
            Sends::Apart(value, key) => Some(payload(places, value, key)),
            Sends::Refused | Sends::Unheld | Sends::Unemitted => None,
        }
    }

    /// Every payload a client sends of the row.
    pub fn sent(&self) -> Vec<String> {
        [self.typescript, self.dart, self.swift, self.kotlin]
            .into_iter()
            .filter_map(|sends| self.payload(sends))
            .collect()
    }
}

/// What each client sends of one row.
#[derive(Clone, Copy)]
struct Clients {
    dart: Sends,
    kotlin: Sends,
    swift: Sends,
    typescript: Sends,
}

/// A `bool`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedFlag {
    pub keyed: BTreeMap<bool, bool>,
    pub value: bool,
    pub values: Vec<bool>,
}

/// A `char`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedLetter {
    pub keyed: BTreeMap<char, bool>,
    pub value: char,
    pub values: Vec<char>,
}

/// A `String`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedText {
    pub keyed: BTreeMap<String, bool>,
    pub value: String,
    pub values: Vec<String>,
}

/// A `u8`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedU8 {
    pub keyed: BTreeMap<u8, bool>,
    pub value: u8,
    pub values: Vec<u8>,
}

/// A `u16`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedU16 {
    pub keyed: BTreeMap<u16, bool>,
    pub value: u16,
    pub values: Vec<u16>,
}

/// A `u32`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedU32 {
    pub keyed: BTreeMap<u32, bool>,
    pub value: u32,
    pub values: Vec<u32>,
}

/// An `i8`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedI8 {
    pub keyed: BTreeMap<i8, bool>,
    pub value: i8,
    pub values: Vec<i8>,
}

/// An `i16`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedI16 {
    pub keyed: BTreeMap<i16, bool>,
    pub value: i16,
    pub values: Vec<i16>,
}

/// An `i32`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedI32 {
    pub keyed: BTreeMap<i32, bool>,
    pub value: i32,
    pub values: Vec<i32>,
}

/// An `i64`, which TypeScript holds as a `bigint`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedI64 {
    pub keyed: BTreeMap<i64, bool>,
    pub value: i64,
    pub values: Vec<i64>,
}

/// An `isize`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedIsize {
    pub keyed: BTreeMap<isize, bool>,
    pub value: isize,
    pub values: Vec<isize>,
}

/// A `u64`, which Swift and Kotlin refuse in a field: the declared default of the parameter, which
/// the Zod schema the declared default is read with names.
#[model_schema(default_types(Number = u64))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedU64<Number> {
    pub value: Number,
}

/// A `usize`, held as [`ConveyedU64`] holds a `u64`.
#[model_schema(default_types(Number = usize))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedUsize<Number> {
    pub value: Number,
}

/// An `f32`, which keys no map: it is neither `Eq` nor `Ord`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedF32 {
    pub value: f32,
    pub values: Vec<f32>,
}

/// An `f64`, which keys no map: it is neither `Eq` nor `Ord`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedF64 {
    pub value: f64,
    pub values: Vec<f64>,
}

/// A numeric literal on a `u8`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedLiteralCount {
    #[model_schema_prop(literal = 214)]
    pub value: u8,
}

/// A boolean literal.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedLiteralFlag {
    #[model_schema_prop(literal = true)]
    pub value: bool,
}

/// A string literal.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedLiteralText {
    #[model_schema_prop(literal = "val")]
    pub value: String,
}

/// A `NaiveDate`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedDay {
    pub keyed: BTreeMap<NaiveDate, bool>,
    pub value: NaiveDate,
    pub values: Vec<NaiveDate>,
}

/// A `NaiveTime`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedHour {
    pub keyed: BTreeMap<NaiveTime, bool>,
    pub value: NaiveTime,
    pub values: Vec<NaiveTime>,
}

/// A `NaiveDateTime`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedStamp {
    pub keyed: BTreeMap<NaiveDateTime, bool>,
    pub value: NaiveDateTime,
    pub values: Vec<NaiveDateTime>,
}

/// A `DateTime<Utc>`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedInstant {
    pub keyed: BTreeMap<DateTime<Utc>, bool>,
    pub value: DateTime<Utc>,
    pub values: Vec<DateTime<Utc>>,
}

/// A `DateTime<Utc>` under `as_number`, which takes no other place.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedMillis {
    #[model_schema_prop(as_number)]
    pub value: DateTime<Utc>,
}

/// A plain enum, which keys a map by the name serde writes for it.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ConveyedSlot {
    Daily,
    Weekly,
}

/// A plain enum held in each place.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedSlotted {
    pub keyed: BTreeMap<ConveyedSlot, bool>,
    pub value: ConveyedSlot,
    pub values: Vec<ConveyedSlot>,
}

/// An `ObjectId`, which keys no map.
#[cfg(feature = "mongodb")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConveyedObjectId {
    pub value: ObjectId,
    pub values: Vec<ObjectId>,
}

/// One row of [`WIRE_FORMS`].
const fn row(
    held_by: &'static str,
    handed: &'static str,
    serde: &'static str,
    clients: Clients,
) -> WireFormRow {
    WireFormRow {
        dart: clients.dart,
        handed,
        held_by,
        kotlin: clients.kotlin,
        serde,
        swift: clients.swift,
        typescript: clients.typescript,
    }
}

/// The places the model `held_by` holds its value in.
pub fn places(held_by: &str) -> Places {
    match held_by {
        "ConveyedF32" | "ConveyedF64" | "ConveyedObjectId" => Places::Listed,
        "ConveyedLiteralCount"
        | "ConveyedLiteralFlag"
        | "ConveyedLiteralText"
        | "ConveyedMillis"
        | "ConveyedU64"
        | "ConveyedUsize" => Places::Lone,
        _ => Places::Keyed,
    }
}

/// The model held in `places`, `value` in each and `key` as the key of `keyed`: a string key as
/// its text, any other as the text of its JSON.
fn payload(places: Places, value: &str, key: &str) -> String {
    let keyed = if key.starts_with('"') {
        key.to_owned()
    } else {
        format!("\"{key}\"")
    };
    match places {
        Places::Keyed => {
            format!(r#"{{"keyed":{{{keyed}:true}},"value":{value},"values":[{value}]}}"#)
        }
        Places::Listed => format!(r#"{{"value":{value},"values":[{value}]}}"#),
        Places::Lone => format!(r#"{{"value":{value}}}"#),
    }
}

/// What serde answers of the model holding `handed`, where `held_by` names it.
fn serde_told(held_by: &str, handed: &str) -> String {
    if handed == "NaN" {
        let written = serde_json::to_string(&ConveyedF64 {
            value: f64::NAN,
            values: vec![f64::NAN],
        })
        .unwrap();
        return format!(
            "written as {written}, which serde refuses: {}",
            told_as(held_by, &written).unwrap_or_else(|| unnamed(held_by))
        );
    }
    told_as(held_by, &payload(places(held_by), handed, handed)).unwrap_or_else(|| unnamed(held_by))
}

/// What serde answers of `json` as the model `held_by` names, and `None` where none has that
/// name.
fn told_as(held_by: &str, json: &str) -> Option<String> {
    match held_by {
        "ConveyedFlag" => Some(told::<ConveyedFlag>(json)),
        "ConveyedLetter" => Some(told::<ConveyedLetter>(json)),
        "ConveyedText" => Some(told::<ConveyedText>(json)),
        "ConveyedU8" => Some(told::<ConveyedU8>(json)),
        "ConveyedU16" => Some(told::<ConveyedU16>(json)),
        "ConveyedU32" => Some(told::<ConveyedU32>(json)),
        "ConveyedI8" => Some(told::<ConveyedI8>(json)),
        "ConveyedI16" => Some(told::<ConveyedI16>(json)),
        "ConveyedI32" => Some(told::<ConveyedI32>(json)),
        "ConveyedI64" => Some(told::<ConveyedI64>(json)),
        "ConveyedIsize" => Some(told::<ConveyedIsize>(json)),
        "ConveyedU64" => Some(told::<ConveyedU64<u64>>(json)),
        "ConveyedUsize" => Some(told::<ConveyedUsize<usize>>(json)),
        "ConveyedF32" => Some(told::<ConveyedF32>(json)),
        "ConveyedF64" => Some(told::<ConveyedF64>(json)),
        "ConveyedLiteralCount" => Some(told::<ConveyedLiteralCount>(json)),
        "ConveyedLiteralFlag" => Some(told::<ConveyedLiteralFlag>(json)),
        "ConveyedLiteralText" => Some(told::<ConveyedLiteralText>(json)),
        "ConveyedDay" => Some(told::<ConveyedDay>(json)),
        "ConveyedHour" => Some(told::<ConveyedHour>(json)),
        "ConveyedStamp" => Some(told::<ConveyedStamp>(json)),
        "ConveyedInstant" => Some(told::<ConveyedInstant>(json)),
        "ConveyedMillis" => Some(told::<ConveyedMillis>(json)),
        "ConveyedSlotted" => Some(told::<ConveyedSlotted>(json)),
        #[cfg(feature = "mongodb")]
        "ConveyedObjectId" => Some(told::<ConveyedObjectId>(json)),
        _ => None,
    }
}

/// What a row names where no model has the name `held_by`.
fn unnamed(held_by: &str) -> String {
    format!("no model named {held_by}")
}

/// What serde answers of `json` as a `T`: `read` where it writes the payload back as it was, what
/// it writes back otherwise, or its refusal.
fn told<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |refused| refused.to_string(),
        |read| {
            let written = serde_json::to_string(&read).unwrap();
            if written == json {
                "read".to_owned()
            } else {
                format!("read, written back as {written}")
            }
        },
    )
}

/// serde answers each value a caller hands a client as [`WIRE_FORMS`] states.
#[test]
fn serde_answers_each_handed_value_as_the_table_states() {
    let told: Vec<String> = WIRE_FORMS
        .iter()
        .map(|row| {
            format!(
                "{} {}: {}",
                row.held_by,
                row.handed,
                serde_told(row.held_by, row.handed)
            )
        })
        .collect();
    let pinned: Vec<String> = WIRE_FORMS
        .iter()
        .map(|row| format!("{} {}: {}", row.held_by, row.handed, row.serde))
        .collect();
    assert_eq!(told, pinned, "serde no longer answers each value as it did");
}

/// serde reads every payload a client sends.
#[test]
fn serde_reads_every_payload_a_client_sends() {
    for row in WIRE_FORMS {
        for sent in row.sent() {
            let told = told_as(row.held_by, &sent).unwrap_or_else(|| unnamed(row.held_by));
            assert!(
                told.starts_with("read"),
                "serde refuses {sent}, which a client sends for {}: {told}",
                row.held_by
            );
        }
    }
}
