//! What the Swift backend writes for a map under a key Swift holds as the text serde writes for it:
//! a `NaiveDate`, a `NaiveTime`, a `NaiveDateTime` and a type parameter. The wrapper the map is read
//! through binds that text as the key, with no `guard`, and a key parsed from its text keeps the
//! `guard` it had. `service_schema_emitted_client_tests/run_swift_text_map_keys.rs` compiles the
//! models of `text_map_keys_models.rs` under a Swift toolchain and reads a payload of each.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::text_map_keys_models::{
    AdjacentBulletin, AdjacentTidings, Almanac, Bulletin, Datebook, Ephemeris, Gazetteer,
    Hourbook, Logsheet, Moorage, TaggedBulletin, TaggedTidings, Tidings, UntaggedBulletin,
    UntaggedTidings, adjacent_bulletin_swift, adjacent_tidings_swift, almanac_swift,
    bulletin_swift, datebook_swift, day, ephemeris_swift, gazetteer_swift, hour, hourbook_swift,
    logsheet_swift, moment, tagged_bulletin_swift, tagged_tidings_swift, tidings_swift,
    untagged_bulletin_swift, untagged_tidings_swift,
};

/// How a wrapper binds a key it holds as its text, ahead of the entry it reads.
const TEXT_KEY: &str = "let wireKey = codingKey.stringValue; let key = wireKey; result[key] = ";

/// The members of [`Tidings::Dated`] as serde writes them.
const DATED: &str = r#"{"days":{"2024-01-02":"d"},"hours":{"03:04:05.678":"h"},"moments":{"2024-01-02T03:04:05.678":"d"}}"#;

/// A plain enum a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Bell {
    First,
    Second,
}

/// A map under each key Swift parses from its text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Keyring {
    pub bells: HashMap<Bell, String>,
    pub counts: HashMap<u32, String>,
    pub flags: HashMap<bool, String>,
    pub offsets: HashMap<i64, String>,
    pub stamps: HashMap<DateTime<Utc>, String>,
}

/// How a wrapper binds a key `parsed` from its text, ahead of the entry it reads.
fn guarded(parsed: &str) -> String {
    format!(
        "let wireKey = codingKey.stringValue; guard let key = {parsed} else {{ \
         throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
         debugDescription: \"bad key \\(wireKey)\") }}; result[key] = "
    )
}

/// The JSON serde writes for `held`, read back by serde as it was written.
fn read_back<Held>(held: &Held) -> String
where
    Held: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(held).unwrap();
    let read: Held = serde_json::from_str(&written).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), written);
    written
}

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for text in written {
        assert!(swift.contains(text), "{text}\nin: {swift}");
    }
}

/// The maps a `Dated` variant and [`Ephemeris`] hold, each of one entry.
fn dated<Held>(
    held: &Held,
) -> (
    HashMap<NaiveDate, Held>,
    HashMap<NaiveTime, String>,
    HashMap<NaiveDateTime, Held>,
)
where
    Held: Clone,
{
    (
        HashMap::from([(day(), held.clone())]),
        HashMap::from([(hour(), "h".to_owned())]),
        HashMap::from([(moment(), held.clone())]),
    )
}

#[test]
fn each_struct_and_brand_is_written_as_serde_writes_it() {
    let (days, hours, moments) = dated(&"d".to_owned());
    assert_eq!(
        read_back(&Almanac {
            berths: HashMap::from([(
                day(),
                Moorage {
                    label: "m".to_owned()
                }
            )]),
            days: days.clone(),
            hours: hours.clone(),
            moments: moments.clone(),
        }),
        r#"{"berths":{"2024-01-02":{"label":"m"}},"days":{"2024-01-02":"d"},"hours":{"03:04:05.678":"h"},"moments":{"2024-01-02T03:04:05.678":"d"}}"#
    );
    assert_eq!(
        read_back(&Ephemeris {
            days: days.clone(),
            hours: hours.clone(),
            moments: moments.clone(),
        }),
        DATED
    );
    assert_eq!(
        read_back(&Gazetteer {
            by_stowed: HashMap::from([("s".to_owned(), 1)]),
        }),
        r#"{"by_stowed":{"s":1}}"#
    );
    assert_eq!(
        read_back(&Gazetteer {
            by_stowed: HashMap::from([(7_u32, 1)]),
        }),
        r#"{"by_stowed":{"7":1}}"#
    );
    assert_eq!(read_back(&Datebook(days)), r#"{"2024-01-02":"d"}"#);
    assert_eq!(read_back(&Hourbook { hours }), r#"{"03:04:05.678":"h"}"#);
    assert_eq!(
        read_back(&Logsheet(moments)),
        r#"{"2024-01-02T03:04:05.678":"d"}"#
    );
}

#[test]
fn each_variant_is_written_as_serde_writes_it_under_each_tagging() {
    let (days, hours, moments) = dated(&"d".to_owned());
    let external = format!(r#"{{"Dated":{DATED}}}"#);
    let internal = format!(r#"{{"kind":"Dated",{}"#, &DATED[1..]);
    let adjacent = format!(r#"{{"kind":"Dated","body":{DATED}}}"#);
    let untagged = DATED.to_owned();
    for (written, expected) in [
        (
            read_back(&Tidings::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            }),
            &external,
        ),
        (
            read_back(&TaggedTidings::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            }),
            &internal,
        ),
        (
            read_back(&AdjacentTidings::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            }),
            &adjacent,
        ),
        (
            read_back(&UntaggedTidings::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            }),
            &untagged,
        ),
        (
            read_back(&Bulletin::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            }),
            &external,
        ),
        (
            read_back(&TaggedBulletin::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            }),
            &internal,
        ),
        (
            read_back(&AdjacentBulletin::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            }),
            &adjacent,
        ),
        (
            read_back(&UntaggedBulletin::Dated {
                days,
                hours,
                moments,
            }),
            &untagged,
        ),
    ] {
        assert_eq!(&written, expected);
    }
}

