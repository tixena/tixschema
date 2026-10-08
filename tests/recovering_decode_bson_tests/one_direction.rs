//! A renaming written as a list that names one key for reading and another, or none, for
//! writing: each value is looked up under the `deserialize` side, where plain serde reads it.
//!
//! A build that describes a type refuses such a list, so this module is compiled where nothing
//! describes one.

use bson::{Bson, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::{Told, invalid, missing, serde_reads, string, unknown, written};

/// What `from_bson_with` lists for `$stored`, less each `reason`, read as `$model` by a decider
/// that rejects it.
macro_rules! listed {
    ($model:ty, $module:ident, $stored:expr) => {
        <$model>::from_bson_with($stored, |_raw, _found| $module::Verdict::Reject)
            .unwrap_err()
            .issues
            .iter()
            .map(|issue| match issue {
                $module::Issue::Invalid {
                    path,
                    expected,
                    found,
                    reason: _reason,
                } => (
                    "Invalid",
                    path.to_string(),
                    format!("{expected:?}"),
                    Some(found.clone()),
                ),
                $module::Issue::Missing { path, expected } => {
                    ("Missing", path.to_string(), format!("{expected:?}"), None)
                }
                $module::Issue::Unknown { path, found } => (
                    "Unknown",
                    path.to_string(),
                    String::new(),
                    Some(found.clone()),
                ),
                $module::Issue::Mistyped { .. }
                | $module::Issue::NoVariant { .. }
                | $module::Issue::Undescribed { .. } => {
                    ("Other", String::new(), String::new(), None)
                }
            })
            .collect::<Vec<Told>>()
    };
}

/// What `from_bson_with` reads `$stored` as, by a decider that counts each run of its own into
/// `$calls`.
macro_rules! read_counting {
    ($model:ty, $module:ident, $stored:expr, $calls:ident) => {
        <$model>::from_bson_with($stored, |_raw, _found| {
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
fn a_row_under_the_keys_serde_reads_lists_no_issue() {
    let stored_row = doc! { "read_too": 2_i32, "read": 1_i32, "plain": 3_i32 };
    assert!(serde_reads::<ReadRenamed>(&stored_row));
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(ReadRenamed, read_renamed_schema, stored_row, calls),
        Ok(ReadRenamed {
            both_ways: 2_u32,
            one_way: 1_u32,
            plain: 3_u32,
        })
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_wrong_value_under_a_read_key_is_listed_at_that_key_and_fixed_there() {
    let stored_row = doc! { "read_too": "two", "read": 1_i32, "plain": 3_i32 };
    let mut seen: Vec<Told> = Vec::new();
    let read = ReadRenamed::from_bson_with(stored_row, |raw, found| {
        for issue in found {
            if let read_renamed_schema::Issue::Invalid {
                path,
                expected,
                found: held,
                reason: _reason,
            } = issue
            {
                seen.push((
                    "Invalid",
                    path.to_string(),
                    format!("{expected:?}"),
                    Some(held.clone()),
                ));
                path.set_in_document(raw, Bson::Int32(2_i32));
            }
        }
        read_renamed_schema::Verdict::Fixed
    });
    assert_eq!(seen, [invalid("read_too", "U32", string("two"))]);
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
            doc! { "both_ways": 2_i32, "one_way": 1_i32, "plain": 3_i32 }
        ),
        [
            missing("read_too", "U32"),
            missing("read", "U32"),
            unknown("both_ways", Bson::Int32(2_i32)),
            unknown("one_way", Bson::Int32(1_i32)),
        ]
    );
    let written_row = written(&ReadRenamed {
        both_ways: 2_u32,
        one_way: 1_u32,
        plain: 3_u32,
    });
    assert_eq!(
        written_row,
        doc! { "written": 2_i64, "one_way": 1_i64, "plain": 3_i64 }
    );
    assert_eq!(
        listed!(ReadRenamed, read_renamed_schema, written_row),
        [
            missing("read_too", "U32"),
            missing("read", "U32"),
            unknown("written", Bson::Int64(2_i64)),
            unknown("one_way", Bson::Int64(1_i64)),
        ]
    );
}

/// The key of the field serde never reads is the one serde writes it under, which the rule
/// written for reading does not case.
#[test]
fn a_rule_written_for_reading_cases_the_keys_serde_reads() {
    let stored_row = doc! { "firstName": "Ada", "kept_note": "n", "lastName": "Lovelace" };
    assert!(serde_reads::<ReadCased>(&stored_row));
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(ReadCased, read_cased_schema, stored_row, calls),
        Ok(read_cased())
    );
    assert_eq!(calls, 0);

    assert_eq!(
        listed!(
            ReadCased,
            read_cased_schema,
            doc! { "firstName": 7_i32, "lastName": "Lovelace" }
        ),
        [invalid("firstName", "String", Bson::Int32(7_i32))]
    );
    let written_row = written(&ReadCased {
        kept_note: "n".to_owned(),
        ..read_cased()
    });
    assert_eq!(
        written_row,
        doc! { "first_name": "Ada", "kept_note": "n", "written_last": "Lovelace" }
    );
    assert_eq!(
        listed!(ReadCased, read_cased_schema, written_row),
        [
            missing("firstName", "String"),
            missing("lastName", "String"),
            unknown("first_name", string("Ada")),
            unknown("written_last", string("Lovelace")),
        ]
    );
}

#[test]
fn a_tag_names_a_variant_by_the_name_serde_reads() {
    let mut calls = 0_u32;
    for (stored_row, haulage) in [
        (
            doc! { "mode": "air", "flightCode": "IB6500" },
            Haulage::Air {
                flight_code: "IB6500".to_owned(),
            },
        ),
        (
            doc! { "mode": "over-land", "roadName": "DR-1" },
            Haulage::OverLand {
                road_name: "DR-1".to_owned(),
            },
        ),
        (
            doc! { "mode": "sea", "SHIP_NAME": "Caribe" },
            Haulage::Sea {
                ship_name: "Caribe".to_owned(),
            },
        ),
    ] {
        assert!(serde_reads::<Haulage>(&stored_row), "for {stored_row}");
        assert_eq!(
            read_counting!(Haulage, haulage_schema, stored_row, calls),
            Ok(haulage)
        );
    }
    assert_eq!(calls, 0);

    assert_eq!(
        listed!(
            Haulage,
            haulage_schema,
            doc! { "mode": "sea", "SHIP_NAME": 7_i32 }
        ),
        [invalid("SHIP_NAME", "String", Bson::Int32(7_i32))]
    );
    assert_eq!(
        listed!(
            Haulage,
            haulage_schema,
            doc! { "mode": "air", "flight_code": "IB6500" }
        ),
        [
            missing("flightCode", "String"),
            unknown("flight_code", string("IB6500")),
        ]
    );
    let written_row = written(&Haulage::Air {
        flight_code: "IB6500".to_owned(),
    });
    assert_eq!(
        written_row,
        doc! { "mode": "byAir", "flight_code": "IB6500" }
    );
    assert_eq!(
        listed!(Haulage, haulage_schema, written_row),
        [invalid(
            "mode",
            "Variants([\"air\", \"over-land\", \"sea\"])",
            string("byAir"),
        )]
    );
}

#[test]
fn a_variant_is_under_the_key_serde_reads_it_as() {
    let mut calls = 0_u32;
    for (stored_row, relay) in [
        (
            doc! { "relay": { "byHand": { "badge_number": 7_i32 } } },
            Relay::Courier {
                badge_number: 7_u32,
            },
        ),
        (
            doc! { "relay": { "drop-box": 12_i32 } },
            Relay::DropBox(12_u32),
        ),
        (doc! { "relay": "not-sent" }, Relay::NotSent),
    ] {
        assert!(serde_reads::<Dispatch>(&stored_row), "for {stored_row}");
        assert_eq!(
            read_counting!(Dispatch, dispatch_schema, stored_row, calls),
            Ok(Dispatch { relay })
        );
    }
    assert_eq!(calls, 0);

    assert_eq!(
        listed!(
            Dispatch,
            dispatch_schema,
            doc! { "relay": { "byHand": { "badge_number": "seven" } } }
        ),
        [invalid("relay.byHand.badge_number", "U32", string("seven"))]
    );
    assert_eq!(
        listed!(Dispatch, dispatch_schema, doc! { "relay": "NotSent" }),
        [invalid(
            "relay",
            "Variants([\"byHand\", \"drop-box\", \"not-sent\"])",
            string("NotSent"),
        )]
    );
}
