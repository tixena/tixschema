//! A bound written on a field is the recovering read's to list, in a BSON document as in a JSON
//! value: one issue per violation, at the value, in the bound's own words.
//!
//! A validator is published only where a schema surface is on, so a build with none lists nothing.

use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundedGroup {
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    human_name: Option<String>,
    #[serde(default)]
    labels: Vec<BoundedLabel>,
    #[model_schema_prop(minimum = 1)]
    level: i32,
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sort_field: Option<BoundedPath>,
    source: BoundedPath,
    #[model_schema_prop(minLength = 1, maxLength = 2)]
    tags: Vec<String>,
    #[model_schema_prop(maxLength = 3, pattern = "^[a-z]+$")]
    title: String,
}

/// A label held to a rule of its own, which is where a rule for each item of a list is written.
/// A brand's rule is read with its schema surface, so a build with none writes none.
#[cfg_attr(
    any(feature = "typescript", feature = "zod", feature = "jsonschema"),
    model_schema(enable = ["decode_with", "mongodb"], minLength = 2)
)]
#[cfg_attr(
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema")),
    model_schema(enable = ["decode_with", "mongodb"])
)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct BoundedLabel(String);

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct BoundedPath(String);

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum BoundedSlots {
    Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
    Many(#[model_schema_prop(maxLength = 1)] Vec<String>),
    Pair(
        #[model_schema_prop(minLength = 2)] String,
        #[model_schema_prop(minimum = 1)] i32,
    ),
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum BoundedSlotsAdjacent {
    Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
    Pair(
        #[model_schema_prop(minLength = 2)] String,
        #[model_schema_prop(minimum = 1)] i32,
    ),
}

/// An untagged member that is a list, which the read counts to choose the variant.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum BoundedChoice {
    Few(#[model_schema_prop(maxLength = 2)] Vec<String>),
    Text(String),
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BoundedChooser {
    choice: BoundedChoice,
}

#[model_schema(enable = ["decode_with", "mongodb"], minLength = 1, maxLength = 5)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum BoundedByEnum {
    Label { text: String },
    Slot(String),
    Tags { tags: Vec<String> },
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BoundedPair(
    #[model_schema_prop(minLength = 2)] String,
    #[model_schema_prop(minimum = 1)] i32,
);

/// A struct of one slot, which serde writes as the slot's value.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BoundedLone(#[model_schema_prop(minLength = 2)] String);

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BoundedHolder {
    lone: BoundedLone,
    pair: BoundedPair,
}

/// A parcel whose fields serde writes under other names than their own.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundedParcel {
    #[model_schema_prop(minLength = 1)]
    fragile_item_codes: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    #[serde(rename = "label")]
    item_label: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
enum BoundedDispatch {
    ByAir {
        #[model_schema_prop(minLength = 1)]
        flight_number: String,
    },
}

/// A stored group that breaks every bound `BoundedGroup` writes.
fn broken() -> Document {
    doc! {
        "humanName": "",
        "level": 0_i32,
        "sortField": "",
        "source": "Items",
        "tags": ["ok", "A", "b"],
        "title": "long",
    }
}

/// The group `broken()` stores, built in code.
fn broken_group() -> BoundedGroup {
    BoundedGroup {
        human_name: Some(String::new()),
        labels: Vec::new(),
        level: 0,
        sort_field: Some(BoundedPath(String::new())),
        source: BoundedPath("Items".to_owned()),
        tags: vec!["ok".to_owned(), "A".to_owned(), "b".to_owned()],
        title: "long".to_owned(),
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_broken_bound_is_listed_at_its_value_once_per_violation() {
    let mut calls = 0_u32;
    let read = BoundedGroup::from_bson_with(broken(), |_raw, _found| {
        calls += 1;
        bounded_group_schema::Verdict::Reject
    });
    assert_eq!(
        read.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
        [
            "humanName: invalid: expected Optional(String), found String(\"\"): too short: minimum \
             length is 1, got 0",
            "level: invalid: expected I32, found Int32(0): too small: minimum is 1, got 0",
            "sortField: invalid: expected Model(\"BoundedPath\"), found String(\"\"): too short: \
             minimum length is 1, got 0",
            "tags: invalid: expected Array(String), found Array([String(\"ok\"), String(\"A\"), \
             String(\"b\")]): too long: maximum length is 2, got 3",
            "title: invalid: expected String, found String(\"long\"): too long: maximum length is \
             3, got 4",
        ]
    );
    assert_eq!(calls, 1);
}

/// The issue's path holds the key serde writes the field under, and its reason holds no name.
#[test]
fn the_reason_of_a_renamed_fields_bound_holds_no_name() {
    let mut listed: Vec<String> = Vec::new();
    let parcel = BoundedParcel::from_bson_with(
        doc! { "fragileItemCodes": [], "label": "" },
        |_raw, found| {
            listed.extend(found.iter().map(|issue| format!("{issue:?}")));
            bounded_parcel_schema::Verdict::Reject
        },
    );
    let mut in_a_variant: Vec<String> = Vec::new();
    let dispatch = BoundedDispatch::from_bson_with(
        doc! { "kind": "ByAir", "flightNumber": "" },
        |_raw, found| {
            in_a_variant.extend(found.iter().map(|issue| format!("{issue:?}")));
            bounded_dispatch_schema::Verdict::Reject
        },
    );
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        assert!(parcel.is_err(), "the callback rejected");
        assert_eq!(
            listed,
            [
                "Invalid { path: Path([Key(\"fragileItemCodes\")]), expected: Array(String), \
                 found: Array([]), reason: \"too short: minimum length is 1, got 0\" }",
                "Invalid { path: Path([Key(\"label\")]), expected: String, found: String(\"\"), \
                 reason: \"too short: minimum length is 1, got 0\" }",
            ]
        );
        assert!(dispatch.is_err(), "the callback rejected");
        assert_eq!(
            in_a_variant,
            [
                "Invalid { path: Path([Key(\"flightNumber\")]), expected: String, found: \
                 String(\"\"), reason: \"too short: minimum length is 1, got 0\" }"
            ]
        );
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(
            parcel,
            Ok(BoundedParcel {
                fragile_item_codes: Vec::new(),
                item_label: String::new(),
            })
        );
        assert_eq!(
            dispatch,
            Ok(BoundedDispatch::ByAir {
                flight_number: String::new()
            })
        );
        assert_eq!((listed, in_a_variant), (Vec::new(), Vec::new()));
    }
}

/// A list that breaks its bound is one issue at the list, whatever its items are.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_list_that_breaks_its_bound_is_one_issue_at_the_list() {
    let stored = doc! { "level": 1_i32, "source": "Items", "tags": [], "title": "ok" };
    let read =
        BoundedGroup::from_bson_with(stored, |_raw, _found| bounded_group_schema::Verdict::Reject);
    assert_eq!(
        read.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
        [
            "tags: invalid: expected Array(String), found Array([]): too short: minimum length is 1, got 0"
        ]
    );
}

/// A callback puts a list inside its bound at the issue's path, and the read answers it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_callback_that_sets_a_list_at_its_path_recovers_the_read() {
    use bounded_group_schema::{Issue, Verdict};

    let stored = doc! { "level": 1_i32, "source": "Items", "tags": ["a", "b", "c"], "title": "ok" };
    let read = BoundedGroup::from_bson_with(stored, |raw, found| {
        let [
            Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            },
        ] = found
        else {
            return Verdict::Reject;
        };
        path.set_in_document(
            raw,
            Bson::Array(vec![
                Bson::String("a".to_owned()),
                Bson::String("b".to_owned()),
            ]),
        );
        Verdict::Fixed
    });
    assert_eq!(read.unwrap().tags, ["a", "b"]);
}

#[test]
fn a_broken_bound_in_a_tagged_slot_is_listed_at_the_slot() {
    use bounded_slots_adjacent_schema::Verdict as AdjacentVerdict;
    use bounded_slots_schema::Verdict;

    let held = BoundedSlots::from_bson_with(doc! { "Code": "ABC" }, |_, _| Verdict::Reject);
    assert_eq!(held, Ok(BoundedSlots::Code("ABC".to_owned())));

    let code = BoundedSlots::from_bson_with(doc! { "Code": "abc" }, |_, _| Verdict::Reject);
    let pair = BoundedSlots::from_bson_with(doc! { "Pair": ["a", 0_i32] }, |_, _| Verdict::Reject);
    let adjacent_code =
        BoundedSlotsAdjacent::from_bson_with(doc! { "data": "abc", "kind": "Code" }, |_, _| {
            AdjacentVerdict::Reject
        });
    let adjacent_pair = BoundedSlotsAdjacent::from_bson_with(
        doc! { "data": ["a", 0_i32], "kind": "Pair" },
        |_, _| AdjacentVerdict::Reject,
    );
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        assert_eq!(
            code.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
            [
                "Code: invalid: expected String, found String(\"abc\"): does not match pattern \
                 '^[A-Z]{3}$'"
            ]
        );
        assert_eq!(
            pair.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
            [
                "Pair[0]: invalid: expected String, found String(\"a\"): too short: minimum \
                 length is 2, got 1",
                "Pair[1]: invalid: expected I32, found Int32(0): too small: minimum is 1, got 0",
            ]
        );
        assert_eq!(
            adjacent_code
                .unwrap_err()
                .to_string()
                .lines()
                .collect::<Vec<&str>>(),
            [
                "data: invalid: expected String, found String(\"abc\"): does not match pattern \
                 '^[A-Z]{3}$'"
            ]
        );
        assert_eq!(
            adjacent_pair
                .unwrap_err()
                .to_string()
                .lines()
                .collect::<Vec<&str>>(),
            [
                "data[0]: invalid: expected String, found String(\"a\"): too short: minimum \
                 length is 2, got 1",
                "data[1]: invalid: expected I32, found Int32(0): too small: minimum is 1, got 0",
            ]
        );
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(code, Ok(BoundedSlots::Code("abc".to_owned())));
        assert_eq!(pair, Ok(BoundedSlots::Pair("a".to_owned(), 0)));
        assert_eq!(
            adjacent_code,
            Ok(BoundedSlotsAdjacent::Code("abc".to_owned()))
        );
        assert_eq!(
            adjacent_pair,
            Ok(BoundedSlotsAdjacent::Pair("a".to_owned(), 0))
        );
    }
}

