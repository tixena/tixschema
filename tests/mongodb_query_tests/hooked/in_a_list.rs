//! A `with` module of the hooked cases' own: a value stored as the one item of a list, which
//! tells what its hook writes from what its own `Serialize` does.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub(super) fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    <[T; 1]>::deserialize(deserializer).map(|[held]| held)
}

pub(super) fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    [value].serialize(serializer)
}
