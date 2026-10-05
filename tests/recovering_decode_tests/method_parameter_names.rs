//! Flagged types that name types their author called `I` and `F`, which is what the methods the
//! flag adds call their own type parameters. Each goes on naming its author's type.

use std::collections::HashMap;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct I {
    number: i32,
}

impl I {
    /// The read hook of [`Scored::score`]: a number that is not negative.
    fn positive<'de, D>(deserializer: D) -> Result<i32, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let number = i32::deserialize(deserializer)?;
        if number < 0_i32 {
            return Err(D::Error::custom("a score is never negative"));
        }
        Ok(number)
    }
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct F {
    label: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Holder {
    inner: I,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Framed {
    frame: F,
}

/// `I` under a map, a list, an `Option` and a tuple.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Collected {
    keyed: HashMap<String, I>,
    listed: Vec<I>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maybe: Option<I>,
    paired: (I, i32),
}

/// Externally tagged, each variant holding `I` in another form.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Carried {
    Alone(I),
    Named { inner: I },
    Paired(I, i32),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Alternate {
    Model(I),
    Text(String),
}

/// `I` is named by the hook alone, which is text and no type of the field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Scored {
    #[serde(deserialize_with = "I::positive")]
    score: i32,
}

#[test]
fn a_field_typed_as_the_walkers_own_parameter_is_called_reads_as_the_authors_type() {
    let mut calls = 0_u32;
    let stored = json!({ "inner": { "number": 1_i32 } });
    assert_eq!(
        read_counting!(Holder, holder_schema, stored, calls),
        Ok(Holder {
            inner: I { number: 1 }
        })
    );
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(
            Holder,
            holder_schema,
            json!({ "inner": { "number": "one" } })
        ),
        [
            "inner.number: invalid: expected I32, found String(\"one\"): invalid type: string \"one\", expected i32"
        ]
    );
}

#[test]
fn a_type_called_as_the_entry_points_own_parameter_is_called_reads_alone_and_in_a_field() {
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(F, f_schema, json!({ "label": "a" }), calls),
        Ok(F {
            label: "a".to_owned()
        })
    );
    let stored = json!({ "frame": { "label": "a" } });
    assert_eq!(
        read_counting!(Framed, framed_schema, stored, calls),
        Ok(Framed {
            frame: F {
                label: "a".to_owned()
            }
        })
    );
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(F, f_schema, json!({ "label": 7_i32 })),
        [
            "label: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
    assert_eq!(
        listed!(
            Framed,
            framed_schema,
            json!({ "frame": { "label": 7_i32 } })
        ),
        [
            "frame.label: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
}

#[test]
fn the_authors_type_is_walked_under_a_map_a_list_an_option_and_a_tuple() {
    let collected = Collected {
        keyed: HashMap::from([("a".to_owned(), I { number: 1 })]),
        listed: vec![I { number: 2 }],
        maybe: Some(I { number: 3 }),
        paired: (I { number: 4 }, 5_i32),
    };
    let mut calls = 0_u32;
    let written = serde_json::to_value(&collected).unwrap();
    assert_eq!(
        read_counting!(Collected, collected_schema, written, calls),
        Ok(collected)
    );
    assert_eq!(calls, 0);

    let stored = json!({
        "keyed": { "a": { "number": "one" } },
        "listed": [{ "number": 2_i32 }, { "number": "two" }],
        "maybe": { "number": "three" },
        "paired": [{ "number": "four" }, 5_i32],
    });
    assert_eq!(
        listed!(Collected, collected_schema, stored),
        [
            "keyed.a.number: invalid: expected I32, found String(\"one\"): invalid type: string \"one\", expected i32",
            "listed[1].number: invalid: expected I32, found String(\"two\"): invalid type: string \"two\", expected i32",
            "maybe.number: invalid: expected I32, found String(\"three\"): invalid type: string \"three\", expected i32",
            "paired[0].number: invalid: expected I32, found String(\"four\"): invalid type: string \"four\", expected i32",
        ]
    );
}

#[test]
fn a_tagged_enums_variant_holding_the_authors_type_is_walked_in_each_form() {
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(
            Carried,
            carried_schema,
            json!({ "Alone": { "number": 1_i32 } }),
            calls
        ),
        Ok(Carried::Alone(I { number: 1 }))
    );
    assert_eq!(calls, 0);
    for (stored, listed) in [
        (
            json!({ "Alone": { "number": "one" } }),
            "Alone.number: invalid: expected I32, found String(\"one\"): invalid type: string \"one\", expected i32",
        ),
        (
            json!({ "Named": { "inner": { "number": "one" } } }),
            "Named.inner.number: invalid: expected I32, found String(\"one\"): invalid type: string \"one\", expected i32",
        ),
        (
            json!({ "Paired": [{ "number": "one" }, 2_i32] }),
            "Paired[0].number: invalid: expected I32, found String(\"one\"): invalid type: string \"one\", expected i32",
        ),
    ] {
        assert_eq!(listed!(Carried, carried_schema, stored), [listed]);
    }
}

/// serde reads an object with one key more as the variant holding `I`, whose walker lists the key.
#[test]
fn an_untagged_enums_variant_holding_the_authors_type_is_walked() {
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(
            Alternate,
            alternate_schema,
            json!({ "number": 1_i32 }),
            calls
        ),
        Ok(Alternate::Model(I { number: 1 }))
    );
    assert_eq!(
        read_counting!(Alternate, alternate_schema, json!("one"), calls),
        Ok(Alternate::Text("one".to_owned()))
    );
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(
            Alternate,
            alternate_schema,
            json!({ "legacy": true, "number": 1_i32 })
        ),
        ["legacy: unknown: found Bool(true)"]
    );
}

#[test]
fn a_hook_reached_through_the_authors_type_is_the_authors_function() {
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Scored, scored_schema, json!({ "score": 7_i32 }), calls),
        Ok(Scored { score: 7_i32 })
    );
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(Scored, scored_schema, json!({ "score": -1_i32 })),
        ["score: invalid: expected I32, found Number(-1): a score is never negative"]
    );
}