#[test]
fn a_list_in_a_variants_slot_is_listed_once_at_the_slot() {
    use bounded_slots_schema::Verdict;

    let held = BoundedSlots::from_bson_with(doc! { "Many": ["only"] }, |_, _| Verdict::Reject);
    assert_eq!(held, Ok(BoundedSlots::Many(vec!["only".to_owned()])));
    let many = BoundedSlots::from_bson_with(doc! { "Many": ["a", "b"] }, |_, _| Verdict::Reject);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        many.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
        [
            "Many: invalid: expected Array(String), found Array([String(\"a\"), String(\"b\")]): \
             too long: maximum length is 1, got 2"
        ]
    );
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    assert_eq!(
        many,
        Ok(BoundedSlots::Many(vec!["a".to_owned(), "b".to_owned()]))
    );
}

#[test]
fn an_untagged_list_member_is_read_through_the_hook_that_counts_it() {
    use bounded_chooser_schema::Verdict;

    let few = BoundedChooser::from_bson_with(doc! { "choice": ["a", "b"] }, |_, _| Verdict::Reject);
    assert_eq!(
        few,
        Ok(BoundedChooser {
            choice: BoundedChoice::Few(vec!["a".to_owned(), "b".to_owned()])
        })
    );
    let many =
        BoundedChooser::from_bson_with(doc! { "choice": ["a", "b", "c"] }, |_, _| Verdict::Reject);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        many.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
        [
            "choice: no variant: found Array([String(\"a\"), String(\"b\"), String(\"c\")]), \
             tried Few, Text"
        ]
    );
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    assert!(
        many.is_ok(),
        "no surface hangs the hook, so the list is read"
    );
}

