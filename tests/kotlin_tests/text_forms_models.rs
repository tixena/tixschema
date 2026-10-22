//! The models of a text serde reads in one form alone, which the Kotlin client holds as a
//! `String`: a `char`, a `NaiveDate`, a `NaiveTime`, a `NaiveDateTime` and a `DateTime<Utc>`, each
//! held by a field, in a list, as a map's key, as a map's value and as the key of a map flattened
//! alone, and [`Chronometer`], a `char` and a date held by a field and as a map's key.
//! [`TEXT_FORMS`] holds what serde, Zod and the Kotlin read answer of each payload: the Kotlin read
//! refuses what Zod refuses, a `DateTime<Utc>` apart, which it reads as any text.
//! `kotlin_tests/text_forms.rs` pins the Kotlin text,
//! `service_schema_emitted_client_tests/run_kotlin_text_forms.rs` runs it, and
//! `generic_types_tests/declarations.rs` holds Zod to [`TEXT_FORMS`].

use alloc::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload, read as its type: one every reader reads, then a text of no form at each place,
/// and a text serde and Zod part on.
pub const TEXT_FORMS: [TextFormRow; 49] = [
    TextFormRow {
        json: r#"{"by_date":{"2024-01-05":2},"by_letter":{"a":1},"day":"2024-01-06","letter":"b"}"#,
        kotlin: "read",
        read_as: "Chronometer",
        serde: "read",
        told: serde_told::<Chronometer>,
        zod: true,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_date":{"2024-01-05":2},"by_letter":{"ab":1},"day":"2024-01-06","#,
            r#""letter":"b"}"#
        ),
        kotlin: r#"expected a character, found "ab""#,
        read_as: "Chronometer",
        serde: r#"invalid value: string "ab", expected a character at line 1 column 45"#,
        told: serde_told::<Chronometer>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"by_date":{"x":2},"by_letter":{"a":1},"day":"2024-01-06","letter":"b"}"#,
        kotlin: r#"expected a date, found "x""#,
        read_as: "Chronometer",
        serde: "input contains invalid characters at line 1 column 15",
        told: serde_told::<Chronometer>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_date":{"2024-01-05":2},"by_letter":{"a":1},"day":"2024-01-06","#,
            r#""letter":"ab"}"#
        ),
        kotlin: r#"expected a character, found "ab""#,
        read_as: "Chronometer",
        serde: r#"invalid value: string "ab", expected a character at line 1 column 80"#,
        told: serde_told::<Chronometer>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"by_date":{"2024-01-05":2},"by_letter":{"a":1},"day":"x","letter":"b"}"#,
        kotlin: r#"expected a date, found "x""#,
        read_as: "Chronometer",
        serde: "input contains invalid characters at line 1 column 57",
        told: serde_told::<Chronometer>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"by_letter":{"a":1},"letter":"b","letters":["c"],"to_letter":{"k":"d"}}"#,
        kotlin: "read",
        read_as: "LetterLog",
        serde: "read",
        told: serde_told::<LetterLog>,
        zod: true,
    },
    TextFormRow {
        json: r#"{"by_letter":{"ab":1},"letter":"b","letters":["c"],"to_letter":{"k":"d"}}"#,
        kotlin: r#"expected a character, found "ab""#,
        read_as: "LetterLog",
        serde: r#"invalid value: string "ab", expected a character at line 1 column 18"#,
        told: serde_told::<LetterLog>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"by_letter":{"a":1},"letter":"ab","letters":["c"],"to_letter":{"k":"d"}}"#,
        kotlin: r#"expected a character, found "ab""#,
        read_as: "LetterLog",
        serde: r#"invalid value: string "ab", expected a character at line 1 column 34"#,
        told: serde_told::<LetterLog>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"by_letter":{"a":1},"letter":"b","letters":["ab"],"to_letter":{"k":"d"}}"#,
        kotlin: r#"expected a character, found "ab""#,
        read_as: "LetterLog",
        serde: r#"invalid value: string "ab", expected a character at line 1 column 49"#,
        told: serde_told::<LetterLog>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"by_letter":{"a":1},"letter":"b","letters":["c"],"to_letter":{"k":"ab"}}"#,
        kotlin: r#"expected a character, found "ab""#,
        read_as: "LetterLog",
        serde: r#"invalid value: string "ab", expected a character at line 1 column 71"#,
        told: serde_told::<LetterLog>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"by_letter":{"a":1},"letter":"","letters":["c"],"to_letter":{"k":"d"}}"#,
        kotlin: r#"expected a character, found """#,
        read_as: "LetterLog",
        serde: r#"invalid value: string "", expected a character at line 1 column 32"#,
        told: serde_told::<LetterLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_letter":{"a":1},"letter":""#,
            "\u{1f600}",
            r#"","letters":["c"],"to_letter":{"k":"d"}}"#
        ),
        kotlin: "read",
        read_as: "LetterLog",
        serde: "read",
        told: serde_told::<LetterLog>,
        zod: true,
    },
    TextFormRow {
        json: r#"{"a":2}"#,
        kotlin: "read",
        read_as: "LetterLines",
        serde: "read",
        told: serde_told::<LetterLines>,
        zod: true,
    },
    TextFormRow {
        json: r#"{"ab":2}"#,
        kotlin: r#"expected a character, found "ab""#,
        read_as: "LetterLines",
        serde: r#"invalid value: string "ab", expected a character at line 1 column 8"#,
        told: serde_told::<LetterLines>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_day":{"2024-01-05":1},"day":"2024-01-06","days":["2024-01-07"],"#,
            r#""to_day":{"k":"2024-01-08"}}"#
        ),
        kotlin: "read",
        read_as: "DayLog",
        serde: "read",
        told: serde_told::<DayLog>,
        zod: true,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_day":{"x":1},"day":"2024-01-06","days":["2024-01-07"],"#,
            r#""to_day":{"k":"2024-01-08"}}"#
        ),
        kotlin: r#"expected a date, found "x""#,
        read_as: "DayLog",
        serde: "input contains invalid characters at line 1 column 14",
        told: serde_told::<DayLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_day":{"2024-01-05":1},"day":"x","days":["2024-01-07"],"#,
            r#""to_day":{"k":"2024-01-08"}}"#
        ),
        kotlin: r#"expected a date, found "x""#,
        read_as: "DayLog",
        serde: "input contains invalid characters at line 1 column 36",
        told: serde_told::<DayLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_day":{"2024-01-05":1},"day":"2024-01-06","days":["x"],"#,
            r#""to_day":{"k":"2024-01-08"}}"#
        ),
        kotlin: r#"expected a date, found "x""#,
        read_as: "DayLog",
        serde: "input contains invalid characters at line 1 column 57",
        told: serde_told::<DayLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_day":{"2024-01-05":1},"day":"2024-01-06","days":["2024-01-07"],"#,
            r#""to_day":{"k":"x"}}"#
        ),
        kotlin: r#"expected a date, found "x""#,
        read_as: "DayLog",
        serde: "input contains invalid characters at line 1 column 85",
        told: serde_told::<DayLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_day":{"2024-01-05":1},"day":"2024-1-6","days":["2024-01-07"],"#,
            r#""to_day":{"k":"2024-01-08"}}"#
        ),
        kotlin: r#"expected a date, found "2024-1-6""#,
        read_as: "DayLog",
        serde: concat!(
            "read, written back as ",
            r#"{"by_day":{"2024-01-05":1},"day":"2024-01-06","days":["2024-01-07"],"#,
            r#""to_day":{"k":"2024-01-08"}}"#
        ),
        told: serde_told::<DayLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_day":{"2024-01-05":1},"day":"2023-02-29","days":["2024-01-07"],"#,
            r#""to_day":{"k":"2024-01-08"}}"#
        ),
        kotlin: r#"expected a date, found "2023-02-29""#,
        read_as: "DayLog",
        serde: "input is out of range at line 1 column 45",
        told: serde_told::<DayLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_day":{"2024-01-05":1},"day":"+12024-01-06","days":["2024-01-07"],"#,
            r#""to_day":{"k":"2024-01-08"}}"#
        ),
        kotlin: r#"expected a date, found "+12024-01-06""#,
        read_as: "DayLog",
        serde: "read",
        told: serde_told::<DayLog>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"2024-01-05":2}"#,
        kotlin: "read",
        read_as: "DayLines",
        serde: "read",
        told: serde_told::<DayLines>,
        zod: true,
    },
    TextFormRow {
        json: r#"{"x":2}"#,
        kotlin: r#"expected a date, found "x""#,
        read_as: "DayLines",
        serde: "input contains invalid characters at line 1 column 7",
        told: serde_told::<DayLines>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_time":{"10:20:30":1},"time":"11:20:30","times":["12:20:30"],"#,
            r#""to_time":{"k":"13:20:30"}}"#
        ),
        kotlin: "read",
        read_as: "TimeLog",
        serde: "read",
        told: serde_told::<TimeLog>,
        zod: true,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_time":{"x":1},"time":"11:20:30","times":["12:20:30"],"#,
            r#""to_time":{"k":"13:20:30"}}"#
        ),
        kotlin: r#"expected a time, found "x""#,
        read_as: "TimeLog",
        serde: "input contains invalid characters at line 1 column 15",
        told: serde_told::<TimeLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_time":{"10:20:30":1},"time":"x","times":["12:20:30"],"#,
            r#""to_time":{"k":"13:20:30"}}"#
        ),
        kotlin: r#"expected a time, found "x""#,
        read_as: "TimeLog",
        serde: "input contains invalid characters at line 1 column 36",
        told: serde_told::<TimeLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_time":{"10:20:30":1},"time":"11:20:30","times":["x"],"#,
            r#""to_time":{"k":"13:20:30"}}"#
        ),
        kotlin: r#"expected a time, found "x""#,
        read_as: "TimeLog",
        serde: "input contains invalid characters at line 1 column 56",
        told: serde_told::<TimeLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_time":{"10:20:30":1},"time":"11:20:30","times":["12:20:30"],"#,
            r#""to_time":{"k":"x"}}"#
        ),
        kotlin: r#"expected a time, found "x""#,
        read_as: "TimeLog",
        serde: "input contains invalid characters at line 1 column 83",
        told: serde_told::<TimeLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_time":{"10:20:30":1},"time":"23:59:60","times":["12:20:30"],"#,
            r#""to_time":{"k":"13:20:30"}}"#
        ),
        kotlin: r#"expected a time, found "23:59:60""#,
        read_as: "TimeLog",
        serde: "read",
        told: serde_told::<TimeLog>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"10:20:30":2}"#,
        kotlin: "read",
        read_as: "TimeLines",
        serde: "read",
        told: serde_told::<TimeLines>,
        zod: true,
    },
    TextFormRow {
        json: r#"{"x":2}"#,
        kotlin: r#"expected a time, found "x""#,
        read_as: "TimeLines",
        serde: "input contains invalid characters at line 1 column 7",
        told: serde_told::<TimeLines>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_stamp":{"2024-01-05T10:20:30":1},"stamp":"2024-01-05T11:20:30","#,
            r#""stamps":["2024-01-05T12:20:30"],"to_stamp":{"k":"2024-01-05T13:20:30"}}"#
        ),
        kotlin: "read",
        read_as: "StampLog",
        serde: "read",
        told: serde_told::<StampLog>,
        zod: true,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_stamp":{"x":1},"stamp":"2024-01-05T11:20:30","#,
            r#""stamps":["2024-01-05T12:20:30"],"to_stamp":{"k":"2024-01-05T13:20:30"}}"#
        ),
        kotlin: r#"expected a date and time, found "x""#,
        read_as: "StampLog",
        serde: "input contains invalid characters at line 1 column 16",
        told: serde_told::<StampLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_stamp":{"2024-01-05T10:20:30":1},"stamp":"x","#,
            r#""stamps":["2024-01-05T12:20:30"],"to_stamp":{"k":"2024-01-05T13:20:30"}}"#
        ),
        kotlin: r#"expected a date and time, found "x""#,
        read_as: "StampLog",
        serde: "input contains invalid characters at line 1 column 49",
        told: serde_told::<StampLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_stamp":{"2024-01-05T10:20:30":1},"stamp":"2024-01-05T11:20:30","#,
            r#""stamps":["x"],"to_stamp":{"k":"2024-01-05T13:20:30"}}"#
        ),
        kotlin: r#"expected a date and time, found "x""#,
        read_as: "StampLog",
        serde: "input contains invalid characters at line 1 column 81",
        told: serde_told::<StampLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_stamp":{"2024-01-05T10:20:30":1},"stamp":"2024-01-05T11:20:30","#,
            r#""stamps":["2024-01-05T12:20:30"],"to_stamp":{"k":"x"}}"#
        ),
        kotlin: r#"expected a date and time, found "x""#,
        read_as: "StampLog",
        serde: "input contains invalid characters at line 1 column 120",
        told: serde_told::<StampLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_stamp":{"2024-01-05T10:20:30":1},"stamp":"2024-01-05T11:20:30Z","#,
            r#""stamps":["2024-01-05T12:20:30"],"to_stamp":{"k":"2024-01-05T13:20:30"}}"#
        ),
        kotlin: "read",
        read_as: "StampLog",
        serde: "trailing input at line 1 column 68",
        told: serde_told::<StampLog>,
        zod: true,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_stamp":{"2024-01-05T10:20:30":1},"stamp":"2024-01-05 11:20:30","#,
            r#""stamps":["2024-01-05T12:20:30"],"to_stamp":{"k":"2024-01-05T13:20:30"}}"#
        ),
        kotlin: r#"expected a date and time, found "2024-01-05 11:20:30""#,
        read_as: "StampLog",
        serde: "input contains invalid characters at line 1 column 67",
        told: serde_told::<StampLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_stamp":{"2024-01-05T10:20:30":1},"stamp":"2024-01-05T11:20","#,
            r#""stamps":["2024-01-05T12:20:30"],"to_stamp":{"k":"2024-01-05T13:20:30"}}"#
        ),
        kotlin: "read",
        read_as: "StampLog",
        serde: "premature end of input at line 1 column 64",
        told: serde_told::<StampLog>,
        zod: true,
    },
    TextFormRow {
        json: r#"{"2024-01-05T10:20:30":2}"#,
        kotlin: "read",
        read_as: "StampLines",
        serde: "read",
        told: serde_told::<StampLines>,
        zod: true,
    },
    TextFormRow {
        json: r#"{"x":2}"#,
        kotlin: r#"expected a date and time, found "x""#,
        read_as: "StampLines",
        serde: "input contains invalid characters at line 1 column 7",
        told: serde_told::<StampLines>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_instant":{"2024-01-05T10:20:30Z":1},"instant":"2024-01-05T11:20:30Z","#,
            r#""instants":["2024-01-05T12:20:30Z"],"to_instant":{"k":"2024-01-05T13:20:30Z"}}"#
        ),
        kotlin: "read",
        read_as: "InstantLog",
        serde: "read",
        told: serde_told::<InstantLog>,
        zod: true,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_instant":{"x":1},"instant":"2024-01-05T11:20:30Z","#,
            r#""instants":["2024-01-05T12:20:30Z"],"to_instant":{"k":"2024-01-05T13:20:30Z"}}"#
        ),
        kotlin: "read",
        read_as: "InstantLog",
        serde: "input contains invalid characters at line 1 column 18",
        told: serde_told::<InstantLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_instant":{"2024-01-05T10:20:30Z":1},"instant":"x","#,
            r#""instants":["2024-01-05T12:20:30Z"],"to_instant":{"k":"2024-01-05T13:20:30Z"}}"#
        ),
        kotlin: "read",
        read_as: "InstantLog",
        serde: "input contains invalid characters at line 1 column 54",
        told: serde_told::<InstantLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_instant":{"2024-01-05T10:20:30Z":1},"instant":"2024-01-05T11:20:30Z","#,
            r#""instants":["x"],"to_instant":{"k":"2024-01-05T13:20:30Z"}}"#
        ),
        kotlin: "read",
        read_as: "InstantLog",
        serde: "input contains invalid characters at line 1 column 89",
        told: serde_told::<InstantLog>,
        zod: false,
    },
    TextFormRow {
        json: concat!(
            r#"{"by_instant":{"2024-01-05T10:20:30Z":1},"instant":"2024-01-05T11:20:30Z","#,
            r#""instants":["2024-01-05T12:20:30Z"],"to_instant":{"k":"x"}}"#
        ),
        kotlin: "read",
        read_as: "InstantLog",
        serde: "input contains invalid characters at line 1 column 131",
        told: serde_told::<InstantLog>,
        zod: false,
    },
    TextFormRow {
        json: r#"{"2024-01-05T10:20:30Z":2}"#,
        kotlin: "read",
        read_as: "InstantLines",
        serde: "read",
        told: serde_told::<InstantLines>,
        zod: true,
    },
    TextFormRow {
        json: r#"{"x":2}"#,
        kotlin: "read",
        read_as: "InstantLines",
        serde: "input contains invalid characters at line 1 column 7",
        told: serde_told::<InstantLines>,
        zod: false,
    },
];

