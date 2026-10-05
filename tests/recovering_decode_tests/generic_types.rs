//! `from_value_with` on types with a type parameter. A value of a parameter's type is read whole
//! with that parameter's own reader, so what fills it needs no flag, and nothing of that type is
//! written back from a JSON value.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

// The JSON schema of a type holding a `Version` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use super::version_schema;
use super::{Version, lines};

#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Page<T> {
    items: Vec<T>,
    total: u32,
}

/// A model type that carries no flag: it only ever fills a parameter.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Plain {
    label: String,
}

/// A generic brand.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Wrapped<T>(T);

#[model_schema(decode_with, default_types(A = String, B = i32))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Pairing<A, B> {
    left: A,
    right: B,
}

/// serde's derive asks `Default` of the parameter here, beyond `Deserialize`.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Lenient<T> {
    #[serde(default)]
    extra: T,
    total: u32,
}

/// A parameter's value under a map, an `Option` and a tuple, a generic type naming another with
/// its own parameter, and itself.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Parcel<T> {
    both: (T, u32),
    extra: HashMap<String, T>,
    inner: Vec<Self>,
    page: Page<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sender: Option<T>,
}

/// Parameters named as the methods the flag adds name their own.
#[model_schema(decode_with, default_types(I = String, F = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Keyed<I, F> {
    format: F,
    id: I,
}

/// A type with no parameter, naming generic types filled with a model type and with text.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Listing {
    page: Page<Version>,
    pages: Vec<Page<String>>,
    wrapped: Wrapped<String>,
}

fn listing() -> Listing {
    Listing {
        page: Page {
            items: vec![Version { number: 1 }],
            total: 1,
        },
        pages: vec![Page {
            items: vec!["a".to_owned()],
            total: 1,
        }],
        wrapped: Wrapped("w".to_owned()),
    }
}

/// An issue inside what fills the parameter is one issue at the field, with serde's message and
/// no path into the item.
#[test]
fn a_value_of_a_parameters_type_is_read_whole_at_its_field() {
    let stored = json!({ "items": [{ "number": 1_i32 }, { "number": "2" }], "total": "two" });
    let read =
        Page::<Version>::from_value_with(stored, |_raw, _found| page_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "items: invalid: expected Array(TypeParam(\"T\")), found Array [Object {\"number\": Number(1)}, Object {\"number\": String(\"2\")}]: invalid type: string \"2\", expected i32",
            "total: invalid: expected U32, found String(\"two\"): invalid type: string \"two\", expected u32",
        ]
    );
    let absent = Page::<Version>::from_value_with(json!({ "legacy": true }), |_raw, _found| {
        page_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&absent.unwrap_err()),
        [
            "items: missing: expected Array(TypeParam(\"T\"))",
            "total: missing: expected U32",
            "legacy: unknown: found Bool(true)",
        ]
    );
}

#[test]
fn a_model_type_filling_a_parameter_needs_no_flag() {
    let page = Page {
        items: vec![Plain {
            label: "a".to_owned(),
        }],
        total: 1,
    };
    let mut calls = 0_u32;
    let read =
        Page::<Plain>::from_value_with(serde_json::to_value(&page).unwrap(), |_raw, _found| {
            calls += 1;
            page_schema::Verdict::Reject
        });
    assert_eq!(read, Ok(page));
    assert_eq!(calls, 0);

    let stored = json!({ "items": [{ "label": 7_i32 }], "total": 1_i32 });
    let refused =
        Page::<Plain>::from_value_with(stored, |_raw, _found| page_schema::Verdict::Reject);
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "items: invalid: expected Array(TypeParam(\"T\")), found Array [Object {\"label\": Number(7)}]: invalid type: integer `7`, expected a string"
        ]
    );
}

/// serde reads a struct from an array of its fields in order. Where the struct's own walker
/// reaches it that is `Mistyped`; where it fills a parameter nothing writes it back to compare.
#[test]
fn a_value_of_a_parameters_type_is_never_mistyped_in_a_json_value() {
    let mut calls = 0_u32;
    let stored = json!({ "left": [3_i32], "right": [[4_i32]] });
    let read = Pairing::<Version, Vec<Version>>::from_value_with(stored, |_raw, _found| {
        calls += 1;
        pairing_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Pairing {
            left: Version { number: 3 },
            right: vec![Version { number: 4 }],
        })
    );
    assert_eq!(calls, 0);
}