#[test]
fn a_bound_written_on_the_enum_is_listed_at_each_string_it_reaches() {
    use bounded_by_enum_schema::Verdict;

    let held = BoundedByEnum::from_bson_with(doc! { "Slot": "abcde" }, |_, _| Verdict::Reject);
    assert_eq!(held, Ok(BoundedByEnum::Slot("abcde".to_owned())));
    let tags =
        BoundedByEnum::from_bson_with(doc! { "Tags": { "tags": ["", "abcdef"] } }, |_, _| {
            Verdict::Reject
        });
    assert_eq!(
        tags,
        Ok(BoundedByEnum::Tags {
            tags: vec![String::new(), "abcdef".to_owned()]
        })
    );

    let label =
        BoundedByEnum::from_bson_with(doc! { "Label": { "text": "" } }, |_, _| Verdict::Reject);
    let slot = BoundedByEnum::from_bson_with(doc! { "Slot": "abcdef" }, |_, _| Verdict::Reject);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        assert_eq!(
            label
                .unwrap_err()
                .to_string()
                .lines()
                .collect::<Vec<&str>>(),
            [
                "Label.text: invalid: expected String, found String(\"\"): too short: minimum \
                 length is 1, got 0"
            ]
        );
        assert_eq!(
            slot.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
            [
                "Slot: invalid: expected String, found String(\"abcdef\"): too long: maximum \
                 length is 5, got 6"
            ]
        );
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(
            label,
            Ok(BoundedByEnum::Label {
                text: String::new()
            })
        );
        assert_eq!(slot, Ok(BoundedByEnum::Slot("abcdef".to_owned())));
    }
}

