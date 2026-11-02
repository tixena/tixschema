//! A `minimum` or a `maximum` keeps the digits it was written with, past what an `f64` holds, in
//! `validate()`, in the read, in the JSON Schema and in the text of each other surface, and a float
//! limit writes what it always wrote.

#![cfg(feature = "serde")]

#[cfg(not(any(feature = "swift", feature = "kotlin")))]
mod the_top_of_a_u64 {
    use super::{ScaleTop, written};

    #[test]
    fn the_limit_at_the_top_builds_and_keeps_its_digits() {
        assert_eq!(ScaleTop { top: u64::MAX }.validate(), Ok(()));
        assert_eq!(
            written(&ScaleTop::json_schema(), "top", "maximum"),
            "18446744073709551615"
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_bound_is_the_limit_at_the_top_and_no_second_one() {
        let schema = ScaleTop::zod_schema();
        assert!(
            schema.contains(
                "z.bigint().min(0n).max(18446744073709551615n, { error: (issue) => `too large: \
                 maximum is 18446744073709551615, got ${String(issue.input)}` })"
            ),
            "{schema}"
        );
    }
}

#[cfg(feature = "zod")]
mod the_zod_text {
    use super::{ScaleDepth, ScaleMixed, ScaleReading, ScaleSpelling, ScaleVast};

    #[test]
    fn a_limit_past_two_to_the_fifty_third_keeps_its_digits_in_the_bound_and_the_sentence() {
        let schema = ScaleReading::zod_schema();
        assert!(
            schema.contains(
                "z.bigint().min(-9223372036854775808n).max(9007199254740993n, { error: (issue) => \
                 `too large: maximum is 9007199254740993, got ${String(issue.input)}` })"
            ),
            "{schema}"
        );
        assert!(
            schema.contains(
                ".min(1.5, { error: (issue) => `too small: minimum is 1.5, got \
                 ${String(issue.input)}` })"
            ),
            "{schema}"
        );
    }

    #[test]
    fn a_negative_limit_keeps_its_digits() {
        let schema = ScaleDepth::zod_schema();
        assert!(
            schema.contains(
                "z.bigint().max(9223372036854775807n).min(-9007199254740993n, { error: (issue) \
                 => `too small: minimum is -9007199254740993, got ${String(issue.input)}` })"
            ),
            "{schema}"
        );
    }

    #[test]
    fn a_float_limit_on_a_bigint_is_compared_whole_and_stated_as_written() {
        let schema = ScaleMixed::zod_schema();
        assert!(
            schema.contains(
                ".max(1n, { error: (issue) => `too large: maximum is 1.5, got \
                 ${String(issue.input)}` })"
            ),
            "{schema}"
        );
        assert!(
            schema.contains(
                ".max(120, { error: (issue) => `too large: maximum is 120, got \
                 ${String(issue.input)}` })"
            ),
            "{schema}"
        );
    }

    #[test]
    fn a_float_limit_past_two_to_the_fifty_third_states_what_it_always_stated() {
        let schema = ScaleVast::zod_schema();
        assert!(
            schema.contains(
                ".max(1000000000000000000000000000000, { error: (issue) => `too large: maximum is \
                 1000000000000000000000000000000, got ${String(issue.input)}` })"
            ),
            "{schema}"
        );
    }

    #[test]
    fn every_written_form_of_an_integer_reads_as_its_digits() {
        let schema = ScaleSpelling::zod_schema();
        assert_eq!(
            schema
                .matches(".max(1000n, { error: (issue) => `too large: maximum is 1000, got ")
                .count(),
            3,
            "{schema}"
        );
    }
}

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

/// The limit [`ScaleReading`] declares: `9007199254740993`, which no `f64` holds.
const DECLARED: i64 = (1_i64 << 53_i32) + 1;

/// A limit past what an `f64` holds, from above, beside a float limit.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ScaleReading {
    #[model_schema_prop(maximum = 9007199254740993)]
    pub size: i64,
    #[model_schema_prop(minimum = 1.5)]
    pub weight: f64,
}

/// A limit past what an `f64` holds, from below.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ScaleDepth {
    #[model_schema_prop(minimum = -9007199254740993)]
    pub depth: i64,
}

/// One limit in each written form of an integer.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ScaleSpelling {
    #[model_schema_prop(maximum = 0x3E8)]
    pub hexadecimal: i64,
    #[model_schema_prop(maximum = 1_000)]
    pub spaced: i64,
    #[model_schema_prop(maximum = 1000i64)]
    pub suffixed: i64,
}