/// A `char` and a `NaiveDate`, each held by a field and as a map's key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Chronometer {
    pub by_date: BTreeMap<NaiveDate, u8>,
    pub by_letter: BTreeMap<char, u8>,
    pub day: NaiveDate,
    pub letter: char,
}

/// A `NaiveDate` as the key of a map flattened alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DayLines {
    #[serde(flatten)]
    pub lines: BTreeMap<NaiveDate, u8>,
}

/// A `NaiveDate` held by a field, in a list, as a map's key and as a map's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DayLog {
    pub by_day: BTreeMap<NaiveDate, u8>,
    pub day: NaiveDate,
    pub days: Vec<NaiveDate>,
    pub to_day: BTreeMap<String, NaiveDate>,
}

/// A `DateTime<Utc>` as the key of a map flattened alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstantLines {
    #[serde(flatten)]
    pub lines: BTreeMap<DateTime<Utc>, u8>,
}

/// A `DateTime<Utc>` held by a field, in a list, as a map's key and as a map's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstantLog {
    pub by_instant: BTreeMap<DateTime<Utc>, u8>,
    pub instant: DateTime<Utc>,
    pub instants: Vec<DateTime<Utc>>,
    pub to_instant: BTreeMap<String, DateTime<Utc>>,
}

/// A `char` as the key of a map flattened alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LetterLines {
    #[serde(flatten)]
    pub lines: BTreeMap<char, u8>,
}

