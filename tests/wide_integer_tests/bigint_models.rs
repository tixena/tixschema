//! Models whose examples and defaults hold an `i64`, written into the Zod source as a bigint
//! literal. The models that hold a `u64`, a `usize` or an `isize` are declared where neither
//! `swift` nor `kotlin` is on, which refuse them.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::sync::Arc;
#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A ledger row.
///
/// ```rust example
/// Ledger { carried: far(), count: 3, signed: 12 }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ledger {
    #[serde(default = "far")]
    pub carried: i64,
    pub count: u32,
    pub signed: i64,
}

/// The extremes of an `i64`.
///
/// ```rust example
/// Extremes { greatest: i64::MAX, least: i64::MIN }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Extremes {
    pub greatest: i64,
    pub least: i64,
}

/// A list of an `i64` with two values.
///
/// ```rust example
/// Lines { lines: vec![12, 240] }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lines {
    #[serde(default = "pair")]
    pub lines: Vec<i64>,
}

/// A list of an `i64` with no value.
///
/// ```rust example
/// NoLines { lines: vec![] }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NoLines {
    #[serde(default)]
    pub lines: Vec<i64>,
}

/// An `i64` in a list beyond what a JavaScript `number` holds exactly.
///
/// ```rust example
/// FarLines { lines: vec![12, far()] }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FarLines {
    pub lines: Vec<i64>,
}

/// Numbers that publish a `number`.
///
/// ```rust example
/// Narrow { byte: 7, half: 1.5, whole: 2.25, wide: 4_000_000_000 }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Narrow {
    pub byte: u8,
    pub half: f32,
    pub whole: f64,
    pub wide: u32,
}

/// A date under `as_number`, which publishes a `number`.
///
/// ```rust example
/// Stamped { at: chrono::DateTime::from_timestamp_millis(1_700_000_000_000).unwrap() }
/// ```
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stamped {
    #[model_schema_prop(as_number)]
    pub at: DateTime<Utc>,
}

/// A date written by a function of its author's, as the milliseconds of its epoch.
///
/// ```rust example
/// Hooked { at: chrono::DateTime::from_timestamp_millis(1_700_000_000_000).unwrap(), text: "123".to_owned() }
/// ```
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hooked {
    #[model_schema_prop(as_number)]
    #[serde(serialize_with = "millis")]
    pub at: DateTime<Utc>,
    pub text: String,
}

/// An `i64` behind a newtype struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tag(pub i64);

/// An `i64` behind a transparent struct.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Plain(pub i64);

/// An externally tagged enum over an `i64`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Wrapper {
    Named { n: i64 },
    Pair(i64, u32),
    Wrapped(i64),
}

/// An adjacently tagged enum over an `i64`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "t", content = "c")]
pub enum Adjacent {
    Held(i64),
}

/// The members a flattened struct holds.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Deep {
    pub deep: i64,
}

/// An `i64` in each position serde writes one in.
///
/// ```rust example
/// Positions {
///     adjacent: Adjacent::Held(9),
///     boxed: Box::new(10),
///     counted: Rc::new(14),
///     deep: Deep { deep: 13 },
///     keyed: BTreeMap::from([("a".to_owned(), 1)]),
///     maybe: Some(5),
///     named: Wrapper::Named { n: 5 },
///     pair: Wrapper::Pair(3, 4),
///     plain: Plain(8),
///     shared: Arc::new(15),
///     tag: Tag(6),
///     wrapped: Wrapper::Wrapped(2),
/// }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Positions {
    pub adjacent: Adjacent,
    pub boxed: Box<i64>,
    pub counted: Rc<i64>,
    #[serde(flatten)]
    pub deep: Deep,
    pub keyed: BTreeMap<String, i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<i64>,
    pub named: Wrapper,
    pub pair: Wrapper,
    pub plain: Plain,
    pub shared: Arc<i64>,
    pub tag: Tag,
    pub wrapped: Wrapper,
}

/// An untagged enum's `Many` over a list beyond what a JavaScript `number` holds exactly.
///
/// ```rust example
/// EitherMany::Many(vec![12, far()])
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EitherMany {
    Many(Vec<i64>),
    One(i64),
}

/// An untagged enum's `One`.
///
/// ```rust example
/// EitherOne::One(240)
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EitherOne {
    Many(Vec<i64>),
    One(i64),
}

/// A transparent struct beyond what a JavaScript `number` holds exactly.
///
/// ```rust example
/// FarPlain(far())
/// ```
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct FarPlain(pub i64);

/// An enum with an `i64` member under a default.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Entry {
    Posted {
        #[serde(default = "far")]
        carried: i64,
        #[serde(default)]
        lines: Vec<i64>,
    },
}

/// Defaults that stay numbers, and a default list.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Defaulted {
    #[serde(default = "half")]
    pub half: f64,
    #[serde(default = "pair")]
    pub pair: Vec<i64>,
    #[serde(default = "small")]
    pub small: u32,
}

/// An `i64` beyond what a JavaScript `number` holds exactly.
pub const fn far() -> i64 {
    (1_i64 << 53_i32) + 1_i64
}

/// The default of `half`.
pub const fn half() -> f64 {
    1.5
}

/// A date as the milliseconds of its epoch, through the serializer it is handed.
#[cfg(feature = "chrono")]
pub fn millis<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i64(value.timestamp_millis())
}

/// The default of `pair`.
pub fn pair() -> Vec<i64> {
    vec![12, 240]
}

/// The default of `small`.
pub const fn small() -> u32 {
    7
}
