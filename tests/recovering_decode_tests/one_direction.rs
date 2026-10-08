//! A renaming written as a list that names one key for reading and another, or none, for
//! writing: each value is looked up under the `deserialize` side, where plain serde reads it.
//!
//! A build that describes a type refuses such a list, so this module is compiled where nothing
//! describes one.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::lines;

/// What `from_value_with` lists for `$stored`, read as `$model` by a decider that rejects it.
macro_rules! listed {
    ($model:ty, $module:ident, $stored:expr) => {
        lines(
            &<$model>::from_value_with($stored, |_raw, _found| $module::Verdict::Reject)
                .unwrap_err(),
        )
    };
}

/// What `from_value_with` reads `$stored` as, by a decider that counts each run of its own into
/// `$calls`.
macro_rules! read_counting {
    ($model:ty, $module:ident, $stored:expr, $calls:ident) => {
        <$model>::from_value_with($stored, |_raw, _found| {
            $calls += 1;
            $module::Verdict::Reject
        })
    };
}

/// A field renamed for both directions apart, and one renamed for reading alone.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ReadRenamed {
    #[serde(rename(serialize = "written", deserialize = "read_too"))]
    both_ways: u32,
    #[serde(rename(deserialize = "read"))]
    one_way: u32,
    plain: u32,
}

/// The rule cases every field for reading alone. One field is renamed for writing alone, and one
/// is written and never read.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all(deserialize = "camelCase"))]
struct ReadCased {
    first_name: String,
    #[serde(skip_deserializing)]
    kept_note: String,
    #[serde(rename(serialize = "written_last"))]
    last_name: String,
}

/// Named by a tag: a variant renamed for reading, the rule of the variants and of their fields
/// written for reading alone, and a variant with a rule of its own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(
    tag = "mode",
    rename_all(deserialize = "kebab-case"),
    rename_all_fields(deserialize = "camelCase")
)]
enum Haulage {
    #[serde(rename(serialize = "byAir", deserialize = "air"))]
    Air {
        flight_code: String,
    },
    OverLand {
        road_name: String,
    },
    #[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
    Sea {
        ship_name: String,
    },
}