/// A `char` held by a field, in a list, as a map's key and as a map's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LetterLog {
    pub by_letter: BTreeMap<char, u8>,
    pub letter: char,
    pub letters: Vec<char>,
    pub to_letter: BTreeMap<String, char>,
}

/// A `NaiveDateTime` as the key of a map flattened alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StampLines {
    #[serde(flatten)]
    pub lines: BTreeMap<NaiveDateTime, u8>,
}

/// A `NaiveDateTime` held by a field, in a list, as a map's key and as a map's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StampLog {
    pub by_stamp: BTreeMap<NaiveDateTime, u8>,
    pub stamp: NaiveDateTime,
    pub stamps: Vec<NaiveDateTime>,
    pub to_stamp: BTreeMap<String, NaiveDateTime>,
}

/// One payload of [`TEXT_FORMS`], the type it is read as, and what each reader answers of it.
pub struct TextFormRow {
    pub json: &'static str,
    /// What the Kotlin read answers: `read`, written back as it was, or the message of the
    /// `SerializationException` it throws.
    pub kotlin: &'static str,
    pub read_as: &'static str,
    /// What serde answers: `read` where it writes the payload back as it was, what it writes back
    /// otherwise, or its refusal.
    pub serde: &'static str,
    /// serde's read of the payload as the type.
    pub told: fn(&str) -> String,
    /// Whether the Zod schema of the type reads the payload.
    pub zod: bool,
}

