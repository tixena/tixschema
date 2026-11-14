//! A `usize` in each place a member holds one: the value itself, a list's item, an `Option`, a
//! map's key and value, a tuple's position, a type's argument, a range's bound, a brand's slot, a
//! `literal` and a default.

use alloc::collections::BTreeMap;
use core::ops::Range;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A brand over a `usize`.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SlotIndex(usize);

/// A type that holds what fills its parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Holding<Held> {
    pub held: Held,
}

/// A `usize` in each place a member holds one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Placed {
    pub at: usize,
    #[serde(default = "top")]
    pub defaulted: usize,
    pub held: Holding<usize>,
    pub indexed: BTreeMap<usize, String>,
    pub keyed: BTreeMap<String, usize>,
    pub listed: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<usize>,
    pub paired: (usize, String),
    #[model_schema_prop(literal = 18446744073709551615)]
    pub pinned: usize,
    pub slot: SlotIndex,
    pub span: Range<usize>,
}

/// `usize::MAX`, the default of [`Placed::defaulted`].
pub const fn top() -> usize {
    usize::MAX
}

/// serde writes each `usize` of a [`Placed`] as its digits, `usize::MAX` among them, and reads an
/// absent `defaulted` as `usize::MAX`.
#[test]
fn serde_writes_and_reads_each_usize_of_placed() {
    let placed = PlacedDraft {
        at: usize::MAX,
        defaulted: 0,
        held: HoldingDraft { held: usize::MAX }.into(),
        indexed: BTreeMap::from([(usize::MAX, "x".to_owned())]),
        keyed: BTreeMap::from([("a".to_owned(), usize::MAX)]),
        listed: vec![usize::MAX],
        maybe: Some(usize::MAX),
        paired: (usize::MAX, "y".to_owned()),
        pinned: usize::MAX,
        slot: slot_index(usize::MAX),
        span: 0..usize::MAX,
    };
    let written = serde_json::to_string(&placed).unwrap();
    assert_eq!(
        written,
        r#"{"at":18446744073709551615,"defaulted":0,"held":{"held":18446744073709551615},"indexed":{"18446744073709551615":"x"},"keyed":{"a":18446744073709551615},"listed":[18446744073709551615],"maybe":18446744073709551615,"paired":[18446744073709551615,"y"],"pinned":18446744073709551615,"slot":18446744073709551615,"span":{"start":0,"end":18446744073709551615}}"#
    );
    assert_eq!(
        serde_json::from_str::<Placed>(&written).unwrap(),
        placed.validate().unwrap()
    );
    let absent = r#"{"at":0,"held":{"held":0},"indexed":{},"keyed":{},"listed":[],"paired":[0,"z"],"pinned":18446744073709551615,"slot":0,"span":{"start":0,"end":0}}"#;
    assert_eq!(
        serde_json::from_str::<Placed>(absent).unwrap().defaulted,
        usize::MAX
    );
}

#[cfg(feature = "serde")]
fn slot_index(value: usize) -> SlotIndex {
    SlotIndex::new(value).unwrap()
}

#[cfg(not(feature = "serde"))]
fn slot_index(value: usize) -> SlotIndex {
    SlotIndexDraft(value).into()
}
