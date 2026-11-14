//! The models and the payloads each emitted client reads a limit from: a limit past what an `f64`
//! holds, on each side of it, at the ends of an `i64`, in each written form, a float limit beside
//! an integer one, and a limit past what an `i32` holds on a float. [`REPORT`] is what each client prints, and what the serde read answers.

use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload: its name, the type it is read as, and the JSON.
pub const ROWS: [(&str, &str, &str); 16] = [
    (
        "below the ceiling",
        "LimitCeiling",
        r#"{"size":9007199254740992}"#,
    ),
    (
        "at the ceiling",
        "LimitCeiling",
        r#"{"size":9007199254740993}"#,
    ),
    (
        "past the ceiling",
        "LimitCeiling",
        r#"{"size":9007199254740994}"#,
    ),
    (
        "above the floor",
        "LimitFloor",
        r#"{"depth":-9007199254740992}"#,
    ),
    (
        "at the floor",
        "LimitFloor",
        r#"{"depth":-9007199254740993}"#,
    ),
    (
        "past the floor",
        "LimitFloor",
        r#"{"depth":-9007199254740994}"#,
    ),
    (
        "written forms at the limit",
        "LimitForms",
        r#"{"spaced":1000,"suffixed":1000}"#,
    ),
    (
        "written forms past the limit",
        "LimitForms",
        r#"{"spaced":1001,"suffixed":1001}"#,
    ),
    (
        "floats within",
        "LimitMixed",
        r#"{"count":1,"load":120.0,"weight":1.5}"#,
    ),
    (
        "floats past",
        "LimitMixed",
        r#"{"count":2,"load":120.5,"weight":1.25}"#,
    ),
    (
        "the ends held",
        "LimitEnds",
        r#"{"greatest":9223372036854775807,"least":-9223372036854775808,"reach":1000000000000000001,"risen":9223372036854775807,"sunk":-9223372036854775808}"#,
    ),
    (
        "the ends past",
        "LimitEnds",
        r#"{"greatest":0,"least":0,"reach":1000000000000000002,"risen":9223372036854775806,"sunk":-9223372036854775807}"#,
    ),
    (
        "below nineteen digits",
        "LimitEnds",
        r#"{"greatest":0,"least":0,"reach":1000000000000000000,"risen":9223372036854775807,"sunk":-9223372036854775808}"#,
    ),
    (
        "wide floats within",
        "LimitWide",
        r#"{"held":1.0,"single":1.0,"spelled":1.0,"sunk":1.0,"vast":2000000000.0,"whole":1.0}"#,
    ),
    (
        "wide floats at the limit",
        "LimitWide",
        r#"{"held":3000000000.0,"single":3000000000.0,"spelled":3000000000.0,"sunk":-3000000000.0,"vast":1e30,"whole":3000000000.0}"#,
    ),
    (
        "wide floats past",
        "LimitWide",
        r#"{"held":4000000000.0,"single":4000000000.0,"spelled":4000000000.0,"sunk":-4000000000.0,"vast":1.0,"whole":4000000000.0}"#,
    ),
];

/// What each client's `validate()` and read print of [`ROWS`], and what the serde read answers.
pub const REPORT: &str = "\
below the ceiling: passes
at the ceiling: passes
past the ceiling: refused
    'size': too large: maximum is 9007199254740993, got 9007199254740994
above the floor: passes
at the floor: passes
past the floor: refused
    'depth': too small: minimum is -9007199254740993, got -9007199254740994
written forms at the limit: passes
written forms past the limit: refused
    'spaced': too large: maximum is 1000, got 1001
    'suffixed': too large: maximum is 1000, got 1001
floats within: passes
floats past: refused
    'count': too large: maximum is 1, got 2
    'load': too large: maximum is 120, got 120.5
    'weight': too small: minimum is 1.5, got 1.25
