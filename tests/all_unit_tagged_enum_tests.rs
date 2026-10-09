//! Tests for an enum whose variants are *all* unit variants under a serde tagging attribute — the
//! canonical shape of a service's error code. Serde keeps the tag key for such an enum exactly as
//! it does when a variant carries a field, so every describing surface has to keep it too rather
//! than falling back to the string union such an enum publishes when no tagging key is named at
//! all.

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "all_unit_tagged_enum_tests/tests.rs"]
mod tests;
