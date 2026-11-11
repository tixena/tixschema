//! What rustc prints for a `minimum` or `maximum` no surface compares alike: a fraction on an integer
//! field, and a number beyond what a float field's type holds. Each case in
//! `bound_representable_refusals/` is a crate of its own, held to the `.stderr` file beside it.

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
        TestCases::new().compile_fail("tests/bound_representable_refusals/*.rs");
    }
}