/// A float limit on an integer, an integer limit on a float, and a whole float limit on a float.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ScaleMixed {
    #[model_schema_prop(maximum = 1.5)]
    pub count: i64,
    #[model_schema_prop(maximum = 120)]
    pub weight: f64,
    #[model_schema_prop(maximum = 10.0)]
    pub whole_float: f64,
}

/// A float limit past what an `f64` holds exactly.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ScaleVast {
    #[model_schema_prop(maximum = 1e30)]
    pub vast: f64,
}

/// A limit at the top of a `u64`, which `swift` and `kotlin` refuse.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ScaleTop {
    #[model_schema_prop(maximum = 18446744073709551615)]
    pub top: u64,
}

const fn reading(size: i64) -> ScaleReading {
    ScaleReading { size, weight: 2.0 }
}

const fn mixed(count: i64, weight: f64) -> ScaleMixed {
    ScaleMixed {
        count,
        weight,
        whole_float: 1.0,
    }
}

/// The number the JSON Schema writes for `keyword` of `member`, as its text.
fn written(schema: &Value, member: &str, keyword: &str) -> String {
    schema["properties"][member][keyword].to_string()
}

#[test]
fn a_limit_past_two_to_the_fifty_third_is_the_limit_written() {
    assert_eq!(reading(DECLARED - 1).validate(), Ok(()));
    assert_eq!(reading(DECLARED).validate(), Ok(()));
    assert_eq!(
        reading(DECLARED + 1).validate(),
        Err(vec![
            "'size': too large: maximum is 9007199254740993, got 9007199254740994".to_owned()
        ])
    );
}

