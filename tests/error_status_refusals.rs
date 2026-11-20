//! What rustc prints for an `error_status` that names no status for a variant of its fault, or a name no variant has. Each case in
//! `error_status_refusals/` is a crate of its own, held to the `.stderr` file beside it.

// Every `.stderr` was written with every feature on, the build the gate runs.
#![cfg(all(
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
        TestCases::new().compile_fail("tests/error_status_refusals/*.rs");
    }
}
