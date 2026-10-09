//! `from_value_with` on enums, in each form serde writes one: a plain enum as one value, a tagged
//! enum by its tag and the variant it names, and an untagged enum as the variant serde reads the
//! value as. Each case holds the walker's list to what plain serde says of the same value.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

// The JSON schema of a type holding a `Version` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use super::version_schema;
use super::{Version, lines};

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

/// The list each variant of an untagged enum earned in the one `NoVariant` among `$issues`.
macro_rules! tried {
    ($module:ident, $issues:expr) => {
        $issues
            .iter()
            .filter_map(|issue| {
                if let $module::Issue::NoVariant {
                    path: _path,
                    found: _found,
                    variants,
                } = issue
                {
                    Some(variants)
                } else {
                    None
                }
            })
            .flatten()
            .map(|(variant, list)| {
                (
                    *variant,
                    lines(&$module::Unrecovered {
                        issues: list.clone(),
                    }),
                )
            })
            .collect::<Vec<(&str, Vec<String>)>>()
    };
}

/// A plain enum: serde writes the variant's name.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Status {
    Draft,
    Published,
}

/// Externally tagged: `"Empty"`, `{"Label": "x"}`, `{"Circle": {"radius": 1.0}}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Outline {
    Circle { radius: f64 },
    Empty,
    Label(String),
}

/// Internally tagged: `{"kind": "Solid", "color": "red"}`, `{"kind": "Versioned", "number": 3}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Fill {
    Clear,
    Solid { color: String },
    Versioned(Version),
}

/// Adjacently tagged: `{"kind": "Dashed", "data": {"gap": 2}}`, `{"kind": "Hairline"}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Stroke {
    Dashed { gap: u32 },
    Hairline,
    Width(u32),
}

/// One enum of each tagged form.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Drawing {
    fill: Fill,
    outline: Outline,
    stroke: Stroke,
}

/// Untagged, with a member whose own check takes its variant out. Only a schema surface hangs it.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Contact {
    Email {
        #[model_schema_prop(minLength = 3)]
        address: String,
    },
    Phone {
        country: i32,
        digits: String,
    },
    Versioned(Version),
}

/// Untagged, over plain values: text, or a list of numbers.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Token {
    Codes(Vec<u32>),
    Word(String),
}

/// A variant holding two values, and one holding a list of model types.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Move {
    Stay,
    Through(Vec<Version>),
    To(i32, i32),
}

/// An adjacently tagged enum whose variants hold an optional value and two values.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Gauge {
    Level(Option<u32>),
    Off,
    Span(u32, u32),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Route {
    contact: Contact,
    gauge: Gauge,
    next: Move,
    status: Status,
}

/// A brand over a plain enum: serde writes it as the variant's name.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct StatusRef(Status);

/// A brand over a plain enum, under a key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Review {
    status: StatusRef,
}

/// Every renaming serde reads off an enum: of its variants, of one variant, of every variant's
/// fields, and of one variant's fields.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum Shipment {
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    ByAir {
        flight_code: String,
    },
    BySea {
        vessel_name: String,
    },
    #[serde(rename = "pickup")]
    InPerson,
}

/// The same renamings on an externally tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
enum Pace {
    GoBack {
        how_far: u32,
    },
    #[serde(rename = "halt")]
    StandStill,
}

/// A tag naming no variant that serde reads all the same, as the variant marked `other`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Signal {
    Go {
        speed: u32,
    },
    #[serde(other)]
    Unrecognized,
}

/// An internally tagged enum under `rename_all`, held in a list.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Shape {
    Circle { radius: f64 },
    Point,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Canvas {
    shapes: Vec<Shape>,
}

/// A generic enum under each walker that reads the whole value: a `T` is read whole where it sits.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Answer<T> {
    Empty,
    Value(T),
}

#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Reply<T> {
    Failed { reason: String },
    Sent { body: T },
}

#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Either<T> {
    Left { left: T },
    Right { right: u32 },
}

/// Internally tagged, with a variant serde also reads under an alias and one it never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Coating {
    #[serde(alias = "Blank")]
    Clear,
    #[serde(skip_deserializing)]
    Hidden,
    Solid {
        color: String,
    },
}

/// Externally tagged, each variant also read under an alias.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Contour {
    #[serde(alias = "Round")]
    Circle { radius: f64 },
    #[serde(alias = "Blank")]
    Empty,
}

/// Externally tagged: a variant under two aliases, one whose field has an alias of its own, one
/// serde neither writes nor reads, and one it reads and never writes.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Trail {
    #[serde(alias = "Hop", alias = "Leap")]
    Jump(i32, i32),
    #[serde(skip)]
    Lost,
    #[serde(skip_serializing)]
    Old { length: u32 },
    #[serde(alias = "Lane")]
    Road {
        #[serde(alias = "len")]
        length: u32,
    },
}

/// Adjacently tagged, with a variant serde also reads under two aliases and one it never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Dash {
    #[serde(alias = "Dotted", alias = "Broken")]
    Dashed {
        gap: u32,
    },
    #[serde(skip_deserializing)]
    Faded(u32),
    Hairline,
}

/// Untagged, with a variant serde never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Reach {
    Email {
        address: String,
    },
    #[serde(skip_deserializing)]
    Pager {
        number: i32,
    },
    Phone {
        country: i32,
        digits: String,
    },
}

/// What plain serde says of `stored`, read as `T`.
fn serde_reads<T>(stored: &Value) -> bool
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(stored).is_ok()
}

fn drawing() -> Drawing {
    Drawing {
        fill: Fill::Versioned(Version { number: 3_i32 }),
        outline: Outline::Circle { radius: 1.5_f64 },
        stroke: Stroke::Dashed { gap: 2 },
    }
}

fn route() -> Route {
    Route {
        contact: Contact::Phone {
            country: 1_i32,
            digits: "555".to_owned(),
        },
        gauge: Gauge::Span(1, 2),
        next: Move::To(1_i32, 2_i32),
        status: Status::Draft,
    }
}

