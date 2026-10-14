//! What rustc prints for a crate tixschema refuses: a support library not written, an `enable`
//! list it cannot read, an initializer written twice or handed an argument, and a type that lists
//! "mongodb" holding one that does not. Each case in `foundation_refusals/` is a crate of its own,
//! held to the `.stderr` file beside it.

// Every `.stderr` was written with every feature on, the build the gate runs.
#![cfg(all(
    feature = "serde",
    feature = "zod",
    feature = "jsonschema",
    feature = "typescript",
    feature = "dart",
    feature = "swift",
    feature = "kotlin",
    feature = "mongodb",
    feature = "chrono",
    feature = "indexmap"
))]

#[cfg(test)]
mod tests {
    use trybuild::TestCases;

    #[test]
    fn each_case_fails_with_the_errors_written_beside_it() {
        TestCases::new().compile_fail("tests/foundation_refusals/*.rs");
    }
}
