//! The pair of structs the Swift emitter writes for a model type.

/// The draft of `swift`, the text a model type writes up to its valid type.
pub fn draft_of(swift: &str) -> &str {
    swift
        .split_once("\n\n@dynamicMemberLookup")
        .map_or(swift, |(draft, _)| draft.trim_end())
}

/// The `validate()` a draft declares, which makes the valid type `valid` or throws what the draft
/// lists.
pub fn validating(valid: &str) -> String {
    format!(
        "public func validate() throws -> {valid} {{ let found = validationIssues(); if \
         !found.isEmpty {{ throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         [], debugDescription: found.joined(separator: \"; \"))) }}; return {valid}(draft: \
         self) }}"
    )
}