#[test]
fn each_variant_beside_the_maps_is_written_as_serde_writes_it() {
    for plain in [
        read_back(&Tidings::Plain {
            note: "n".to_owned(),
        }),
        read_back(&TaggedTidings::Plain {
            note: "n".to_owned(),
        }),
        read_back(&AdjacentTidings::Plain {
            note: "n".to_owned(),
        }),
        read_back(&UntaggedTidings::Plain {
            note: "n".to_owned(),
        }),
        read_back(&Bulletin::Plain {
            note: "n".to_owned(),
        }),
        read_back(&TaggedBulletin::Plain {
            note: "n".to_owned(),
        }),
        read_back(&AdjacentBulletin::Plain {
            note: "n".to_owned(),
        }),
        read_back(&UntaggedBulletin::Plain {
            note: "n".to_owned(),
        }),
    ] {
        assert!(plain.contains(r#""note":"n""#), "{plain}");
    }
}

/// Every read of a wrapper, its own and the one `Codable` calls, binds the text as the key: one
/// read per wrapper, and two for the wrapper of a map whose value is a model.
#[test]
fn a_key_held_as_its_text_is_bound_with_no_guard() {
    for (swift, reads) in [
        (almanac_swift::swift_definition(), 5),
        (ephemeris_swift::swift_definition(), 3),
        (gazetteer_swift::swift_definition(), 1),
        (tidings_swift::swift_definition(), 3),
        (tagged_tidings_swift::swift_definition(), 3),
        (adjacent_tidings_swift::swift_definition(), 3),
        (untagged_tidings_swift::swift_definition(), 3),
        (bulletin_swift::swift_definition(), 3),
        (tagged_bulletin_swift::swift_definition(), 3),
        (adjacent_bulletin_swift::swift_definition(), 3),
        (untagged_bulletin_swift::swift_definition(), 3),
        (datebook_swift::swift_definition(), 1),
        (hourbook_swift::swift_definition(), 1),
        (logsheet_swift::swift_definition(), 1),
    ] {
        assert!(
            !swift.contains("let wireKey = codingKey.stringValue; guard"),
            "{swift}"
        );
        assert_eq!(swift.matches(TEXT_KEY).count(), reads, "{swift}");
    }
}

#[test]
fn a_wrapper_reads_a_model_under_a_key_held_as_its_text_with_no_judgement() {
    holds(
        &almanac_swift::swift_definition(),
        &[
            "init(swiftSchemaUnjudged decoder: Decoder) throws { \
             let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
             var result: [String: Moorage] = [:]; for codingKey in container.allKeys { \
             let wireKey = codingKey.stringValue; let key = wireKey; \
             result[key] = try Moorage(swiftSchemaUnjudged: \
             container.superDecoder(forKey: codingKey)) }; storage = result }; ",
            "public let berths: [String: Moorage]; ",
        ],
    );
}

#[test]
fn the_parameter_as_a_key_is_held_as_its_text() {
    holds(
        &gazetteer_swift::swift_definition(),
        &[
            "public struct GazetteerByStowedKeyedMap<Stowed: Codable & Sendable>: Codable, \
             Sendable { var storage: [String: UInt32]; ",
            "public let byStowed: [String: UInt32]; ",
            &format!("{TEXT_KEY}try container.decode(UInt32.self, forKey: codingKey) }}; "),
        ],
    );
}

#[test]
fn a_key_parsed_from_its_text_keeps_its_guard() {
    let swift = keyring_swift::swift_definition();
    for parsed in [
        "Bell(rawValue: wireKey)",
        "UInt32(wireKey)",
        "(wireKey == \"true\" ? true : wireKey == \"false\" ? false : nil)",
        "Int64(wireKey)",
        "Self.isoFormatterForKeys.date(from: wireKey)",
    ] {
        holds(&swift, &[&guarded(parsed)]);
    }
    assert!(!swift.contains(TEXT_KEY), "{swift}");
    let keyring = Keyring {
        bells: HashMap::from([(Bell::First, "b".to_owned())]),
        counts: HashMap::from([(1, "c".to_owned())]),
        flags: HashMap::from([(true, "f".to_owned())]),
        offsets: HashMap::from([(-1, "o".to_owned())]),
        stamps: HashMap::new(),
    };
    assert_eq!(
        read_back(&keyring),
        r#"{"bells":{"First":"b"},"counts":{"1":"c"},"flags":{"true":"f"},"offsets":{"-1":"o"},"stamps":{}}"#
    );
}

/// serde writes an `ObjectId` as an object, which no JSON object key is: it writes no map keyed by
/// one, and the expansion refuses such a map where a web surface is built.
#[cfg(feature = "mongodb")]
#[test]
fn serde_writes_no_map_keyed_by_an_object_id() {
    use mongodb::bson::oid::ObjectId;

    let keyed = HashMap::from([(ObjectId::from_bytes([0; 12]), 1_u8)]);
    let refused = serde_json::to_string(&keyed).unwrap_err();
    assert_eq!(refused.to_string(), "key must be a string");
}