/// The whole value of a generic type is not written back either: held as an array of its fields
/// in order, serde reads it and nothing is listed.
#[test]
fn a_generic_type_held_in_another_shape_is_listed_only_where_serde_refuses_it() {
    let mut calls = 0_u32;
    let read = Page::<String>::from_value_with(json!([["a"], 1_i32]), |_raw, _found| {
        calls += 1;
        page_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Page {
            items: vec!["a".to_owned()],
            total: 1,
        })
    );
    assert_eq!(calls, 0);

    let refused =
        Page::<String>::from_value_with(json!("page"), |_raw, _found| page_schema::Verdict::Reject);
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "the value itself: invalid: expected Model(\"Page\"), found String(\"page\"): invalid type: string \"page\", expected struct Page"
        ]
    );
}

#[test]
fn a_struct_with_two_parameters_reads_each_under_its_own_name() {
    let stored = json!({ "left": 1_i32, "right": "x" });
    let read = Pairing::<String, i32>::from_value_with(stored, |raw, found| {
        let seen = lines(&pairing_schema::Unrecovered {
            issues: found.to_vec(),
        });
        assert_eq!(
            seen,
            [
                "left: invalid: expected TypeParam(\"A\"), found Number(1): invalid type: integer `1`, expected a string",
                "right: invalid: expected TypeParam(\"B\"), found String(\"x\"): invalid type: string \"x\", expected i32",
            ]
        );
        for issue in found {
            if let pairing_schema::Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                let fixed = if *expected == pairing_schema::Expected::TypeParam("A") {
                    json!("one")
                } else {
                    json!(2_i32)
                };
                path.set_in_value(raw, fixed);
            }
        }
        pairing_schema::Verdict::Fixed
    });
    assert_eq!(
        read,
        Ok(Pairing {
            left: "one".to_owned(),
            right: 2_i32,
        })
    );
}

/// What the derive asks of the parameter beyond `Deserialize` is carried by the bound on the type
/// itself, so the methods are there wherever serde reads the type.
#[test]
fn a_generic_struct_with_a_defaulted_field_of_the_parameters_type_builds_and_reads() {
    let mut calls = 0_u32;
    let read = Lenient::<String>::from_value_with(json!({ "total": 1_i32 }), |_raw, _found| {
        calls += 1;
        lenient_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Lenient {
            extra: String::new(),
            total: 1,
        })
    );
    assert_eq!(calls, 0);

    let stored = json!({ "extra": 7_i32, "total": 1_i32 });
    let refused =
        Lenient::<String>::from_value_with(stored, |_raw, _found| lenient_schema::Verdict::Reject);
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "extra: invalid: expected TypeParam(\"T\"), found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
}

#[test]
fn a_generic_brand_is_read_as_the_value_it_holds() {
    let read = Wrapped::<u32>::from_value_with(json!("seven"), |_raw, _found| {
        wrapped_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "the value itself: invalid: expected TypeParam(\"T\"), found String(\"seven\"): invalid type: string \"seven\", expected u32"
        ]
    );
    assert_eq!(
        Wrapped::<u32>::from_value_with(json!(7_i32), |_raw, _found| {
            wrapped_schema::Verdict::Reject
        }),
        Ok(Wrapped(7))
    );
}

