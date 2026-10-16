//! The models the four std ranges are written on, and the payloads serde writes and refuses for
//! them. Every surface and every client is held to the same payloads.

use alloc::collections::BTreeMap;
use core::ops::{Range, RangeFrom, RangeInclusive, RangeTo};

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A model with a bound of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Milestone {
    #[model_schema_prop(minLength = 1)]
    pub mark: String,
}

/// A brand with a bound of its own.
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Waymark(pub String);

/// A type that declares a parameter, which a range fills.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Itinerary<Held> {
    pub held: Held,
}

/// Each of the four ranges, of a number.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spanned {
    pub closed: RangeInclusive<u32>,
    pub from: RangeFrom<u32>,
    pub span: Range<u32>,
    pub to: RangeTo<u32>,
}

/// A range in each position that holds a value: a type argument, a map's value, a list, an
/// `Option` and a tuple's position.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Positioned {
    pub filled: Itinerary<Range<u32>>,
    pub keyed: BTreeMap<String, Range<u32>>,
    pub listed: Vec<Range<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<Range<u32>>,
    pub paired: (Range<u32>, String),
}

/// A second holder of a range in a type argument, whose client types are declared beside the
/// first's.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Detour {
    pub filled: Itinerary<Range<u32>>,
}

/// A range of the type's own parameter.
#[model_schema(default_types(Bound = u32))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stride<Bound> {
    pub span: Range<Bound>,
}

/// A range of a model and one of a brand, each with a bound of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Signposted {
    pub marks: RangeInclusive<Milestone>,
    pub span: Range<Waymark>,
}

/// The two ranges of one bound, of a brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Frontier {
    pub from: RangeFrom<Waymark>,
    pub to: RangeTo<Waymark>,
}

/// A range in a variant's slot and in a variant's member, of the type's own parameter.
#[model_schema(default_types(Bound = u32))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Course<Bound> {
    Named { span: Range<Bound> },
    Span(Range<Bound>),
}

/// An untagged enum whose variants a range tells apart: one of a bounded brand, which the bound of
/// each brand chooses, and one of a number.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Compass {
    Coded { codes: Range<Waymark> },
    Plain { span: Range<u32> },
}

/// A map of ranges of a bounded brand, flattened into [`Excursion`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Atlas {
    pub keyed: BTreeMap<String, Range<Waymark>>,
}

/// Ranges of a model beside a flattened member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Excursion {
    #[serde(flatten)]
    pub atlas: Atlas,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<Range<Milestone>>,
    pub span: Range<Milestone>,
}

/// Ranges of a model and of a bounded brand in the variants of an internally tagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Waypost {
    Codes {
        codes: Vec<Range<Waymark>>,
    },
    Marks {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        marks: Option<RangeInclusive<Milestone>>,
    },
}

/// A range of dates.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rostered {
    pub shift: Range<chrono::DateTime<chrono::Utc>>,
}

/// One payload: its name, the type it is read as, and the JSON.
pub struct Payload {
    pub json: String,
    pub name: &'static str,
    pub read_as: &'static str,
    /// Whether what refuses it is a key no range writes.
    pub undeclared_key: bool,
}

/// A milestone marked `mark`.
pub fn milestone(mark: &str) -> Milestone {
    Milestone {
        mark: mark.to_owned(),
    }
}

/// A waymark of `code`.
pub fn waymark(code: &str) -> Waymark {
    Waymark(code.to_owned())
}

/// The four ranges of a number.
pub const fn spanned() -> Spanned {
    Spanned {
        closed: 1..=2,
        from: 8..,
        span: 8..13,
        to: ..13,
    }
}

/// A range in each position, with `maybe` holding one where `held` says so.
pub fn positioned(held: bool) -> Positioned {
    Positioned {
        filled: Itinerary { held: 1..2 },
        keyed: BTreeMap::from([("a".to_owned(), 3..4)]),
        listed: vec![5..6, 7..8],
        maybe: held.then_some(9..10),
        paired: (11..12, "b".to_owned()),
    }
}

/// A range of a model and one of a brand: where `broken` says so, the start of the first and both
/// bounds of the second break their bound.
pub fn signposted(broken: bool) -> Signposted {
    let (mark, start, end) = if broken {
        ("", "a", "b")
    } else {
        ("a", "ab", "cd")
    };
    Signposted {
        marks: milestone(mark)..=milestone("b"),
        span: waymark(start)..waymark(end),
    }
}

/// Ranges of a model beside a flattened map of ranges of a brand, every bound broken where
/// `broken` says so.
pub fn excursion(broken: bool) -> Excursion {
    let (code, mark) = if broken { ("a", "") } else { ("ab", "a") };
    Excursion {
        atlas: Atlas {
            keyed: BTreeMap::from([("k".to_owned(), waymark(code)..waymark("cd"))]),
        },
        maybe: Some(milestone(mark)..milestone(mark)),
        span: milestone(mark)..milestone("b"),
    }
}