#[test]
fn what_serde_wrote_for_each_enum_form_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let written = serde_json::to_value(drawing()).unwrap();
    assert_eq!(
        written,
        json!({
            "fill": { "kind": "Versioned", "number": 3_i32 },
            "outline": { "Circle": { "radius": 1.5_f64 } },
            "stroke": { "kind": "Dashed", "data": { "gap": 2_i32 } },
        })
    );
    let read = Drawing::from_value_with(written, |_raw, _found| {
        calls += 1;
        drawing_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(drawing()));

    let units = json!({
        "fill": { "kind": "Clear" },
        "outline": "Empty",
        "stroke": { "kind": "Hairline" },
    });
    let unit_read = Drawing::from_value_with(units, |_raw, _found| {
        calls += 1;
        drawing_schema::Verdict::Reject
    });
    assert_eq!(
        unit_read,
        Ok(Drawing {
            fill: Fill::Clear,
            outline: Outline::Empty,
            stroke: Stroke::Hairline,
        })
    );

    let single = json!({
        "fill": { "kind": "Solid", "color": "red" },
        "outline": { "Label": "x" },
        "stroke": { "kind": "Width", "data": 4_i32 },
    });
    let single_read = Drawing::from_value_with(single, |_raw, _found| {
        calls += 1;
        drawing_schema::Verdict::Reject
    });
    assert_eq!(
        single_read,
        Ok(Drawing {
            fill: Fill::Solid {
                color: "red".to_owned(),
            },
            outline: Outline::Label("x".to_owned()),
            stroke: Stroke::Width(4),
        })
    );

    let routed = serde_json::to_value(route()).unwrap();
    assert_eq!(
        routed,
        json!({
            "contact": { "country": 1_i32, "digits": "555" },
            "gauge": { "kind": "Span", "data": [1_i32, 2_i32] },
            "next": { "To": [1_i32, 2_i32] },
            "status": "Draft",
        })
    );
    let route_read = Route::from_value_with(routed, |_raw, _found| {
        calls += 1;
        route_schema::Verdict::Reject
    });
    assert_eq!(route_read, Ok(route()));
    assert_eq!(calls, 0);
}

/// A name the plain enum does not declare is serde's refusal, at the value.
#[test]
fn a_plain_enum_is_one_value_read_with_its_own_reader() {
    let stored = json!("Archived");
    assert!(!serde_reads::<Status>(&stored));
    assert_eq!(
        listed!(Status, status_schema, stored),
        [
            "the value itself: invalid: expected Model(\"Status\"), found String(\"Archived\"): unknown variant `Archived`, expected `Draft` or `Published`"
        ]
    );
    let mut held = serde_json::to_value(route()).unwrap();
    held["status"] = json!(7_i32);
    assert_eq!(
        listed!(Route, route_schema, held),
        [
            "status: invalid: expected Model(\"Status\"), found Number(7): invalid type: integer `7`, expected string or map"
        ]
    );
}

#[test]
fn a_brand_over_a_plain_enum_is_read_as_the_name_the_enum_writes() {
    let mut calls = 0_u32;
    let written = serde_json::to_value(Review {
        status: StatusRef(Status::Draft),
    })
    .unwrap();
    assert_eq!(written, json!({ "status": "Draft" }));
    assert_eq!(
        read_counting!(Review, review_schema, written, calls),
        Ok(Review {
            status: StatusRef(Status::Draft),
        })
    );
    assert_eq!(
        read_counting!(StatusRef, status_ref_schema, json!("Published"), calls),
        Ok(StatusRef(Status::Published))
    );
    assert_eq!(calls, 0);

    let stored = json!("Archived");
    assert!(!serde_reads::<StatusRef>(&stored));
    assert_eq!(
        listed!(StatusRef, status_ref_schema, stored),
        [
            "the value itself: invalid: expected Model(\"Status\"), found String(\"Archived\"): unknown variant `Archived`, expected `Draft` or `Published`"
        ]
    );
    assert_eq!(
        listed!(Review, review_schema, json!({ "status": "Archived" })),
        [
            "status: invalid: expected Model(\"Status\"), found String(\"Archived\"): unknown variant `Archived`, expected `Draft` or `Published`"
        ]
    );
}

#[test]
fn a_plain_enum_held_as_an_object_serde_reads_is_mistyped() {
    let stored = json!({ "Draft": null });
    assert!(serde_reads::<Status>(&stored));
    assert_eq!(
        listed!(Status, status_schema, stored),
        ["the value itself: mistyped: expected Model(\"Status\"), found Object {\"Draft\": Null}"]
    );
}

#[test]
fn a_tag_naming_no_variant_is_invalid_with_the_variants_the_enum_accepts() {
    let stored = json!({
        "fill": { "color": "red", "kind": "Striped" },
        "outline": { "Hexagon": {} },
        "stroke": { "data": 1_i32, "kind": "Dotted" },
    });
    assert!(!serde_reads::<Drawing>(&stored));
    assert_eq!(
        listed!(Drawing, drawing_schema, stored),
        [
            "fill.kind: invalid: expected Variants([\"Clear\", \"Solid\", \"Versioned\"]), found String(\"Striped\"): unknown variant `Striped`, expected one of `Clear`, `Solid`, `Versioned`",
            "outline: invalid: expected Variants([\"Circle\", \"Empty\", \"Label\"]), found Object {\"Hexagon\": Object {}}: unknown variant `Hexagon`, expected one of `Circle`, `Empty`, `Label`",
            "stroke.kind: invalid: expected Variants([\"Dashed\", \"Hairline\", \"Width\"]), found String(\"Dotted\"): unknown variant `Dotted`, expected one of `Dashed`, `Hairline`, `Width`",
        ]
    );
}

