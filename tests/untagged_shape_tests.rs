//! An untagged enum chooses its variant by shape alone, as serde's derive does: a `literal` or the
//! bound of a brand a member holds chooses nothing, and the enum's read judges the value once the
//! variant is chosen.

#[cfg(test)]
#[path = "untagged_shape_tests/tests.rs"]
mod tests;
