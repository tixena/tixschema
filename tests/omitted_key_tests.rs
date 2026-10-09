//! A field whose key serde leaves out of the output has an optional *key*, and every surface has
//! to say so.

#[cfg(test)]
#[cfg(any(
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema",
    feature = "dart"
))]
#[path = "omitted_key_tests/tests.rs"]
mod tests;