#[test]
fn the_read_refuses_what_is_past_the_limit_written_and_no_more() {
    let read = |size: &str| {
        serde_json::from_str::<ScaleReading>(&format!(r#"{{"size":{size},"weight":2.0}}"#))
    };
    assert_eq!(read("9007199254740992").unwrap().size, DECLARED - 1);
    assert_eq!(read("9007199254740993").unwrap().size, DECLARED);
    assert_eq!(
        read("9007199254740994").unwrap_err().to_string(),
        "'size': too large: maximum is 9007199254740993, got 9007199254740994"
    );
}

#[test]
fn a_negative_limit_keeps_its_digits() {
    let depth = |depth: i64| ScaleDepth { depth }.validate();
    assert_eq!(depth(1 - DECLARED), Ok(()));
    assert_eq!(depth(-DECLARED), Ok(()));
    assert_eq!(
        depth(-DECLARED - 1),
        Err(vec![
            "'depth': too small: minimum is -9007199254740993, got -9007199254740994".to_owned()
        ])
    );
    let schema = ScaleDepth::json_schema();
    assert_eq!(
        schema["properties"]["depth"],
        json!({ "type": "integer", "minimum": -DECLARED })
    );
    assert_eq!(written(&schema, "depth", "minimum"), "-9007199254740993");
}

#[test]
fn the_json_schema_writes_an_integer_limit_as_the_integer_and_a_float_limit_as_written() {
    let reading_schema = ScaleReading::json_schema();
    assert_eq!(
        reading_schema["properties"]["size"],
        json!({ "type": "integer", "maximum": DECLARED })
    );
    assert_eq!(
        written(&reading_schema, "size", "maximum"),
        "9007199254740993"
    );
    assert_eq!(written(&reading_schema, "weight", "minimum"), "1.5");

    let mixed_schema = ScaleMixed::json_schema();
    assert_eq!(written(&mixed_schema, "count", "maximum"), "1.5");
    assert_eq!(written(&mixed_schema, "whole_float", "maximum"), "10.0");
    assert_eq!(written(&mixed_schema, "weight", "maximum"), "120");
    assert_eq!(
        written(&ScaleVast::json_schema(), "vast", "maximum"),
        "1e+30"
    );
}

#[test]
fn an_integer_limit_on_a_float_is_compared_as_that_float() {
    assert_eq!(mixed(1, 120.0).validate(), Ok(()));
    assert_eq!(
        mixed(1, 120.5).validate(),
        Err(vec![
            "'weight': too large: maximum is 120, got 120.5".to_owned()
        ])
    );
}

#[test]
fn a_float_limit_on_an_integer_is_compared_with_its_fraction_dropped_and_stated_as_written() {
    assert_eq!(mixed(1, 1.0).validate(), Ok(()));
    assert_eq!(
        mixed(2, 1.0).validate(),
        Err(vec!["'count': too large: maximum is 1.5, got 2".to_owned()])
    );
}

#[test]
fn a_float_limit_states_what_it_always_stated() {
    let thin = ScaleReading {
        size: 1,
        weight: 1.25,
    };
    assert_eq!(
        thin.validate(),
        Err(vec![
            "'weight': too small: minimum is 1.5, got 1.25".to_owned()
        ])
    );
}

#[test]
fn every_written_form_of_an_integer_reads_as_its_digits() {
    let spelled = |value: i64| {
        ScaleSpelling {
            hexadecimal: value,
            spaced: value,
            suffixed: value,
        }
        .validate()
    };
    assert_eq!(spelled(1000), Ok(()));
    assert_eq!(
        spelled(1001),
        Err(vec![
            "'hexadecimal': too large: maximum is 1000, got 1001".to_owned(),
            "'spaced': too large: maximum is 1000, got 1001".to_owned(),
            "'suffixed': too large: maximum is 1000, got 1001".to_owned(),
        ])
    );
    let schema = ScaleSpelling::json_schema();
    for member in ["hexadecimal", "spaced", "suffixed"] {
        assert_eq!(written(&schema, member, "maximum"), "1000", "{member}");
    }
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_check_and_sentence_write_the_digits() {
    let reading = scale_reading_dart::dart_definition();
    assert!(
        reading.contains(
            "if (size > 9007199254740993) \"'size': too large: maximum is 9007199254740993, got \
             ${size}\""
        ),
        "{reading}"
    );
    let depth = scale_depth_dart::dart_definition();
    assert!(depth.contains("if (depth < -9007199254740993)"), "{depth}");
    let spelled = scale_spelling_dart::dart_definition();
    assert!(spelled.contains("if (spaced > 1000)"), "{spelled}");
    assert!(spelled.contains("if (suffixed > 1000)"), "{spelled}");
    let held = scale_mixed_dart::dart_definition();
    assert!(held.contains("if (count > 1.5)"), "{held}");
    assert!(held.contains("if (weight > 120)"), "{held}");
    let vast = scale_vast_dart::dart_definition();
    assert!(
        vast.contains("if (vast > 1000000000000000000000000000000.0)"),
        "{vast}"
    );
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_check_and_sentence_write_the_digits() {
    let reading = scale_reading_swift::swift_definition();
    assert!(
        reading.contains(
            "if size > 9007199254740993 { errors.append(\"'size': too large: maximum is \
             9007199254740993, got \\(size)\") }"
        ),
        "{reading}"
    );
    let depth = scale_depth_swift::swift_definition();
    assert!(depth.contains("if depth < -9007199254740993"), "{depth}");
    let spelled = scale_spelling_swift::swift_definition();
    assert!(spelled.contains("if spaced > 1000"), "{spelled}");
    assert!(spelled.contains("if suffixed > 1000"), "{spelled}");
    let held = scale_mixed_swift::swift_definition();
    assert!(held.contains("if count > 1 {"), "{held}");
    assert!(held.contains("if weight > 120 {"), "{held}");
    let vast = scale_vast_swift::swift_definition();
    assert!(
        vast.contains("if vast > 1000000000000000000000000000000 {"),
        "{vast}"
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_check_and_sentence_write_the_digits() {
    let reading = scale_reading_kotlin::kotlin_definition();
    assert!(
        reading.contains(
            "if (size > 9007199254740993) violations.add(\"'size': too large: maximum is \
             9007199254740993, got ${size}\")"
        ),
        "{reading}"
    );
    let depth = scale_depth_kotlin::kotlin_definition();
    assert!(depth.contains("if (depth < -9007199254740993)"), "{depth}");
    let spelled = scale_spelling_kotlin::kotlin_definition();
    assert!(spelled.contains("if (spaced > 1000)"), "{spelled}");
    assert!(spelled.contains("if (suffixed > 1000)"), "{spelled}");
    let held = scale_mixed_kotlin::kotlin_definition();
    assert!(held.contains("if (count.toDouble() > 1.5)"), "{held}");
    assert!(held.contains("if (weight > 120.0)"), "{held}");
    let vast = scale_vast_kotlin::kotlin_definition();
    assert!(
        vast.contains("if (vast > 1000000000000000000000000000000.0)"),
        "{vast}"
    );
}