/// A problem inside the variant the tag names reaches the callback at its full path.
#[test]
fn an_issue_inside_a_variant_is_listed_at_its_full_path() {
    let stored = json!({
        "fill": { "draft": true, "kind": "Versioned", "number": "3" },
        "outline": { "Circle": { "extra": 1_i32, "radius": "wide" } },
        "stroke": { "data": {}, "kind": "Dashed" },
    });
    assert!(!serde_reads::<Drawing>(&stored));
    assert_eq!(
        listed!(Drawing, drawing_schema, stored),
        [
            "fill.number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32",
            "fill.draft: unknown: found Bool(true)",
            "outline.Circle.radius: invalid: expected F64, found String(\"wide\"): invalid type: string \"wide\", expected f64",
            "outline.Circle.extra: unknown: found Number(1)",
            "stroke.data.gap: missing: expected U32",
        ]
    );
}

/// A decider repairs a value inside a variant through the path it is handed.
#[test]
fn a_decider_repairs_what_a_variant_holds_by_the_path_it_is_handed() {
    let stored = json!({
        "fill": { "draft": true, "kind": "Versioned", "number": "3" },
        "outline": { "Circle": { "radius": "1.5" } },
        "stroke": { "data": {}, "kind": "Dashed" },
    });
    let read = Drawing::from_value_with(stored, |raw, found| {
        for issue in found {
            if let drawing_schema::Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                let fixed = if *expected == drawing_schema::Expected::F64 {
                    json!(1.5_f64)
                } else {
                    json!(3_i32)
                };
                path.set_in_value(raw, fixed);
            } else if let drawing_schema::Issue::Missing {
                path,
                expected: _expected,
            } = issue
            {
                path.set_in_value(raw, json!(2_i32));
            } else if let drawing_schema::Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_value(raw);
            } else {
                return drawing_schema::Verdict::Reject;
            }
        }
        drawing_schema::Verdict::Fixed
    });
    assert_eq!(read, Ok(drawing()));
}

#[test]
fn a_struct_variants_content_held_as_no_object_is_undescribed() {
    let external = json!({ "Circle": 5_i32 });
    assert!(!serde_reads::<Outline>(&external));
    assert_eq!(
        listed!(Outline, outline_schema, external),
        ["undescribed: invalid type: integer `5`, expected struct variant"]
    );
    let adjacent = json!({ "data": 5_i32, "kind": "Dashed" });
    assert!(!serde_reads::<Stroke>(&adjacent));
    assert_eq!(
        listed!(Stroke, stroke_schema, adjacent),
        ["undescribed: invalid type: integer `5`, expected struct variant Stroke::Dashed"]
    );
}

