//! A `DateTime<Utc>` and a `DateTime<FixedOffset>` read and written back by the emitted Kotlin
//! types compiled with `-Werror`, at whole seconds and at each fraction chrono writes: Kotlin
//! writes the text chrono writes, `Z` for a `Utc` and the offset it was handed for a
//! `FixedOffset`, in a field, a list and a map's key. A text that is no RFC 3339 date and time is
//! refused as it is written.

#![cfg(all(feature = "kotlin", feature = "chrono"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read and written back, or `refused`.
const TOLD: &str = r#"
fun told(name: String, read: () -> String) {
    try {
        println("$name: ${read()}")
    } catch (refused: Exception) {
        println("$name: refused")
    }
}
"#;

/// The texts handed, each read by serde and written back as chrono writes it.
const HANDED: [&str; 12] = [
    "2024-01-05T10:20:30Z",
    "2024-01-05T10:20:30.000Z",
    "2024-01-05T10:20:30.5Z",
    "2024-01-05T10:20:30.123Z",
    "2024-01-05T10:20:30.123456789Z",
    "2024-01-05T10:20:30.1234Z",
    "2024-01-05T10:20:30+02:00",
    "2024-01-05T10:20:30.5+02:00",
    "2024-01-05T10:20:30.123456789-05:30",
    "2024-01-05T10:20:30+00:00",
    "2024-01-05T10:20:30-00:00",
    "2024-01-05T23:59:59.999-12:00",
];

/// A `DateTime<Utc>`, and a `DateTime<FixedOffset>` in a field, a list and a map's key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Offsets {
    pub by_fixed: BTreeMap<DateTime<FixedOffset>, u8>,
    pub fixed: DateTime<FixedOffset>,
    pub listed: Vec<DateTime<FixedOffset>>,
    pub utc: DateTime<Utc>,
}

fn payload(text: &str) -> String {
    format!(r#"{{"by_fixed":{{"{text}":1}},"fixed":"{text}","listed":["{text}"],"utc":"{text}"}}"#)
}

/// What serde writes back for `payload` read as `Offsets`.
fn serde_back(payload: &str) -> String {
    serde_json::to_string(&serde_json::from_str::<Offsets>(payload).unwrap()).unwrap()
}

/// Each text is written by the Kotlin types as chrono writes it, and a text no date and time is
/// refused.
#[test]
fn kotlin_writes_a_date_and_time_as_chrono_does_in_each_zone() {
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for (index, text) in HANDED.iter().enumerate() {
        let held = payload(text);
        let _ = writeln!(
            main,
            "    told(\"{index}\") {{ Json.encodeToString(Offsets.serializer(), \
             Json.decodeFromString(Offsets.serializer(), \"\"\"{held}\"\"\")) }}"
        );
        let _ = writeln!(expected, "{index}: {}", serde_back(&held));
    }
    let refused = payload("hello");
    let _ = writeln!(
        main,
        "    told(\"hello\") {{ Json.encodeToString(Offsets.serializer(), \
         Json.decodeFromString(Offsets.serializer(), \"\"\"{refused}\"\"\")) }}"
    );
    let _ = writeln!(expected, "hello: refused");
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        offsets_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin write is not chrono's");
}