/// A `NaiveTime` as the key of a map flattened alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TimeLines {
    #[serde(flatten)]
    pub lines: BTreeMap<NaiveTime, u8>,
}

/// A `NaiveTime` held by a field, in a list, as a map's key and as a map's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TimeLog {
    pub by_time: BTreeMap<NaiveTime, u8>,
    pub time: NaiveTime,
    pub times: Vec<NaiveTime>,
    pub to_time: BTreeMap<String, NaiveTime>,
}

/// What serde answers of `json` as a `T`: `read` where it writes the payload back as it was, what
/// it writes back otherwise, or its refusal.
fn serde_told<T>(json: &str) -> String
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

/// serde answers each payload as [`TEXT_FORMS`] states.
#[test]
fn serde_answers_each_text_as_the_table_states() {
    let told: Vec<String> = TEXT_FORMS.iter().map(|row| (row.told)(row.json)).collect();
    let pinned: Vec<&str> = TEXT_FORMS.iter().map(|row| row.serde).collect();
    assert_eq!(
        told, pinned,
        "serde no longer answers each payload as it did"
    );
}

/// The Kotlin read refuses a payload exactly where Zod refuses it, where serde does too and where
/// the two part, and reads every payload of a `DateTime<Utc>`, whose text no check is held to.
#[test]
fn the_kotlin_read_refuses_what_zod_refuses_but_a_date_and_time_with_its_zone() {
    for row in &TEXT_FORMS {
        let zoned = row.read_as.starts_with("Instant");
        assert_eq!(
            row.kotlin == "read",
            row.zod || zoned,
            "the Kotlin read parts from Zod on `{}` as {}",
            row.json,
            row.read_as
        );
    }
}