/// A list, a map, an `Option` or a tuple that holds a value of the parameter's type is read whole
/// with it. A generic model type is walked by its own walker, and so is the type itself.
#[test]
fn what_holds_a_parameters_value_is_read_whole_and_a_generic_model_type_is_walked() {
    let stored = json!({
        "both": [1_i32, "x"],
        "extra": { "k": 1_i32 },
        "inner": [{
            "both": ["a", 1_i32],
            "extra": {},
            "inner": [],
            "page": { "items": [], "total": "none" },
        }],
        "page": { "items": ["a", 2_i32], "legacy": true, "total": 2_i32 },
        "sender": 4_i32,
    });
    let read =
        Parcel::<String>::from_value_with(stored, |_raw, _found| parcel_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "both: invalid: expected Tuple([TypeParam(\"T\"), U32]), found Array [Number(1), String(\"x\")]: invalid type: integer `1`, expected a string",
            "extra: invalid: expected Map(TypeParam(\"T\")), found Object {\"k\": Number(1)}: invalid type: integer `1`, expected a string",
            "inner[0].page.total: invalid: expected U32, found String(\"none\"): invalid type: string \"none\", expected u32",
            "page.items: invalid: expected Array(TypeParam(\"T\")), found Array [String(\"a\"), Number(2)]: invalid type: integer `2`, expected a string",
            "page.legacy: unknown: found Bool(true)",
            "sender: invalid: expected Optional(TypeParam(\"T\")), found Number(4): invalid type: integer `4`, expected a string",
        ]
    );
}

/// A generic flagged type reached through a field is walked by its own walker at the field's
/// path, whatever fills its parameter.
#[test]
fn a_generic_type_reached_through_a_field_is_walked_at_the_fields_path() {
    let mut calls = 0_u32;
    let written = serde_json::to_value(listing()).unwrap();
    let read = Listing::from_value_with(written, |_raw, _found| {
        calls += 1;
        listing_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(listing()));
    assert_eq!(calls, 0);

    let stored = json!({
        "page": { "draft": true, "items": [{ "number": "1" }], "total": "one" },
        "pages": [{ "items": ["a"], "total": 1_i32 }, { "items": [2_i32] }],
        "wrapped": 5_i32,
    });
    let listed = Listing::from_value_with(stored, |_raw, _found| listing_schema::Verdict::Reject);
    assert_eq!(
        lines(&listed.unwrap_err()),
        [
            "page.items: invalid: expected Array(TypeParam(\"T\")), found Array [Object {\"number\": String(\"1\")}]: invalid type: string \"1\", expected i32",
            "page.total: invalid: expected U32, found String(\"one\"): invalid type: string \"one\", expected u32",
            "page.draft: unknown: found Bool(true)",
            "pages[1].items: invalid: expected Array(TypeParam(\"T\")), found Array [Number(2)]: invalid type: integer `2`, expected a string",
            "pages[1].total: missing: expected U32",
            "wrapped: invalid: expected TypeParam(\"T\"), found Number(5): invalid type: integer `5`, expected a string",
        ]
    );
    let absent = Listing::from_value_with(json!({ "pages": [] }), |_raw, _found| {
        listing_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&absent.unwrap_err()),
        [
            "page: missing: expected Model(\"Page\")",
            "wrapped: missing: expected Model(\"Wrapped\")",
        ]
    );
}

/// The walker of a generic type is handed whatever constructor its caller holds, as any other.
#[test]
fn a_generic_types_walker_builds_the_issues_of_whoever_calls_it() {
    let mut out: Vec<listing_schema::Issue<Value>> = Vec::new();
    Page::<u8>::decode_with_value_issues(
        &json!({ "items": [], "total": true }),
        &[Ok("page".to_owned())],
        listing_schema::issue_from_parts,
        &mut out,
    );
    assert_eq!(
        lines(&listing_schema::Unrecovered { issues: out }),
        [
            "page.total: invalid: expected U32, found Bool(true): invalid type: boolean `true`, expected u32"
        ]
    );
}

/// The methods are generic over a decider and an issue type of their own, which take other names
/// where the type's parameters are called what they would be.
#[test]
fn a_type_whose_parameters_are_named_as_the_methods_own_builds_and_reads() {
    let stored = json!({ "format": "csv", "id": 7_i32 });
    let read =
        Keyed::<u32, String>::from_value_with(stored, |_raw, _found| keyed_schema::Verdict::Reject);
    assert_eq!(
        read,
        Ok(Keyed {
            format: "csv".to_owned(),
            id: 7,
        })
    );
    let refused =
        Keyed::<u32, String>::from_value_with(json!({ "id": "seven" }), |_raw, _found| {
            keyed_schema::Verdict::Reject
        });
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "format: missing: expected TypeParam(\"F\")",
            "id: invalid: expected TypeParam(\"I\"), found String(\"seven\"): invalid type: string \"seven\", expected u32",
        ]
    );
}
