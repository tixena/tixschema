//! What rustc prints for each use of a valid constant, or of a `From`, that the macro does not
//! write. Each case in `valid_constant_refusals/` is a crate of its own, held to the `.stderr` file
//! beside it.

// Every `.stderr` was written with every feature on, the build the gate runs.
#![cfg(all(
    feature = "serde",
    feature = "zod",
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
        TestCases::new().compile_fail("tests/valid_constant_refusals/*.rs");
    }
}
