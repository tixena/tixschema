//! Maps keyed by a `u64` or an `i64`, whose key TypeScript holds as a `number`, in each position
//! the TypeScript surface writes one. A build with `swift` or `kotlin` refuses a member that holds
//! a `u64`, so the model that holds one is declared where neither is on.

use alloc::collections::BTreeMap;
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// An alias of an `i64`, which keys a map below.
#[model_schema()]
pub type WideKeyedRevision = i64;

/// A type that holds what fills its parameter.
#[model_schema(default_types(Held = bool))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WideKeyedHolder<Held> {
    pub held: Held,
}

/// A map keyed by an `i64` as the value of a struct of one slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WideKeyedSlot(pub BTreeMap<i64, bool>);

/// A map keyed by an `i64` in a struct of two slots.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WideKeyedSlots(pub BTreeMap<i64, bool>, pub u8);

/// An alias of a map keyed by an `i64`.
#[model_schema()]
pub type WideKeyedAlias = BTreeMap<i64, bool>;

/// A brand over a map keyed by an `i64`.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WideKeyedBrand(pub BTreeMap<i64, bool>);

/// A map keyed by an `i64` in each place a member holds one, a key through an alias of an `i64`
/// and a map through each type above among them, and a map keyed by a `u32`, whose every key a
/// `number` holds exactly.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WideKeyed {
    pub aliased: BTreeMap<WideKeyedRevision, bool>,
    pub branded: WideKeyedBrand,
    pub direct: BTreeMap<i64, bool>,
    pub filled: WideKeyedHolder<BTreeMap<i64, bool>>,
    pub listed: Vec<BTreeMap<i64, bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<BTreeMap<i64, bool>>,
    pub named: WideKeyedAlias,
    pub narrow: BTreeMap<u32, bool>,
    pub nested: BTreeMap<String, BTreeMap<i64, bool>>,
    pub paired: (BTreeMap<i64, bool>, u8),
    pub slots: WideKeyedSlots,
    pub slotted: Vec<WideKeyedSlot>,
}

/// A map keyed by an `i64` in each kind of variant, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WideKeyedEvent {
    Counted(BTreeMap<i64, bool>, u8),
    Kept { keyed: BTreeMap<i64, bool> },
    Narrow { keyed: BTreeMap<u32, bool> },
    Quiet,
    Tallied(BTreeMap<i64, bool>),
}

/// A map keyed by an `i64` in each kind of variant, under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum WideKeyedAdjacent {
    Counted(BTreeMap<i64, bool>, u8),
    Kept { keyed: BTreeMap<i64, bool> },
    Tallied(BTreeMap<i64, bool>),
}

/// A map keyed by a `u64` in each place a member holds one.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WideUnsignedKeyed {
    pub direct: HashMap<u64, bool>,
    pub listed: Vec<HashMap<u64, bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<HashMap<u64, bool>>,
    pub nested: HashMap<String, HashMap<u64, bool>>,
}