#[test]
fn a_broken_bound_in_a_tuple_structs_slot_is_listed_at_the_slot() {
    use bounded_holder_schema::Verdict;

    let held =
        BoundedHolder::from_bson_with(doc! { "lone": "ok", "pair": ["ab", 1_i32] }, |_, _| {
            Verdict::Reject
        });
    assert_eq!(
        held,
        Ok(BoundedHolder {
            lone: BoundedLone("ok".to_owned()),
            pair: BoundedPair("ab".to_owned(), 1),
        })
    );

    let broken =
        BoundedHolder::from_bson_with(doc! { "lone": "a", "pair": ["a", 0_i32] }, |_, _| {
            Verdict::Reject
        });
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        broken
            .unwrap_err()
            .to_string()
            .lines()
            .collect::<Vec<&str>>(),
        [
            "lone: invalid: expected String, found String(\"a\"): too short: minimum length is \
             2, got 1",
            "pair[0]: invalid: expected String, found String(\"a\"): too short: minimum length \
             is 2, got 1",
            "pair[1]: invalid: expected I32, found Int32(0): too small: minimum is 1, got 0",
        ]
    );
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    assert_eq!(
        broken,
        Ok(BoundedHolder {
            lone: BoundedLone("a".to_owned()),
            pair: BoundedPair("a".to_owned(), 0),
        })
    );
}

