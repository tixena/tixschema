//! `CarillonService` on `ws_rpc`, whose one operation takes a message that holds a struct in a
//! list, in a map, in an externally, an adjacently and an internally tagged enum, beside a
//! flattened member and in a struct with its own tag, numbers alone and in a list, a map keyed by a
//! brand with a check, one keyed by a brand with a check over another and one keyed by an `i64`,
//! a `Result`, a plain enum, a member with a bound, and answers the message it was handed.
//! What the Rust dispatcher answers to each payload names the field every emitted dispatcher and
//! client is measured against. Nothing here waits on a clock.

use alloc::collections::BTreeMap;
use core::cmp::Ordering;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

use super::carillon_ws_rpc_transport;

/// The message read whole.
pub const STRUCK: &str = r#"{"bells":[{"weight":3},{"weight":4}],"towers":{"north":{"weight":5}},"peal":{"Swung":{"arc":1}},"swing":{"kind":"Wide","data":{"arc":2}},"turn":{"kind":"Half","arc":3,"echo":{"Err":"x"},"pull":{"Swung":{"arc":6}}},"strokes":2,"label":"a","frame":{"span":1,"oak":2,"reach":5,"deep":{"inner":4},"echo":{"Ok":5},"pull":"Still"},"yoke":{"kind":"CarillonYoke","pin":3},"rounds":[1,2],"depth":-9223372036854775808,"pitches":{"3":1},"tally":{"Ok":4},"ring":1.5,"sway":2.5,"outer":{"ring":1},"tolls":{"3":1},"reaches":{"3":1},"gauges":{"1.5":1},"slims":{"1.5":1},"chord":"Still"}"#;

