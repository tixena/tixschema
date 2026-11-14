//! What rustc prints for a struct that declares its own `new` without `new = false`. Each case in
//! `generated_new_refusals/` is a crate of its own, held to the `.stderr` file beside it.

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
        TestCases::new().compile_fail("tests/generated_new_refusals/*.rs");
    }
}
