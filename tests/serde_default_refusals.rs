//! What rustc prints for an enum a member of whose variant writes a `#[serde(default)]` no target
//! but Rust reads as serde does. Each case in `serde_default_refusals/` is a crate of its own, held
//! to the `.stderr` file beside it.

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
        TestCases::new().compile_fail("tests/serde_default_refusals/*.rs");
    }
}