/// A list of ranges of a brand in an internally tagged variant, both bounds broken where `broken`
/// says so.
pub fn waypost(broken: bool) -> Waypost {
    let (start, end) = if broken { ("a", "b") } else { ("ab", "cd") };
    Waypost::Codes {
        codes: vec![waymark(start)..waymark(end)],
    }
}

/// The two ranges of one bound, each bound broken where `broken` says so.
pub fn frontier(broken: bool) -> Frontier {
    let code = if broken { "a" } else { "ab" };
    Frontier {
        from: waymark(code)..,
        to: ..waymark(code),
    }
}

/// What serde writes of `value`, read as `read_as`.
fn written<T>(name: &'static str, read_as: &'static str, value: &T) -> Payload
where
    T: Serialize,
{
    Payload {
        json: serde_json::to_string(value).unwrap(),
        name,
        read_as,
        undeclared_key: false,
    }
}

/// A payload of `read_as`, written as it stands, refused for a key no range writes where
/// `undeclared_key` says so.
fn payload(name: &'static str, read_as: &'static str, json: &str, undeclared_key: bool) -> Payload {
    Payload {
        json: json.to_owned(),
        name,
        read_as,
        undeclared_key,
    }
}

/// What serde writes of a range of a model and of a brand, sound and with broken bounds, which
/// `validate()` refuses.
pub fn judged_payloads() -> Vec<Payload> {
    vec![
        written("sound bounds", "Signposted", &signposted(false)),
        written("broken bounds", "Signposted", &signposted(true)),
        written("sound single bounds", "Frontier", &frontier(false)),
        written("broken single bounds", "Frontier", &frontier(true)),
        written("sound flattened bounds", "Excursion", &excursion(false)),
        written("broken flattened bounds", "Excursion", &excursion(true)),
        written("sound tagged bounds", "Waypost", &waypost(false)),
        written("broken tagged bounds", "Waypost", &waypost(true)),
        written(
            "an optional range of a model in a tagged variant",
            "Waypost",
            &Waypost::Marks {
                marks: Some(milestone("a")..=milestone("b")),
            },
        ),
    ]
}

/// A range of two dates, each written with its milliseconds, which every client writes back.
#[cfg(feature = "chrono")]
pub fn rostered() -> Rostered {
    Rostered {
        shift: "2026-10-16T12:00:00.125Z".parse().unwrap()
            .."2026-10-16T13:30:00.250Z".parse().unwrap(),
    }
}

/// What serde writes of a value of each model that holds a range of a number or of a date.
pub fn read_payloads() -> Vec<Payload> {
    vec![
        written("the four ranges", "Spanned", &spanned()),
        written("each position", "Positioned", &positioned(true)),
        written("no maybe", "Positioned", &positioned(false)),
        written(
            "a second holder of a range in a type argument",
            "Detour",
            &Detour {
                filled: Itinerary { held: 2..3 },
            },
        ),
        written(
            "a range of a parameter",
            "Stride",
            &Stride { span: 8_u32..13 },
        ),
        written(
            "a range in the slot of a variant",
            "Course",
            &Course::Span(8_u32..13),
        ),
        written(
            "a range in a member of a variant",
            "Course",
            &Course::Named { span: 1_u32..2 },
        ),
        written(
            "an untagged variant of a range of a brand",
            "Compass",
            &Compass::Coded {
                codes: waymark("ab")..waymark("cd"),
            },
        ),
        written(
            "an untagged variant of a range of a number",
            "Compass",
            &Compass::Plain { span: 8..13 },
        ),
        #[cfg(feature = "chrono")]
        written("a range of dates", "Rostered", &rostered()),
    ]
}

/// Payloads serde refuses: a key no range writes, a bound left out, and a bound of another type.
pub fn refused_payloads() -> Vec<Payload> {
    vec![
        payload(
            "an untagged variant no bound of a brand admits",
            "Compass",
            r#"{"codes":{"start":"a","end":"bb"}}"#,
            false,
        ),
        payload(
            "a key no range writes",
            "Spanned",
            r#"{"closed":{"start":1,"end":2},"from":{"start":8},"span":{"start":8,"end":13,"x":1},"to":{"end":13}}"#,
            true,
        ),
        payload(
            "an end left out",
            "Spanned",
            r#"{"closed":{"start":1},"from":{"start":8},"span":{"start":8,"end":13},"to":{"end":13}}"#,
            false,
        ),
        payload(
            "an end on a RangeFrom",
            "Spanned",
            r#"{"closed":{"start":1,"end":2},"from":{"start":8,"end":9},"span":{"start":8,"end":13},"to":{"end":13}}"#,
            true,
        ),
        payload(
            "a start on a RangeTo",
            "Spanned",
            r#"{"closed":{"start":1,"end":2},"from":{"start":8},"span":{"start":8,"end":13},"to":{"start":1,"end":13}}"#,
            true,
        ),
        payload(
            "a bound of another type",
            "Spanned",
            r#"{"closed":{"start":1,"end":2},"from":{"start":8},"span":{"start":true,"end":13},"to":{"end":13}}"#,
            false,
        ),
    ]
}
