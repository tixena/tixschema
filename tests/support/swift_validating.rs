//! The `validate()` a Swift draft declares.

/// The `validate()` a draft declares, which makes the valid type `valid` or throws what the draft
/// lists.
pub fn validating(valid: &str) -> String {
    format!(
        "public func validate() throws -> {valid} {{ let found = validationIssues(); if \
         !found.isEmpty {{ throw {valid}Refusal(issues: found) }}; return {valid}(draft: self) }}"
    )
}