the ends held: passes
the ends past: refused
    'reach': too large: maximum is 1000000000000000001, got 1000000000000000002
    'risen': too small: minimum is 9223372036854775807, got 9223372036854775806
    'sunk': too large: maximum is -9223372036854775808, got -9223372036854775807
below nineteen digits: passes
wide floats within: passes
wide floats at the limit: passes
wide floats past: refused
    'held': too large: maximum is 3000000000, got 4000000000
    'single': too large: maximum is 3000000000, got 4000000000
    'spelled': too large: maximum is 3000000000, got 4000000000
    'sunk': too small: minimum is -3000000000, got -4000000000
    'whole': too large: maximum is 3000000000, got 4000000000
";

/// A limit past what an `f64` holds, from above.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LimitCeiling {
    #[model_schema_prop(maximum = 9007199254740993)]
    pub size: i64,
}

/// A limit past what an `f64` holds, from below.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LimitFloor {
    #[model_schema_prop(minimum = -9007199254740993)]
    pub depth: i64,
}

/// A limit written with a separator and with a suffix.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LimitForms {
    #[model_schema_prop(maximum = 1_000)]
    pub spaced: i64,
    #[model_schema_prop(maximum = 1000i64)]
    pub suffixed: i64,
}

/// A whole float limit on an integer, an integer limit on a float, and a float limit on a float.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LimitMixed {
    #[model_schema_prop(maximum = 1.0)]
    pub count: i64,
    #[model_schema_prop(maximum = 120)]
    pub load: f64,
    #[model_schema_prop(minimum = 1.5)]
    pub weight: f64,
}

/// Each end of an `i64` as a limit from either side, and a limit of nineteen digits.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LimitEnds {
    #[model_schema_prop(maximum = 9223372036854775807)]
    pub greatest: i64,
    #[model_schema_prop(minimum = -9223372036854775808)]
    pub least: i64,
    #[model_schema_prop(maximum = 1000000000000000001)]
    pub reach: i64,
    #[model_schema_prop(minimum = 9223372036854775807)]
    pub risen: i64,
    #[model_schema_prop(maximum = -9223372036854775808)]
    pub sunk: i64,
}

/// Limits past what an `i32` holds on a float: an integer, a float written whole, one from below,
/// an `f32` under an integer and under a float, and one under an `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LimitWide {
    #[model_schema_prop(maximum = 3000000000)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held: Option<f64>,
    #[model_schema_prop(maximum = 3000000000)]
    pub single: f32,
    #[model_schema_prop(maximum = 3e9)]
    pub spelled: f64,
    #[model_schema_prop(minimum = -3000000000)]
    pub sunk: f64,
    #[model_schema_prop(maximum = 1e30)]
    pub vast: f32,
    #[model_schema_prop(maximum = 3000000000)]
    pub whole: f64,
}

/// What the serde read prints of `json`, read as `T`: the messages of a refusal are the ones
/// `validate()` answers.
fn printed<T>(name: &str, json: &str) -> String
where
    T: DeserializeOwned,
{
    match serde_json::from_str::<T>(json) {
        Ok(_) => format!("{name}: passes\n"),
        Err(refused) => {
            let mut said = format!("{name}: refused\n");
            for message in refused.to_string().split("; ") {
                let _ = writeln!(said, "    {message}");
            }
            said
        }
    }
}

#[test]
fn the_serde_read_answers_the_report_every_client_is_held_to() {
    let report: String = ROWS
        .iter()
        .map(|&(name, read_as, json)| match read_as {
            "LimitCeiling" => printed::<LimitCeiling>(name, json),
            "LimitFloor" => printed::<LimitFloor>(name, json),
            "LimitForms" => printed::<LimitForms>(name, json),
            "LimitMixed" => printed::<LimitMixed>(name, json),
            "LimitWide" => printed::<LimitWide>(name, json),
            _ => printed::<LimitEnds>(name, json),
        })
        .collect();
    assert_eq!(report, REPORT);
}