/// Named by the key each variant is under.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
enum Relay {
    #[serde(rename(deserialize = "byHand"))]
    Courier {
        badge_number: u32,
    },
    DropBox(u32),
    NotSent,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Dispatch {
    relay: Relay,
}

fn read_cased() -> ReadCased {
    ReadCased {
        first_name: "Ada".to_owned(),
        kept_note: String::new(),
        last_name: "Lovelace".to_owned(),
    }
}

#[test]
fn a_record_under_the_keys_serde_reads_lists_no_issue() {
    let stored = json!({ "read_too": 2_u32, "read": 1_u32, "plain": 3_u32 });
    let renamed = ReadRenamed {
        both_ways: 2_u32,
        one_way: 1_u32,
        plain: 3_u32,
    };
    assert_eq!(
        serde_json::from_value::<ReadRenamed>(stored.clone()).unwrap(),
        renamed
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(ReadRenamed, read_renamed_schema, stored, calls),
        Ok(renamed)
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_wrong_value_under_a_read_key_is_listed_at_that_key_and_fixed_there() {
    let stored = json!({ "read_too": "two", "read": 1_u32, "plain": 3_u32 });
    let mut seen: Vec<String> = Vec::new();
    let read = ReadRenamed::from_value_with(stored, |raw, found| {
        seen = lines(&read_renamed_schema::Unrecovered {
            issues: found.to_vec(),
        });
        for issue in found {
            if let read_renamed_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_value(raw, Value::from(2_u32));
            }
        }
        read_renamed_schema::Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            "read_too: invalid: expected U32, found String(\"two\"): invalid type: string \"two\", expected u32"
        ]
    );
    assert_eq!(
        read,
        Ok(ReadRenamed {
            both_ways: 2_u32,
            one_way: 1_u32,
            plain: 3_u32,
        })
    );
}

/// Neither a field's own name nor the name it is written under is a key serde reads it from.
#[test]
fn a_field_renamed_for_reading_is_missing_under_any_other_key() {
    assert_eq!(
        listed!(
            ReadRenamed,
            read_renamed_schema,
            json!({ "both_ways": 2_u32, "one_way": 1_u32, "plain": 3_u32 })
        ),
        [
            "read_too: missing: expected U32",
            "read: missing: expected U32",
            "both_ways: unknown: found Number(2)",
            "one_way: unknown: found Number(1)",
        ]
    );
    let written = serde_json::to_value(ReadRenamed {
        both_ways: 2_u32,
        one_way: 1_u32,
        plain: 3_u32,
    })
    .unwrap();
    assert_eq!(
        written,
        json!({ "written": 2_u32, "one_way": 1_u32, "plain": 3_u32 })
    );
    assert_eq!(
        listed!(ReadRenamed, read_renamed_schema, written),
        [
            "read_too: missing: expected U32",
            "read: missing: expected U32",
            "written: unknown: found Number(2)",
            "one_way: unknown: found Number(1)",
        ]
    );
}

/// The key of the field serde never reads is the one serde writes it under, which the rule
/// written for reading does not case.
#[test]
fn a_rule_written_for_reading_cases_the_keys_serde_reads() {
    let stored = json!({ "firstName": "Ada", "kept_note": "n", "lastName": "Lovelace" });
    assert_eq!(
        serde_json::from_value::<ReadCased>(stored.clone()).unwrap(),
        read_cased()
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(ReadCased, read_cased_schema, stored, calls),
        Ok(read_cased())
    );
    assert_eq!(calls, 0);

    assert_eq!(
        listed!(
            ReadCased,
            read_cased_schema,
            json!({ "firstName": 7_u32, "lastName": "Lovelace" })
        ),
        [
            "firstName: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
    let written = serde_json::to_value(ReadCased {
        kept_note: "n".to_owned(),
        ..read_cased()
    })
    .unwrap();
    assert_eq!(
        written,
        json!({ "first_name": "Ada", "kept_note": "n", "written_last": "Lovelace" })
    );
    assert_eq!(
        listed!(ReadCased, read_cased_schema, written),
        [
            "firstName: missing: expected String",
            "lastName: missing: expected String",
            "first_name: unknown: found String(\"Ada\")",
            "written_last: unknown: found String(\"Lovelace\")",
        ]
    );
}

#[test]
fn a_tag_names_a_variant_by_the_name_serde_reads() {
    let mut calls = 0_u32;
    for (stored, haulage) in [
        (
            json!({ "mode": "air", "flightCode": "IB6500" }),
            Haulage::Air {
                flight_code: "IB6500".to_owned(),
            },
        ),
        (
            json!({ "mode": "over-land", "roadName": "DR-1" }),
            Haulage::OverLand {
                road_name: "DR-1".to_owned(),
            },
        ),
        (
            json!({ "mode": "sea", "SHIP_NAME": "Caribe" }),
            Haulage::Sea {
                ship_name: "Caribe".to_owned(),
            },
        ),
    ] {
        assert_eq!(
            serde_json::from_value::<Haulage>(stored.clone()).unwrap(),
            haulage
        );
        assert_eq!(
            read_counting!(Haulage, haulage_schema, stored, calls),
            Ok(haulage)
        );
    }
    assert_eq!(calls, 0);

    assert_eq!(
        listed!(
            Haulage,
            haulage_schema,
            json!({ "mode": "sea", "SHIP_NAME": 7_u32 })
        ),
        [
            "SHIP_NAME: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
    assert_eq!(
        listed!(
            Haulage,
            haulage_schema,
            json!({ "mode": "air", "flight_code": "IB6500" })
        ),
        [
            "flightCode: missing: expected String",
            "flight_code: unknown: found String(\"IB6500\")",
        ]
    );
    let written = serde_json::to_value(Haulage::Air {
        flight_code: "IB6500".to_owned(),
    })
    .unwrap();
    assert_eq!(written, json!({ "mode": "byAir", "flight_code": "IB6500" }));
    assert_eq!(
        listed!(Haulage, haulage_schema, written),
        [
            "mode: invalid: expected Variants([\"air\", \"over-land\", \"sea\"]), found String(\"byAir\"): unknown variant `byAir`, expected one of `air`, `over-land`, `sea`"
        ]
    );
}

#[test]
fn a_variant_is_under_the_key_serde_reads_it_as() {
    let mut calls = 0_u32;
    for (stored, relay) in [
        (
            json!({ "relay": { "byHand": { "badge_number": 7_u32 } } }),
            Relay::Courier {
                badge_number: 7_u32,
            },
        ),
        (
            json!({ "relay": { "drop-box": 12_u32 } }),
            Relay::DropBox(12_u32),
        ),
        (json!({ "relay": "not-sent" }), Relay::NotSent),
    ] {
        let dispatch = Dispatch { relay };
        assert_eq!(
            serde_json::from_value::<Dispatch>(stored.clone()).unwrap(),
            dispatch
        );
        assert_eq!(
            read_counting!(Dispatch, dispatch_schema, stored, calls),
            Ok(dispatch)
        );
    }
    assert_eq!(calls, 0);

    assert_eq!(
        listed!(
            Dispatch,
            dispatch_schema,
            json!({ "relay": { "byHand": { "badge_number": "seven" } } })
        ),
        [
            "relay.byHand.badge_number: invalid: expected U32, found String(\"seven\"): invalid type: string \"seven\", expected u32"
        ]
    );
    assert_eq!(
        listed!(Dispatch, dispatch_schema, json!({ "relay": "NotSent" })),
        [
            "relay: invalid: expected Variants([\"byHand\", \"drop-box\", \"not-sent\"]), found String(\"NotSent\"): unknown variant `NotSent`, expected one of `byHand`, `drop-box`, `not-sent`"
        ]
    );
}
