//! What rustc prints for an enum a member of whose variant writes a `#[serde(default)]` no target
//! but Rust reads as serde does. Each case in `serde_default_refusals/` is a crate of its own, held
//! to the `.stderr` file beside it.

// Every `.stderr` was written with every feature on but one date feature, as the gate builds.
#![cfg(all(
    feature = "zod",
    feature = "typescript",
    feature = "dart",
    feature = "swift",
    feature = "kotlin",
    feature = "mongodb",
    any(feature = "chrono", feature = "jiff"),
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
