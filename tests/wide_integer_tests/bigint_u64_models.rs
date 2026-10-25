//! Models whose examples and defaults hold a `u64`, a `usize` or an `isize`, declared where
//! neither `swift` nor `kotlin` is on, which refuse them.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// An account number with an `i64`, a `u64` and a list of them, each beyond what a JavaScript
/// `number` holds exactly.
///
/// ```rust example
/// AccountNumber {
///     numbers: vec![12, far_unsigned()],
///     signed: far_signed(),
///     unsigned: far_unsigned(),
/// }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountNumber {
    pub numbers: Vec<u64>,
    pub signed: i64,
    pub unsigned: u64,
}

/// The greatest `u64`.
///
/// ```rust example
/// Greatest { top: u64::MAX }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Greatest {
    pub top: u64,
}

/// A `usize` and an `isize`, which publish a `number` up to what it holds exactly.
///
/// ```rust example
/// Sizes { count: 3, far: 9_007_199_254_740_993, step: -3 }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sizes {
    pub count: usize,
    pub far: usize,
    pub step: isize,
}

/// A map keyed by a `u64`, whose key is a JSON string.
///
/// ```rust example
/// Keyed { keyed: BTreeMap::from([(42, 1)]) }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Keyed {
    pub keyed: BTreeMap<u64, u32>,
}

/// A `u64` under a default.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnsignedDefault {
    #[serde(default = "far_unsigned")]
    pub unsigned: u64,
}

/// An `i64` beyond what a JavaScript `number` holds exactly.
pub const fn far_signed() -> i64 {
    (1_i64 << 53_i32) + 1_i64
}

/// A `u64` beyond what a JavaScript `number` holds exactly.
pub const fn far_unsigned() -> u64 {
    (1_u64 << 53_i32) + 1_u64
}
