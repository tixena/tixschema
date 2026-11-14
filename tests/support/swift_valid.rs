//! The pair of structs the Swift emitter writes for a model type.

/// What closes the refusal struct a model type's text opens with.
const REFUSAL_CLOSES: &str = "(issues.joined(separator: \", \"))]\" } }; ";

/// The draft of `swift`, the text a model type writes after its refusal and up to its valid type.
pub fn draft_of(swift: &str) -> &str {
    let after_the_refusal = swift
        .split_once(REFUSAL_CLOSES)
        .filter(|(refusal, _)| refusal.contains(": Error, CustomStringConvertible"))
        .map_or(swift, |(_, rest)| rest);
    after_the_refusal
        .split_once("\n\n@dynamicMemberLookup")
        .map_or(after_the_refusal, |(draft, _)| draft.trim_end())
}

/// The `validate()` a draft declares, which makes the valid type `valid` or throws what the draft
/// lists.
pub fn validating(valid: &str) -> String {
    format!(
        "public func validate() throws -> {valid} {{ let found = validationIssues(); if \
         !found.isEmpty {{ throw {valid}Refusal(issues: found) }}; return {valid}(draft: self) }}"
    )
}