/// A callback sets each slot at its issue's path, and the read answers the repaired struct.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_callback_that_sets_a_tuple_structs_slots_at_their_paths_recovers_the_read() {
    use bounded_holder_schema::{Issue, Verdict};

    let stored = doc! { "lone": "a", "pair": ["ab", 0_i32] };
    let read = BoundedHolder::from_bson_with(stored, |raw, found| {
        let [
            Issue::Invalid {
                path: lone,
                expected: _lone_expected,
                found: _lone_found,
                reason: _lone_reason,
            },
            Issue::Invalid {
                path: second,
                expected: _second_expected,
                found: _second_found,
                reason: _second_reason,
            },
        ] = found
        else {
            return Verdict::Reject;
        };
        lone.set_in_document(raw, Bson::String("ok".to_owned()));
        second.set_in_document(raw, Bson::Int32(1));
        Verdict::Fixed
    });
    assert_eq!(
        read,
        Ok(BoundedHolder {
            lone: BoundedLone("ok".to_owned()),
            pair: BoundedPair("ab".to_owned(), 1),
        })
    );
}

/// The callback drops each value a bound refuses, and the read answers what is left.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_callback_that_removes_what_breaks_a_bound_recovers_the_read() {
    let stored = doc! {
        "humanName": "", "labels": ["A", "ok"], "level": 1_i32, "sortField": "", "source": "Items",
        "tags": ["A"], "title": "ok",
    };
    let read = BoundedGroup::from_bson_with(stored, |raw, found| {
        for issue in found {
            if let bounded_group_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.remove_from_document(raw);
            }
        }
        bounded_group_schema::Verdict::Fixed
    });
    assert_eq!(
        read,
        Ok(BoundedGroup {
            human_name: None,
            labels: vec![BoundedLabel("ok".to_owned())],
            level: 1,
            sort_field: None,
            source: BoundedPath("Items".to_owned()),
            tags: vec!["A".to_owned()],
            title: "ok".to_owned(),
        })
    );
}

/// A title that breaks two bounds is two issues at one path, alike but for the reason.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn two_bounds_broken_by_one_value_are_repaired_once_at_its_path() {
    use bounded_group_schema::{Issue, Verdict};

    let stored = || {
        doc! { "level": 1_i32, "source": "Items", "tags": ["ok"], "title": "LONG" }
    };
    let listed = BoundedGroup::from_bson_with(stored(), |_raw, _found| Verdict::Reject);
    assert_eq!(
        listed
            .unwrap_err()
            .to_string()
            .lines()
            .collect::<Vec<&str>>(),
        [
            "title: invalid: expected String, found String(\"LONG\"): too long: maximum length is \
             3, got 4",
            "title: invalid: expected String, found String(\"LONG\"): does not match pattern \
             '^[a-z]+$'",
        ]
    );

    let set_once = BoundedGroup::from_bson_with(stored(), |raw, found| {
        let [
            Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            },
            _pattern,
        ] = found
        else {
            return Verdict::Reject;
        };
        path.set_in_document(raw, Bson::String("aa".to_owned()));
        Verdict::Fixed
    });
    assert_eq!(set_once.unwrap().title, "aa");

    let set_for_each = BoundedGroup::from_bson_with(stored(), |raw, found| {
        for issue in found {
            if let Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_document(raw, Bson::String("ok".to_owned()));
            }
        }
        Verdict::Fixed
    });
    assert_eq!(set_for_each.unwrap().title, "ok");
}

/// Serde's own read refuses what breaks a bound, with what `validate()` answers of the value. A
/// build with no schema surface publishes no `validate()`, and reads it.
#[test]
fn plain_serde_refuses_what_breaks_a_bound() {
    let read = BoundedGroup::deserialize(bson::Deserializer::new(Bson::Document(broken())));
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        let refused = read.unwrap_err().to_string();
        let reported = broken_group().validate().unwrap_err().join("; ");
        assert!(refused.contains(&reported), "got: {refused}");
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        let admitted = read.unwrap();
        assert_eq!(admitted, broken_group());
        let mut calls = 0_u32;
        let walked = BoundedGroup::from_bson_with(broken(), |_raw, _found| {
            calls += 1;
            bounded_group_schema::Verdict::Reject
        });
        assert_eq!(walked.unwrap(), admitted);
        assert_eq!(calls, 0);
    }
}
