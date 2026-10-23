//! The models of the digit strings: one per place a member holds a `u64` or an `i64` in, and the
//! places it keeps the number. A build with `swift` or `kotlin` refuses a member that holds a
//! `u64`, so the model that holds one is declared where neither is on.

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use core::ops::Range;
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// An `i64` in each place a member of a struct holds one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigitTally {
    pub boxed: Box<i64>,
    pub grid: [i64; 2],
    pub keyed: BTreeMap<String, i64>,
    pub listed: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<i64>,
    pub nested: Vec<Option<Vec<i64>>>,
    pub paired: (i64, String),
    pub set: BTreeSet<i64>,
    pub signed: i64,
}

/// A type that holds what fills its parameter, written as that value's own impl writes it.
#[model_schema(default_types(Held = i64))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigitHolder<Held> {
    pub held: Held,
}

/// The places an `i64` keeps the number serde's own impl writes: a map's key, a range's bound,
/// a `Result`'s arm, what fills another type's parameter and a member under a `literal`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigitKept {
    pub filled: DigitHolder<i64>,
    #[model_schema_prop(literal = 214)]
    pub fixed: i64,
    pub keyed: BTreeMap<i64, String>,
    pub ranged: Range<i64>,
    pub resulted: Result<i64, String>,
}

/// A brand over an `i64`, written as its digit string.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DigitId(pub i64);

/// A struct of two slots, each written at its position.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigitPair(pub i64, pub String);

/// An `i64` in a member of each kind of variant, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DigitEvent {
    Counted(i64),
    Moved { from: i64, to: i64 },
    Pair(i64, String),
    Reset,
}

/// An `i64` in the members of a variant under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum DigitTagged {
    Moved { from: i64, to: i64 },
    Reset,
}

/// An `i64` under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum DigitAdjacent {
    Counted(i64),
    Moved { from: i64, to: i64 },
}

/// An `i64` in an untagged enum, under a bound that chooses the variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DigitUntagged {
    Named { count: i64 },
    Small(#[model_schema_prop(maximum = 9)] i64),
    Text(String),
}

/// An `i64` held to a bound as an integer, and one read as its default where the key is left out.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigitBounded {
    #[model_schema_prop(minimum = -5, maximum = 5)]
    pub level: i64,
    #[serde(default = "seven")]
    pub start: i64,
}

/// A `u64` in each place a member holds one.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigitWide {
    pub big: u64,
    pub keyed: HashMap<String, u64>,
    pub listed: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<u64>,
    pub signed: i64,
}

/// The default of [`DigitBounded::start`].
const fn seven() -> i64 {
    7
}