/// The bare name of a unit variant is no issue.
#[test]
fn an_externally_tagged_value_in_no_form_the_walker_walks_is_read_whole() {
    let keyed = json!({ "Empty": null });
    assert!(serde_reads::<Outline>(&keyed));
    assert_eq!(
        listed!(Outline, outline_schema, keyed),
        [
            "the value itself: mistyped: expected Variants([\"Circle\", \"Empty\", \"Label\"]), found Object {\"Empty\": Null}"
        ]
    );
    let two_keys = json!({ "Circle": { "radius": 1.5_f64 }, "Label": "x" });
    assert!(!serde_reads::<Outline>(&two_keys));
    assert_eq!(
        listed!(Outline, outline_schema, two_keys),
        [
            "the value itself: invalid: expected Variants([\"Circle\", \"Empty\", \"Label\"]), found Object {\"Circle\": Object {\"radius\": Number(1.5)}, \"Label\": String(\"x\")}: invalid value: map, expected map with a single key"
        ]
    );
    assert_eq!(
        listed!(Outline, outline_schema, json!(7_i32)),
        [
            "the value itself: invalid: expected Variants([\"Circle\", \"Empty\", \"Label\"]), found Number(7): invalid type: integer `7`, expected string or map"
        ]
    );
    assert_eq!(
        listed!(Outline, outline_schema, json!({ "Label": 7_i32 })),
        [
            "Label: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
}

/// An absent tag is the one issue, at the tag's key.
#[test]
fn an_absent_tag_is_missing_at_the_tags_key() {
    let internal = json!({ "color": "red" });
    assert!(!serde_reads::<Fill>(&internal));
    assert_eq!(
        listed!(Fill, fill_schema, internal),
        ["kind: missing: expected Variants([\"Clear\", \"Solid\", \"Versioned\"])"]
    );
    let adjacent = json!({ "data": { "gap": 2_i32 } });
    assert!(!serde_reads::<Stroke>(&adjacent));
    assert_eq!(
        listed!(Stroke, stroke_schema, adjacent),
        ["kind: missing: expected Variants([\"Dashed\", \"Hairline\", \"Width\"])"]
    );
}

/// A tag naming no variant is `Mistyped` where serde reads the whole value all the same.
#[test]
fn a_tag_naming_no_variant_that_serde_reads_is_mistyped() {
    let stored = json!({ "kind": "Warp", "speed": 9_i32 });
    assert_eq!(
        serde_json::from_value::<Signal>(stored.clone()).unwrap(),
        Signal::Unrecognized
    );
    assert_eq!(
        listed!(Signal, signal_schema, stored),
        ["kind: mistyped: expected Variants([\"Go\", \"Unrecognized\"]), found String(\"Warp\")"]
    );
    let refused = json!({ "kind": 7.5_f64 });
    assert!(!serde_reads::<Signal>(&refused));
    assert_eq!(
        listed!(Signal, signal_schema, refused),
        [
            "kind: invalid: expected Variants([\"Go\", \"Unrecognized\"]), found Number(7.5): invalid type: floating point `7.5`, expected variant identifier"
        ]
    );
}

#[test]
fn a_tagged_enum_is_an_object_whose_other_keys_are_the_variants() {
    assert_eq!(
        listed!(Fill, fill_schema, json!("Clear")),
        [
            "the value itself: invalid: expected Model(\"Fill\"), found String(\"Clear\"): invalid type: string \"Clear\", expected internally tagged enum Fill"
        ]
    );
    let listing = json!(["Solid", "red"]);
    assert!(serde_reads::<Fill>(&listing));
    assert_eq!(
        listed!(Fill, fill_schema, listing),
        [
            "the value itself: mistyped: expected Model(\"Fill\"), found Array [String(\"Solid\"), String(\"red\")]"
        ]
    );
    let internal = json!({ "extra": 1_i32, "kind": "Clear" });
    assert!(serde_reads::<Fill>(&internal));
    assert_eq!(
        listed!(Fill, fill_schema, internal),
        ["extra: unknown: found Number(1)"]
    );
    let adjacent = json!({ "extra": 1_i32, "kind": "Hairline" });
    assert!(serde_reads::<Stroke>(&adjacent));
    assert_eq!(
        listed!(Stroke, stroke_schema, adjacent),
        ["extra: unknown: found Number(1)"]
    );
    let solid = json!({ "extra": 1_i32, "kind": "Solid" });
    assert_eq!(
        listed!(Fill, fill_schema, solid),
        [
            "color: missing: expected String",
            "extra: unknown: found Number(1)",
        ]
    );
}

#[test]
fn an_adjacently_tagged_variants_content_is_walked_under_the_content_key() {
    assert_eq!(
        listed!(Stroke, stroke_schema, json!({ "kind": "Dashed" })),
        ["data: missing: expected Model(\"Stroke\")"]
    );
    assert_eq!(
        listed!(Stroke, stroke_schema, json!({ "kind": "Width" })),
        ["data: missing: expected U32"]
    );
    assert_eq!(
        listed!(
            Stroke,
            stroke_schema,
            json!({ "data": "wide", "kind": "Width" })
        ),
        [
            "data: invalid: expected U32, found String(\"wide\"): invalid type: string \"wide\", expected u32"
        ]
    );
    assert_eq!(
        listed!(
            Gauge,
            gauge_schema,
            json!({ "data": [1_i32, "2", 3_i32], "kind": "Span" })
        ),
        [
            "data[1]: invalid: expected U32, found String(\"2\"): invalid type: string \"2\", expected u32",
            "data[2]: unknown: found Number(3)",
        ]
    );
    assert_eq!(
        listed!(
            Gauge,
            gauge_schema,
            json!({ "data": 7_i32, "kind": "Span" })
        ),
        ["data: invalid: expected Tuple([U32, U32]), found Number(7): not an array"]
    );
    assert_eq!(
        listed!(Gauge, gauge_schema, json!({ "kind": "Span" })),
        ["data: missing: expected Tuple([U32, U32])"]
    );
}

#[test]
fn an_adjacently_tagged_unit_variant_walks_nothing_under_the_content_key() {
    let mut calls = 0_u32;
    let read = Stroke::from_value_with(
        json!({ "data": null, "kind": "Hairline" }),
        |_raw, _found| {
            calls += 1;
            stroke_schema::Verdict::Reject
        },
    );
    assert_eq!(read, Ok(Stroke::Hairline));
    assert_eq!(calls, 0);
    let stored = json!({ "data": 5_i32, "kind": "Hairline" });
    assert!(!serde_reads::<Stroke>(&stored));
    assert_eq!(
        listed!(Stroke, stroke_schema, stored),
        ["undescribed: invalid type: integer `5`, expected unit variant Stroke::Hairline"]
    );
}

/// serde reads a single optional value as `None` where the content key is missing.
#[test]
fn an_adjacently_tagged_optional_value_reads_its_absence() {
    let mut calls = 0_u32;
    for stored in [
        json!({ "kind": "Level" }),
        json!({ "data": null, "kind": "Level" }),
    ] {
        let read = Gauge::from_value_with(stored, |_raw, _found| {
            calls += 1;
            gauge_schema::Verdict::Reject
        });
        assert_eq!(read, Ok(Gauge::Level(None)));
    }
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(
            Gauge,
            gauge_schema,
            json!({ "data": "high", "kind": "Level" })
        ),
        [
            "data: invalid: expected Optional(U32), found String(\"high\"): invalid type: string \"high\", expected u32"
        ]
    );
}

#[test]
fn what_a_variant_holds_is_walked_by_its_kind() {
    let mut stored = serde_json::to_value(route()).unwrap();
    stored["next"] = json!({ "To": [1_i32, "2", 3_i32] });
    assert!(!serde_reads::<Route>(&stored));
    assert_eq!(
        listed!(Route, route_schema, stored),
        [
            "next.To[1]: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "next.To[2]: unknown: found Number(3)",
        ]
    );
    assert_eq!(
        listed!(Move, move_schema, json!({ "To": [1_i32] })),
        ["To[1]: missing: expected I32"]
    );
    assert_eq!(
        listed!(Move, move_schema, json!({ "To": 7_i32 })),
        ["To: invalid: expected Tuple([I32, I32]), found Number(7): not an array"]
    );
    assert_eq!(
        listed!(
            Move,
            move_schema,
            json!({ "Through": [{ "number": 1_i32 }, { "draft": true, "number": "2" }] })
        ),
        [
            "Through[1].number: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "Through[1].draft: unknown: found Bool(true)",
        ]
    );
}

#[test]
fn an_untagged_value_no_variant_reads_is_one_no_variant_with_each_variants_list() {
    let stored = json!({ "country": "one", "digits": "555" });
    assert!(!serde_reads::<Contact>(&stored));
    let read = Contact::from_value_with(stored, |_raw, _found| contact_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        lines(&contact_schema::Unrecovered {
            issues: issues.clone(),
        }),
        [
            "the value itself: no variant: found Object {\"country\": String(\"one\"), \"digits\": String(\"555\")}, tried Email, Phone, Versioned"
        ]
    );
    assert_eq!(
        tried!(contact_schema, issues),
        [
            (
                "Email",
                vec![
                    "address: missing: expected String".to_owned(),
                    "country: unknown: found String(\"one\")".to_owned(),
                    "digits: unknown: found String(\"555\")".to_owned(),
                ]
            ),
            (
                "Phone",
                vec![
                    "country: invalid: expected I32, found String(\"one\"): invalid type: string \"one\", expected i32"
                        .to_owned()
                ]
            ),
            (
                "Versioned",
                vec![
                    "number: missing: expected I32".to_owned(),
                    "country: unknown: found String(\"one\")".to_owned(),
                    "digits: unknown: found String(\"555\")".to_owned(),
                ]
            ),
        ]
    );
}

/// The one `NoVariant` sits at the path the enum sits at, and so does every issue in its lists.
#[test]
fn a_no_variant_inside_another_type_sits_at_the_enums_path() {
    let mut stored = serde_json::to_value(route()).unwrap();
    stored["contact"] = json!(7_i32);
    let read = Route::from_value_with(stored, |_raw, _found| route_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        lines(&route_schema::Unrecovered {
            issues: issues.clone(),
        }),
        ["contact: no variant: found Number(7), tried Email, Phone, Versioned"]
    );
    assert_eq!(
        tried!(route_schema, issues),
        [
            (
                "Email",
                vec![
                    "contact: invalid: expected Model(\"Contact\"), found Number(7): data did not match any variant of untagged enum Contact"
                        .to_owned()
                ]
            ),
            (
                "Phone",
                vec![
                    "contact: invalid: expected Model(\"Contact\"), found Number(7): data did not match any variant of untagged enum Contact"
                        .to_owned()
                ]
            ),
            (
                "Versioned",
                vec![
                    "contact: invalid: expected Model(\"Version\"), found Number(7): invalid type: integer `7`, expected struct Version"
                        .to_owned()
                ]
            ),
        ]
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_untagged_enums_constrained_member_is_read_through_its_hook() {
    let stored = json!({ "address": "ab" });
    assert!(!serde_reads::<Contact>(&stored));
    let read = Contact::from_value_with(stored, |_raw, _found| contact_schema::Verdict::Reject);
    assert_eq!(
        tried!(contact_schema, read.unwrap_err().issues),
        [
            (
                "Email",
                vec![
                    "address: invalid: expected String, found String(\"ab\"): 'address': too short: minimum length is 3, got 2"
                        .to_owned()
                ]
            ),
            (
                "Phone",
                vec![
                    "country: missing: expected I32".to_owned(),
                    "digits: missing: expected String".to_owned(),
                    "address: unknown: found String(\"ab\")".to_owned(),
                ]
            ),
            (
                "Versioned",
                vec![
                    "number: missing: expected I32".to_owned(),
                    "address: unknown: found String(\"ab\")".to_owned(),
                ]
            ),
        ]
    );
}

/// serde says which variant reads the value, and that variant alone is walked.
#[test]
fn an_untagged_value_is_walked_as_the_variant_serde_reads_it_as() {
    let email = json!({ "address": "ann@example.org", "legacy": true });
    assert_eq!(
        serde_json::from_value::<Contact>(email.clone()).unwrap(),
        Contact::Email {
            address: "ann@example.org".to_owned(),
        }
    );
    assert_eq!(
        listed!(Contact, contact_schema, email),
        ["legacy: unknown: found Bool(true)"]
    );
    let versioned = json!({ "draft": true, "number": 3_i32 });
    assert_eq!(
        serde_json::from_value::<Contact>(versioned.clone()).unwrap(),
        Contact::Versioned(Version { number: 3_i32 })
    );
    assert_eq!(
        listed!(Contact, contact_schema, versioned),
        ["draft: unknown: found Bool(true)"]
    );
    let mut calls = 0_u32;
    for (stored, token) in [
        (json!("word"), Token::Word("word".to_owned())),
        (json!([1_i32, 2_i32]), Token::Codes(vec![1, 2])),
    ] {
        let read = Token::from_value_with(stored, |_raw, _found| {
            calls += 1;
            token_schema::Verdict::Reject
        });
        assert_eq!(read, Ok(token));
    }
    assert_eq!(calls, 0);
}

/// An untagged variant over a plain value lists what its own reader says of the value.
#[test]
fn an_untagged_variant_over_a_plain_value_is_read_whole() {
    let stored = json!([1_i32, "2"]);
    assert!(!serde_reads::<Token>(&stored));
    let read = Token::from_value_with(stored, |_raw, _found| token_schema::Verdict::Reject);
    assert_eq!(
        tried!(token_schema, read.unwrap_err().issues),
        [
            (
                "Codes",
                vec![
                    "[1]: invalid: expected U32, found String(\"2\"): invalid type: string \"2\", expected u32"
                        .to_owned()
                ]
            ),
            (
                "Word",
                vec![
                    "the value itself: invalid: expected String, found Array [Number(1), String(\"2\")]: invalid type: sequence, expected a string"
                        .to_owned()
                ]
            ),
        ]
    );
}

/// Tags and keys are walked under the names serde writes, and `Variants` lists them.
#[test]
fn variants_and_their_fields_are_walked_under_their_wire_names() {
    let mut calls = 0_u32;
    for shipment in [
        Shipment::ByAir {
            flight_code: "TX1".to_owned(),
        },
        Shipment::BySea {
            vessel_name: "Ada".to_owned(),
        },
        Shipment::InPerson,
    ] {
        let written = serde_json::to_value(&shipment).unwrap();
        let read = Shipment::from_value_with(written, |_raw, _found| {
            calls += 1;
            shipment_schema::Verdict::Reject
        });
        assert_eq!(read, Ok(shipment));
    }
    assert_eq!(calls, 0);
    assert_eq!(
        serde_json::to_value(Shipment::ByAir {
            flight_code: "TX1".to_owned(),
        })
        .unwrap(),
        json!({ "type": "by_air", "FLIGHT_CODE": "TX1" })
    );
    assert_eq!(
        listed!(
            Shipment,
            shipment_schema,
            json!({ "flight_code": "TX1", "type": "by_air" })
        ),
        [
            "FLIGHT_CODE: missing: expected String",
            "flight_code: unknown: found String(\"TX1\")",
        ]
    );
    assert_eq!(
        listed!(
            Shipment,
            shipment_schema,
            json!({ "type": "by_sea", "vesselName": 7_i32 })
        ),
        [
            "vesselName: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
    assert_eq!(
        listed!(Shipment, shipment_schema, json!({ "type": "InPerson" })),
        [
            "type: invalid: expected Variants([\"by_air\", \"by_sea\", \"pickup\"]), found String(\"InPerson\"): unknown variant `InPerson`, expected one of `by_air`, `by_sea`, `pickup`"
        ]
    );

    assert_eq!(
        Pace::from_value_with(json!("halt"), |_raw, _found| pace_schema::Verdict::Reject),
        Ok(Pace::StandStill)
    );
    assert_eq!(
        listed!(
            Pace,
            pace_schema,
            json!({ "goBack": { "howFar": "far", "how_far": 1_i32 } })
        ),
        [
            "goBack.howFar: invalid: expected U32, found String(\"far\"): invalid type: string \"far\", expected u32",
            "goBack.how_far: unknown: found Number(1)",
        ]
    );
    assert_eq!(
        listed!(Pace, pace_schema, json!("StandStill")),
        [
            "the value itself: invalid: expected Variants([\"goBack\", \"halt\"]), found String(\"StandStill\"): unknown variant `StandStill`, expected `goBack` or `halt`"
        ]
    );
}

#[test]
fn a_list_of_tagged_enums_is_walked_item_by_item() {
    let stored = json!({
        "shapes": [{ "kind": "circle", "radius": "2" }, { "kind": "square" }, { "kind": "point" }],
    });
    assert_eq!(
        listed!(Canvas, canvas_schema, stored),
        [
            "shapes[0].radius: invalid: expected F64, found String(\"2\"): invalid type: string \"2\", expected f64",
            "shapes[1].kind: invalid: expected Variants([\"circle\", \"point\"]), found String(\"square\"): unknown variant `square`, expected `circle` or `point`",
        ]
    );
}

#[test]
fn a_tagged_enums_fields_walker_returns_the_keys_that_are_its_own() {
    let versioned = json!({ "draft": true, "kind": "Versioned", "number": "3" });
    let mut out: Vec<fill_schema::Issue<Value>> = Vec::new();
    let declared = Fill::decode_with_value_fields(
        versioned.as_object().unwrap(),
        &[Ok("fill".to_owned())],
        fill_schema::issue_from_parts,
        &mut out,
    );
    assert_eq!(declared, ["number", "kind"]);
    assert_eq!(
        lines(&fill_schema::Unrecovered { issues: out }),
        [
            "fill.number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32"
        ]
    );

    let solid = json!({ "color": "red", "kind": "Solid" });
    let mut none: Vec<fill_schema::Issue<Value>> = Vec::new();
    assert_eq!(
        Fill::decode_with_value_fields(
            solid.as_object().unwrap(),
            &[],
            fill_schema::issue_from_parts,
            &mut none,
        ),
        ["kind", "color"]
    );
    let dashed = json!({ "data": { "gap": 2_i32 }, "kind": "Dashed", "legacy": true });
    assert_eq!(
        Stroke::decode_with_value_fields(
            dashed.as_object().unwrap(),
            &[],
            fill_schema::issue_from_parts,
            &mut none,
        ),
        ["kind", "data"]
    );
    let circle = json!({ "Circle": { "radius": 1.5_f64 }, "legacy": true });
    assert_eq!(
        Outline::decode_with_value_fields(
            circle.as_object().unwrap(),
            &[],
            fill_schema::issue_from_parts,
            &mut none,
        ),
        ["Circle"]
    );
    assert_eq!(none, Vec::new());

    let keyless = json!({ "legacy": true });
    assert_eq!(
        Outline::decode_with_value_fields(
            keyless.as_object().unwrap(),
            &[],
            fill_schema::issue_from_parts,
            &mut none,
        ),
        Vec::<&str>::new()
    );
    assert_eq!(
        lines(&fill_schema::Unrecovered { issues: none }),
        ["the value itself: missing: expected Variants([\"Circle\", \"Empty\", \"Label\"])"]
    );
}

#[test]
fn a_generic_enum_reads_a_parameters_value_whole() {
    let refused = json!({ "Value": { "number": "x" } });
    assert_eq!(
        listed!(Answer::<Version>, answer_schema, refused),
        [
            "Value: invalid: expected TypeParam(\"T\"), found Object {\"number\": String(\"x\")}: invalid type: string \"x\", expected i32"
        ]
    );
    let mut calls = 0_u32;
    for (stored, answer) in [
        (json!("Empty"), Answer::Empty),
        (
            json!({ "Value": { "number": 3_i32 } }),
            Answer::Value(Version { number: 3_i32 }),
        ),
    ] {
        let read = Answer::<Version>::from_value_with(stored, |_raw, _found| {
            calls += 1;
            answer_schema::Verdict::Reject
        });
        assert_eq!(read, Ok(answer));
    }
    assert_eq!(calls, 0);

    assert_eq!(
        listed!(
            Reply::<u32>,
            reply_schema,
            json!({ "body": "none", "kind": "Sent", "legacy": true })
        ),
        [
            "body: invalid: expected TypeParam(\"T\"), found String(\"none\"): invalid type: string \"none\", expected u32",
            "legacy: unknown: found Bool(true)",
        ]
    );
    assert_eq!(
        listed!(Reply::<u32>, reply_schema, json!({ "kind": "Lost" })),
        [
            "kind: invalid: expected Variants([\"Failed\", \"Sent\"]), found String(\"Lost\"): unknown variant `Lost`, expected `Failed` or `Sent`"
        ]
    );
    assert_eq!(
        Either::<String>::from_value_with(json!({ "left": "l" }), |_raw, _found| {
            either_schema::Verdict::Reject
        }),
        Ok(Either::Left {
            left: "l".to_owned(),
        })
    );
    let neither = Either::<String>::from_value_with(json!({ "left": 7_i32 }), |_raw, _found| {
        either_schema::Verdict::Reject
    });
    assert_eq!(
        tried!(either_schema, neither.unwrap_err().issues),
        [
            (
                "Left",
                vec![
                    "left: invalid: expected TypeParam(\"T\"), found Number(7): invalid type: integer `7`, expected a string"
                        .to_owned()
                ]
            ),
            (
                "Right",
                vec![
                    "right: missing: expected U32".to_owned(),
                    "left: unknown: found Number(7)".to_owned(),
                ]
            ),
        ]
    );
}

#[test]
fn a_tag_stored_as_an_alias_names_its_variant_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Coating, coating_schema, json!({ "kind": "Blank" }), calls),
        Ok(Coating::Clear)
    );
    assert_eq!(
        read_counting!(
            Coating,
            coating_schema,
            json!({ "color": "red", "kind": "Solid" }),
            calls
        ),
        Ok(Coating::Solid {
            color: "red".to_owned(),
        })
    );
    assert_eq!(
        read_counting!(Contour, contour_schema, json!("Blank"), calls),
        Ok(Contour::Empty)
    );
    assert_eq!(
        read_counting!(
            Contour,
            contour_schema,
            json!({ "Round": { "radius": 1.5_f64 } }),
            calls
        ),
        Ok(Contour::Circle { radius: 1.5_f64 })
    );
    for key in ["Jump", "Hop", "Leap"] {
        assert_eq!(
            read_counting!(Trail, trail_schema, json!({ key: [1_i32, 2_i32] }), calls),
            Ok(Trail::Jump(1_i32, 2_i32)),
            "for {key}"
        );
    }
    assert_eq!(
        read_counting!(
            Trail,
            trail_schema,
            json!({ "Lane": { "len": 3_i32 } }),
            calls
        ),
        Ok(Trail::Road { length: 3 })
    );
    for tag in ["Dashed", "Dotted", "Broken"] {
        assert_eq!(
            read_counting!(
                Dash,
                dash_schema,
                json!({ "data": { "gap": 2_i32 }, "kind": tag }),
                calls
            ),
            Ok(Dash::Dashed { gap: 2 }),
            "for {tag}"
        );
    }
    assert_eq!(
        read_counting!(Dash, dash_schema, json!({ "kind": "Hairline" }), calls),
        Ok(Dash::Hairline)
    );
    assert_eq!(calls, 0);
}

#[test]
fn an_issue_inside_a_variant_stored_under_an_alias_is_listed_under_the_stored_key() {
    let undeclared = json!({ "Round": { "extra": 1_i32, "radius": 1.5_f64 } });
    assert!(serde_reads::<Contour>(&undeclared));
    assert_eq!(
        listed!(Contour, contour_schema, undeclared),
        ["Round.extra: unknown: found Number(1)"]
    );
    let refused = json!({ "Round": { "radius": "wide" } });
    assert!(!serde_reads::<Contour>(&refused));
    assert_eq!(
        listed!(Contour, contour_schema, refused),
        [
            "Round.radius: invalid: expected F64, found String(\"wide\"): invalid type: string \"wide\", expected f64"
        ]
    );
    let positions = json!({ "Leap": [1_i32, "2", 3_i32] });
    assert!(!serde_reads::<Trail>(&positions));
    assert_eq!(
        listed!(Trail, trail_schema, positions),
        [
            "Leap[1]: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "Leap[2]: unknown: found Number(3)",
        ]
    );
    assert_eq!(
        listed!(Trail, trail_schema, json!({ "Hop": 7_i32 })),
        ["Hop: invalid: expected Tuple([I32, I32]), found Number(7): not an array"]
    );
    let twice_aliased = json!({ "Lane": { "len": "far", "width": 1_i32 } });
    assert!(!serde_reads::<Trail>(&twice_aliased));
    assert_eq!(
        listed!(Trail, trail_schema, twice_aliased),
        [
            "Lane.len: invalid: expected U32, found String(\"far\"): invalid type: string \"far\", expected u32",
            "Lane.width: unknown: found Number(1)",
        ]
    );
    let absent = json!({ "Lane": {} });
    assert!(!serde_reads::<Trail>(&absent));
    assert_eq!(
        listed!(Trail, trail_schema, absent),
        ["Lane.length: missing: expected U32"]
    );
}

#[test]
fn a_decider_repairs_what_an_aliased_variant_holds_by_the_path_it_is_handed() {
    let stored = json!({ "Round": { "radius": "1.5" } });
    let read = Contour::from_value_with(stored, |raw, found| {
        for issue in found {
            if let contour_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                assert_eq!(path.to_string(), "Round.radius");
                path.set_in_value(raw, json!(1.5_f64));
            } else {
                return contour_schema::Verdict::Reject;
            }
        }
        contour_schema::Verdict::Fixed
    });
    assert_eq!(read, Ok(Contour::Circle { radius: 1.5_f64 }));
}

#[test]
fn an_adjacent_tag_stored_as_an_alias_walks_its_content_under_the_content_key() {
    let refused = json!({ "data": { "gap": "wide", "legacy": true }, "kind": "Broken" });
    assert!(!serde_reads::<Dash>(&refused));
    assert_eq!(
        listed!(Dash, dash_schema, refused),
        [
            "data.gap: invalid: expected U32, found String(\"wide\"): invalid type: string \"wide\", expected u32",
            "data.legacy: unknown: found Bool(true)",
        ]
    );
    let undeclared = json!({ "data": { "gap": 2_i32, "legacy": true }, "kind": "Dotted" });
    assert!(serde_reads::<Dash>(&undeclared));
    assert_eq!(
        listed!(Dash, dash_schema, undeclared),
        ["data.legacy: unknown: found Bool(true)"]
    );
    let absent = json!({ "kind": "Dotted" });
    assert!(!serde_reads::<Dash>(&absent));
    assert_eq!(
        listed!(Dash, dash_schema, absent),
        ["data: missing: expected Model(\"Dash\")"]
    );
}

#[test]
fn a_tag_naming_a_variant_serde_never_reads_names_no_variant() {
    let hidden = serde_json::to_value(Coating::Hidden).unwrap();
    assert_eq!(hidden, json!({ "kind": "Hidden" }));
    assert!(!serde_reads::<Coating>(&hidden));
    assert_eq!(
        listed!(Coating, coating_schema, hidden),
        [
            "kind: invalid: expected Variants([\"Clear\", \"Blank\", \"Solid\"]), found String(\"Hidden\"): unknown variant `Hidden`, expected one of `Blank`, `Clear`, `Solid`"
        ]
    );
    assert_eq!(
        listed!(Coating, coating_schema, json!({})),
        ["kind: missing: expected Variants([\"Clear\", \"Blank\", \"Solid\"])"]
    );

    let faded = serde_json::to_value(Dash::Faded(3)).unwrap();
    assert_eq!(faded, json!({ "data": 3_i32, "kind": "Faded" }));
    assert!(!serde_reads::<Dash>(&faded));
    assert_eq!(
        listed!(Dash, dash_schema, faded),
        [
            "kind: invalid: expected Variants([\"Dashed\", \"Dotted\", \"Broken\", \"Hairline\"]), found String(\"Faded\"): unknown variant `Faded`, expected one of `Broken`, `Dashed`, `Dotted`, `Hairline`"
        ]
    );

    assert_eq!(
        serde_json::to_value(Trail::Lost).unwrap_err().to_string(),
        "the enum variant Trail::Lost cannot be serialized"
    );
    let named = json!("Lost");
    assert!(!serde_reads::<Trail>(&named));
    assert_eq!(
        listed!(Trail, trail_schema, named),
        [
            "the value itself: invalid: expected Variants([\"Jump\", \"Hop\", \"Leap\", \"Old\", \"Road\", \"Lane\"]), found String(\"Lost\"): unknown variant `Lost`, expected one of `Hop`, `Jump`, `Leap`, `Old`, `Lane`, `Road`"
        ]
    );
    let keyed = json!({ "Lost": null });
    assert!(!serde_reads::<Trail>(&keyed));
    assert_eq!(
        listed!(Trail, trail_schema, keyed),
        [
            "the value itself: invalid: expected Variants([\"Jump\", \"Hop\", \"Leap\", \"Old\", \"Road\", \"Lane\"]), found Object {\"Lost\": Null}: unknown variant `Lost`, expected one of `Hop`, `Jump`, `Leap`, `Old`, `Lane`, `Road`"
        ]
    );
}

#[test]
fn an_untagged_variant_serde_never_reads_gets_no_list_of_its_own() {
    let stored = serde_json::to_value(Reach::Pager { number: 7_i32 }).unwrap();
    assert_eq!(stored, json!({ "number": 7_i32 }));
    assert!(!serde_reads::<Reach>(&stored));
    let read = Reach::from_value_with(stored, |_raw, _found| reach_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        lines(&reach_schema::Unrecovered {
            issues: issues.clone(),
        }),
        ["the value itself: no variant: found Object {\"number\": Number(7)}, tried Email, Phone"]
    );
    assert_eq!(
        tried!(reach_schema, issues),
        [
            (
                "Email",
                vec![
                    "address: missing: expected String".to_owned(),
                    "number: unknown: found Number(7)".to_owned(),
                ]
            ),
            (
                "Phone",
                vec![
                    "country: missing: expected I32".to_owned(),
                    "digits: missing: expected String".to_owned(),
                    "number: unknown: found Number(7)".to_owned(),
                ]
            ),
        ]
    );

    let email = json!({ "address": "ann@example.org", "legacy": true });
    assert_eq!(
        serde_json::from_value::<Reach>(email.clone()).unwrap(),
        Reach::Email {
            address: "ann@example.org".to_owned(),
        }
    );
    assert_eq!(
        listed!(Reach, reach_schema, email),
        ["legacy: unknown: found Bool(true)"]
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(
            Reach,
            reach_schema,
            json!({ "country": 1_i32, "digits": "555" }),
            calls
        ),
        Ok(Reach::Phone {
            country: 1_i32,
            digits: "555".to_owned(),
        })
    );
    assert_eq!(calls, 0);
}

/// serde still reads a variant it never writes, so one under `skip_serializing` alone is walked.
#[test]
fn a_variant_serde_reads_and_never_writes_is_walked() {
    assert_eq!(
        serde_json::to_value(Trail::Old { length: 3 })
            .unwrap_err()
            .to_string(),
        "the enum variant Trail::Old cannot be serialized"
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(
            Trail,
            trail_schema,
            json!({ "Old": { "length": 3_i32 } }),
            calls
        ),
        Ok(Trail::Old { length: 3 })
    );
    assert_eq!(calls, 0);
    let stored = json!({ "Old": { "extra": true, "length": "x" } });
    assert!(!serde_reads::<Trail>(&stored));
    assert_eq!(
        listed!(Trail, trail_schema, stored),
        [
            "Old.length: invalid: expected U32, found String(\"x\"): invalid type: string \"x\", expected u32",
            "Old.extra: unknown: found Bool(true)",
        ]
    );
}

/// The key an externally tagged variant is stored under is the enum's own, alias or name.
#[test]
fn an_externally_tagged_enums_fields_walker_returns_the_key_the_variant_is_stored_under() {
    let mut none: Vec<contour_schema::Issue<Value>> = Vec::new();
    for (stored, own) in [
        (
            json!({ "Round": { "radius": 1.5_f64 }, "legacy": true }),
            "Round",
        ),
        (
            json!({ "Circle": { "radius": 1.5_f64 }, "legacy": true }),
            "Circle",
        ),
        (json!({ "Blank": null, "legacy": true }), "Blank"),
        (json!({ "Empty": null, "legacy": true }), "Empty"),
    ] {
        assert_eq!(
            Contour::decode_with_value_fields(
                stored.as_object().unwrap(),
                &[],
                contour_schema::issue_from_parts,
                &mut none,
            ),
            [own]
        );
    }
    assert_eq!(none, Vec::new());
}