/// What each case of [`CASES`] changes in [`STRUCK`]: the dotted place, and the value written
/// there, `None` taking the key out.
pub const CASES: [(&str, &str, Option<&str>); 162] = [
    ("read whole", "", None),
    ("a key beside a list item", "bells.1.stray", Some("1")),
    ("a key beside a map value", "towers.north.stray", Some("1")),
    (
        "a key beside an external variant",
        "peal.Swung.stray",
        Some("1"),
    ),
    (
        "a key beside an adjacent variant",
        "swing.data.stray",
        Some("1"),
    ),
    ("a key beside an internal variant", "turn.stray", Some("1")),
    ("a list item without its member", "bells.0.weight", None),
    ("an internal variant without its tag", "turn.kind", None),
    (
        "an adjacent variant without its content",
        "swing.data",
        None,
    ),
    ("a text for a number", "strokes", Some(r#""x""#)),
    (
        "a text for a number in a list item",
        "bells.1.weight",
        Some(r#""x""#),
    ),
    (
        "a negative number for a u32",
        "towers.north.weight",
        Some("-1"),
    ),
    ("a null for a number", "strokes", Some("null")),
    ("a list for a number", "strokes", Some("[1]")),
    (
        "a boolean for a number in a variant",
        "peal.Swung.arc",
        Some("true"),
    ),
    ("a number for a text", "label", Some("7")),
    ("a text for a list", "bells", Some(r#""x""#)),
    ("a label below its bound", "label", Some(r#""""#)),
    ("a weight above its bound", "bells.1.weight", Some("101")),
    ("an unknown variant", "peal", Some(r#"{"Rung":{}}"#)),
    (
        "a text for a number in a flattened member",
        "frame.oak",
        Some(r#""x""#),
    ),
    (
        "a text for a number beside a flattened member",
        "frame.span",
        Some(r#""x""#),
    ),
    (
        "a text for a number in a tagged struct",
        "yoke.pin",
        Some(r#""x""#),
    ),
    ("a flattened member without its member", "frame.oak", None),
    (
        "a weight above its bound in a flattened member",
        "frame.oak",
        Some("101"),
    ),
    (
        "a number beyond a u32 in a flattened member with a bound",
        "frame.oak",
        Some("4294967296"),
    ),
    (
        "a negative number in a flattened member with a bound",
        "frame.oak",
        Some("-1"),
    ),
    (
        "a number beyond a u32 in an internal variant with a bound",
        "turn.arc",
        Some("4294967296"),
    ),
    (
        "a negative number in an internal variant with a bound",
        "turn.arc",
        Some("-1"),
    ),
    ("a key beside a flattened member", "frame.stray", Some("1")),
    ("a number beyond a u32", "strokes", Some("4294967296")),
    ("a fraction for a u32", "strokes", Some("1.5")),
    (
        "a number beyond an i64",
        "depth",
        Some("9223372036854775808"),
    ),
    (
        "a number below an i64",
        "depth",
        Some("-9223372036854775809"),
    ),
    ("a negative number in a list", "rounds.1", Some("-1")),
    ("a negative number for a map value", "pitches.3", Some("-1")),
    ("a key its check refuses", "pitches.13", Some("1")),
    ("a text in an arm of a Result", "tally.Ok", Some(r#""x""#)),
    (
        "a negative number in an arm of a Result",
        "tally.Ok",
        Some("-1"),
    ),
    (
        "an unknown arm of a Result",
        "tally",
        Some(r#"{"Maybe":1}"#),
    ),
    (
        "a Result of two arms",
        "tally",
        Some(r#"{"Ok":4,"Err":"x"}"#),
    ),
    ("a Result of no arm", "tally", Some("{}")),
    ("a key that is no number", "pitches.x", Some("1")),
    ("an f32 beyond its range", "ring", Some("1e39")),
    ("an f32 just beyond its range", "ring", Some("3.5e38")),
    ("an f32 beyond its range below zero", "ring", Some("-1e39")),
    ("an f32 beyond an f64", "ring", Some("1e400")),
    ("an f64 beyond an f32", "sway", Some("1e39")),
    ("an f64 beyond its range", "sway", Some("1e400")),
    (
        "a key the inner brand's check refuses",
        "outer.mute",
        Some("1"),
    ),
    (
        "a key the outer brand's check refuses",
        "outer.soft",
        Some("1"),
    ),
    ("an integral fraction for a u32", "strokes", Some("1.0")),
    ("an exponent for a u32", "strokes", Some("1e2")),
    ("a capital exponent for a u32", "strokes", Some("1E2")),
    ("a fraction of zero for a u32", "strokes", Some("0.0")),
    (
        "a fraction of zero below zero for a u32",
        "strokes",
        Some("-0.0"),
    ),
    ("a zero below zero for a u32", "strokes", Some("-0")),
    ("an integral fraction for an i64", "depth", Some("1.0")),
    ("an exponent for an i64", "depth", Some("1e2")),
    ("a capital exponent for an i64", "depth", Some("1E2")),
    ("a fraction of zero for an i64", "depth", Some("0.0")),
    (
        "a fraction of zero below zero for an i64",
        "depth",
        Some("-0.0"),
    ),
    ("a zero below zero for an i64", "depth", Some("-0")),
    ("a one for a u32", "strokes", Some("1")),
    ("a zero for a u32", "strokes", Some("0")),
    ("a one for an i64", "depth", Some("1")),
    ("a zero for an i64", "depth", Some("0")),
    (
        "a number beyond a u32 in a flattened member",
        "frame.reach",
        Some("4294967296"),
    ),
    (
        "a negative number in a flattened member",
        "frame.reach",
        Some("-1"),
    ),
    (
        "a flattened member without its nested member",
        "frame.deep.inner",
        None,
    ),
    (
        "a text for a number in an internal variant",
        "turn.arc",
        Some(r#""x""#),
    ),
    ("an internal variant without its member", "turn.arc", None),
    (
        "a weight above its bound in an internal variant",
        "turn.arc",
        Some("101"),
    ),
    ("an external enum of no variant", "peal", Some("{}")),
    (
        "an external enum of two variants",
        "peal",
        Some(r#"{"Swung":{"arc":1},"Still":null}"#),
    ),
    ("a number for an external enum", "peal", Some("1")),
    ("a list for an external enum", "peal", Some("[]")),
    ("a null for an external enum", "peal", Some("null")),
    ("a number for a Result", "tally", Some("1")),
    ("a list for a Result", "tally", Some("[]")),
    ("a boolean for a Result", "tally", Some("true")),
    ("a null for a Result", "tally", Some("null")),
    (
        "a brand key with a letter after it",
        "pitches.1x",
        Some("1"),
    ),
    ("a brand key with a space after it", "pitches.1 ", Some("1")),
    ("a brand key of a minus alone", "pitches.-", Some("1")),
    ("a brand key with a leading zero", "pitches.01", Some("1")),
    ("a brand key beyond an f64", "pitches.1e400", Some("1")),
    ("a brand key with an exponent", "pitches.1e3", Some("1")),
    ("a brand key of minus zero", "pitches.-0", Some("1")),
    ("a brand key of zero", "pitches.0", Some("1")),
    ("a negative brand key", "pitches.-1", Some("1")),
    ("the largest brand key", "pitches.4294967295", Some("1")),
    ("a brand key beyond a u32", "pitches.4294967296", Some("1")),
    ("an i64 key with a letter after it", "tolls.1x", Some("1")),
    ("an i64 key with a space after it", "tolls.1 ", Some("1")),
    ("an i64 key of a minus alone", "tolls.-", Some("1")),
    ("an i64 key with a leading zero", "tolls.01", Some("1")),
    ("an i64 key beyond an f64", "tolls.1e400", Some("1")),
    ("an i64 key with an exponent", "tolls.1e3", Some("1")),
    ("an i64 key of minus zero", "tolls.-0", Some("1")),
    ("an i64 key of zero", "tolls.0", Some("1")),
    ("a negative i64 key", "tolls.-1", Some("1")),
    (
        "the largest i64 key",
        "tolls.9223372036854775807",
        Some("1"),
    ),
    ("the least i64 key", "tolls.-9223372036854775808", Some("1")),
    (
        "an i64 key beyond an i64",
        "tolls.9223372036854775808",
        Some("1"),
    ),
    (
        "an i64 key below an i64",
        "tolls.-9223372036854775809",
        Some("1"),
    ),
    (
        "a number for a Result in a flattened member",
        "frame.echo",
        Some("1"),
    ),
    (
        "a list for a Result in a flattened member",
        "frame.echo",
        Some("[]"),
    ),
    (
        "an empty object for a Result in a flattened member",
        "frame.echo",
        Some("{}"),
    ),
    (
        "two arms for a Result in a flattened member",
        "frame.echo",
        Some(r#"{"Ok":4,"Err":"x"}"#),
    ),
    (
        "a number for an external enum in a flattened member",
        "frame.pull",
        Some("1"),
    ),
    (
        "a list for an external enum in a flattened member",
        "frame.pull",
        Some("[]"),
    ),
    (
        "an empty object for an external enum in a flattened member",
        "frame.pull",
        Some("{}"),
    ),
    (
        "two variants for an external enum in a flattened member",
        "frame.pull",
        Some(r#"{"Swung":{"arc":1},"Still":null}"#),
    ),
    (
        "a number for a Result in an internal variant",
        "turn.echo",
        Some("1"),
    ),
    (
        "a list for a Result in an internal variant",
        "turn.echo",
        Some("[]"),
    ),
    (
        "an empty object for a Result in an internal variant",
        "turn.echo",
        Some("{}"),
    ),
    (
        "two arms for a Result in an internal variant",
        "turn.echo",
        Some(r#"{"Ok":4,"Err":"x"}"#),
    ),
    (
        "a number for an external enum in an internal variant",
        "turn.pull",
        Some("1"),
    ),
    (
        "a list for an external enum in an internal variant",
        "turn.pull",
        Some("[]"),
    ),
    (
        "an empty object for an external enum in an internal variant",
        "turn.pull",
        Some("{}"),
    ),
    (
        "two variants for an external enum in an internal variant",
        "turn.pull",
        Some(r#"{"Swung":{"arc":1},"Still":null}"#),
    ),
    (
        "a text naming an arm of a Result that carries data",
        "tally",
        Some(r#""Ok""#),
    ),
    (
        "a text naming a variant that carries data",
        "peal",
        Some(r#""Swung""#),
    ),
    ("a number for a plain enum", "chord", Some("1")),
    ("a list for a plain enum", "chord", Some("[]")),
    ("a boolean for a plain enum", "chord", Some("true")),
    ("an empty object for a plain enum", "chord", Some("{}")),
    (
        "an object of two keys for a plain enum",
        "chord",
        Some(r#"{"Still":null,"Rung":null}"#),
    ),
    (
        "an object of one key over null for a plain enum",
        "chord",
        Some(r#"{"Still":null}"#),
    ),
    (
        "an object of one key over a number for a plain enum",
        "chord",
        Some(r#"{"Still":1}"#),
    ),
    ("an unknown text for a plain enum", "chord", Some(r#""x""#)),
    (
        "a float brand key with an exponent",
        "gauges.1e3",
        Some("1"),
    ),
    (
        "a float brand key with a capital exponent",
        "gauges.1E3",
        Some("1"),
    ),
    (
        "a float brand key with a signed exponent",
        "gauges.1e+3",
        Some("1"),
    ),
    ("a float brand key of minus zero", "gauges.-0", Some("1")),
    ("a float brand key of zero", "gauges.0", Some("1")),
    ("a float brand key beyond an f32", "gauges.1e39", Some("1")),
    ("a float brand key beyond an f64", "gauges.1e400", Some("1")),
    (
        "a float brand key with a leading zero",
        "gauges.01",
        Some("1"),
    ),
    (
        "a float brand key with a letter after it",
        "gauges.1x",
        Some("1"),
    ),
    (
        "a float brand key with a space after it",
        "gauges.1 ",
        Some("1"),
    ),
    ("a float brand key of a minus alone", "gauges.-", Some("1")),
    ("a float brand key that is no number", "gauges.x", Some("1")),
    ("an f32 brand key with an exponent", "slims.1e3", Some("1")),
    ("an f32 brand key of minus zero", "slims.-0", Some("1")),
    ("an f32 brand key beyond its range", "slims.1e39", Some("1")),
    (
        "an f32 brand key beyond its range below zero",
        "slims.-1e39",
        Some("1"),
    ),
    ("an f32 brand key beyond an f64", "slims.1e400", Some("1")),
    (
        "an f32 brand key with a leading zero",
        "slims.01",
        Some("1"),
    ),
    ("an f32 brand key that is no number", "slims.x", Some("1")),
    (
        "an isize key with a letter after it",
        "reaches.1x",
        Some("1"),
    ),
    (
        "an isize key with a space after it",
        "reaches.1 ",
        Some("1"),
    ),
    ("an isize key of a minus alone", "reaches.-", Some("1")),
    ("an isize key with a leading zero", "reaches.01", Some("1")),
    ("an isize key beyond an f64", "reaches.1e400", Some("1")),
    ("an isize key with an exponent", "reaches.1e3", Some("1")),
    ("an isize key of minus zero", "reaches.-0", Some("1")),
    ("an isize key that is no number", "reaches.x", Some("1")),
    ("an isize key of zero", "reaches.0", Some("1")),
    ("a negative isize key", "reaches.-1", Some("1")),
    ("an isize key of three", "reaches.3", Some("1")),
];

/// The cases where the Rust dispatcher names a place other than the one the case changes: serde
/// reads a flattened member, and the members of an internally tagged variant, from the content
/// their holder buffers, and names the holder for a value inside it that it refuses, a number
/// beyond its width among them, and the key alone for a nested key missing; it names the map for a
/// key of an integer type it reads as a float; and it names the variant under a member for a
/// string naming a variant that carries data and for a value under a plain enum's key. Every
/// emitted read names the place the Rust dispatcher names.
pub const NAMED_BY_SERDE: [(&str, &str); 36] = [
    ("a text for a number in a flattened member", "frame"),
    ("a number beyond a u32 in a flattened member", "frame"),
    ("a negative number in a flattened member", "frame"),
    (
        "a number beyond a u32 in a flattened member with a bound",
        "frame",
    ),
    (
        "a negative number in a flattened member with a bound",
        "frame",
    ),
    (
        "a number beyond a u32 in an internal variant with a bound",
        "turn",
    ),
    (
        "a negative number in an internal variant with a bound",
        "turn",
    ),
    (
        "a flattened member without its nested member",
        "frame.inner",
    ),
    ("a text for a number in an internal variant", "turn"),
    ("a brand key with an exponent", "pitches"),
    ("a brand key of minus zero", "pitches"),
    ("an i64 key with an exponent", "tolls"),
    ("an i64 key of minus zero", "tolls"),
    ("an i64 key below an i64", "tolls"),
    ("a number for a Result in a flattened member", "frame"),
    ("a list for a Result in a flattened member", "frame"),
    (
        "an empty object for a Result in a flattened member",
        "frame",
    ),
    ("two arms for a Result in a flattened member", "frame"),
    (
        "a number for an external enum in a flattened member",
        "frame",
    ),
    ("a list for an external enum in a flattened member", "frame"),
    (
        "an empty object for an external enum in a flattened member",
        "frame",
    ),
    (
        "two variants for an external enum in a flattened member",
        "frame",
    ),
    ("a number for a Result in an internal variant", "turn"),
    ("a list for a Result in an internal variant", "turn"),
    (
        "an empty object for a Result in an internal variant",
        "turn",
    ),
    ("two arms for a Result in an internal variant", "turn"),
    (
        "a number for an external enum in an internal variant",
        "turn",
    ),
    ("a list for an external enum in an internal variant", "turn"),
    (
        "an empty object for an external enum in an internal variant",
        "turn",
    ),
    (
        "two variants for an external enum in an internal variant",
        "turn",
    ),
    (
        "a text naming an arm of a Result that carries data",
        "tally.Ok",
    ),
    ("a text naming a variant that carries data", "peal.Swung"),
    (
        "an object of one key over a number for a plain enum",
        "chord.Still",
    ),
    ("an unknown text for a plain enum", "chord"),
    ("an isize key with an exponent", "reaches"),
    ("an isize key of minus zero", "reaches"),
];

/// The cases the Rust dispatcher reads: serde reads an `f32` beyond its range as an infinity of the
/// sign written, an integer written plainly, and a key of an integer type within its range.
pub const READ_BY_RUST: [&str; 27] = [
    "an object of one key over null for a plain enum",
    "an f32 beyond its range",
    "an f32 just beyond its range",
    "an f32 beyond its range below zero",
    "an f64 beyond an f32",
    "a one for a u32",
    "a zero for a u32",
    "a one for an i64",
    "a zero for an i64",
    "a brand key of zero",
    "an i64 key of zero",
    "a negative i64 key",
    "the largest i64 key",
    "the least i64 key",
    "a float brand key with an exponent",
    "a float brand key with a capital exponent",
    "a float brand key with a signed exponent",
    "a float brand key of minus zero",
    "a float brand key of zero",
    "a float brand key beyond an f32",
    "an f32 brand key with an exponent",
    "an f32 brand key of minus zero",
    "an f32 brand key beyond its range",
    "an f32 brand key beyond its range below zero",
    "an isize key of zero",
    "a negative isize key",
    "an isize key of three",
];

/// The cases the Rust dispatcher answers `undeserializable-payload` for, naming no field: `serde_json`
/// refuses each as text that is not the document, not as a value of the wrong kind.
pub const NOT_A_DOCUMENT: [&str; 42] = [
    "a number for a plain enum",
    "a list for a plain enum",
    "a boolean for a plain enum",
    "an empty object for a plain enum",
    "an object of two keys for a plain enum",
    "a Result of two arms",
    "a Result of no arm",
    "a key that is no number",
    "an external enum of no variant",
    "an external enum of two variants",
    "a number for an external enum",
    "a list for an external enum",
    "a null for an external enum",
    "a number for a Result",
    "a list for a Result",
    "a boolean for a Result",
    "a null for a Result",
    "a brand key with a letter after it",
    "a brand key with a space after it",
    "a brand key of a minus alone",
    "a brand key with a leading zero",
    "a brand key beyond an f64",
    "an i64 key with a letter after it",
    "an i64 key with a space after it",
    "an i64 key of a minus alone",
    "an i64 key with a leading zero",
    "an i64 key beyond an f64",
    "a float brand key beyond an f64",
    "a float brand key with a leading zero",
    "a float brand key with a letter after it",
    "a float brand key with a space after it",
    "a float brand key of a minus alone",
    "a float brand key that is no number",
    "an f32 brand key beyond an f64",
    "an f32 brand key with a leading zero",
    "an f32 brand key that is no number",
    "an isize key with a letter after it",
    "an isize key with a space after it",
    "an isize key of a minus alone",
    "an isize key with a leading zero",
    "an isize key beyond an f64",
    "an isize key that is no number",
];

/// The cases whose frame the Rust dispatcher reads as no frame, and answers nothing: a number
/// beyond an `f64` is no JSON `serde_json` reads.
pub const NOT_A_FRAME: [&str; 2] = ["an f32 beyond an f64", "an f64 beyond its range"];

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonBell {
    #[model_schema_prop(maximum = 100)]
    pub weight: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CarillonChord {
    Rung,
    Still,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CarillonPeal {
    Still,
    Swung { arc: u32 },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum CarillonSwing {
    Wide { arc: u32 },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum CarillonTurn {
    Half {
        #[model_schema_prop(maximum = 100)]
        arc: u32,
        echo: Result<u32, String>,
        pull: CarillonPeal,
    },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonBeam {
    pub deep: CarillonDeep,
    pub echo: Result<u32, String>,
    #[model_schema_prop(maximum = 100)]
    pub oak: u32,
    pub pull: CarillonPeal,
    pub reach: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonDeep {
    pub inner: u32,
}

#[model_schema(enable = ["jsonschema"], check = ["withinScale"])]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CarillonPitch(u32);

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CarillonGauge(f64);

impl Eq for CarillonGauge {}

impl Ord for CarillonGauge {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_inner().total_cmp(other.as_inner())
    }
}

impl PartialOrd for CarillonGauge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CarillonSlim(f32);

impl Eq for CarillonSlim {}

impl Ord for CarillonSlim {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_inner().total_cmp(other.as_inner())
    }
}

impl PartialOrd for CarillonSlim {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[model_schema(enable = ["jsonschema"], check = ["audible"])]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CarillonTone(String);

#[model_schema(enable = ["jsonschema"], check = ["loud"])]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CarillonOuterTone(CarillonTone);

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonFrame {
    #[serde(flatten)]
    pub beam: CarillonBeam,
    pub span: u32,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct CarillonYoke {
    pub pin: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CarillonChime {
    pub bells: Vec<CarillonBell>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chord: Option<CarillonChord>,
    pub depth: i64,
    pub frame: CarillonFrame,
    pub gauges: BTreeMap<CarillonGauge, u32>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    pub outer: BTreeMap<CarillonOuterTone, u32>,
    pub peal: CarillonPeal,
    pub pitches: BTreeMap<CarillonPitch, u32>,
    pub reaches: BTreeMap<isize, u32>,
    pub ring: f32,
    pub rounds: Vec<u32>,
    pub slims: BTreeMap<CarillonSlim, u32>,
    pub strokes: u32,
    pub sway: f64,
    pub swing: CarillonSwing,
    pub tally: Result<u32, String>,
    pub tolls: BTreeMap<i64, u32>,
    pub towers: BTreeMap<String, CarillonBell>,
    pub turn: CarillonTurn,
    pub yoke: CarillonYoke,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum CarillonError {
    Cracked,
}

/// Strikes the bells of a carillon.
#[service_schema(transports = ["ws_rpc"])]
pub trait CarillonService<Ctx> {
    /// Strikes the chime the message names, and answers it.
    async fn strike(&self, ctx: &Ctx, req: CarillonChime) -> Result<CarillonChime, CarillonError>;
}

/// Counts the strikes it is handed.
#[derive(Default)]
pub struct Carillon {
    struck: Mutex<u32>,
}

impl Carillon {
    /// How many strikes reached the handler.
    pub fn struck(&self) -> u32 {
        *self.struck.lock().unwrap()
    }
}

impl CarillonService<()> for Carillon {
    fn strike(
        &self,
        _ctx: &(),
        req: CarillonChime,
    ) -> impl Future<Output = Result<CarillonChime, CarillonError>> {
        *self.struck.lock().unwrap() += 1;
        ready(Ok(req))
    }
}

/// The check of [`CarillonPitch`]: a pitch within the twelve tones of the scale.
fn within_scale(value: &CarillonPitch) -> Result<(), Vec<String>> {
    if *value.as_inner() <= 12 {
        Ok(())
    } else {
        Err(vec![format!(
            "{} is outside of the scale",
            value.as_inner()
        )])
    }
}

/// The check of [`CarillonTone`]: a tone that sounds.
fn audible(value: &CarillonTone) -> Result<(), Vec<String>> {
    if value.as_inner() == "mute" {
        Err(vec!["mute is silent".to_owned()])
    } else {
        Ok(())
    }
}

/// The check of [`CarillonOuterTone`]: a tone louder than soft.
fn loud(value: &CarillonOuterTone) -> Result<(), Vec<String>> {
    if value.as_inner().as_inner() == "soft" {
        Err(vec!["soft is quiet".to_owned()])
    } else {
        Ok(())
    }
}

/// [`STRUCK`] with the change `case` names. A number is written into the text as it is: `Value`
/// writes `1e2` as `100.0` and `-0` as `-0.0`, and holds no number beyond an `f64`.
pub fn payload((_, place, value): (&str, &str, Option<&str>)) -> String {
    const VERBATIM: &str = "the number written as it is";
    let mut changed: Value = serde_json::from_str(STRUCK).unwrap();
    if place.is_empty() {
        return changed.to_string();
    }
    let steps: Vec<&str> = place.split('.').collect();
    let (last, ahead) = steps.split_last().unwrap();
    let mut at = &mut changed;
    for step in ahead {
        at = match step.parse::<usize>() {
            Ok(index) if at.is_array() => &mut at[index],
            _ => &mut at[*step],
        };
    }
    let replaced = value.map(|text| match serde_json::from_str(text) {
        Ok(Value::Number(_)) | Err(_) => json!(VERBATIM),
        Ok(read) => read,
    });
    match (replaced, last.parse::<usize>()) {
        (Some(written), Ok(index)) if at.is_array() => at[index] = written,
        (Some(written), _) => at[*last] = written,
        (None, _) => {
            at.as_object_mut().unwrap().remove(*last);
        }
    }
    let text = changed.to_string();
    match value {
        Some(number) if text.contains(VERBATIM) => text.replace(&format!("\"{VERBATIM}\""), number),
        _ => text,
    }
}

/// The request frame `id` that carries `payload` to `strike`.
pub fn frame(id: &str, payload: &str) -> String {
    format!(
        r#"{{"kind":"request","id":"{id}","service":"CarillonService","operation":"strike","payload":{payload}}}"#
    )
}

/// The reply the Rust `ws_rpc` dispatcher writes to `payload`, `None` for a frame it answers
/// nothing, and how many strikes reached the handler.
pub fn over_ws(payload: &str) -> (Option<Value>, u32) {
    let carillon = Carillon::default();
    let framed = frame("ID", payload);
    let mut answering = pin!(carillon_ws_rpc_transport::answer(&framed, &carillon, &()));
    let polled = answering
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    // The dispatcher answers on its first poll: the handler never suspends.
    assert!(polled.is_ready(), "the dispatcher suspended on {payload}");
    let reply = match polled {
        Poll::Ready(answered) => answered,
        Poll::Pending => None,
    };
    (
        reply.map(|text| serde_json::from_str(&text).unwrap()),
        carillon.struck(),
    )
}

/// What the Rust dispatcher answers each case: `"read"` for a message its handler is handed,
/// `"dropped"` for a frame it answers nothing, and the fault's kind and field otherwise.
pub fn rust_answers() -> Vec<Value> {
    CASES
        .into_iter()
        .map(|case| match over_ws(&payload(case)).0 {
            None => json!("dropped"),
            Some(reply) if reply["ok"] == true => json!("read"),
            Some(reply) => json!({
                "kind": reply["error"]["fault"]["kind"],
                "field": reply["error"]["fault"]["issues"][0]["field"],
            }),
        })
        .collect()
}

#[test]
fn the_rust_dispatcher_names_the_place_of_each_case() {
    let (answered, struck) = over_ws(STRUCK);
    let read = answered.unwrap();
    assert_eq!(read["ok"], true, "{read}");
    assert_eq!(
        read["value"],
        serde_json::from_str::<Value>(STRUCK).unwrap()
    );
    assert_eq!(struck, 1);
    for (case, answer) in CASES.into_iter().zip(rust_answers()).skip(1) {
        let (_, strikes) = over_ws(&payload(case));
        let expected = if READ_BY_RUST.contains(&case.0) {
            json!("read")
        } else if NOT_A_FRAME.contains(&case.0) {
            json!("dropped")
        } else if NOT_A_DOCUMENT.contains(&case.0) {
            json!({ "kind": "undeserializable-payload", "field": null })
        } else {
            let place = NAMED_BY_SERDE
                .into_iter()
                .find_map(|(named, above)| (named == case.0).then_some(above))
                .unwrap_or(case.1);
            json!({ "kind": "failed-validation", "field": place })
        };
        assert_eq!(answer, expected, "{}", case.0);
        assert_eq!(strikes, u32::from(answer == "read"), "{}", case.0);
    }
}
