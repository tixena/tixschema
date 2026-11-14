//! The `validate()` of the draft of a type that declares no rule.

use super::swift_valid::validating;

/// [`validating`], and the `valid` a draft declares where no rule judges a value of it.
pub fn validating_free(valid: &str) -> String {
    format!(
        "{}; public var valid: {valid} {{ {valid}(draft: self) }}",
        validating(valid)
    )
}
