//! Tests for the description an item falls back to when it carries no doc comment — the `JSDoc`
//! header a `TypeScript` definition opens with and the `description` a Zod schema publishes.

#[cfg(any(feature = "typescript", feature = "zod"))]
#[cfg(test)]
#[path = "item_description_tests/tests.rs"]
mod tests;
